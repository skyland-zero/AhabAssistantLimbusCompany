from __future__ import annotations

import threading
from types import SimpleNamespace
from unittest.mock import Mock

import pytest

import module.automation.input_handlers.simulator.mumu_control as mumu_module
from module.automation.input_handlers.simulator.mumu_control import MumuControl
from module.automation.input_handlers.simulator.runner_policy import (
    RunnerDevicePolicyError,
    RunnerPolicy,
)
from module.automation.input_handlers.simulator.scrcpy_control import ScrcpyControl
from module.automation.input_handlers.simulator.simulator_control import SimulatorControl


def test_runner_policy_defaults_and_explicit_environment(monkeypatch) -> None:
    monkeypatch.delenv("AALC_RUNNER_MODE", raising=False)
    monkeypatch.delenv("AALC_ALLOW_EMULATOR_LAUNCH", raising=False)
    assert RunnerPolicy.from_env() == RunnerPolicy(runner_mode=False, allow_emulator_launch=True)

    monkeypatch.setenv("AALC_RUNNER_MODE", "true")
    assert RunnerPolicy.from_env() == RunnerPolicy(runner_mode=True, allow_emulator_launch=False)

    monkeypatch.setenv("AALC_ALLOW_EMULATOR_LAUNCH", "1")
    assert RunnerPolicy.from_env() == RunnerPolicy(runner_mode=True, allow_emulator_launch=True)


def test_explicit_exit_is_separate_from_automatic_recovery() -> None:
    policy = RunnerPolicy(runner_mode=True, allow_emulator_launch=False)

    with pytest.raises(RunnerDevicePolicyError, match="successful task"):
        policy.assert_explicit_exit_allowed(task_succeeded=False, lease_active=True)
    with pytest.raises(RunnerDevicePolicyError, match="successful task"):
        policy.assert_explicit_exit_allowed(task_succeeded=True, lease_active=False)

    # The explicit completion-only exception remains available even though
    # automatic launch/recovery is disabled.
    policy.assert_explicit_exit_allowed(task_succeeded=True, lease_active=True)


def _mumu_runner_controller() -> MumuControl:
    controller = object.__new__(MumuControl)
    controller._runner_policy = RunnerPolicy(runner_mode=True, allow_emulator_launch=False)
    controller.multi_instance_number = 0
    controller.device = None
    controller.package_list = True
    controller.start_game_times = 0
    controller.game_package_name = "com.ProjectMoon.LimbusCompany"
    return controller


def test_mumu_runner_start_fails_before_launch(monkeypatch) -> None:
    controller = _mumu_runner_controller()
    monkeypatch.setattr(controller, "can_attach_without_launch", lambda: False)
    launch = Mock()
    close = Mock()
    monkeypatch.setattr(mumu_module, "run_as_user", launch)
    monkeypatch.setattr(controller, "close_simulator", close)

    with pytest.raises(RunnerDevicePolicyError, match="forbids emulator launch"):
        controller.start()

    launch.assert_not_called()
    close.assert_not_called()


def test_mumu_runner_adb_retries_fail_closed_without_shutdown(monkeypatch) -> None:
    controller = _mumu_runner_controller()
    controller.get_mumu_adb_port = lambda: "127.0.0.1:16384"
    close = Mock()
    restart = Mock()
    monkeypatch.setattr(controller, "close_simulator", close)
    monkeypatch.setattr(controller, "start", restart)
    monkeypatch.setattr(mumu_module.adb, "connect", lambda _port: "failed")

    with pytest.raises(RunnerDevicePolicyError, match="could not connect"):
        controller.adb_connect()

    close.assert_not_called()
    restart.assert_not_called()


def test_mumu_runner_start_game_does_not_retry_or_inspect_packages(monkeypatch) -> None:
    controller = _mumu_runner_controller()

    class FailingDevice:
        def app_start(self, _package: str) -> None:
            raise RuntimeError("device offline")

    controller.device = FailingDevice()
    sleep = Mock()
    monkeypatch.setattr(controller, "interruptible_sleep", sleep)

    with pytest.raises(RunnerDevicePolicyError, match="could not start the game"):
        controller.start_game()

    sleep.assert_not_called()


def test_mumu_explicit_exit_requires_success_and_lease(monkeypatch) -> None:
    controller = _mumu_runner_controller()
    controller.exe_path = "MuMuManager.exe"
    close = Mock()
    monkeypatch.setattr(controller, "_close_simulator_unchecked", close)

    with pytest.raises(RunnerDevicePolicyError):
        controller.exit_emulator(task_succeeded=True, lease_active=False)
    close.assert_not_called()

    controller.exit_emulator(task_succeeded=True, lease_active=True)
    close.assert_called_once_with()


def test_generic_runner_connection_recovery_fails_closed() -> None:
    controller = object.__new__(SimulatorControl)
    controller._runner_policy = RunnerPolicy(runner_mode=True, allow_emulator_launch=False)

    with pytest.raises(RunnerDevicePolicyError, match="recovery is disabled"):
        controller.reconnect("device offline")


def test_scrcpy_run_identifiers_are_safe_and_injected_into_server_command() -> None:
    command = ScrcpyControl._build_server_shell_command(
        "/data/local/tmp/scrcpy-server.jar",
        scid="0x1234abcd",
        socket_name="scrcpy_1234abcd",
    )

    assert "scid=0x1234abcd" in command
    assert "scid=0x1234abcd;" not in command
    with pytest.raises(ValueError):
        ScrcpyControl._build_server_shell_command("jar", scid="bad;kill")


def test_scrcpy_rejects_mismatched_or_unrepresentable_socket_identity() -> None:
    with pytest.raises(ValueError, match="socket_name"):
        ScrcpyControl._build_server_shell_command(
            "jar",
            scid="0x00000001",
            socket_name="scrcpy_00000002",
        )
    with pytest.raises(ValueError, match="requires"):
        ScrcpyControl._build_server_shell_command("jar", socket_name="other-session")
    with pytest.raises(ValueError, match="unsupported"):
        ScrcpyControl._build_server_shell_command("jar;kill", scid="0x00000001")


def _cleanup_scrcpy_controller(remote_cleanup) -> ScrcpyControl:
    control = object.__new__(ScrcpyControl)
    control._runner_policy = RunnerPolicy(runner_mode=True, allow_emulator_launch=False)
    control._run_id = "run-1"
    control._scid = "0x00000001"
    control._socket_name = "scrcpy_00000001"
    control._remote_socket_name = "scrcpy_00000001"
    control.serial = "emulator-5554"
    control.device = None
    control._forward_port = 43210
    control._forward_remote = "localabstract:scrcpy_00000001"
    control._forward_owned = True
    control._remote_cleanup = remote_cleanup
    control._cleanup_lock = threading.RLock()
    control._cleanup_done = False
    control._cleanup_result = None
    control._running = True
    control._video_socket = None
    control._control_socket = None
    control._decode_thread = None
    control._server_proc = None
    control._first_frame_ready = threading.Event()
    control._stream_eof = threading.Event()
    control._frame_lock = threading.Lock()
    control._frame_condition = threading.Condition(control._frame_lock)
    control._latest_frame = None
    control._recv_buffer = bytearray()
    control._codec = None
    control._state_lock = threading.Lock()
    control._decode_state = "ACTIVE"
    control._decoder_needs_keyframe = False
    control._decoder_has_config = True
    control._resync_started_mono = 1.0
    control._last_config_packet = None
    control._last_config_pts = None
    return control


def test_scrcpy_cleanup_is_idempotent_and_only_removes_owned_forward() -> None:
    class FakeDevice:
        serial = "emulator-5554"

        def __init__(self) -> None:
            self.removed: list[str] = []

        def forward_list(self):
            return [
                SimpleNamespace(
                    serial=self.serial,
                    local="tcp:43210",
                    remote="localabstract:scrcpy_00000001",
                )
            ]

        def forward_remove(self, local: str, *, raise_non_found: bool = True) -> None:
            self.removed.append(local)

    remote_calls: list[dict[str, object]] = []
    control = _cleanup_scrcpy_controller(remote_calls.append)
    control.device = FakeDevice()

    first = control.cleanup_session(reason="test")
    second = control.cleanup_session(reason="again")

    assert first["forward"] == "removed"
    assert first["remote"] == "requested"
    assert second["alreadyCleaned"] is True
    assert control.device.removed == ["tcp:43210"]
    assert len(remote_calls) == 1
    assert remote_calls[0]["scid"] == "0x00000001"


def test_scrcpy_runner_cleanup_does_not_remove_unverifiable_forward() -> None:
    class DeviceWithoutForwardListing:
        serial = "emulator-5554"

        def __init__(self) -> None:
            self.removed = 0

        def forward_remove(self, _local: str, **_kwargs) -> None:
            self.removed += 1

    control = _cleanup_scrcpy_controller(None)
    control.device = DeviceWithoutForwardListing()
    result = control.cleanup_session()

    assert result["forward"] == "skipped_identity_unknown"
    assert control.device.removed == 0
    assert result["remote"] == "unbound_cleanup_ledger"
