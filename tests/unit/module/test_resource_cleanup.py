from __future__ import annotations

import threading
import time
from types import SimpleNamespace
from unittest.mock import Mock

import pytest

from module.execution.cleanup_ledger import CleanupLedger
from module.resource_cleanup import (
    AdbCleanupBackend,
    CleanupActionExecutor,
    CleanupIdentityError,
    CleanupTimeouts,
    Win32CleanupBackend,
    build_default_cleanup_executor,
)


def _pc_ledger(run_id: str = "run-pc") -> dict:
    return {
        "runId": run_id,
        "target": {"id": "pc:limbus", "kind": "pc", "hwnd": 42},
        "originalWindowState": {
            "rect": [10, 20, 1010, 820],
            "style": 0x1234,
            "exStyle": 0x5678,
            "topmost": True,
        },
        "pressedMouseButtons": ["left", "left", "right"],
        "pressedKeys": [65, 65, "enter"],
        "resources": [],
    }


class FakeWindowBackend:
    def __init__(self) -> None:
        self.released_mouse: list[tuple[int, str, str]] = []
        self.released_keys: list[tuple[int, int | str, str]] = []
        self.restored: list[dict] = []
        self.activated = False
        self.online = True

    def is_window(self, *, hwnd: int, **_: object) -> bool:
        return self.online

    def release_mouse(self, *, hwnd: int, button: str, run_id: str) -> None:
        self.released_mouse.append((hwnd, button, run_id))

    def release_key(self, *, hwnd: int, key: int | str, run_id: str) -> None:
        self.released_keys.append((hwnd, key, run_id))

    def restore_window(self, *, hwnd: int, state: dict, run_id: str, **kwargs: object) -> None:
        self.restored.append({"hwnd": hwnd, "state": state, "runId": run_id, **kwargs})

    def activate(self, **_: object) -> None:
        self.activated = True


class FakeAdbBackend:
    def __init__(self, *, online: bool = True) -> None:
        self.online = online
        self.touches: list[tuple[str, int, str]] = []
        self.forwards: list[dict] = []
        self.kill_server_called = False

    def is_online(self, *, serial: str, **_: object) -> bool:
        return self.online

    def cancel_touch(self, *, serial: str, pointer_id: int, run_id: str) -> None:
        self.touches.append((serial, pointer_id, run_id))

    def remove_forward(
        self,
        *,
        serial: str,
        local_port: int,
        remote: str | None,
        scid: str | None,
        socket_name: str | None,
        run_id: str,
    ) -> None:
        self.forwards.append(
            {
                "serial": serial,
                "localPort": local_port,
                "remote": remote,
                "scid": scid,
                "socketName": socket_name,
                "runId": run_id,
            }
        )


class FakeUser32:
    def __init__(self) -> None:
        self.messages: list[tuple[int, int, int, int]] = []
        self.styles: list[tuple[int, int, int]] = []
        self.positions: list[tuple[int, int, int, int, int, int, int]] = []
        self.window_exists = True

    def IsWindow(self, hwnd: int) -> bool:
        return self.window_exists

    def PostMessageW(self, hwnd: int, message: int, wparam: int, lparam: int) -> bool:
        self.messages.append((hwnd, message, wparam, lparam))
        return True

    def SetWindowLongPtrW(self, hwnd: int, index: int, value: int) -> int:
        self.styles.append((hwnd, index, value))
        return 0

    def SetWindowPos(
        self,
        hwnd: int,
        insert_after: int,
        left: int,
        top: int,
        width: int,
        height: int,
        flags: int,
    ) -> bool:
        self.positions.append((hwnd, insert_after, left, top, width, height, flags))
        return True


class FakeCtypesLastError:
    def __init__(self) -> None:
        self.error = 0

    def set_last_error(self, value: int) -> None:
        self.error = value

    def get_last_error(self) -> int:
        return self.error


class FailingStyleUser32(FakeUser32):
    def __init__(self, errors: FakeCtypesLastError) -> None:
        super().__init__()
        self.errors = errors

    def SetWindowLongPtrW(self, hwnd: int, index: int, value: int) -> int:
        self.styles.append((hwnd, index, value))
        # Win32 uses zero both for a valid previous style and for failure.
        # The non-zero last error is the only evidence of failure here.
        self.errors.error = 5
        return 0


class FakeControlSocket:
    def __init__(self) -> None:
        self.packets: list[bytes] = []
        self.closed = False

    def sendall(self, packet: bytes) -> None:
        self.packets.append(packet)

    def close(self) -> None:
        self.closed = True


class FakeAdbDevice:
    def __init__(self, serial: str, *, state: str = "device") -> None:
        self.serial = serial
        self.state = state
        self.forwards = [
            SimpleNamespace(
                serial=serial,
                local="tcp:40123",
                remote="localabstract:scrcpy_00000001",
            )
        ]
        self.removed: list[str] = []

    def forward_list(self) -> list[object]:
        return list(self.forwards)

    def forward_remove(self, local: str, *, raise_non_found: bool = True) -> None:
        self.removed.append(local)


class FakeAdbClient:
    def __init__(self, device: FakeAdbDevice) -> None:
        self._device_value = device

    def device(self, serial: str) -> FakeAdbDevice:
        assert serial == self._device_value.serial
        return self._device_value


def _adb_ledger(run_id: str = "run-adb") -> dict:
    return {
        "runId": run_id,
        "target": {"id": "adb:phone", "kind": "adb", "endpoint": "phone"},
        "deviceSerial": "phone",
        "scid": "0x00000001",
        "socketName": "scrcpy_00000001",
        "adbForwardPort": 40123,
        "activeTouches": [{"pointerId": 1}, {"pointerId": 2}, {"pointerId": 1}],
        "resources": [
            {
                "resourceType": "adb_forward",
                "resourceId": "run-adb:forward:40123",
                "released": False,
                "metadata": {"remote": "localabstract:scrcpy_00000001"},
            },
            {
                "resourceType": "scrcpy_server",
                "resourceId": "run-adb:server:0x00000001",
                "released": False,
                "metadata": {},
            },
        ],
    }


def test_pc_cleanup_releases_recorded_drag_inputs_and_restores_window_without_activation() -> None:
    backend = FakeWindowBackend()
    result = CleanupActionExecutor(run_id="run-pc", win32_backend=backend, platform_name="Windows").execute(_pc_ledger())

    assert result["status"] == "success"
    assert backend.released_mouse == [(42, "left", "run-pc"), (42, "right", "run-pc")]
    assert [entry[:2] for entry in backend.released_keys] == [(42, 65), (42, 13)]
    assert backend.restored[0]["state"]["rect"] == [10, 20, 1010, 820]
    assert backend.restored[0]["topmost"] is True
    assert backend.activated is False


def test_pc_cleanup_is_idempotent_but_allows_retry_after_a_failed_action() -> None:
    backend = FakeWindowBackend()
    calls = 0

    def fail_once(*, hwnd: int, button: str, run_id: str) -> None:
        nonlocal calls
        calls += 1
        if calls == 1:
            raise RuntimeError("mouse backend failed")
        backend.released_mouse.append((hwnd, button, run_id))

    backend.release_mouse = fail_once  # type: ignore[method-assign]
    executor = CleanupActionExecutor(run_id="run-pc", win32_backend=backend, platform_name="Windows")
    first = executor.execute(_pc_ledger())
    second = executor.execute(_pc_ledger())

    assert first["status"] == "failed"
    assert second["status"] == "success"
    assert second["steps"]["window_restore"]["alreadyDone"] is True
    assert len(backend.restored) == 1


def test_pc_cleanup_non_windows_is_a_safe_noop() -> None:
    result = CleanupActionExecutor(run_id="run-pc", platform_name="Linux").execute(_pc_ledger())

    assert result["status"] == "success"
    assert result["steps"]["window_input_release"]["skipped"] is True
    assert result["steps"]["window_restore"]["skipped"] is True


def test_adb_cleanup_is_scoped_to_serial_and_run_identifiers() -> None:
    adb = FakeAdbBackend()
    remote_cleanup = Mock(return_value={"status": "success"})
    result = CleanupActionExecutor(
        run_id="run-adb",
        adb_backend=adb,
        remote_cleanup=remote_cleanup,
        platform_name="Linux",
    ).execute(_adb_ledger())

    assert result["status"] == "success"
    assert adb.touches == [("phone", 1, "run-adb"), ("phone", 2, "run-adb")]
    assert adb.forwards == [
        {
            "serial": "phone",
            "localPort": 40123,
            "remote": "localabstract:scrcpy_00000001",
            "scid": "0x00000001",
            "socketName": "scrcpy_00000001",
            "runId": "run-adb",
        }
    ]
    remote_cleanup.assert_called_once_with(
        reservation={
            "runId": "run-adb",
            "serial": "phone",
            "scid": "0x00000001",
            "socketName": "scrcpy_00000001",
            "adbForwardPort": 40123,
        },
        run_id="run-adb",
    )
    assert adb.kill_server_called is False


def test_adb_cleanup_returns_deferred_when_device_is_offline_without_remote_kill() -> None:
    adb = FakeAdbBackend(online=False)
    remote_cleanup = Mock()
    result = CleanupActionExecutor(
        run_id="run-adb",
        adb_backend=adb,
        remote_cleanup=remote_cleanup,
        platform_name="Linux",
    ).execute(_adb_ledger())

    assert result["status"] == "deferred"
    assert all(step["status"] == "deferred" for step in result["steps"].values())
    remote_cleanup.assert_not_called()
    assert adb.touches == []
    assert adb.forwards == []


def test_cleanup_rejects_wrong_journal_or_resource_run_identifier() -> None:
    with pytest.raises(CleanupIdentityError):
        CleanupActionExecutor(run_id="run-a").execute(_pc_ledger("run-b"))

    ledger = _pc_ledger("run-a")
    ledger["resources"] = [
        {
            "resourceType": "mouse",
            "resourceId": "run-b:mouse:left",
            "released": False,
            "metadata": {"runId": "run-b", "button": "left"},
        }
    ]
    with pytest.raises(CleanupIdentityError):
        CleanupActionExecutor(run_id="run-a").execute(ledger)


def test_cleanup_step_timeout_is_bounded_and_does_not_duplicate_inflight_action() -> None:
    backend = FakeWindowBackend()
    entered = Mock()

    def slow_mouse(*, hwnd: int, button: str, run_id: str) -> None:
        entered()
        time.sleep(0.2)

    backend.release_mouse = slow_mouse  # type: ignore[method-assign]
    executor = CleanupActionExecutor(
        run_id="run-pc",
        win32_backend=backend,
        platform_name="Windows",
        timeouts=CleanupTimeouts(window_input=0.02, window_restore=0.02),
    )
    started = time.monotonic()
    first = executor.execute(_pc_ledger())
    elapsed = time.monotonic() - started
    second = executor.execute(_pc_ledger())

    assert elapsed < 0.15
    assert first["steps"]["window_input_release"]["status"] == "deferred"
    assert second["steps"]["window_input_release"]["status"] == "deferred"
    entered.assert_called_once()


def test_repeated_timeout_recovery_keeps_one_worker_and_close_reports_drain() -> None:
    backend = FakeWindowBackend()
    entered = threading.Event()
    release = threading.Event()
    calls = 0

    def blocked_mouse(*, hwnd: int, button: str, run_id: str) -> None:
        nonlocal calls
        calls += 1
        entered.set()
        release.wait()

    backend.release_mouse = blocked_mouse  # type: ignore[method-assign]
    executor = CleanupActionExecutor(
        run_id="run-pc",
        win32_backend=backend,
        platform_name="Windows",
        timeouts=CleanupTimeouts(window_input=0.01, window_restore=0.01),
    )
    ledger = _pc_ledger()
    ledger["pressedMouseButtons"] = ["left"]
    ledger["pressedKeys"] = []

    first = executor.execute(ledger)
    assert first["steps"]["window_input_release"]["status"] == "deferred"
    assert entered.wait(0.2)
    for _ in range(5):
        repeated = executor.execute(ledger)
        assert repeated["steps"]["window_input_release"]["status"] == "deferred"

    diagnostic = executor.diagnostics()
    assert diagnostic["bounded"] is True
    assert diagnostic["workerCount"] == 1
    assert diagnostic["maxWorkers"] == 1
    assert diagnostic["maxInflight"] == 1
    assert diagnostic["inflight"] is True
    assert calls == 1

    closed = executor.close(timeout=0.01)
    assert closed["closed"] is True
    assert closed["drained"] is False
    assert closed["inflight"] is True
    assert closed["workerCount"] == 1

    release.set()
    drained = executor.drain(timeout=1.0)
    assert drained["drained"] is True
    assert drained["inflight"] is False
    assert executor.close(timeout=1.0)["workerAlive"] is False
    assert calls == 1


def test_executor_close_is_idempotent_after_a_successful_recover() -> None:
    backend = FakeWindowBackend()
    executor = CleanupActionExecutor(run_id="run-pc", win32_backend=backend, platform_name="Windows")
    executor.execute(_pc_ledger())

    first = executor.close(timeout=1.0)
    second = executor.close(timeout=1.0)

    assert first["closed"] is True
    assert first["drained"] is True
    assert second["closed"] is True
    assert second["drained"] is True
    assert second["workerCount"] == first["workerCount"] == 1
    assert second["workerAlive"] is False


def test_duplicate_cleanup_ledger_recover_reuses_the_same_inflight_worker(tmp_path) -> None:
    ledger = CleanupLedger(tmp_path / "run-pc.json", run_id="run-pc")
    ledger.reserve(
        hwnd=42,
        originalWindowState={"rect": [10, 20, 1010, 820]},
        inputObligations={"pressedMouseButtons": ["left"]},
    )
    backend = FakeWindowBackend()
    entered = threading.Event()
    release = threading.Event()
    calls = 0

    def blocked_mouse(*, hwnd: int, button: str, run_id: str) -> None:
        nonlocal calls
        calls += 1
        entered.set()
        release.wait()

    backend.release_mouse = blocked_mouse  # type: ignore[method-assign]
    executor = CleanupActionExecutor(
        run_id="run-pc",
        win32_backend=backend,
        platform_name="Windows",
        timeouts=CleanupTimeouts(window_input=0.01, window_restore=0.01),
    )

    first = ledger.recover(executor, delete_complete=False)
    assert first.complete is False
    assert entered.wait(0.2)
    second = ledger.recover(executor, delete_complete=False)
    assert second.complete is False
    assert executor.diagnostics()["workerCount"] == 1
    assert calls == 1

    release.set()
    assert executor.drain(timeout=1.0)["drained"] is True
    third = ledger.recover(executor, delete_complete=False)
    assert third.complete is True
    assert calls == 1
    executor.close(timeout=1.0)


def test_executor_is_directly_compatible_with_cleanup_ledger_recover(tmp_path) -> None:
    ledger = CleanupLedger(tmp_path / "run-pc.json", run_id="run-pc")
    ledger.reserve(
        hwnd=42,
        originalWindowState={"rect": [10, 20, 1010, 820], "topmost": False},
        inputObligations={"pressedMouseButtons": ["left"]},
    )
    backend = FakeWindowBackend()

    recovery = ledger.recover(
        CleanupActionExecutor(run_id="run-pc", win32_backend=backend, platform_name="Windows"),
        delete_complete=False,
    )

    assert recovery.complete is True
    assert [item[1] for item in backend.released_mouse] == ["left"]
    assert ledger.complete is True


def test_default_windows_backend_uses_ctypes_and_never_activates_window() -> None:
    user32 = FakeUser32()
    backend = Win32CleanupBackend(user32=user32)
    result = CleanupActionExecutor(
        run_id="run-pc",
        win32_backend=backend,
        platform_name="Windows",
    ).execute(_pc_ledger())

    assert result["status"] == "success"
    # left/right/key-up are posted to the recorded HWND only; no foreground
    # or activation API is present in the injected user32 surface.
    assert [message[0] for message in user32.messages] == [42, 42, 42, 42]
    assert [message[1] for message in user32.messages] == [0x0202, 0x0205, 0x0101, 0x0101]
    assert user32.styles == [(42, -16, 0x1234), (42, -20, 0x5678)]
    assert user32.positions == [(42, -1, 10, 20, 1000, 800, 0x0030)]


def test_win32_zero_previous_style_is_success_when_last_error_is_zero() -> None:
    errors = FakeCtypesLastError()
    user32 = FakeUser32()
    backend = Win32CleanupBackend(user32=user32, ctypes_module=errors)

    backend.restore_window(hwnd=42, state={"style": 0, "exStyle": 0})

    assert user32.styles == [(42, -16, 0), (42, -20, 0)]


def test_win32_style_restore_reports_zero_return_with_last_error() -> None:
    errors = FakeCtypesLastError()
    user32 = FailingStyleUser32(errors)
    executor = CleanupActionExecutor(
        run_id="run-pc",
        win32_backend=Win32CleanupBackend(user32=user32, ctypes_module=errors),
        platform_name="Windows",
    )

    result = executor.execute(_pc_ledger())

    assert result["status"] == "failed"
    assert result["steps"]["window_restore"]["status"] == "failed"
    assert user32.positions == []


def test_default_adb_backend_cancels_touches_removes_exact_forward_and_kills_only_scoped_server() -> None:
    device = FakeAdbDevice("phone")
    adb_client = FakeAdbClient(device)
    sockets: list[FakeControlSocket] = []
    commands: list[tuple[list[str], dict[str, object]]] = []

    def socket_factory(address: tuple[str, int], *, timeout: float) -> FakeControlSocket:
        assert address == ("127.0.0.1", 40123)
        assert timeout > 0
        socket = FakeControlSocket()
        sockets.append(socket)
        return socket

    def command_runner(argv: list[str], **kwargs: object) -> SimpleNamespace:
        commands.append((argv, kwargs))
        assert "kill-server" not in argv
        assert kwargs.get("timeout", 0) > 0
        if argv[-3:] == ["ps", "-A", "-o"]:
            raise AssertionError("unexpected split ps command")
        if argv[-4:] == ["ps", "-A", "-o", "PID,ARGS"]:
            return SimpleNamespace(
                returncode=0,
                stdout="1234 u0_a123 0 0 S app_process scrcpy scid=0x00000001 socket_name=scrcpy_00000001\n",
            )
        assert argv[-3:] == ["kill", "-TERM", "1234"]
        return SimpleNamespace(returncode=0, stdout="")

    executor = build_default_cleanup_executor(
        "run-adb",
        _adb_ledger(),
        platform_name="Linux",
        adb_module=adb_client,
        adb_path="adb",
        socket_factory=socket_factory,
        command_runner=command_runner,
    )
    result = executor.execute(_adb_ledger())

    assert result["status"] == "success"
    assert len(sockets) == 2
    assert all(socket.closed and len(socket.packets) == 2 for socket in sockets)
    assert device.removed == ["tcp:40123"]
    assert [command[0] for command in commands] == [
        ["adb", "-s", "phone", "shell", "ps", "-A", "-o", "PID,ARGS"],
        ["adb", "-s", "phone", "shell", "kill", "-TERM", "1234"],
    ]


def test_default_factory_defers_when_optional_dependency_is_unavailable() -> None:
    executor = build_default_cleanup_executor(
        "run-pc",
        _pc_ledger(),
        platform_name="Windows",
    )
    result = executor.execute(_pc_ledger())

    assert result["status"] == "deferred"
    assert result["steps"]["window_input_release"]["status"] == "deferred"
    assert result["steps"]["window_restore"]["status"] == "deferred"


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("deviceSerial", "phone;kill-server"),
        ("adbForwardPort", 65536),
        ("scid", "0x1;kill"),
        ("socketName", "scrcpy_1;kill"),
    ],
)
def test_default_factory_rejects_untrusted_adb_identifiers(field: str, value: object) -> None:
    record = _adb_ledger()
    record[field] = value
    with pytest.raises(CleanupIdentityError):
        build_default_cleanup_executor("run-adb", record, platform_name="Linux")


def test_default_adb_backend_rejects_mismatched_scid_socket_without_touching_device() -> None:
    device = FakeAdbDevice("phone")
    backend = AdbCleanupBackend(adb_module=FakeAdbClient(device))
    with pytest.raises(CleanupIdentityError):
        backend.remove_forward(
            serial="phone",
            local_port=40123,
            scid="0x00000001",
            socket_name="scrcpy_00000002",
        )
    assert device.removed == []


def test_remote_process_matching_requires_scrcpy_argument_tokens() -> None:
    lines = [
        "1234 unrelated not_scid=0x00000001",
        "1235 unrelated scid=0x00000001",
        "1236 app_process scrcpy scid=0x00000001 socket_name=scrcpy_00000002",
        "1237 app_process scrcpy scid=0x00000001 socket_name=scrcpy_00000001",
    ]

    assert AdbCleanupBackend._matching_remote_process(
        lines,
        scid="0x00000001",
        socket_name="scrcpy_00000001",
    ) == [1237]


def test_remote_cleanup_rejects_reservation_from_another_run_before_device_access() -> None:
    device = FakeAdbDevice("phone")
    backend = AdbCleanupBackend(adb_module=FakeAdbClient(device))

    with pytest.raises(CleanupIdentityError, match="runId"):
        backend.remote_cleanup(
            reservation={
                "runId": "other-run",
                "serial": "phone",
                "scid": "0x00000001",
                "socketName": "scrcpy_00000001",
            },
            run_id="run-adb",
        )
    assert device.removed == []
