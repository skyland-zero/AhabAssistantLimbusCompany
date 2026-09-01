from __future__ import annotations

import sys
from types import ModuleType
from typing import Any

import pytest

from module.backend_application import BackendApplication
from module.config import TeamSetting
from module.config.config import Config
from module.device_manager import DeviceManager, DeviceSession
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
            "wxpusher_spt": "",
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


class FakeNotificationService:
    def __init__(self) -> None:
        self.tests: list[str] = []
        self.completions: list[tuple[str, str, int]] = []
        self.finals: list[tuple[str, dict[str, Any]]] = []
        self.failures: list[tuple[str, str]] = []
        self.closed = False

    def send_test(self, spt: str) -> dict[str, Any]:
        self.tests.append(spt)
        return {"code": 1000}

    def enqueue_completion(self, spt: str, kind: str, count: int) -> bool:
        self.completions.append((spt, kind, count))
        return True

    def enqueue_final(self, spt: str, current_run: dict[str, Any]) -> bool:
        self.finals.append((spt, current_run))
        return True

    def enqueue_failure(self, spt: str, error: str) -> bool:
        self.failures.append((spt, error))
        return True

    def close(self) -> None:
        self.closed = True


def make_application(preview_capture=None, stats_path=None, notifications=None) -> BackendApplication:
    return BackendApplication(
        FakeDeviceManager(),
        version="test",
        config=FakeConfig(),
        theme_list=FakeThemeStore(),
        preview_capture=preview_capture,
        stats_path=stats_path,
        notifications=notifications,
    )


def test_event_bus_adds_ordered_sequence_to_application_events() -> None:
    app = make_application()
    events = []
    app.add_event_listener(lambda event, payload, sequence: events.append((event, payload, sequence)))

    app.emit("app.notice", {"level": "info", "message": "ready"})
    app.emit("app.notice", {"level": "info", "message": "done"})

    assert [event[2] for event in events] == [1, 2]
    app.close()


def test_application_close_is_idempotent() -> None:
    preview = FakePreviewCapture()
    app = make_application(preview)

    app.close()
    app.close()

    assert preview.close_count == 1


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


def test_preview_control_is_idempotent_and_gates_connected_devices() -> None:
    preview = FakePreviewCapture()
    app = make_application(preview)

    assert app.preview_set_enabled({"enabled": False}) == {
        "enabled": False,
        "running": False,
    }
    assert app.preview_set_enabled({"enabled": False}) == {
        "enabled": False,
        "running": False,
    }
    app._on_device_event(
        "device.status",
        {"deviceId": "pc:limbus", "status": "connected"},
    )
    assert preview.started == []
    assert preview.stop_count == 1

    assert app.preview_set_enabled({"enabled": True}) == {
        "enabled": True,
        "running": True,
    }
    assert app.preview_set_enabled({"enabled": True}) == {
        "enabled": True,
        "running": True,
    }
    assert preview.started == ["pc:limbus"]

    app._on_device_event(
        "device.status",
        {"deviceId": None, "status": "disconnected"},
    )
    assert preview.stop_count == 2
    app.close()


def test_dispatcher_validates_preview_control_params() -> None:
    app = make_application(FakePreviewCapture())
    dispatcher = RpcDispatcher(application=app, version="test")

    response = dispatcher.dispatch(
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "preview.setEnabled",
            "params": {"enabled": False},
        }
    )
    invalid = dispatcher.dispatch(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "preview.setEnabled",
            "params": {"enabled": "no"},
        }
    )

    assert response["result"] == {"enabled": False, "running": False}
    assert invalid["error"]["code"] == -32602
    assert RpcDispatcher.is_mutating("preview.setEnabled")
    app.close()


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
    assert responses[1]["result"]["schemaVersion"] == 2
    assert responses[2]["result"]["schemaVersion"] == 2
    assert responses[5]["result"][0]["id"] == "yi_sang"
    assert responses[6]["result"]["packs"][0]["id"] == "alpha"
    app.close()


def test_system_settings_persist_wxpusher_spt_and_test_unsaved_value() -> None:
    notifications = FakeNotificationService()
    app = make_application(notifications=notifications)
    dispatcher = RpcDispatcher(application=app, version="test")
    app.config.values["wxpusher_spt"] = "SPT_saved-value"

    settings = dispatcher.dispatch({"jsonrpc": "2.0", "id": 1, "method": "systemSettings.get"})
    assert settings["result"]["wxpusher_spt"] == "SPT_saved-value"

    saved = dispatcher.dispatch(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "systemSettings.set",
            "params": {"wxpusher_spt": "SPT_new-value"},
        }
    )
    assert saved["result"] is True
    assert app.config.values["wxpusher_spt"] == "SPT_new-value"

    tested = dispatcher.dispatch(
        {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "notification.test",
            "params": {"spt": "SPT_unsaved-value"},
        }
    )
    assert tested["result"] == {"accepted": True}
    assert notifications.tests == ["SPT_unsaved-value"]
    assert app.config.values["wxpusher_spt"] == "SPT_new-value"
    app.close()


def test_notification_test_rejects_invalid_spt_without_echoing_secret() -> None:
    app = make_application(notifications=FakeNotificationService())
    dispatcher = RpcDispatcher(application=app, version="test")

    response = dispatcher.dispatch(
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "notification.test",
            "params": {"spt": "not-a-real-secret"},
        }
    )

    assert response["error"]["code"] == -32602
    assert "not-a-real-secret" not in response["error"]["message"]
    app.close()


def test_task_completion_notifications_batch_progress_and_skip_manual_stop(tmp_path) -> None:
    notifications = FakeNotificationService()
    app = make_application(
        stats_path=tmp_path / "runtime_stats.json",
        notifications=notifications,
    )
    app.config.values["wxpusher_spt"] = "SPT_progress-value"
    app.stats.start_run("run-1", {"exp": 3, "thread": 2, "mirror": 1})
    app._execution_run_id = "run-1"
    app._execution_state = "running"

    app._on_task_completed("exp", 3)
    app._on_task_completed("mirror", 1)
    assert notifications.completions == [
        ("SPT_progress-value", "exp", 3),
        ("SPT_progress-value", "mirror", 1),
    ]

    app._execution_stop.set()
    app._on_task_completed("thread", 2)
    assert len(notifications.completions) == 2

    app._execution_stop.clear()
    app._queue_run_notification("run-1", "final")
    assert notifications.finals[0][0] == "SPT_progress-value"
    assert notifications.finals[0][1]["completed"] == {"exp": 3, "thread": 2, "mirror": 1}

    app._queue_run_notification("run-1", "failure", "safe failure")
    assert notifications.failures == [("SPT_progress-value", "safe failure")]
    app.close()


def test_run_execution_queues_final_failure_and_skips_manual_stop_notifications(monkeypatch, tmp_path) -> None:
    class FakeWorker:
        def __init__(self, exception=None) -> None:
            self.exception = exception

        def start(self) -> None:
            return None

        def join(self) -> None:
            return None

    worker = FakeWorker()
    fake_tasks = ModuleType("tasks.base.script_task_scheme")
    fake_tasks.my_script_task = lambda: worker
    monkeypatch.setitem(sys.modules, "tasks.base.script_task_scheme", fake_tasks)

    notifications = FakeNotificationService()
    app = make_application(stats_path=tmp_path / "normal.json", notifications=notifications)
    app.config.values["wxpusher_spt"] = "SPT_run-value"
    app.stats.start_run("normal", {"exp": 1, "thread": 0, "mirror": 0})
    app._execution_run_id = "normal"
    app._execution_state = "running"
    app._run_execution("normal")
    assert len(notifications.finals) == 1
    assert notifications.failures == []

    worker.exception = RuntimeError("failure SPT_run-value")
    app.stats.start_run("failed", {"exp": 1, "thread": 0, "mirror": 0})
    app._execution_run_id = "failed"
    app._execution_state = "running"
    app._run_execution("failed")
    assert len(notifications.failures) == 1
    assert "SPT_run-value" not in notifications.failures[0][1]

    worker.exception = None
    app.stats.start_run("stopped", {"exp": 1, "thread": 0, "mirror": 0})
    app._execution_run_id = "stopped"
    app._execution_state = "running"
    app._execution_stop.set()
    app._run_execution("stopped")
    assert len(notifications.finals) == 1
    assert len(notifications.failures) == 1
    app.close()


def test_stats_rpc_exposes_period_and_daily_summaries(tmp_path) -> None:
    app = make_application(stats_path=tmp_path / "runtime_stats.json")
    app.stats.start_run("run-1", {"exp": 2, "thread": 1, "mirror": 1})
    app._execution_run_id = "run-1"
    app._on_task_completed("exp", 2)
    app._on_task_completed("mirror", 1)
    dispatcher = RpcDispatcher(application=app, version="test")

    summary = dispatcher.dispatch({"jsonrpc": "2.0", "id": 1, "method": "stats.getSummary"})
    assert summary["result"]["currentRun"]["completed"]["exp"] == 2
    assert summary["result"]["today"]["mirror"] == 1

    daily = dispatcher.dispatch(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "stats.getDailySummary",
            "params": {"dateFrom": "2026-01-01", "dateTo": "2026-01-02"},
        }
    )
    assert daily["result"]["days"]
    assert daily["result"]["days"][0]["date"] == "2026-01-02"
    app.close()


def test_task_started_bridges_current_task_to_status_and_stats(tmp_path) -> None:
    from core.events import mediator

    app = make_application(stats_path=tmp_path / "runtime_stats.json")
    app.stats.start_run("run-1", {"exp": 1, "mirror": 1})
    app._execution_state = "running"
    app._execution_run_id = "run-1"
    events = []
    app.add_event_listener(lambda event, payload, sequence: events.append((event, payload, sequence)))

    try:
        mediator.task_started.emit("daily_task")
        mediator.task_started.emit("mirror")
        mediator.task_started.emit("unknown")

        status_tasks = [payload["currentTaskId"] for event, payload, _ in events if event == "execution.status"]
        stats_tasks = [
            payload["currentRun"]["currentTaskId"] for event, payload, _ in events if event == "execution.stats"
        ]
        assert status_tasks == ["daily_task", "mirror"]
        assert stats_tasks == ["daily_task", "mirror"]

        app._execution_state = "stopping"
        mediator.task_started.emit("get_reward")
        assert [payload["currentTaskId"] for event, payload, _ in events if event == "execution.status"] == [
            "daily_task",
            "mirror",
        ]
    finally:
        app._execution_state = "idle"
        app._execution_run_id = None
        app.close()


def test_team_contract_preserves_python_order_and_exposes_full_mirror_projection() -> None:
    app = make_application()
    app.config.config.teams = {
        "1": TeamSetting(
            team_number=1,
            chosen_sinners=[1, 1] + [0] * 10,
            sinner_order=[2, 1] + [0] * 10,
            shopping_strategy=True,
            shopping_strategy_select=5,
            reward_cards=True,
            opening_items=True,
            opening_items_system=3,
        )
    }
    app.config.values["teams_active_queue"] = [1]

    team = app.team_list()[0]

    assert team["schemaVersion"] == 2
    assert team["sinners"] == ["faust", "yi_sang"]
    assert team["enabled"] is True
    assert team["mirrorConfig"]["shopping_strategy"] is True
    assert team["mirrorConfig"]["shopping_strategy_select"] == 5
    assert team["mirrorConfig"]["reward_cards"] is True
    assert team["mirrorConfig"]["opening_items_system"] == 3
    app.close()


def test_builtin_team_preset_catalog_returns_stable_ids_and_full_team_templates() -> None:
    app = make_application()
    presets = app.team_preset_list()

    assert [preset["presetId"] for preset in presets] == ["hos_ryoshu_solo", "spiderweb_family"]
    solo = presets[0]
    assert solo["routeId"] == "hos_ryoshu_solo_route"
    assert solo["team"]["id"] == ""
    assert len(solo["team"]["sinners"]) == 12
    assert solo["team"]["mirrorConfig"]["mirror_route_profile"] == "hos_ryoshu_solo_route"
    assert solo["team"]["mirrorConfig"]["observe_ego_gift_selected"] == [
        "spiderweb_entangled_in_red"
    ]
    assert solo["team"]["mirrorConfig"]["opening_bonus"] == [3] * 10
    assert solo["team"]["mirrorConfig"]["opening_items"] is True
    assert solo["team"]["mirrorConfig"]["opening_items_system"] == 4
    assert solo["team"]["mirrorConfig"]["use_team_code"] is True
    assert solo["team"]["mirrorConfig"]["skill_replacement"] is True
    assert solo["description"]["zhCn"]
    app.close()


def test_team_save_allocates_id_and_isolates_luxcavation_from_mirror_queue() -> None:
    app = make_application()
    app.config.config.teams = {
        "1": TeamSetting(
            team_number=1,
            purpose="mirror",
            shopping_strategy=True,
            shopping_strategy_select=4,
            chosen_sinners=[1] + [0] * 11,
            sinner_order=[1] + [0] * 11,
        )
    }
    app.config.values["teams_active_queue"] = [1]

    created = app.team_save(
        {
            "id": "",
            "name": "经验本队伍",
            "purpose": "luxcavation",
            "sinners": ["faust", "yi_sang"],
            "mirrorConfig": None,
            "enabled": False,
        }
    )

    assert created["id"] == "team-2"
    assert created["purpose"] == "luxcavation"
    assert created["enabled"] is False
    assert created["mirrorConfig"] is None
    assert app.config.config.teams["2"].purpose == "luxcavation"
    assert app.config.config.teams["2"].shopping_strategy is False

    converted = app.team_save(
        {
            "id": "team-1",
            "name": "经验本改用途",
            "purpose": "luxcavation",
            "mirrorConfig": None,
            "enabled": True,
        }
    )
    assert converted["enabled"] is False
    assert converted["mirrorConfig"] is None
    assert app.config.config.teams["1"].shopping_strategy is True
    assert app.config.values["teams_active_queue"] == []
    app.close()


def test_team_save_can_bind_a_new_team_to_a_slot_and_update_enabled_partially() -> None:
    app = make_application()
    app.config.config.teams = {
        "1": TeamSetting(
            team_number=1,
            purpose="mirror",
            chosen_sinners=[1] + [0] * 11,
            sinner_order=[1] + [0] * 11,
        )
    }
    app.config.values["teams_active_queue"] = [1]

    created = app.team_save(
        {
            "id": "",
            "teamNumber": 5,
            "name": "五号镜牢",
            "purpose": "mirror",
            "sinners": ["faust"],
            "mirrorConfig": None,
            "enabled": False,
        }
    )

    assert created["id"] == "team-5"
    assert "5" in app.config.config.teams
    assert app.config.values["teams_active_queue"] == [1]

    with pytest.raises(ValueError, match="已被占用"):
        app.team_save(
            {
                "id": "",
                "teamNumber": 5,
                "name": "重复编号",
                "purpose": "mirror",
                "enabled": False,
            }
        )

    app.team_save({"id": "team-1", "enabled": False})
    assert app.config.values["teams_active_queue"] == []
    app.team_save({"id": "team-1", "enabled": True})
    assert app.config.values["teams_active_queue"] == [1]
    app.close()


def test_team_save_rejects_duplicate_and_unknown_sinners() -> None:
    app = make_application()
    common = {
        "id": "",
        "name": "invalid",
        "purpose": "mirror",
        "mirrorConfig": None,
        "enabled": False,
    }

    try:
        app.team_save({**common, "sinners": ["faust", "faust"]})
    except ValueError as error:
        assert "重复人格" in str(error)
    else:
        raise AssertionError("duplicate sinner should be rejected")
    app.config.config.teams.clear()
    with_unknown = {**common, "sinners": ["not-a-sinner"]}
    try:
        app.team_save(with_unknown)
    except ValueError as error:
        assert "未知人格" in str(error)
    else:
        raise AssertionError("unknown sinner should be rejected")
    app.close()


def test_team_save_requires_ryoshu_and_two_family_members_for_pseudo_solo() -> None:
    app = make_application()
    common = {
        "id": "",
        "name": "伪单通校验",
        "purpose": "mirror",
        "enabled": False,
        "mirrorConfig": {"defense_for_solo": True},
    }

    with pytest.raises(ValueError, match="必须包含良秀"):
        app.team_save({**common, "sinners": ["yi_sang", "faust", "don_quixote"]})

    with pytest.raises(ValueError, match="至少需要"):
        app.team_save({**common, "sinners": ["ryoshu", "faust"]})

    saved = app.team_save({**common, "sinners": ["ryoshu", "faust", "yi_sang"]})
    assert saved["sinners"] == ["ryoshu", "faust", "yi_sang"]
    assert saved["mirrorConfig"]["defense_for_solo"] is True
    app.close()


def test_builtin_team_preset_merge_preserves_custom_teams_and_queue() -> None:
    config = object.__new__(Config)
    config._defaults = {
        "team_presets_revision": 1,
        "teams": {
            "2": {"remark_name": "小指良伪单通", "team_number": 2},
            "3": {"remark_name": "蜘蛛巢全家桶", "team_number": 3},
        },
    }
    loaded = {
        "team_presets_revision": 0,
        "teams": {"1": {"remark_name": "我的队伍"}, "2": {"remark_name": "旧自定义队伍"}},
        "teams_active_queue": [1, 2],
    }

    assert config._merge_builtin_team_presets(loaded) is True
    assert loaded["teams"]["1"]["remark_name"] == "我的队伍"
    assert loaded["teams"]["2"]["remark_name"] == "旧自定义队伍"
    assert loaded["teams"]["3"]["remark_name"] == "小指良伪单通"
    assert loaded["teams"]["4"]["remark_name"] == "蜘蛛巢全家桶"
    assert loaded["teams"]["3"]["team_number"] == 3
    assert loaded["teams"]["4"]["team_number"] == 4
    assert loaded["teams_active_queue"] == [1, 2]
    assert loaded["team_presets_revision"] == 1

    removed_after_migration = {"team_presets_revision": 1, "teams": {"1": {"remark_name": "我的队伍"}}}
    assert config._merge_builtin_team_presets(removed_after_migration) is False
    assert list(removed_after_migration["teams"]) == ["1"]


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


def test_tasks_get_config_normalizes_legacy_integer_booleans() -> None:
    app = make_application()
    app.config.values["hard_mirror"] = 1
    app.config.values["no_weekly_bonuses"] = 0

    config = app.tasks_get_config()

    assert config["mirror"]["hard_mirror"] is True
    assert config["mirror"]["no_weekly_bonuses"] is False
    app.close()


def test_tasks_get_config_uses_three_thread_runs_when_value_is_missing() -> None:
    app = make_application()
    app.config.values.pop("set_thread_count")

    assert app.tasks_get_config()["daily_task"]["set_thread_count"] == 3
    app.close()


def test_tasks_get_config_preserves_explicit_zero_thread_runs() -> None:
    app = make_application()
    app.config.values["set_thread_count"] = 0

    assert app.tasks_get_config()["daily_task"]["set_thread_count"] == 0
    app.close()


def test_tasks_get_config_preserves_other_saved_thread_run_values() -> None:
    app = make_application()
    app.config.values["set_thread_count"] = 7

    assert app.tasks_get_config()["daily_task"]["set_thread_count"] == 7
    app.close()


def test_window_position_uses_python_screen_names_and_accepts_legacy_values() -> None:
    app = make_application()
    app.config.values["set_win_position"] = "2"
    app.config.config.set_win_position = "2"
    app.config.config._values["set_win_position"] = "2"

    assert app.tasks_get_config()["set_windows"]["set_win_position"] == "right_top"

    app.tasks_set_config({"set_windows": {"set_win_position": "left_top"}})
    assert app.config.values["set_win_position"] == "left_top"
    app.close()


def test_execution_and_tool_entries_reject_missing_active_device() -> None:
    app = make_application()
    app.config.values["mirror"] = True
    dispatcher = RpcDispatcher(application=app, version="test")

    execution = dispatcher.dispatch({"jsonrpc": "2.0", "id": 1, "method": "execution.start"})
    tool = dispatcher.dispatch(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tool.start",
            "params": {"id": "screenshot"},
        }
    )

    assert execution["error"]["code"] == -32020
    assert "未连接设备" in execution["error"]["message"]
    assert tool["error"]["code"] == -32020
    app.close()


def test_manager_release_stops_preview_before_controller_cleanup() -> None:
    manager = DeviceManager()
    manager._active = DeviceSession(DeviceManager._make_mumu_target(0), object())
    preview = FakePreviewCapture()
    app = BackendApplication(
        manager,
        version="test",
        config=FakeConfig(),
        theme_list=FakeThemeStore(),
        preview_capture=preview,
    )

    manager.release_after_task()

    assert preview.stop_count == 1
    app.close()
    assert preview.close_count == 1
