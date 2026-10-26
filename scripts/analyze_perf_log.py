"""Summarize the independent JSONL performance log.

Usage:
    python scripts/analyze_perf_log.py logs/perfLog.log
    python scripts/analyze_perf_log.py logs/perfLog.log logs/perfLog.log.1 --top 30

The script intentionally uses only the standard library so it can be run
against a log copied from another machine.
"""

from __future__ import annotations

import argparse
import json
import statistics
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


def iter_records(paths: Iterable[str | Path]) -> Iterable[dict[str, Any]]:
    for raw_path in paths:
        path = Path(raw_path)
        if not path.exists():
            print(f"[warn] log file not found: {path}")
            continue
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            for line in handle:
                line = line.strip()
                if not line:
                    continue
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if isinstance(record, dict):
                    yield record


def _duration_ms(record: dict[str, Any]) -> float:
    try:
        return max(0.0, float(record.get("duration_ms") or record.get("elapsed_ms") or 0.0))
    except (TypeError, ValueError):
        return 0.0


def _number(record: dict[str, Any], key: str) -> float:
    try:
        return float(record.get(key) or 0.0)
    except (TypeError, ValueError):
        return 0.0


def _fmt_ms(value: float) -> str:
    if value >= 60_000:
        return f"{value / 60_000:.1f}min"
    if value >= 1_000:
        return f"{value / 1_000:.2f}s"
    return f"{value:.1f}ms"


def _group_by_run(records: Iterable[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        grouped[str(record.get("run") or "unknown")].append(record)
    return grouped


def _task_summary(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    grouped: dict[str, list[float]] = defaultdict(list)
    for record in records:
        if record.get("event") == "task_end":
            grouped[str(record.get("name") or "unknown")].append(_duration_ms(record))
    rows = []
    for name, durations in grouped.items():
        rows.append(
            {
                "name": name,
                "calls": len(durations),
                "total_ms": sum(durations),
                "avg_ms": statistics.fmean(durations) if durations else 0.0,
                "max_ms": max(durations) if durations else 0.0,
            }
        )
    return sorted(rows, key=lambda row: row["total_ms"], reverse=True)


def _top_callers(records: list[dict[str, Any]], event: str, top: int) -> list[tuple[str, int, float]]:
    grouped: dict[str, list[float]] = defaultdict(list)
    for record in records:
        if record.get("event") == event:
            grouped[str(record.get("caller") or "unknown")].append(_duration_ms(record))
    rows = [
        (caller, len(durations), sum(durations))
        for caller, durations in grouped.items()
    ]
    return sorted(rows, key=lambda row: row[2], reverse=True)[:top]


def _frame_gaps(records: list[dict[str, Any]], top: int) -> list[dict[str, Any]]:
    screenshots: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        if record.get("event") == "screenshot":
            screenshots[(str(record.get("run")), str(record.get("thread")))].append(record)

    rows: list[dict[str, Any]] = []
    for (_run, thread), frames in screenshots.items():
        frames.sort(key=lambda item: float(item.get("ts") or 0.0))
        for previous, current in zip(frames, frames[1:]):
            previous_ts = float(previous.get("ts") or 0.0)
            current_ts = float(current.get("ts") or 0.0)
            gap_ms = _number(current, "gap_ms")
            if gap_ms <= 0:
                gap_ms = max(0.0, (current_ts - previous_ts) * 1000.0)
            components = defaultdict(float)
            for record in records:
                if record.get("thread") != thread:
                    continue
                ts = float(record.get("ts") or 0.0)
                if not (previous_ts <= ts <= current_ts):
                    continue
                event = str(record.get("event") or "")
                if event in {"match", "multi_match", "ocr", "sleep", "action", "screenshot"}:
                    components[event] += _duration_ms(record)
            accounted = sum(components.values())
            rows.append(
                {
                    "gap_ms": gap_ms,
                    "unaccounted_ms": max(0.0, gap_ms - accounted),
                    "task": str(current.get("task") or previous.get("task") or ""),
                    "caller": str(current.get("caller") or ""),
                    "thread": thread,
                    "components": dict(components),
                }
            )
    return sorted(rows, key=lambda row: row["unaccounted_ms"], reverse=True)[:top]


def _target_summary(
    records: list[dict[str, Any]],
    event: str,
    top: int,
) -> list[dict[str, Any]]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        if record.get("event") == event:
            grouped[str(record.get("target") or "unknown")].append(record)
    rows = []
    for target, items in grouped.items():
        durations = [_duration_ms(item) for item in items]
        rows.append(
            {
                "target": target,
                "calls": len(items),
                "total_ms": sum(durations),
                "avg_ms": statistics.fmean(durations) if durations else 0.0,
                "max_ms": max(durations) if durations else 0.0,
                "fullscreen": sum(1 for item in items if item.get("fullscreen")),
                "cache_hits": sum(1 for item in items if item.get("from_cache")),
                "hits": sum(1 for item in items if item.get("matched")),
            }
        )
    return sorted(rows, key=lambda row: row["total_ms"], reverse=True)[:top]


def _ocr_summary(records: list[dict[str, Any]], top: int) -> dict[str, Any]:
    ocr_records = [record for record in records if record.get("event") == "ocr"]
    durations = [_duration_ms(record) for record in ocr_records if not record.get("cache_hit")]
    regions: dict[str, list[float]] = defaultdict(list)
    texts: Counter[str] = Counter()
    for record in ocr_records:
        region = record.get("region")
        key = str(tuple(region)) if isinstance(region, list) else "fullscreen"
        regions[key].append(_duration_ms(record))
        for text in record.get("texts") or []:
            texts[str(text)] += 1
    region_rows = [
        {"region": region, "calls": len(values), "total_ms": sum(values)}
        for region, values in regions.items()
    ]
    region_rows.sort(key=lambda row: row["total_ms"], reverse=True)
    return {
        "calls": len(ocr_records),
        "misses": len(durations),
        "cache_hits": len(ocr_records) - len(durations),
        "total_ms": sum(durations),
        "avg_ms": statistics.fmean(durations) if durations else 0.0,
        "max_ms": max(durations) if durations else 0.0,
        "top_regions": region_rows[:top],
        "top_texts": texts.most_common(top),
    }


def summarize(records: list[dict[str, Any]], top: int = 20) -> list[dict[str, Any]]:
    report = []
    for run, run_records in sorted(_group_by_run(records).items()):
        timestamps = [float(record.get("ts") or 0.0) for record in run_records]
        starts = [float(record.get("ts") or 0.0) for record in run_records if record.get("event") == "run_start"]
        ends = [float(record.get("ts") or 0.0) for record in run_records if record.get("event") == "run_end"]
        start_ts = starts[0] if starts else (min(timestamps) if timestamps else 0.0)
        end_ts = ends[-1] if ends else (max(timestamps) if timestamps else 0.0)
        events = Counter(str(record.get("event") or "") for record in run_records)
        report.append(
            {
                "run": run,
                "start_ts": start_ts,
                "end_ts": end_ts,
                "duration_ms": max(0.0, (end_ts - start_ts) * 1000.0),
                "events": events,
                "tasks": _task_summary(run_records),
                "screenshot_callers": _top_callers(run_records, "screenshot", top),
                "match_targets": _target_summary(run_records, "match", top),
                "multi_targets": _target_summary(run_records, "multi_match", top),
                "ocr": _ocr_summary(run_records, top),
                "sleep_callers": _top_callers(run_records, "sleep", top),
                "action_callers": _top_callers(run_records, "action", top),
                "array_callers": _top_callers(run_records, "array", top),
                "frame_gaps": _frame_gaps(run_records, top),
                "theme_packs": [
                    record
                    for record in run_records
                    if record.get("event") == "theme_pack"
                ][-top:],
            }
        )
    return report


def _print_rows(title: str, rows: list[dict[str, Any]], columns: list[tuple[str, str]]) -> None:
    if not rows:
        return
    print(f"\n[{title}]")
    header = "  ".join(label for _key, label in columns)
    print(f"  {header}")
    for row in rows:
        values = []
        for key, _label in columns:
            value = row.get(key, "")
            if key.endswith("_ms"):
                values.append(_fmt_ms(float(value or 0.0)))
            else:
                values.append(str(value))
        print(f"  {'  '.join(values)}")


def print_report(reports: list[dict[str, Any]], top: int = 20) -> None:
    for report in reports:
        print("=" * 96)
        event_count = sum(report["events"].values())
        print(
            f"run={report['run']}  duration={_fmt_ms(report['duration_ms'])}  "
            f"events={event_count}"
        )
        _print_rows(
            "TASKS",
            report["tasks"],
            [("name", "task"), ("calls", "calls"), ("total_ms", "total"), ("avg_ms", "avg"), ("max_ms", "max")],
        )
        _print_rows(
            "SCREENSHOT CALLERS",
            [
                {"caller": caller, "calls": calls, "total_ms": total}
                for caller, calls, total in report["screenshot_callers"]
            ],
            [("caller", "caller"), ("calls", "calls"), ("total_ms", "total")],
        )
        _print_rows(
            "MATCH TARGETS",
            report["match_targets"],
            [
                ("target", "target"),
                ("calls", "calls"),
                ("total_ms", "total"),
                ("avg_ms", "avg"),
                ("fullscreen", "full"),
                ("cache_hits", "cache"),
                ("hits", "hits"),
            ],
        )
        _print_rows(
            "MULTI TARGETS",
            report["multi_targets"],
            [
                ("target", "target"),
                ("calls", "calls"),
                ("total_ms", "total"),
                ("avg_ms", "avg"),
                ("fullscreen", "full"),
                ("cache_hits", "cache"),
            ],
        )
        ocr = report["ocr"]
        print("\n[OCR]")
        print(
            f"  calls={ocr['calls']} misses={ocr['misses']} cache_hits={ocr['cache_hits']} "
            f"total={_fmt_ms(ocr['total_ms'])} avg={_fmt_ms(ocr['avg_ms'])} max={_fmt_ms(ocr['max_ms'])}"
        )
        for region in ocr["top_regions"][: min(10, top)]:
            print(f"  region={region['region']} calls={region['calls']} total={_fmt_ms(region['total_ms'])}")
        if ocr["top_texts"]:
            print(f"  top_texts={ocr['top_texts'][:10]}")
        _print_rows(
            "SLEEP CALLERS",
            [
                {"caller": caller, "calls": calls, "total_ms": total}
                for caller, calls, total in report["sleep_callers"]
            ],
            [("caller", "caller"), ("calls", "calls"), ("total_ms", "total")],
        )
        _print_rows(
            "ACTION CALLERS",
            [
                {"caller": caller, "calls": calls, "total_ms": total}
                for caller, calls, total in report["action_callers"]
            ],
            [("caller", "caller"), ("calls", "calls"), ("total_ms", "total")],
        )
        _print_rows(
            "ARRAY CONVERSION >=0.5ms",
            [
                {"caller": caller, "calls": calls, "total_ms": total}
                for caller, calls, total in report["array_callers"]
            ],
            [("caller", "caller"), ("calls", "calls"), ("total_ms", "total")],
        )
        print("\n[FRAME GAPS / UNACCOUNTED]")
        for row in report["frame_gaps"][: min(15, top)]:
            parts = " ".join(f"{key}={_fmt_ms(value)}" for key, value in sorted(row["components"].items()))
            print(
                f"  gap={_fmt_ms(row['gap_ms'])} unaccounted={_fmt_ms(row['unaccounted_ms'])} "
                f"task={row['task']} caller={row['caller']} {parts}"
            )
        if report["theme_packs"]:
            print("\n[THEME PACKS]")
            for record in report["theme_packs"]:
                print(
                    f"  status={record.get('status')} floor={record.get('floor')} "
                    f"selected={record.get('selected')} elapsed={_fmt_ms(_duration_ms(record))}"
                )
    print("=" * 96)


def main() -> int:
    parser = argparse.ArgumentParser(description="Summarize AALC perfLog JSONL files")
    parser.add_argument("paths", nargs="+", help="perfLog.log and/or rotated .1 files")
    parser.add_argument("--top", type=int, default=20, help="number of rows per section")
    args = parser.parse_args()
    records = list(iter_records(args.paths))
    if not records:
        print("No performance records found.")
        return 1
    print_report(summarize(records, top=args.top), top=args.top)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
