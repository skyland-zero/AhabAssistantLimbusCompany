import importlib.util
from pathlib import Path

_SCRIPT = Path(__file__).resolve().parents[2] / "scripts" / "analyze_perf_log.py"
_SPEC = importlib.util.spec_from_file_location("analyze_perf_log", _SCRIPT)
assert _SPEC is not None and _SPEC.loader is not None
_ANALYZER = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_ANALYZER)


def test_analyzer_summarizes_tasks_and_targets() -> None:
    records = [
        {"run": "run-1", "ts": 1.0, "event": "run_start", "thread": "MainThread"},
        {
            "run": "run-1",
            "ts": 2.0,
            "event": "task_end",
            "name": "镜牢商店",
            "duration_ms": 250.0,
            "thread": "MainThread",
        },
        {
            "run": "run-1",
            "ts": 3.0,
            "event": "match",
            "target": "demo.png",
            "duration_ms": 10.0,
            "caller": "a.py:1",
            "thread": "MainThread",
        },
        {
            "run": "run-1",
            "ts": 4.0,
            "event": "match",
            "target": "demo.png",
            "duration_ms": 20.0,
            "caller": "a.py:2",
            "thread": "MainThread",
        },
        {
            "run": "run-1",
            "ts": 5.0,
            "event": "sleep",
            "duration_ms": 30.0,
            "caller": "b.py:1",
            "thread": "MainThread",
        },
        {
            "run": "run-1",
            "ts": 6.0,
            "event": "screenshot",
            "duration_ms": 2.0,
            "gap_ms": 100.0,
            "caller": "loop",
            "thread": "MainThread",
        },
        {"run": "run-1", "ts": 7.0, "event": "run_end", "thread": "MainThread"},
    ]

    report = _ANALYZER.summarize(records, top=10)[0]

    assert report["run"] == "run-1"
    assert report["tasks"][0]["name"] == "镜牢商店"
    assert report["tasks"][0]["total_ms"] == 250.0
    assert report["match_targets"][0]["target"] == "demo.png"
    assert report["match_targets"][0]["calls"] == 2
    assert report["match_targets"][0]["total_ms"] == 30.0
    assert report["sleep_callers"][0][0] == "b.py:1"
    assert report["screenshot_callers"][0][0] == "loop"


def test_analyzer_handles_empty_input() -> None:
    assert _ANALYZER.summarize([]) == []
