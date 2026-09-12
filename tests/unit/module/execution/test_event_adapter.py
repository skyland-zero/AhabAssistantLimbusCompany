"""Contract tests for the Runner → sidecar event adapter.

The GPUI log pane consumes ``log.entry`` with ``ts`` (epoch milliseconds) and
the canonical ``debug/info/warn/error`` vocabulary.  The Runner emits
``timestamp`` (epoch seconds) and raw ``logging`` level names, so these tests
pin the boundary that prevents Runner logs from being silently dropped.
"""

from __future__ import annotations

from module.execution.event_adapter import RunnerEventAdapter


def _collect() -> tuple[list[tuple[str, dict]], RunnerEventAdapter]:
    received: list[tuple[str, dict]] = []
    adapter = RunnerEventAdapter(
        lambda name, payload: received.append((name, dict(payload))),
        run_id="run-1",
    )
    return received, adapter


def test_runner_log_entry_is_normalised_to_the_gpui_wire_contract() -> None:
    received, adapter = _collect()

    adapter.forward(
        {
            "type": "log.entry",
            "runId": "run-1",
            "seq": 7,
            "timestamp": 1_725_000_000.5,
            "level": "warning",
            "logger": "task.mirror",
            "message": "runner warning",
        }
    )

    assert received == [
        (
            "execution.log",
            {
                "runId": "run-1",
                "level": "warn",
                "logger": "task.mirror",
                "message": "runner warning",
                "ts": 1_725_000_000_500,
            },
        )
    ]


def test_runner_critical_level_maps_to_error() -> None:
    received, adapter = _collect()

    adapter.forward({"type": "log.entry", "timestamp": 1_725_000_000, "level": "critical", "message": "boom"})

    assert received[0][1]["level"] == "error"
    assert received[0][1]["ts"] == 1_725_000_000_000


def test_millisecond_timestamps_are_not_rescaled() -> None:
    received, adapter = _collect()

    adapter.forward({"type": "log.entry", "ts": 1_725_000_000_500, "level": "info", "message": "already ms"})

    assert received[0][1]["ts"] == 1_725_000_000_500


def test_missing_timestamp_is_replaced_with_a_usable_millisecond_value() -> None:
    received, adapter = _collect()

    adapter.forward({"type": "log.entry", "level": "debug", "message": "no timestamp"})

    assert received[0][1]["ts"] > 0


def test_unknown_level_falls_back_to_info() -> None:
    received, adapter = _collect()

    adapter.forward({"type": "log.entry", "timestamp": 1, "level": "not-a-level", "message": "x"})

    assert received[0][1]["level"] == "info"


def test_internal_heartbeat_is_not_forwarded_to_the_wire() -> None:
    received, adapter = _collect()

    assert adapter.forward({"type": "heartbeat", "seq": 9, "monotonic": 12.5}) is None
    assert received == []
