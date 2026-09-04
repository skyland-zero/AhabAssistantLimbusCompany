from __future__ import annotations

from threading import Event
from types import SimpleNamespace

import pytest

import module.device_manager as device_manager_module
import tasks.base.script_task_scheme as script_task_scheme
from core.execution_control import bind_cancel_event
from module.my_error.my_error import userStopError


def test_onetime_mirror_process_propagates_user_stop(monkeypatch) -> None:
    class StoppingMirror:
        def __init__(self, *_args, **_kwargs) -> None:
            pass

        def run(self) -> None:
            raise userStopError("用户已请求停止任务")

    monkeypatch.setattr(script_task_scheme.cfg, "auto_hard_mirror", False, raising=False)
    monkeypatch.setattr(script_task_scheme, "Mirror", StoppingMirror)

    with pytest.raises(userStopError, match="用户已请求停止任务"):
        script_task_scheme.onetime_mir_process(object(), 1)


def _stub_script_setup(monkeypatch) -> None:
    monkeypatch.setattr(script_task_scheme, "init_game", lambda: None)
    monkeypatch.setattr(script_task_scheme, "_warn_if_game_monitor_hdr_enabled", lambda: None)
    monkeypatch.setattr(script_task_scheme, "_is_simulator_runtime", lambda: False)
    monkeypatch.setattr(script_task_scheme.path_manager, "initialize_paths", lambda: None)
    monkeypatch.setattr(script_task_scheme.auto, "clear_img_cache", lambda: None)
    monkeypatch.setattr(script_task_scheme.auto, "click_element", lambda *_args, **_kwargs: False)
    monkeypatch.setattr(script_task_scheme.retry_monitor, "start", lambda: None)
    monkeypatch.setattr(script_task_scheme.retry_monitor, "stop", lambda: None)
    monkeypatch.setattr(script_task_scheme.cfg, "daily_task", False, raising=False)
    monkeypatch.setattr(script_task_scheme.cfg, "get_reward", False, raising=False)
    monkeypatch.setattr(script_task_scheme.cfg, "buy_enkephalin", False, raising=False)
    monkeypatch.setattr(script_task_scheme.cfg, "mirror", False, raising=False)
    monkeypatch.setattr(script_task_scheme.cfg, "resonate_with_Ahab", False, raising=False)
    monkeypatch.setattr(script_task_scheme.cfg, "set_reduce_miscontact", False, raising=False)
    monkeypatch.setattr(script_task_scheme.cfg, "skip_enkephalin", False, raising=False)


def test_runner_success_returns_serialized_completion_request_without_side_effects(monkeypatch) -> None:
    _stub_script_setup(monkeypatch)
    toasts: list[tuple[object, ...]] = []
    noops: list[tuple[object, ...]] = []
    power_calls: list[tuple[object, ...]] = []
    monkeypatch.setattr(script_task_scheme, "send_toast", lambda *args, **kwargs: toasts.append(args))
    monkeypatch.setattr(script_task_scheme, "noop", lambda *args, **kwargs: noops.append(args))
    monkeypatch.setattr(
        script_task_scheme,
        "execute_after_completion",
        lambda *args, **kwargs: power_calls.append(args),
    )
    monkeypatch.setattr(
        script_task_scheme,
        "get_after_completion_config",
        lambda: (["exit_game", "exit_emulator", "exit_aalc"], "shutdown"),
    )
    monkeypatch.setenv("AALC_RUN_ID", "runner-success")
    monkeypatch.setenv("AALC_RUNNER_MODE", "1")

    result = script_task_scheme.script_task(runner_mode=True)

    assert result == {
        "actions": ["exit_game", "exit_emulator", "exit_aalc"],
        "runnerActions": ["exit_game", "exit_emulator"],
        "sidecarActions": ["exit_aalc"],
        "powerAction": "shutdown",
        "runId": "runner-success",
        "outcome": "completed",
        "forced": False,
        "requiresDeviceLease": True,
        "deviceDisposition": "restore",
    }
    assert toasts == []
    assert noops == []
    assert power_calls == []


def test_runner_device_actions_require_lease_and_report_disposition(monkeypatch) -> None:
    class Controller:
        def __init__(self) -> None:
            self.calls: list[str] = []

        def close_current_app(self) -> None:
            self.calls.append("game")

        def close_simulator(self) -> None:
            self.calls.append("emulator")

    controller = Controller()
    context = SimpleNamespace(
        runner_owned_controller=controller,
        device_lease_valid=True,
        spec=SimpleNamespace(device_target={"id": "mumu:0"}),
    )
    monkeypatch.setattr(
        script_task_scheme,
        "get_after_completion_config",
        lambda: (["exit_game", "exit_emulator", "exit_aalc"], "none"),
    )

    request = script_task_scheme.get_after_completion_request(context=context)
    assert controller.calls == ["game", "emulator"]
    assert request["deviceDisposition"] == "emulator_closed"

    controller.calls.clear()
    invalid = script_task_scheme.get_after_completion_request(context=SimpleNamespace(device_lease_valid=False))
    assert controller.calls == []
    assert invalid["runnerActions"] == ["exit_game", "exit_emulator"]
    assert invalid["deviceDisposition"] == "restore"

    stopped = script_task_scheme.get_after_completion_request(outcome="stopped", forced=True)
    assert stopped["actions"] == []
    assert stopped["powerAction"] == "none"
    assert stopped["runnerActions"] == []
    assert stopped["requiresDeviceLease"] is False


def test_runner_init_game_rejects_legacy_device_creation_without_session(monkeypatch) -> None:
    monkeypatch.setenv("AALC_RUNNER_MODE", "1")
    monkeypatch.setattr(
        device_manager_module,
        "get_device_manager",
        lambda: SimpleNamespace(active_session=None),
    )

    with pytest.raises(device_manager_module.DeviceError, match="私有设备 session"):
        script_task_scheme.init_game()


def test_legacy_success_keeps_toast_and_after_completion(monkeypatch) -> None:
    _stub_script_setup(monkeypatch)
    toasts: list[tuple[object, ...]] = []
    executed: list[tuple[object, ...]] = []
    monkeypatch.setattr(script_task_scheme, "send_toast", lambda *args, **kwargs: toasts.append(args))
    monkeypatch.setattr(script_task_scheme, "noop", lambda *args, **kwargs: None)
    monkeypatch.setattr(
        script_task_scheme,
        "get_after_completion_config",
        lambda: (["exit_game"], "none"),
    )
    monkeypatch.setattr(
        script_task_scheme,
        "execute_after_completion",
        lambda *args, **kwargs: executed.append(args) or True,
    )
    monkeypatch.setattr(script_task_scheme.platform, "system", lambda: "Windows")
    monkeypatch.setattr(device_manager_module, "get_device_manager", lambda: SimpleNamespace(active_session=None))
    monkeypatch.delenv("AALC_RUNNER_MODE", raising=False)
    monkeypatch.delenv("AALC_EXECUTION_RUNNER", raising=False)

    result = script_task_scheme.script_task(runner_mode=False)

    assert result == 0
    assert toasts
    assert executed == [(["exit_game"], "none")]


def test_my_script_task_exposes_runner_request_and_returns_it(monkeypatch) -> None:
    captured: list[dict[str, object]] = []
    request = {"actions": ["exit_game"], "deviceDisposition": "game_closed"}
    context = SimpleNamespace(
        after_completion=lambda value: captured.append(dict(value)),
        set_device_disposition=lambda _value: None,
    )
    monkeypatch.setattr(script_task_scheme, "script_task", lambda **_: request)
    monkeypatch.setattr(script_task_scheme.auto, "clear_img_cache", lambda: None)

    task = script_task_scheme.my_script_task(context)
    result = task._run()

    assert result == request
    assert task.after_completion_request == request
    assert task.get_after_completion_request() == request
    assert captured == [request]


def test_cancelled_runner_does_not_create_completion_request(monkeypatch) -> None:
    event = Event()
    event.set()
    bind_cancel_event(event)
    try:
        with pytest.raises(userStopError):
            script_task_scheme.script_task(runner_mode=True)
    finally:
        bind_cancel_event(None)
