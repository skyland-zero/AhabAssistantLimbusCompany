from __future__ import annotations

import json
from datetime import datetime
from zoneinfo import ZoneInfo

from module.execution_stats import MIRROR_HISTORY_LIMIT, ExecutionStatsStore, game_day

SEOUL = ZoneInfo("Asia/Seoul")


def mirror_details() -> dict[str, object]:
    return {
        "completedAt": "2026-08-31T08:00:00+09:00",
        "totalSeconds": 1800.5,
        "battleSeconds": 1200.25,
        "eventSeconds": 180.0,
        "shopSeconds": 90.75,
        "findRoadSeconds": 329.5,
        "themePackSeconds": 0.0,
        "rewardCardSeconds": 0.0,
        "egoGiftSeconds": 0.0,
        "settlementSeconds": 0.0,
        "otherSeconds": 0.0,
        "eventCount": 4,
    }


def test_game_day_rolls_over_at_six_am_seoul() -> None:
    assert game_day(datetime(2026, 8, 29, 5, 59, tzinfo=SEOUL)).isoformat() == "2026-08-28"
    assert game_day(datetime(2026, 8, 29, 6, 0, tzinfo=SEOUL)).isoformat() == "2026-08-29"


def test_execution_stats_accumulate_periods_and_persist(tmp_path) -> None:
    current = [datetime(2026, 8, 31, 7, 0, tzinfo=SEOUL)]
    path = tmp_path / "runtime_stats.json"
    store = ExecutionStatsStore(path, now=lambda: current[0])

    store.start_run("run-1", {"exp": 3, "thread": 1, "mirror": 2})
    store.record_completion("exp", 2, run_id="run-1")
    store.record_completion("mirror", 1, run_id="run-1")

    summary = store.summary()
    assert summary["currentRun"]["completed"] == {"exp": 2, "thread": 0, "mirror": 1}
    assert summary["today"] == {"exp": 2, "thread": 0, "mirror": 1}
    assert summary["week"] == {"exp": 2, "thread": 0, "mirror": 1}

    current[0] = datetime(2026, 9, 1, 7, 0, tzinfo=SEOUL)
    store.record_completion("thread", 1, run_id="run-1")
    days = store.daily_summary(date_from="2026-08-31", date_to="2026-09-01")["days"]
    assert days[0] == {
        "date": "2026-09-01",
        "exp": 0,
        "thread": 1,
        "mirror": 0,
        "total": 1,
    }
    assert days[1]["date"] == "2026-08-31"
    assert days[1]["total"] == 3

    reloaded = ExecutionStatsStore(path, now=lambda: current[0])
    assert reloaded.summary()["week"] == {"exp": 2, "thread": 1, "mirror": 1}


def test_completion_from_another_run_is_ignored(tmp_path) -> None:
    current = [datetime(2026, 8, 29, 8, 0, tzinfo=SEOUL)]
    store = ExecutionStatsStore(tmp_path / "runtime_stats.json", now=lambda: current[0])
    store.start_run("run-1", {"exp": 1})

    assert store.record_completion("exp", 1, run_id="run-2") is None
    assert store.summary()["today"] == {"exp": 0, "thread": 0, "mirror": 0}


def test_last_mirror_details_are_normalised_and_persisted(tmp_path) -> None:
    current = [datetime(2026, 8, 31, 8, 0, tzinfo=SEOUL)]
    path = tmp_path / "runtime_stats.json"
    store = ExecutionStatsStore(path, now=lambda: current[0])
    store.start_run("run-1", {"mirror": 1})

    payload = store.record_completion(
        "mirror",
        1,
        run_id="run-1",
        details=mirror_details(),
    )

    expected = {**mirror_details(), "runId": "run-1"}
    assert payload is not None
    assert payload["lastMirror"] == expected
    assert payload["mirrorHistory"] == [expected]
    assert store.summary()["lastMirror"] == expected
    assert store.summary()["mirrorHistory"] == [expected]

    reloaded = ExecutionStatsStore(path, now=lambda: current[0])
    assert reloaded.summary()["lastMirror"] == expected
    assert reloaded.summary()["mirrorHistory"] == [expected]


def test_mirror_history_keeps_latest_thirty_records_and_promotes_legacy_last(tmp_path) -> None:
    current = [datetime(2026, 8, 31, 8, 0, tzinfo=SEOUL)]
    path = tmp_path / "runtime_stats.json"
    store = ExecutionStatsStore(path, now=lambda: current[0])
    store.start_run("run-1", {"mirror": 1})

    for index in range(MIRROR_HISTORY_LIMIT + 5):
        details = mirror_details()
        details["completedAt"] = f"2026-08-31T08:{index:02}:00+09:00"
        details["team"] = {"name": f"队伍 {index}", "number": index + 1, "sinners": ["faust"]}
        store.record_completion("mirror", 1, run_id="run-1", details=details)

    history = store.summary()["mirrorHistory"]
    assert len(history) == MIRROR_HISTORY_LIMIT
    assert history[0]["completedAt"] == "2026-08-31T08:34:00+09:00"
    assert history[-1]["completedAt"] == "2026-08-31T08:05:00+09:00"
    assert store.summary()["lastMirror"] == history[0]

    reloaded = ExecutionStatsStore(path, now=lambda: current[0])
    assert reloaded.summary()["mirrorHistory"] == history


def test_legacy_last_mirror_is_available_as_first_history_record(tmp_path) -> None:
    legacy = mirror_details()
    path = tmp_path / "runtime_stats.json"
    path.write_text(
        '{"schemaVersion":1,"daily":{},"lastMirror":' + json.dumps(legacy) + '}',
        encoding="utf-8",
    )

    store = ExecutionStatsStore(path, now=lambda: datetime(2026, 8, 31, 8, 0, tzinfo=SEOUL))

    assert store.summary()["mirrorHistory"] == [legacy]


def test_old_stats_file_without_last_mirror_remains_readable(tmp_path) -> None:
    path = tmp_path / "runtime_stats.json"
    path.write_text('{"schemaVersion":1,"daily":{}}', encoding="utf-8")

    store = ExecutionStatsStore(path)

    assert store.summary()["lastMirror"] is None


def test_current_task_updates_only_for_the_active_run(tmp_path) -> None:
    store = ExecutionStatsStore(tmp_path / "runtime_stats.json")
    store.start_run("run-1", {"mirror": 1})

    assert store.set_current_task("mirror", run_id="run-2") is None
    assert store.summary()["currentRun"]["currentTaskId"] is None

    payload = store.set_current_task("mirror", run_id="run-1")
    assert payload is not None
    assert payload["currentRun"]["currentTaskId"] == "mirror"

    store.finish_run("run-1")
    assert store.set_current_task("mirror", run_id="run-1") is None
