"""Persistent, process-safe execution counters for the GPUI console.

The store deliberately lives outside the task configuration.  It records
successful dungeon completions in game-day buckets so the UI can display the
current run, today's totals, this week's totals, and a compact daily history.
"""

from __future__ import annotations

import json
import math
import threading
from copy import deepcopy
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any, Callable, Mapping
from zoneinfo import ZoneInfo

from core.atomic_write import atomic_write_text

STAT_KINDS = ("exp", "thread", "mirror")
SCHEMA_VERSION = 1
RETENTION_DAYS = 56
DEFAULT_DAILY_DAYS = 30
MIRROR_HISTORY_LIMIT = 30
GAME_TIMEZONE = ZoneInfo("Asia/Seoul")
GAME_DAY_RESET_HOUR = 6


def _zero_counts() -> dict[str, int]:
    return {kind: 0 for kind in STAT_KINDS}


def _positive_int(value: Any) -> int:
    if isinstance(value, bool):
        return 0
    try:
        return max(0, int(value))
    except (TypeError, ValueError):
        return 0


def _normalise_counts(value: Any) -> dict[str, int]:
    if not isinstance(value, Mapping):
        return _zero_counts()
    return {kind: _positive_int(value.get(kind, 0)) for kind in STAT_KINDS}


def _non_negative_float(value: Any) -> float:
    if isinstance(value, bool):
        return 0.0
    try:
        number = float(value)
    except (TypeError, ValueError):
        return 0.0
    if not math.isfinite(number):
        return 0.0
    return max(0.0, number)


def _normalise_mirror_details(value: Any) -> dict[str, Any] | None:
    if not isinstance(value, Mapping):
        return None
    completed_at = value.get("completedAt")
    if not isinstance(completed_at, str) or not completed_at.strip():
        return None

    details: dict[str, Any] = {
        "completedAt": completed_at.strip(),
        "totalSeconds": _non_negative_float(value.get("totalSeconds")),
        "battleSeconds": _non_negative_float(value.get("battleSeconds")),
        "eventSeconds": _non_negative_float(value.get("eventSeconds")),
        "shopSeconds": _non_negative_float(value.get("shopSeconds")),
        "findRoadSeconds": _non_negative_float(value.get("findRoadSeconds")),
        "themePackSeconds": _non_negative_float(value.get("themePackSeconds")),
        "rewardCardSeconds": _non_negative_float(value.get("rewardCardSeconds")),
        "egoGiftSeconds": _non_negative_float(value.get("egoGiftSeconds")),
        "settlementSeconds": _non_negative_float(value.get("settlementSeconds")),
        "otherSeconds": _non_negative_float(value.get("otherSeconds")),
        "eventCount": _positive_int(value.get("eventCount")),
    }
    run_id = value.get("runId")
    if isinstance(run_id, str) and run_id.strip():
        details["runId"] = run_id.strip()
    failed = value.get("failed")
    if isinstance(failed, bool):
        details["failed"] = failed
    failure_reason = value.get("failureReason")
    if isinstance(failure_reason, str) and failure_reason.strip():
        details["failureReason"] = failure_reason.strip()

    team = _normalise_mirror_team(value.get("team"))
    if team is None:
        # Accept the flat shape used by early development builds so a record
        # can still be shown after a sidecar update.
        flat_team = {
            key: value.get(key)
            for key in (
                "id",
                "teamId",
                "name",
                "teamName",
                "number",
                "teamNumber",
                "sinners",
                "sinnerNames",
                "sinnerNamesEn",
                "system",
                "teamSystem",
                "accessoryScheme",
            )
            if key in value
        }
        team = _normalise_mirror_team(flat_team)
    if team is not None:
        details["team"] = team

    for key in ("mode", "routeId", "routeName", "routeNameEn"):
        item = value.get(key)
        if isinstance(item, str) and item.strip():
            details[key] = item.strip()
    hard_mode = value.get("hardMode")
    if isinstance(hard_mode, bool):
        details["hardMode"] = hard_mode
    elif isinstance(details.get("mode"), str):
        details["hardMode"] = details["mode"].casefold() in {"hard", "困难", "hard_mirror"}
    floor_count = value.get("floorCount")
    if floor_count is not None:
        details["floorCount"] = _positive_int(floor_count)
    return details


def _normalise_mirror_team(value: Any) -> dict[str, Any] | None:
    if not isinstance(value, Mapping):
        return None

    team: dict[str, Any] = {}
    for output_key, input_keys in (
        ("id", ("id", "teamId")),
        ("name", ("name", "teamName")),
        ("system", ("system", "teamSystem", "accessoryScheme")),
        ("accessoryScheme", ("accessoryScheme", "system", "teamSystem")),
    ):
        item = next((value.get(key) for key in input_keys if isinstance(value.get(key), str)), None)
        if isinstance(item, str) and item.strip():
            team[output_key] = item.strip()

    number = value.get("number", value.get("teamNumber"))
    if number is not None:
        number = _positive_int(number)
        if number > 0:
            team["number"] = number

    for output_key in ("sinners", "sinnerNames", "sinnerNamesEn"):
        items = value.get(output_key)
        if isinstance(items, (list, tuple)):
            normalized = [item.strip() for item in items if isinstance(item, str) and item.strip()]
            if normalized:
                team[output_key] = normalized[:12]

    return team or None


def game_day(now: datetime | None = None) -> date:
    """Return the game's date, which rolls over at 06:00 Seoul time."""

    current = now or datetime.now(GAME_TIMEZONE)
    if current.tzinfo is None:
        current = current.replace(tzinfo=GAME_TIMEZONE)
    else:
        current = current.astimezone(GAME_TIMEZONE)
    if current.hour < GAME_DAY_RESET_HOUR:
        current -= timedelta(days=1)
    return current.date()


class ExecutionStatsStore:
    """Thread-safe store for execution statistics.

    ``now`` is injectable for deterministic boundary tests.  Passing
    ``path=None`` keeps the store in memory, which is useful for isolated
    protocol tests and never creates files in the repository.
    """

    def __init__(
        self,
        path: str | Path | None = None,
        *,
        now: Callable[[], datetime] | None = None,
    ) -> None:
        self.path = Path(path) if path is not None else None
        self._now = now or (lambda: datetime.now(GAME_TIMEZONE))
        self._lock = threading.RLock()
        self._daily: dict[str, dict[str, int]] = {}
        self._mirror_history: list[dict[str, Any]] = []
        self._current: dict[str, Any] = self._empty_current()
        self._load()

    @staticmethod
    def _empty_current() -> dict[str, Any]:
        return {
            "runId": None,
            "state": "idle",
            "currentTaskId": None,
            "startedAt": None,
            "targets": _zero_counts(),
            "completed": _zero_counts(),
            "isMirrorInfinite": False,
            "updatedAt": None,
        }

    def start_run(
        self,
        run_id: str,
        targets: Mapping[str, Any],
        *,
        current_task_id: str | None = None,
    ) -> dict[str, Any]:
        with self._lock:
            self._prune(game_day(self._now()))
            target_counts = _zero_counts()
            for kind in STAT_KINDS:
                target_counts[kind] = _positive_int(targets.get(kind, 0))
            self._current = {
                "runId": run_id,
                "state": "running",
                "currentTaskId": current_task_id,
                "startedAt": self._timestamp(),
                "targets": target_counts,
                "completed": _zero_counts(),
                "isMirrorInfinite": bool(targets.get("mirrorInfinite", False)),
                "updatedAt": self._timestamp(),
            }
            return self.snapshot()

    def set_state(self, state: str) -> dict[str, Any]:
        with self._lock:
            if self._current["runId"] is not None:
                self._current["state"] = state
                self._current["updatedAt"] = self._timestamp()
            return self.snapshot()

    def set_current_task(
        self,
        task_id: str,
        *,
        run_id: str | None = None,
    ) -> dict[str, Any] | None:
        with self._lock:
            if self._current["runId"] is None:
                return None
            if self._current["state"] != "running":
                return None
            if run_id is not None and self._current["runId"] != run_id:
                return None
            self._current["currentTaskId"] = task_id
            self._current["updatedAt"] = self._timestamp()
            return self.snapshot()

    def finish_run(self, run_id: str | None) -> dict[str, Any]:
        with self._lock:
            if run_id is not None and self._current["runId"] == run_id:
                self._current["state"] = "idle"
                self._current["currentTaskId"] = None
                self._current["updatedAt"] = self._timestamp()
            return self.snapshot()

    def record_completion(
        self,
        kind: str,
        count: int = 1,
        *,
        run_id: str | None = None,
        details: Mapping[str, Any] | None = None,
    ) -> dict[str, Any] | None:
        if kind not in STAT_KINDS:
            return None
        amount = _positive_int(count)
        if amount <= 0:
            return None
        with self._lock:
            if run_id is not None and self._current["runId"] != run_id:
                return None
            self._current["completed"][kind] += amount
            today = game_day(self._now()).isoformat()
            bucket = self._daily.setdefault(today, _zero_counts())
            bucket[kind] += amount
            if kind == "mirror" and details is not None:
                mirror_details = _normalise_mirror_details(details)
                if mirror_details is not None:
                    if run_id is not None:
                        mirror_details["runId"] = run_id
                    # Newest records are returned first.  Keeping the list
                    # bounded here also bounds the persisted JSON payload.
                    self._mirror_history = [
                        record for record in self._mirror_history if record != mirror_details
                    ]
                    self._mirror_history.insert(0, mirror_details)
                    self._mirror_history = self._mirror_history[:MIRROR_HISTORY_LIMIT]
            self._prune(game_day(self._now()))
            self._current["updatedAt"] = self._timestamp()
            self._save()
            return self.snapshot()

    def summary(self) -> dict[str, Any]:
        with self._lock:
            current = game_day(self._now())
            self._prune(current)
            today_counts = self._daily.get(current.isoformat(), _zero_counts())
            week_start = current - timedelta(days=current.weekday())
            week_counts = _zero_counts()
            for offset in range(7):
                bucket = self._daily.get((week_start + timedelta(days=offset)).isoformat())
                if bucket:
                    for kind in STAT_KINDS:
                        week_counts[kind] += bucket[kind]
            return {
                "schemaVersion": SCHEMA_VERSION,
                "currentRun": self._copy_current(),
                "lastMirror": self._copy_last_mirror(),
                "mirrorHistory": self._copy_mirror_history(),
                "today": dict(today_counts),
                "week": week_counts,
                "updatedAt": self._timestamp(),
            }

    def daily_summary(
        self,
        *,
        date_from: str | None = None,
        date_to: str | None = None,
    ) -> dict[str, Any]:
        with self._lock:
            current = game_day(self._now())
            self._prune(current)
            end = self._parse_date(date_to) if date_to else current
            start = self._parse_date(date_from) if date_from else end - timedelta(days=DEFAULT_DAILY_DAYS - 1)
            if start > end:
                raise ValueError("dateFrom must not be later than dateTo")
            if (end - start).days >= 366:
                raise ValueError("daily summary range is limited to 366 days")
            days = []
            cursor = end
            while cursor >= start:
                counts = self._daily.get(cursor.isoformat(), _zero_counts())
                days.append({"date": cursor.isoformat(), **counts, "total": sum(counts.values())})
                cursor -= timedelta(days=1)
            return {
                "schemaVersion": SCHEMA_VERSION,
                "dateFrom": start.isoformat(),
                "dateTo": end.isoformat(),
                "days": days,
                "updatedAt": self._timestamp(),
            }

    def snapshot(self) -> dict[str, Any]:
        return {
            "schemaVersion": SCHEMA_VERSION,
            "currentRun": self._copy_current(),
            "lastMirror": self._copy_last_mirror(),
            "mirrorHistory": self._copy_mirror_history(),
            "today": dict(self._daily.get(game_day(self._now()).isoformat(), _zero_counts())),
            "week": self._week_counts(game_day(self._now())),
            "updatedAt": self._timestamp(),
        }

    def _week_counts(self, current: date) -> dict[str, int]:
        week_start = current - timedelta(days=current.weekday())
        counts = _zero_counts()
        for offset in range(7):
            bucket = self._daily.get((week_start + timedelta(days=offset)).isoformat())
            if bucket:
                for kind in STAT_KINDS:
                    counts[kind] += bucket[kind]
        return counts

    def _copy_current(self) -> dict[str, Any]:
        return {
            **self._current,
            "targets": dict(self._current["targets"]),
            "completed": dict(self._current["completed"]),
        }

    def _copy_last_mirror(self) -> dict[str, Any] | None:
        return deepcopy(self._mirror_history[0]) if self._mirror_history else None

    def _copy_mirror_history(self) -> list[dict[str, Any]]:
        return deepcopy(self._mirror_history[:MIRROR_HISTORY_LIMIT])

    def _timestamp(self) -> int:
        current = self._now()
        if current.tzinfo is None:
            current = current.replace(tzinfo=GAME_TIMEZONE)
        return int(current.timestamp() * 1000)

    @staticmethod
    def _parse_date(value: str) -> date:
        try:
            return date.fromisoformat(value)
        except (TypeError, ValueError) as error:
            raise ValueError("dates must use YYYY-MM-DD format") from error

    def _prune(self, current: date) -> None:
        cutoff = current - timedelta(days=RETENTION_DAYS - 1)
        self._daily = {
            key: _normalise_counts(value)
            for key, value in self._daily.items()
            if self._valid_date_key(key) and self._parse_date(key) >= cutoff
        }
        self._mirror_history = self._mirror_history[:MIRROR_HISTORY_LIMIT]

    @staticmethod
    def _valid_date_key(value: Any) -> bool:
        if not isinstance(value, str):
            return False
        try:
            date.fromisoformat(value)
        except ValueError:
            return False
        return True

    def _load(self) -> None:
        if self.path is None or not self.path.exists():
            return
        try:
            raw = json.loads(self.path.read_text(encoding="utf-8"))
            if not isinstance(raw, Mapping) or raw.get("schemaVersion") != SCHEMA_VERSION:
                return
            daily = raw.get("daily", {})
            if isinstance(daily, Mapping):
                self._daily = {
                    str(key): _normalise_counts(value) for key, value in daily.items() if self._valid_date_key(key)
                }
            history: list[dict[str, Any]] = []
            raw_history = raw.get("mirrorHistory", [])
            if isinstance(raw_history, list):
                for item in raw_history:
                    normalized = _normalise_mirror_details(item)
                    if normalized is not None and normalized not in history:
                        history.append(normalized)
            # Files written before mirrorHistory was introduced only have a
            # single lastMirror entry.  Promote it without duplicating it
            # when both fields are present.
            legacy_last = _normalise_mirror_details(raw.get("lastMirror"))
            if legacy_last is not None and legacy_last not in history:
                history.insert(0, legacy_last)
            self._mirror_history = history[:MIRROR_HISTORY_LIMIT]
            self._prune(game_day(self._now()))
        except (OSError, ValueError, TypeError, json.JSONDecodeError):
            # A broken history file must never prevent the sidecar from
            # starting.  The next successful completion will repair it.
            self._daily = {}
            self._mirror_history = []

    def _save(self) -> None:
        if self.path is None:
            return
        try:
            payload = {
                "schemaVersion": SCHEMA_VERSION,
                "daily": self._daily,
                "lastMirror": self._copy_last_mirror(),
                "mirrorHistory": self._copy_mirror_history(),
            }
            atomic_write_text(
                self.path,
                json.dumps(payload, ensure_ascii=False, separators=(",", ":")),
            )
        except (OSError, UnicodeError, TypeError, ValueError):
            # Statistics are observability data; never fail an automation run
            # because its optional history file cannot be written.
            return


__all__ = [
    "DEFAULT_DAILY_DAYS",
    "ExecutionStatsStore",
    "GAME_DAY_RESET_HOUR",
    "GAME_TIMEZONE",
    "MIRROR_HISTORY_LIMIT",
    "RETENTION_DAYS",
    "SCHEMA_VERSION",
    "STAT_KINDS",
    "game_day",
]
