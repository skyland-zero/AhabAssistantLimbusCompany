import json

from module.perf_log import PerfLog


def _read_records(path):
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]


def test_perf_log_writes_jsonl_with_task_and_frame_context(tmp_path) -> None:
    path = tmp_path / "perf.log"
    logger = PerfLog(path, enabled=True, run_id="test-run")

    with logger.task("demo"):
        logger.set_frame(7)
        logger.match(target="demo.png", duration_ms=1.25, matched=True)
        logger.ocr(region=[0, 0, 10, 10], duration_ms=2.5, cache_hit=False)
    logger.close()

    records = _read_records(path)
    events = [record["event"] for record in records]
    assert events[0] == "run_start"
    assert "task_start" in events
    assert "match" in events
    assert "ocr" in events
    assert "task_end" in events
    assert events[-1] == "run_end"

    match = next(record for record in records if record["event"] == "match")
    assert match["run"] == "test-run"
    assert match["task"] == "demo"
    assert match["frame"] == 7
    assert match["target"] == "demo.png"
    assert match["duration_ms"] == 1.25


def test_perf_log_disabled_writes_nothing(tmp_path) -> None:
    path = tmp_path / "perf.log"
    logger = PerfLog(path, enabled=False, run_id="disabled")
    logger.match(target="demo.png", duration_ms=1.0)
    logger.close()
    assert not path.exists()
