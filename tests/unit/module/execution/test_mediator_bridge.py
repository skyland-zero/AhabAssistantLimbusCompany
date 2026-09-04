from __future__ import annotations

from threading import Event
from types import SimpleNamespace

from core.events import Event as MediatorEvent
from module.execution.event_adapter import RunnerEventAdapter
from module.execution.mediator_bridge import MediatorBridge


def _mediator() -> SimpleNamespace:
    return SimpleNamespace(
        task_started=MediatorEvent("task_started"),
        task_completed=MediatorEvent("task_completed"),
        mirror_signal=MediatorEvent("mirror_signal"),
        mirror_floor_signal=MediatorEvent("mirror_floor_signal"),
        warning=MediatorEvent("warning"),
        hdr_warning=MediatorEvent("hdr_warning"),
        request_focus=MediatorEvent("request_focus"),
    )


def test_mediator_bridge_maps_events_and_acknowledges_hdr_after_enqueue() -> None:
    mediator = _mediator()
    received: list[tuple[str, dict, bytes]] = []

    def sink(message_type: str, payload: bytes = b"", **fields: object) -> None:
        received.append((message_type, dict(fields), bytes(payload)))

    bridge = MediatorBridge(
        sink,
        mediator=mediator,
        run_id="run-1",
        config={"hard_mirror": True, "infinite_dungeons": True},
    )
    bridge.start()
    acknowledgement = Event()

    mediator.task_started.emit("mirror")
    mediator.task_completed.emit("mirror", 2, {"duration": 3})
    mediator.mirror_signal.emit(4, 9)
    mediator.mirror_floor_signal.emit(3, 5)
    mediator.warning.emit("warning text")
    mediator.hdr_warning.emit(acknowledgement)
    mediator.request_focus.emit()

    assert [item[0] for item in received] == [
        "task.started",
        "task.completed",
        "mirror.progress",
        "mirror.floor",
        "warning",
        "hdr.warning",
        "app.focusRequested",
    ]
    assert received[0][1]["runId"] == "run-1"
    assert received[1][1]["result"] == {"count": 2, "details": {"duration": 3}}
    assert received[2][1]["completed"] == 4
    assert received[2][1]["isHard"] is True
    assert received[3][1]["floorTotal"] == 5
    assert received[5][1]["code"] == "HDR_WARNING"
    assert acknowledgement.is_set()

    bridge.close()
    mediator.warning.emit("must not be forwarded")
    assert len(received) == 7


def test_hdr_ack_is_not_set_when_event_sink_rejects_message() -> None:
    mediator = _mediator()
    acknowledgement = Event()

    def reject(message_type: str, **fields: object) -> bool:
        del message_type, fields
        return False

    bridge = MediatorBridge(reject, mediator=mediator)
    bridge.start()
    assert bridge.on_hdr_warning(acknowledgement) is False
    assert not acknowledgement.is_set()


def test_backend_event_adapter_preserves_current_backend_payload_names() -> None:
    received: list[tuple[str, dict]] = []
    adapter = RunnerEventAdapter(lambda name, payload: received.append((name, dict(payload))), run_id="run-2")
    adapter(
        {
            "type": "mirror.progress",
            "protocol": 1,
            "runId": "run-2",
            "seq": 4,
            "binaryLength": 0,
            "completed": 2,
            "total": 8,
        }
    )
    adapter(
        {
            "type": "task.completed",
            "protocol": 1,
            "runId": "run-2",
            "seq": 5,
            "binaryLength": 0,
            "taskId": "mirror",
            "result": {"count": 1, "details": {"floor": 3}},
        }
    )
    adapter(
        {
            "type": "hdr.warning",
            "protocol": 1,
            "runId": "run-2",
            "seq": 6,
            "binaryLength": 0,
            "message": "HDR",
        }
    )

    assert received[0] == (
        "execution.mirrorProgress",
        {"runId": "run-2", "completed": 2, "total": 8, "current": 2},
    )
    assert received[1][0] == "execution.taskCompleted"
    assert received[1][1]["kind"] == "mirror"
    assert received[1][1]["count"] == 1
    assert received[2] == (
        "app.notice",
        {"runId": "run-2", "message": "HDR", "level": "warn"},
    )
