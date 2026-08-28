from __future__ import annotations

from typing import Any

from module.backend_application import BackendApplication
from module.rpc_dispatcher import RpcDispatcher


class FakeConfigModel:
    def __init__(self, values: dict[str, Any]) -> None:
        for key, value in values.items():
            setattr(self, key, value)
        self.teams: dict[str, Any] = {}
        self._values = values

    def model_dump(self) -> dict[str, Any]:
        return {**self._values, "teams": self.teams}


class FakeConfig:
    def __init__(self) -> None:
        values = {
            "daily_task": False,
            "get_reward": False,
            "buy_enkephalin": False,
            "mirror": False,
            "resonate_with_Ahab": False,
            "set_win_size": 1080,
            "set_win_position": "free",
            "set_reduce_miscontact": True,
            "screenshot_interval": 0.5,
            "mouse_action_interval": 0.3,
            "mouse_down_duration": 0.1,
            "use_post_message": False,
            "set_EXP_count": 1,
            "set_thread_count": 0,
            "daily_teams": 1,
            "use_continuous_combat": False,
            "use_continuous_combat_select": 1,
            "targeted_teaming_EXP": False,
            "EXP_day_1_2": 1,
            "EXP_day_3_4": 1,
            "EXP_day_5_6": 1,
            "EXP_day_7": 1,
            "targeted_teaming_thread": False,
            "thread_day_1": 1,
            "thread_day_2": 1,
            "thread_day_3": 1,
            "thread_day_4": 1,
            "thread_day_5": 1,
            "thread_day_6": 1,
            "thread_day_7": 1,
            "set_get_prize": 0,
            "set_lunacy_to_enkephalin": 2,
            "Dr_Grandet_mode": False,
            "skip_enkephalin": False,
            "set_mirror_count": 1,
            "infinite_dungeons": False,
            "hard_mirror": False,
            "no_weekly_bonuses": False,
            "floor_3_exit": False,
            "save_rewards": False,
            "hard_mirror_single_bonuses": False,
            "select_event_pack": False,
            "skip_event_pack": False,
            "re_claim_rewards": False,
            "not_skip_whitegossypium": False,
            "fight_to_last_man": False,
            "mirror_keyboard_navigation": False,
            "mirror_keyboard_simple_pathfinding": False,
            "after_completion_actions": [],
            "after_completion_power_action": "none",
            "keep_after_completion": False,
            "shutdown_hotkey": "",
            "pause_hotkey": "",
            "resume_hotkey": "",
            "teams_active_queue": [],
            "image_resource_source": "Auto",
            "simulator": True,
            "simulator_type": 0,
            "simulator_port": 16384,
            "start_emulator_timeout": 60,
            "memory_protection": True,
            "minimize_to_tray": True,
            "autostart": False,
            "experimental_keep_screen_awake": True,
            "experimental_hdr_warning": True,
            "update_prerelease_enable": False,
            "update_source": "GitHub",
            "mirrorchyan_cdk": "",
        }
        self.values = values
        self.config = FakeConfigModel(values)

    def get_value(self, key: str, default: Any = None) -> Any:
        return self.values.get(key, default)

    def unsaved_set_value(self, key: str, value: Any, **_: Any) -> None:
        self.values[key] = value
        setattr(self.config, key, value)

    def save(self, **_: Any) -> None:
        return None


class FakeDeviceManager:
    def __init__(self) -> None:
        self.busy_checker = lambda: False
        self.status_listeners = []
        self.notice_listeners = []

    def set_busy_checker(self, checker):
        self.busy_checker = checker

    def add_status_listener(self, listener):
        self.status_listeners.append(listener)

    def remove_status_listener(self, listener):
        self.status_listeners.remove(listener)

    def add_notice_listener(self, listener):
        self.notice_listeners.append(listener)

    def remove_notice_listener(self, listener):
        self.notice_listeners.remove(listener)

    def list_devices(self):
        return [{"id": "pc:limbus", "name": "Limbus Company"}]

    def connect(self, device_id):
        return {"deviceId": device_id, "status": "connected"}

    def disconnect(self):
        return {"status": "disconnected"}

    def close(self):
        return None


class FakePreviewCapture:
    def __init__(self) -> None:
        self.started: list[str] = []
        self.stop_count = 0
        self.close_count = 0

    def start(self, device_id: str) -> bool:
        self.started.append(device_id)
        return True

    def stop(self) -> bool:
        self.stop_count += 1
        return True

    def close(self) -> None:
        self.close_count += 1


class FakeThemeStore:
    def __init__(self) -> None:
        self.config = {
            "theme_pack_list": {"alpha": 1, "disabled": -2},
            "theme_pack_list_hard": {"hard": 3},
        }
        self.theme_pack_list_path = "theme_pack_list.yaml"

    def save_config(self, *, path, config_data):
        self.config = config_data


def make_application(preview_capture=None) -> BackendApplication:
    return BackendApplication(
        FakeDeviceManager(),
        version="test",
        config=FakeConfig(),
        theme_list=FakeThemeStore(),
        preview_capture=preview_capture,
    )


def test_event_bus_adds_ordered_sequence_to_application_events() -> None:
    app = make_application()
    events = []
    app.add_event_listener(lambda event, payload, sequence: events.append((event, payload, sequence)))

    app.emit("app.notice", {"level": "info", "message": "ready"})
    app.emit("app.notice", {"level": "info", "message": "done"})

    assert [event[2] for event in events] == [1, 2]
    app.close()


def test_device_events_control_continuous_preview_lifecycle() -> None:
    preview = FakePreviewCapture()
    app = make_application(preview)

    app._on_device_event(
        "device.status",
        {"deviceId": "pc:limbus", "status": "connected"},
    )
    app._on_device_event(
        "device.status",
        {"deviceId": "pc:limbus", "status": "connecting"},
    )
    app._on_device_event(
        "device.status",
        {"deviceId": None, "status": "disconnected"},
    )

    assert preview.started == ["pc:limbus"]
    assert preview.stop_count == 2
    app.close()
    assert preview.close_count == 1


def test_dispatcher_routes_real_configuration_and_all_read_models() -> None:
    app = make_application()
    dispatcher = RpcDispatcher(application=app, version="test")

    methods = [
        "app.ping",
        "app.version",
        "tasks.getConfig",
        "execution.getState",
        "team.list",
        "sinner.list",
        "themePack.list",
        "resource.status",
        "hotkey.get",
        "systemSettings.get",
        "device.list",
    ]
    responses = [
        dispatcher.dispatch({"jsonrpc": "2.0", "id": index, "method": method})
        for index, method in enumerate(methods, start=1)
    ]

    assert all("error" not in response for response in responses)
    assert responses[1]["result"]["schemaVersion"] == 1
    assert responses[2]["result"]["schemaVersion"] == 1
    assert responses[5]["result"][0]["id"] == "yi_sang"
    assert responses[6]["result"]["packs"][0]["id"] == "alpha"
    app.close()


def test_task_patch_preserves_unknown_config_values_and_rejects_bad_values() -> None:
    app = make_application()
    dispatcher = RpcDispatcher(application=app, version="test")
    app.config.values["future_field"] = {"kept": True}

    response = dispatcher.dispatch(
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tasks.setConfig",
            "params": {
                "enabledTasks": {"mirror": True},
                "daily_task": {"set_EXP_count": 4},
            },
        }
    )
    assert response["result"] is True
    assert app.config.values["mirror"] is True
    assert app.config.values["set_EXP_count"] == 4
    assert app.config.values["future_field"] == {"kept": True}

    invalid = dispatcher.dispatch(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tasks.setConfig",
            "params": {"daily_task": {"set_EXP_count": "bad"}},
        }
    )
    assert invalid["error"]["code"] == -32602
    app.close()
