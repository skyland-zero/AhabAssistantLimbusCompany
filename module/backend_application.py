"""Headless application services exposed by the GPUI sidecar.

The sidecar is the narrow runtime context shared by RPC handlers and the
WebSocket event publisher.  Services are deliberately kept behind small
methods: the transport does not need to know where configuration, devices, or
task execution are implemented.
"""

from __future__ import annotations

import copy
import os
import re
import threading
import time
import uuid
from collections.abc import Callable, Mapping
from pathlib import Path
from typing import Any

from module.config.theme_pack_catalog import theme_pack_display_name
from module.device_manager import DeviceError, DeviceManager, get_device_manager
from module.logger import log
from module.preview_capture import PreviewCapture, encode_screenshot_frame

SCHEMA_VERSION = 1

SINNER_IDS = (
    "yi_sang",
    "faust",
    "don_quixote",
    "ryoshu",
    "meursault",
    "hong_lu",
    "heathcliff",
    "ishmael",
    "rodion",
    "sinclair",
    "outis",
    "gregor",
)
SINNER_NAMES = (
    "李箱",
    "浮士德",
    "堂吉诃德",
    "良秀",
    "默尔索",
    "鸿璐",
    "希斯克利夫",
    "以实玛利",
    "罗佳",
    "辛克莱",
    "奥提斯",
    "格雷戈尔",
)
SYSTEM_NAMES = (
    "burn",
    "bleed",
    "tremor",
    "rupture",
    "poise",
    "sinking",
    "charge",
    "slash",
    "pierce",
    "blunt",
)


class BackendEventBus:
    """Thread-safe event publisher with a monotonically increasing sequence."""

    def __init__(self) -> None:
        self._lock = threading.RLock()
        self._listeners: list[Callable[[str, dict[str, Any], int], None]] = []
        self._sequence = 0

    def add_listener(self, listener: Callable[[str, dict[str, Any], int], None]) -> None:
        with self._lock:
            if listener not in self._listeners:
                self._listeners.append(listener)

    def remove_listener(self, listener: Callable[[str, dict[str, Any], int], None]) -> None:
        with self._lock:
            if listener in self._listeners:
                self._listeners.remove(listener)

    def emit(self, event: str, payload: Mapping[str, Any] | None = None) -> int:
        with self._lock:
            self._sequence += 1
            sequence = self._sequence
            listeners = list(self._listeners)
        value = dict(payload or {})
        for listener in listeners:
            try:
                listener(event, value, sequence)
            except Exception:
                log.exception("sidecar 事件监听器执行失败：%s", event)
        return sequence


class BackendApplication:
    """Runtime service context for one sidecar process.

    ``config`` and ``theme_list`` are injectable to keep protocol tests
    independent from the process-wide configuration singleton.  In the real
    sidecar they default to the existing compatibility-aware stores.
    """

    def __init__(
        self,
        device_manager: DeviceManager | Any | None = None,
        *,
        version: str = "unknown",
        shutdown: Callable[[], None] | None = None,
        config: Any | None = None,
        theme_list: Any | None = None,
        resource_service: Any | None = None,
        preview_capture: PreviewCapture | None = None,
    ) -> None:
        self.version = version
        self.events = BackendEventBus()
        self._lock = threading.RLock()
        self._shutdown = shutdown
        self._config = config
        self._theme_list = theme_list
        self._resource_service = resource_service
        self._resource_check: Any | None = None
        self._resource_plan: Any | None = None
        self._resource_run: str | None = None
        self._execution_state = "idle"
        self._execution_task_id: str | None = None
        self._execution_run_id: str | None = None
        self._execution_thread: threading.Thread | None = None
        self._execution_worker: Any | None = None
        self._execution_stop = threading.Event()
        self._tools: dict[str, dict[str, Any]] = {}
        self._tool_workers: dict[str, tuple[threading.Thread, threading.Event, Any]] = {}
        self._tool_runtimes: dict[str, Any] = {}
        self._hotkey_enabled = True
        self._hotkey_listener: Any | None = None
        self._mediator_bindings: list[tuple[Any, Callable[..., Any]]] = []
        self._preview_capture = preview_capture or PreviewCapture(self.emit)

        self.device_manager = device_manager or get_device_manager()
        if hasattr(self.device_manager, "set_busy_checker"):
            self.device_manager.set_busy_checker(self.is_busy)
        if hasattr(self.device_manager, "add_status_listener"):
            self.device_manager.add_status_listener(self._on_device_event)
        if hasattr(self.device_manager, "add_notice_listener"):
            self.device_manager.add_notice_listener(self._on_device_event)
        self._bind_core_events()

        # Hotkeys are optional on non-Windows hosts and in headless test
        # environments.  A malformed user value must not prevent RPC startup.
        self._refresh_hotkeys()

    @property
    def config(self) -> Any:
        if self._config is None:
            from module.config import cfg

            self._config = cfg
        return self._config

    @property
    def theme_store(self) -> Any:
        if self._theme_list is None:
            from module.config import theme_list

            self._theme_list = theme_list
        return self._theme_list

    def add_event_listener(self, listener: Callable[[str, dict[str, Any], int], None]) -> None:
        self.events.add_listener(listener)

    def remove_event_listener(self, listener: Callable[[str, dict[str, Any], int], None]) -> None:
        self.events.remove_listener(listener)

    def set_busy_checker(self, checker: Callable[[], bool]) -> None:
        """Override the device switch guard for embedding/tests."""
        if hasattr(self.device_manager, "set_busy_checker"):
            self.device_manager.set_busy_checker(checker)

    def emit(self, event: str, payload: Mapping[str, Any] | None = None) -> int:
        return self.events.emit(event, payload)

    def app_ping(self) -> str:
        return "pong"

    def app_version(self) -> dict[str, Any]:
        return {"ui": "gpui", "backend": self.version, "schemaVersion": SCHEMA_VERSION}

    def app_shutdown(self) -> bool:
        if self._shutdown is not None:
            self._shutdown()
        return True

    def tasks_get_config(self) -> dict[str, Any]:
        raw = self._config_snapshot()
        return {
            "schemaVersion": SCHEMA_VERSION,
            "enabledTasks": {
                "daily_task": bool(raw.get("daily_task", False)),
                "get_reward": bool(raw.get("get_reward", False)),
                "buy_enkephalin": bool(raw.get("buy_enkephalin", False)),
                "mirror": bool(raw.get("mirror", False)),
                "resonate_with_Ahab": bool(raw.get("resonate_with_Ahab", False)),
            },
            "set_windows": {
                key: raw.get(key, default)
                for key, default in {
                    "set_win_size": 1080,
                    "set_win_position": "free",
                    "set_reduce_miscontact": True,
                    "screenshot_interval": 0.5,
                    "mouse_action_interval": 0.3,
                    "mouse_down_duration": 0.1,
                    "use_post_message": False,
                }.items()
            },
            "daily_task": {
                key: raw.get(key, default)
                for key, default in {
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
                }.items()
            },
            "get_reward": {"set_get_prize": raw.get("set_get_prize", 0)},
            "buy_enkephalin": {
                "set_lunacy_to_enkephalin": raw.get("set_lunacy_to_enkephalin", 2),
                "Dr_Grandet_mode": raw.get("Dr_Grandet_mode", False),
                "skip_enkephalin": raw.get("skip_enkephalin", False),
            },
            "mirror": {
                key: raw.get(key, default)
                for key, default in {
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
                }.items()
            },
            "resonate_with_Ahab": {"enabled": bool(raw.get("resonate_with_Ahab", False))},
            "afterCompletion": {
                "actions": list(raw.get("after_completion_actions", []) or []),
                "powerAction": raw.get("after_completion_power_action", "none"),
                "keepAfterCompletion": bool(raw.get("keep_after_completion", False)),
            },
        }

    def tasks_set_config(self, params: Any) -> bool:
        values = self._require_mapping(params, "tasks.setConfig")
        updates: dict[str, Any] = {}
        known_flat_fields = {
            "daily_task",
            "get_reward",
            "buy_enkephalin",
            "mirror",
            "resonate_with_Ahab",
            "set_win_size",
            "set_win_position",
            "set_reduce_miscontact",
            "screenshot_interval",
            "mouse_action_interval",
            "mouse_down_duration",
            "use_post_message",
            "set_EXP_count",
            "set_thread_count",
            "daily_teams",
            "use_continuous_combat",
            "use_continuous_combat_select",
            "targeted_teaming_EXP",
            "EXP_day_1_2",
            "EXP_day_3_4",
            "EXP_day_5_6",
            "EXP_day_7",
            "targeted_teaming_thread",
            "thread_day_1",
            "thread_day_2",
            "thread_day_3",
            "thread_day_4",
            "thread_day_5",
            "thread_day_6",
            "thread_day_7",
            "set_get_prize",
            "set_lunacy_to_enkephalin",
            "Dr_Grandet_mode",
            "skip_enkephalin",
            "set_mirror_count",
            "infinite_dungeons",
            "hard_mirror",
            "no_weekly_bonuses",
            "floor_3_exit",
            "save_rewards",
            "hard_mirror_single_bonuses",
            "select_event_pack",
            "skip_event_pack",
            "re_claim_rewards",
            "not_skip_whitegossypium",
            "fight_to_last_man",
            "mirror_keyboard_navigation",
            "mirror_keyboard_simple_pathfinding",
            "after_completion_actions",
            "after_completion_power_action",
            "keep_after_completion",
        }
        updates.update(
            {
                key: value
                for key, value in values.items()
                if key in known_flat_fields and not isinstance(value, Mapping)
            }
        )
        enabled = values.get("enabledTasks")
        if isinstance(enabled, Mapping):
            updates.update(
                {
                    key: self._require_bool(enabled[key], f"enabledTasks.{key}")
                    for key in (
                        "daily_task",
                        "get_reward",
                        "buy_enkephalin",
                        "mirror",
                        "resonate_with_Ahab",
                    )
                    if key in enabled
                }
            )
        sections = {
            "set_windows": (
                "set_win_size",
                "set_win_position",
                "set_reduce_miscontact",
                "screenshot_interval",
                "mouse_action_interval",
                "mouse_down_duration",
                "use_post_message",
            ),
            "daily_task": (
                "set_EXP_count",
                "set_thread_count",
                "daily_teams",
                "use_continuous_combat",
                "use_continuous_combat_select",
                "targeted_teaming_EXP",
                "EXP_day_1_2",
                "EXP_day_3_4",
                "EXP_day_5_6",
                "EXP_day_7",
                "targeted_teaming_thread",
                "thread_day_1",
                "thread_day_2",
                "thread_day_3",
                "thread_day_4",
                "thread_day_5",
                "thread_day_6",
                "thread_day_7",
            ),
            "get_reward": ("set_get_prize",),
            "buy_enkephalin": ("set_lunacy_to_enkephalin", "Dr_Grandet_mode", "skip_enkephalin"),
            "mirror": (
                "set_mirror_count",
                "infinite_dungeons",
                "hard_mirror",
                "no_weekly_bonuses",
                "floor_3_exit",
                "save_rewards",
                "hard_mirror_single_bonuses",
                "select_event_pack",
                "skip_event_pack",
                "re_claim_rewards",
                "not_skip_whitegossypium",
                "fight_to_last_man",
                "mirror_keyboard_navigation",
                "mirror_keyboard_simple_pathfinding",
            ),
        }
        for section, fields in sections.items():
            section_values = values.get(section)
            if section_values is None:
                continue
            section_values = self._require_mapping(section_values, f"tasks.setConfig.{section}")
            for field in fields:
                if field in section_values:
                    updates[field] = section_values[field]

        completion = values.get("afterCompletion")
        if completion is not None:
            completion = self._require_mapping(completion, "tasks.setConfig.afterCompletion")
            if "actions" in completion:
                actions = completion["actions"]
                if not isinstance(actions, list) or not all(isinstance(item, str) for item in actions):
                    raise ValueError("afterCompletion.actions must be a string list")
                updates["after_completion_actions"] = list(actions)
            if "powerAction" in completion:
                updates["after_completion_power_action"] = self._require_string(
                    completion["powerAction"], "afterCompletion.powerAction"
                )
            if "keepAfterCompletion" in completion:
                updates["keep_after_completion"] = self._require_bool(
                    completion["keepAfterCompletion"], "afterCompletion.keepAfterCompletion"
                )

        resonate = values.get("resonate_with_Ahab")
        if isinstance(resonate, Mapping) and "enabled" in resonate:
            updates["resonate_with_Ahab"] = self._require_bool(
                resonate["enabled"], "resonate_with_Ahab.enabled"
            )

        self._apply_config_updates(updates)
        return True

    def execution_get_state(self) -> dict[str, Any]:
        with self._lock:
            return self._execution_payload()

    def execution_start(self, params: Any = None) -> dict[str, Any]:
        with self._lock:
            if self._execution_state != "idle":
                raise DeviceError("任务已经在运行")
            tasks = self.tasks_get_config()
            enabled = tasks["enabledTasks"]
            if not any(enabled.get(name, False) for name in ("daily_task", "get_reward", "buy_enkephalin", "mirror")):
                return {"accepted": False, "runId": None, "reason": "没有选择可执行任务"}
            run_id = uuid.uuid4().hex
            task_id = None
            if isinstance(params, Mapping) and isinstance(params.get("taskId"), str):
                task_id = params["taskId"]
            self._execution_state = "running"
            self._execution_task_id = task_id
            self._execution_run_id = run_id
            self._execution_stop.clear()
            self.emit("execution.status", self._execution_payload())
            worker = threading.Thread(target=self._run_execution, args=(run_id,), name="AALCExecution", daemon=True)
            self._execution_thread = worker
            worker.start()
            return {"accepted": True, "runId": run_id}

    def execution_stop(self) -> dict[str, Any]:
        with self._lock:
            if self._execution_state == "idle":
                return {"accepted": False, "runId": None}
            self._execution_stop.set()
            worker = self._execution_worker
            thread = self._execution_thread
            run_id = self._execution_run_id
        if worker is not None and hasattr(worker, "terminate"):
            # The legacy task has no complete cancellation injection point yet.
            # Ask it to finish cooperatively first; use its existing hard-stop
            # fallback only when it is still alive after a short grace period.
            try:
                if thread is not None:
                    thread.join(timeout=0.2)
                if thread is not None and thread.is_alive():
                    worker.terminate()
            except Exception:
                log.exception("停止任务线程失败")
        with self._lock:
            if self._execution_state != "idle":
                self._execution_state = "idle"
                self._execution_task_id = None
                self._execution_run_id = None
                self._execution_worker = None
                self.emit("execution.status", self._execution_payload(run_id=run_id))
        return {"accepted": True, "runId": run_id}

    def execution_pause(self) -> dict[str, Any]:
        with self._lock:
            if self._execution_state != "running":
                raise DeviceError("任务当前不可暂停")
            self._toggle_automation_pause()
            self._execution_state = "paused"
            payload = self._execution_payload()
            self.emit("execution.status", payload)
            return {"accepted": True, "runId": self._execution_run_id}

    def execution_resume(self) -> dict[str, Any]:
        with self._lock:
            if self._execution_state != "paused":
                raise DeviceError("任务当前不可恢复")
            self._toggle_automation_pause()
            self._execution_state = "running"
            payload = self._execution_payload()
            self.emit("execution.status", payload)
            return {"accepted": True, "runId": self._execution_run_id}

    def team_list(self) -> list[dict[str, Any]]:
        config = self.config
        teams = getattr(getattr(config, "config", None), "teams", {}) or {}
        queue = set(config.get_value("teams_active_queue", []) or [])
        return [self._team_detail(int(number), setting, int(number) in queue) for number, setting in sorted(teams.items(), key=self._team_sort_key)]

    def team_save(self, params: Any) -> bool:
        values = self._require_mapping(params, "team.save")
        team_id = values.get("id", "")
        if not isinstance(team_id, str):
            raise ValueError("team.save.id must be a string")
        numbers = self._team_numbers()
        match = re.fullmatch(r"team-(\d+)", team_id)
        team_number = int(match.group(1)) if match else (max(numbers, default=0) + 1)
        if team_number < 1:
            team_number = 1
        try:
            from module.config import TeamSetting
        except ImportError as error:
            raise RuntimeError(f"无法加载队伍配置模型：{error}") from error
        current = getattr(self.config.config, "teams", {}).get(str(team_number))
        setting = current.model_copy(deep=True) if current is not None else TeamSetting(team_number=team_number)
        current_name = getattr(setting, "remark_name", None) or f"编队 {team_number}"
        name = self._require_string(values.get("name", current_name), "team.save.name").strip()
        if not name:
            raise ValueError("队伍名称不能为空")
        setting.remark_name = name
        if "sinners" in values:
            self._write_team_sinners(setting, values["sinners"])
        if "mirrorConfig" in values and values["mirrorConfig"] is not None:
            mirror = self._require_mapping(values["mirrorConfig"], "team.save.mirrorConfig")
            self._write_mirror_setting(setting, mirror)
        elif "accessoryScheme" in values:
            self._write_accessory_scheme(setting, values["accessoryScheme"])
        if "purpose" in values:
            purpose = self._require_string(values["purpose"], "team.save.purpose")
            if purpose not in {"mirror", "luxcavation", "general"}:
                raise ValueError("team.save.purpose 无效")
            setting.purpose = purpose
        self.config.config.teams[str(team_number)] = setting
        enabled = values.get("enabled", self._team_enabled(team_number))
        if not isinstance(enabled, bool):
            raise ValueError("team.save.enabled requires a boolean")
        if hasattr(self.config, "set_team_enabled"):
            self.config.set_team_enabled(team_number, enabled)
        self._persist_config()
        return True

    def team_delete(self, params: Any) -> bool:
        values = self._require_mapping(params, "team.delete")
        team_id = self._require_string(values.get("id"), "team.delete.id")
        match = re.fullmatch(r"team-(\d+)", team_id)
        if match is None:
            raise ValueError("team.delete.id 无效")
        number = int(match.group(1))
        self.config.config.teams.pop(str(number), None)
        if hasattr(self.config, "remove_team_from_queue"):
            self.config.remove_team_from_queue(number)
        if hasattr(self.theme_store, "delete_team_weight_config"):
            self.theme_store.delete_team_weight_config(number)
        self._persist_config()
        return True

    def sinner_list(self) -> list[dict[str, str]]:
        return [{"id": sinner_id, "name": name} for sinner_id, name in zip(SINNER_IDS, SINNER_NAMES, strict=True)]

    def theme_pack_list(self) -> dict[str, Any]:
        store = self.theme_store
        config = getattr(store, "config", {}) or {}
        packs: list[dict[str, Any]] = []
        seen: set[str] = set()
        for section_name in ("theme_pack_list", "theme_pack_list_hard"):
            section = config.get(section_name, {}) or {}
            if not isinstance(section, Mapping):
                continue
            for pack_id, raw_weight in section.items():
                pack_id = str(pack_id)
                if pack_id in seen:
                    continue
                seen.add(pack_id)
                try:
                    raw_weight = int(raw_weight)
                except (TypeError, ValueError):
                    raw_weight = 0
                packs.append(
                    {
                        "id": pack_id,
                        "name": theme_pack_display_name(
                            pack_id,
                            hard=section_name.endswith("hard"),
                        ),
                        "weight": min(10, abs(raw_weight)),
                        "enabled": raw_weight >= 0,
                        "tier": "HARD" if section_name.endswith("hard") else "NORMAL",
                    }
                )
        return {
            "schemaVersion": SCHEMA_VERSION,
            "hardMirrorActive": bool(self._config_value("hard_mirror", False)),
            "packs": packs,
        }

    def theme_pack_update_all(self, params: Any) -> bool:
        values = self._require_mapping(params, "themePack.updateAll")
        packs = values.get("packs")
        if not isinstance(packs, list):
            raise ValueError("themePack.updateAll.packs must be a list")
        store = self.theme_store
        config = copy.deepcopy(getattr(store, "config", {}) or {})
        sections = [config.get(name, {}) for name in ("theme_pack_list", "theme_pack_list_hard")]
        known = {str(key) for section in sections if isinstance(section, Mapping) for key in section}
        for pack in packs:
            pack = self._require_mapping(pack, "themePack.updateAll.pack")
            pack_id = self._require_string(pack.get("id"), "themePack.id")
            if pack_id not in known:
                raise ValueError(f"未知主题包：{pack_id}")
            try:
                weight = int(pack.get("weight", 0))
            except (TypeError, ValueError) as error:
                raise ValueError(f"主题包权重无效：{pack_id}") from error
            if not 0 <= weight <= 10 or not isinstance(pack.get("enabled"), bool):
                raise ValueError(f"主题包权重或启用状态无效：{pack_id}")
            raw_weight = weight if pack["enabled"] else -max(1, weight)
            for section_name in ("theme_pack_list", "theme_pack_list_hard"):
                section = config.get(section_name)
                if isinstance(section, dict) and pack_id in section:
                    section[pack_id] = raw_weight
        store.config = config
        store.save_config(path=store.theme_pack_list_path, config_data=config)
        return True

    def theme_pack_reset_weights(self) -> dict[str, Any]:
        store = self.theme_store
        example_path = Path("assets/config/theme_pack_list.example.yaml")
        if example_path.is_file():
            default = store.yaml.load(example_path.read_text(encoding="utf-8")) or {}
            store.config = copy.deepcopy(default)
            store.save_config(path=store.theme_pack_list_path, config_data=store.config)
        return self.theme_pack_list()

    def resource_status(self) -> list[dict[str, Any]]:
        state_path = Path("assets/config/image_resource_state.json")
        local_version = "unknown"
        try:
            import json

            value = json.loads(state_path.read_text(encoding="utf-8"))
            local_version = str(value.get("last_applied_manifest_id") or "unknown")
        except (OSError, ValueError, TypeError):
            pass
        return [
            {
                "id": "images",
                "name": "图片资源",
                "localVersion": local_version,
                "remoteVersion": None,
                "lastSyncAt": None,
            }
        ]

    def resource_check_update(self) -> list[dict[str, Any]]:
        service = self._get_resource_service()
        result = service.check_for_updates(self._config_value("image_resource_source", "Auto"))
        self._resource_check = result
        groups = self.resource_status()
        if result.remote_manifest is not None:
            remote_version = result.remote_manifest.manifest_id
            for group in groups:
                group["remoteVersion"] = remote_version
        if result.status.value == "error":
            raise RuntimeError(result.message or "资源更新检查失败")
        return groups

    def resource_sync_start(self, params: Any = None) -> dict[str, Any]:
        with self._lock:
            if self._resource_run is not None:
                raise DeviceError("资源同步已经在运行")
            run_id = uuid.uuid4().hex
            self._resource_run = run_id
        scope = "all"
        if isinstance(params, Mapping) and isinstance(params.get("scope"), str):
            scope = params["scope"]
        worker = threading.Thread(target=self._run_resource_sync, args=(run_id, scope), name="AALCResourceSync", daemon=True)
        worker.start()
        return {"accepted": True, "runId": run_id}

    def tool_start(self, params: Any) -> dict[str, Any]:
        tool_id = self._tool_id(params)
        with self._lock:
            existing = self._tool_workers.get(tool_id)
            if existing is not None and existing[0].is_alive():
                return {"accepted": False, "runId": existing[2]}
            run_id = uuid.uuid4().hex
            stop_event = threading.Event()
            runtime = {"runId": run_id, "running": True}
            self._tools[tool_id] = runtime
            self.emit("tool.status", {"toolId": tool_id, "running": True, "runId": run_id})
            worker = threading.Thread(
                target=self._run_tool,
                args=(tool_id, run_id, stop_event),
                name=f"AALCTool-{tool_id}",
                daemon=True,
            )
            self._tool_workers[tool_id] = (worker, stop_event, run_id)
            worker.start()
            return {"accepted": True, "runId": run_id}

    def tool_stop(self, params: Any) -> dict[str, Any]:
        tool_id = self._tool_id(params)
        with self._lock:
            runtime = self._tool_workers.get(tool_id)
            if runtime is None:
                self._tools[tool_id] = {"runId": None, "running": False}
                self.emit("tool.status", {"toolId": tool_id, "running": False})
                return {"accepted": False, "runId": None}
            run_id = runtime[2]
            runtime[1].set()
            tool_runtime = self._tool_runtimes.get(tool_id)
            if tool_runtime is not None and hasattr(tool_runtime, "running"):
                tool_runtime.running = False
            self.emit("tool.status", {"toolId": tool_id, "running": False, "runId": run_id})
            return {"accepted": True, "runId": run_id}

    def tool_screenshot(self) -> dict[str, Any]:
        from module.automation import auto

        image = auto.take_screenshot(gray=False)
        if image is None:
            raise RuntimeError("截图失败，请确认设备已连接且游戏处于运行状态")
        directory = Path("screenshots")
        directory.mkdir(parents=True, exist_ok=True)
        path = directory / f"screenshot_{time.strftime('%Y%m%d_%H%M%S')}.jpg"
        image.save(path, format="JPEG", quality=92)
        try:
            self.emit(
                "screenshot.frame",
                encode_screenshot_frame(image, "default", max_width=None, quality=80),
            )
        except Exception:
            log.debug("截图事件编码失败", exc_info=True)
        return {"path": str(path), "accepted": True}

    def hotkey_get(self) -> dict[str, Any]:
        return {
            "schemaVersion": SCHEMA_VERSION,
            "startStop": self._config_value("shutdown_hotkey", "<ctrl>+q"),
            "pauseResume": self._config_value("pause_hotkey", "<alt>+p"),
            "enabled": self._hotkey_enabled,
        }

    def hotkey_set(self, params: Any) -> bool:
        values = self._require_mapping(params, "hotkey.set")
        updates: dict[str, Any] = {}
        if "startStop" in values:
            updates["shutdown_hotkey"] = self._require_string(values["startStop"], "hotkey.startStop")
        if "pauseResume" in values:
            pause = values["pauseResume"]
            if pause is not None and not isinstance(pause, str):
                raise ValueError("hotkey.pauseResume must be a string or null")
            updates["pause_hotkey"] = pause or ""
            updates["resume_hotkey"] = pause or ""
        if "enabled" in values:
            self._hotkey_enabled = self._require_bool(values["enabled"], "hotkey.enabled")
        self._apply_config_updates(updates)
        self._refresh_hotkeys()
        return True

    def system_settings_get(self) -> dict[str, Any]:
        keys = (
            "simulator",
            "simulator_type",
            "simulator_port",
            "start_emulator_timeout",
            "memory_protection",
            "minimize_to_tray",
            "autostart",
            "experimental_keep_screen_awake",
            "experimental_hdr_warning",
            "update_prerelease_enable",
            "update_source",
            "mirrorchyan_cdk",
        )
        result = {key: self._config_value(key, None) for key in keys}
        result["schemaVersion"] = SCHEMA_VERSION
        return result

    def system_settings_set(self, params: Any) -> bool:
        values = self._require_mapping(params, "systemSettings.set")
        allowed = {
            "simulator",
            "simulator_type",
            "simulator_port",
            "start_emulator_timeout",
            "memory_protection",
            "minimize_to_tray",
            "autostart",
            "experimental_keep_screen_awake",
            "experimental_hdr_warning",
            "update_prerelease_enable",
            "update_source",
            "mirrorchyan_cdk",
        }
        self._apply_config_updates({key: value for key, value in values.items() if key in allowed})
        return True

    def app_check_update(self) -> dict[str, Any]:
        """Run the existing framework-free update checker synchronously.

        The caller already executes RPC work away from the WebSocket loop.  A
        synchronous checker here keeps the legacy source fallback and version
        comparison in one place; GPUI receives the result without importing
        update implementation details.
        """

        try:
            from module.update.checker import UpdateThread

            checker = UpdateThread(timeout=10, flag=False)
            checker.run()
            latest = checker.new_version or self.version
            return {
                "schemaVersion": SCHEMA_VERSION,
                "updateAvailable": bool(latest and latest != self.version and not checker.is_current_version_latest),
                "latest": latest,
            }
        except Exception as error:
            log.warning("更新检查失败：%s", error)
            return {"schemaVersion": SCHEMA_VERSION, "updateAvailable": False, "latest": self.version}

    def close(self) -> None:
        self._preview_capture.close()
        self.execution_stop()
        for tool_id in tuple(self._tool_workers):
            try:
                self.tool_stop({"id": tool_id})
            except Exception:
                log.exception("关闭工具失败：%s", tool_id)
        self._stop_hotkeys()
        for event, handler in self._mediator_bindings:
            try:
                event.disconnect(handler)
            except Exception:
                log.debug("解绑核心事件失败", exc_info=True)
        self._mediator_bindings.clear()
        if hasattr(self.device_manager, "remove_status_listener"):
            self.device_manager.remove_status_listener(self._on_device_event)
        if hasattr(self.device_manager, "remove_notice_listener"):
            self.device_manager.remove_notice_listener(self._on_device_event)
        if hasattr(self.device_manager, "close"):
            self.device_manager.close()

    def is_busy(self) -> bool:
        with self._lock:
            return self._execution_state != "idle"

    def _run_execution(self, run_id: str) -> None:
        worker = None
        try:
            from tasks.base.script_task_scheme import my_script_task

            worker = my_script_task()
            with self._lock:
                if self._execution_run_id != run_id:
                    return
                self._execution_worker = worker
            worker.start()
            worker.join()
            if getattr(worker, "exception", None) is not None:
                raise worker.exception
            self.emit("app.notice", {"level": "info", "message": "所有任务已完成"})
        except Exception as error:
            log.exception("任务执行失败")
            self.emit("app.notice", {"level": "error", "message": f"任务执行失败：{error}"})
        finally:
            with self._lock:
                if self._execution_run_id == run_id:
                    self._execution_state = "idle"
                    self._execution_task_id = None
                    self._execution_run_id = None
                    self._execution_thread = None
                    self._execution_worker = None
                    self.emit("execution.status", self._execution_payload(run_id=run_id))

    def _run_resource_sync(self, run_id: str, scope: str) -> None:
        try:
            service = self._get_resource_service()
            result = service.check_for_updates(self._config_value("image_resource_source", "Auto"))
            self._resource_check = result
            if result.remote_manifest is None:
                raise RuntimeError(result.message or "远端资源清单不可用")
            plan = service.build_sync_plan(result.remote_manifest)
            self._resource_plan = plan
            if not plan.has_changes:
                service.accept_remote_state(result)
                self.emit("resource.sync.progress", {"scope": scope, "progress": 100, "runId": run_id})
                return

            def progress(current: int, total: int) -> None:
                value = 100 if total <= 0 else max(0, min(100, round(current * 100 / total)))
                self.emit("resource.sync.progress", {"scope": scope, "progress": value, "runId": run_id})

            service.apply_sync_plan(check_result=result, sync_plan=plan, progress_callback=progress)
            self.emit("resource.sync.progress", {"scope": scope, "progress": 100, "runId": run_id})
        except Exception as error:
            self.emit("app.notice", {"level": "error", "message": f"资源同步失败：{error}", "runId": run_id})
            log.exception("资源同步失败")
        finally:
            with self._lock:
                if self._resource_run == run_id:
                    self._resource_run = None

    def _run_tool(self, tool_id: str, run_id: str, stop_event: threading.Event) -> None:
        runtime = None
        try:
            if tool_id == "infinite_battle":
                from tasks.base.script_task_scheme import init_game
                from tasks.battle.battle import Battle

                init_game()
                runtime = Battle(is_tool=True)
                with self._lock:
                    self._tool_runtimes[tool_id] = runtime
                runtime.fight(infinite_battle=True)
            elif tool_id == "enkephalin":
                from tasks.base.back_init_menu import back_init_menu
                from tasks.base.make_enkephalin_module import make_enkephalin_module

                while not stop_event.is_set():
                    back_init_menu(allow_restart=False)
                    make_enkephalin_module(cancel=False, skip=False)
                    stop_event.wait(1.0)
            elif tool_id == "screenshot":
                self.tool_screenshot()
            else:
                raise ValueError(f"未知工具：{tool_id}")
        except Exception as error:
            if not stop_event.is_set():
                self.emit("app.notice", {"level": "error", "message": f"工具 {tool_id} 失败：{error}"})
                log.exception("工具执行失败：%s", tool_id)
        finally:
            if runtime is not None and hasattr(runtime, "running"):
                runtime.running = False
            with self._lock:
                self._tools[tool_id] = {"runId": run_id, "running": False}
                self._tool_workers.pop(tool_id, None)
                self._tool_runtimes.pop(tool_id, None)
                self.emit("tool.status", {"toolId": tool_id, "running": False, "runId": run_id})

    def _get_resource_service(self) -> Any:
        if self._resource_service is None:
            from module.resource_sync.service import ResourceSyncService

            self._resource_service = ResourceSyncService()
        return self._resource_service

    def _toggle_automation_pause(self) -> None:
        try:
            from module.automation import auto

            auto.set_pause()
        except Exception as error:
            log.debug("切换自动化暂停状态失败：%s", error)

    def _config_snapshot(self) -> dict[str, Any]:
        config = getattr(self.config, "config", None)
        if config is None:
            return {}
        if hasattr(config, "model_dump"):
            return copy.deepcopy(config.model_dump())
        if isinstance(config, Mapping):
            return copy.deepcopy(dict(config))
        return {}

    def _config_value(self, key: str, default: Any = None) -> Any:
        try:
            value = self.config.get_value(key, default)
        except Exception:
            value = default
        return copy.deepcopy(value) if isinstance(value, (dict, list, set)) else value

    def _apply_config_updates(self, updates: Mapping[str, Any]) -> None:
        if not updates:
            return
        with self._lock:
            for key, value in updates.items():
                self._validate_config_value(key, value)
                if hasattr(self.config, "unsaved_set_value"):
                    self.config.unsaved_set_value(key, copy.deepcopy(value), stacklevel=3)
                elif hasattr(self.config, "set_value"):
                    self.config.set_value(key, copy.deepcopy(value))
                else:
                    setattr(self.config.config, key, copy.deepcopy(value))
            self._persist_config()

    def _validate_config_value(self, key: str, value: Any) -> None:
        current = getattr(getattr(self.config, "config", None), key, None)
        if current is None:
            return
        if isinstance(current, bool):
            if not isinstance(value, bool):
                raise ValueError(f"{key} requires a boolean")
            return
        if isinstance(current, int) and not isinstance(current, bool):
            if not isinstance(value, int) or isinstance(value, bool):
                raise ValueError(f"{key} requires an integer")
            if key in {"set_mirror_count"} and not 1 <= value <= 99:
                raise ValueError(f"{key} must be between 1 and 99")
            if key in {"use_continuous_combat_select"} and not 1 <= value <= 10:
                raise ValueError(f"{key} must be between 1 and 10")
            if key in {"set_get_prize"} and not 0 <= value <= 2:
                raise ValueError(f"{key} must be between 0 and 2")
            return
        if isinstance(current, float):
            if not isinstance(value, (int, float)) or isinstance(value, bool):
                raise ValueError(f"{key} requires a number")
            return
        if isinstance(current, str) and not isinstance(value, str):
            raise ValueError(f"{key} requires a string")

    def _persist_config(self) -> None:
        try:
            self.config.save(instant=True)
        except TypeError:
            self.config.save()

    def _execution_payload(self, *, run_id: str | None = None) -> dict[str, Any]:
        return {
            "state": self._execution_state,
            "currentTaskId": self._execution_task_id,
            "runId": run_id if run_id is not None else self._execution_run_id,
            "schemaVersion": SCHEMA_VERSION,
        }

    def _on_device_event(self, event: str, payload: dict[str, Any]) -> None:
        self.emit(event, payload)
        if event != "device.status":
            return

        status = payload.get("status")
        device_id = payload.get("deviceId")
        try:
            if status == "connected" and isinstance(device_id, str) and device_id:
                self._preview_capture.start(device_id)
            elif status in {"connecting", "disconnected"}:
                self._preview_capture.stop()
        except Exception as error:
            log.exception("实时预览生命周期处理失败")
            if isinstance(device_id, str) and device_id:
                self.emit(
                    "preview.status",
                    {
                        "deviceId": device_id,
                        "status": "error",
                        "error": str(error),
                    },
                )

    def _bind_core_events(self) -> None:
        """Bridge framework-free core events into the RPC event contract."""
        try:
            from core.events import mediator

            bindings = (
                (mediator.mirror_signal, self._on_mirror_signal),
                (mediator.warning, self._on_core_warning),
                (mediator.hdr_warning, self._on_hdr_warning),
                (mediator.update_progress, self._on_update_progress),
                (mediator.download_complete, self._on_download_complete),
            )
            for event, handler in bindings:
                event.connect(handler)
                self._mediator_bindings.append((event, handler))
        except Exception:
            # A core event bridge is an enhancement; it must not stop the
            # sidecar from serving read-only RPC when an optional module is
            # unavailable in a packaged environment.
            log.debug("核心事件桥接不可用", exc_info=True)

    def _on_mirror_signal(self, current: Any, total: Any) -> None:
        self.emit(
            "execution.mirrorProgress",
            {
                "current": max(0, int(current)),
                "total": max(0, int(total)),
                "isHard": bool(self._config_value("hard_mirror", False)),
                "isInfinite": bool(self._config_value("infinite_dungeons", False)),
                "runId": self._execution_run_id,
            },
        )

    def _on_core_warning(self, message: Any) -> None:
        self.emit("app.notice", {"level": "warn", "message": str(message)})

    def _on_hdr_warning(self, acknowledgement: Any) -> None:
        self.emit(
            "app.notice",
            {"level": "warn", "message": "检测到游戏所在显示器已开启 HDR，可能影响图像识别"},
        )
        if hasattr(acknowledgement, "set"):
            acknowledgement.set()

    def _on_update_progress(self, progress: Any) -> None:
        self.emit("app.notice", {"level": "info", "message": f"更新下载进度：{int(progress)}%"})

    def _on_download_complete(self, file_name: Any) -> None:
        self.emit("app.notice", {"level": "info", "message": f"更新下载完成：{file_name}"})

    def _team_numbers(self) -> list[int]:
        teams = getattr(self.config.config, "teams", {}) or {}
        numbers: list[int] = []
        for key in teams:
            try:
                number = int(key)
            except (TypeError, ValueError):
                continue
            if number > 0:
                numbers.append(number)
        return sorted(numbers)

    @staticmethod
    def _team_sort_key(item: tuple[Any, Any]) -> tuple[int, str]:
        try:
            return int(item[0]), str(item[0])
        except (TypeError, ValueError):
            return 0, str(item[0])

    def _team_detail(self, number: int, setting: Any, enabled: bool) -> dict[str, Any]:
        values = setting.model_dump() if hasattr(setting, "model_dump") else dict(setting)
        chosen = values.get("chosen_sinners", []) or []
        sinners = [SINNER_IDS[index] for index, selected in enumerate(chosen[: len(SINNER_IDS)]) if selected]
        mirror = {
            "team_system": values.get("team_system", 0),
            "shop_strategy": values.get("shop_strategy", 0),
            "discard_systems": {
                name: bool(values.get(f"system_{name}", False)) for name in SYSTEM_NAMES
            },
        }
        fields = (
            "do_not_heal",
            "do_not_buy",
            "do_not_fuse",
            "do_not_sell",
            "do_not_enhance",
            "only_aggressive_fuse",
            "do_not_system_fuse",
            "only_system_fuse",
            "aggressive_also_enhance",
            "aggressive_save_systems",
            "after_level_IV",
            "after_level_IV_select",
            "max_keyword_refresh",
            "max_normal_refresh",
            "second_system",
            "second_system_select",
            "second_system_setting",
            "avoid_skill_3",
            "prioritize_skill_3",
            "re_formation_each_floor",
            "defense_first_round",
            "defense_for_solo",
            "defense_for_solo_turns",
            "skill_replacement",
            "skill_replacement_select",
            "skill_replacement_mode",
            "use_starlight",
            "opening_bonus",
            "fixed_team_use",
            "fixed_team_use_select",
            "use_team_code",
            "team_code",
            "use_custom_theme_pack_weight",
            "observe_ego_gift",
            "observe_ego_gift_selected",
        )
        mirror.update({field: copy.deepcopy(values.get(field, getattr(self._team_default(), field, None))) for field in fields})
        mirror["ignore_shop"] = [bool(value) for value in (values.get("ignore_shop", []) or [])[:5]]
        mirror["second_system_fuse_IV"] = bool((values.get("second_system_action", []) or [True])[0])
        mirror["second_system_buy"] = bool((values.get("second_system_action", []) or [True, True])[1])
        mirror["second_system_select_reward"] = bool((values.get("second_system_action", []) or [True, True, True])[2])
        mirror["second_system_power_up"] = bool((values.get("second_system_action", []) or [True, True, True, True])[3])
        purpose = values.get("purpose", "mirror")
        if purpose not in {"mirror", "luxcavation", "general"}:
            purpose = "mirror"
        return {
            "id": f"team-{number}",
            "name": values.get("remark_name") or f"编队 {number}",
            "sinners": sinners,
            "purpose": purpose,
            "accessoryScheme": SYSTEM_NAMES[int(values.get("team_system", 0)) % len(SYSTEM_NAMES)],
            "enabled": enabled,
            "mirrorConfig": mirror,
        }

    @staticmethod
    def _team_default() -> Any:
        from module.config import TeamSetting

        return TeamSetting()

    def _write_team_sinners(self, setting: Any, sinners: Any) -> None:
        if not isinstance(sinners, list) or not all(isinstance(item, str) for item in sinners):
            raise ValueError("team.save.sinners must be a string list")
        selected = [0] * len(SINNER_IDS)
        order = [0] * len(SINNER_IDS)
        for position, sinner_id in enumerate(sinners[: len(SINNER_IDS)], start=1):
            if sinner_id not in SINNER_IDS:
                raise ValueError(f"未知人格：{sinner_id}")
            index = SINNER_IDS.index(sinner_id)
            selected[index] = 1
            order[index] = position
        setting.chosen_sinners = selected
        setting.sinner_order = order
        setting.sinners_be_select = sum(selected)

    def _team_enabled(self, number: int) -> bool:
        return number in set(self.config.get_value("teams_active_queue", []) or [])

    @staticmethod
    def _write_accessory_scheme(setting: Any, value: Any) -> None:
        scheme = BackendApplication._require_string(value, "team.save.accessoryScheme")
        if scheme in SYSTEM_NAMES:
            setting.team_system = SYSTEM_NAMES.index(scheme)

    @staticmethod
    def _write_mirror_setting(setting: Any, mirror: Mapping[str, Any]) -> None:
        discard = mirror.get("discard_systems")
        if discard is not None and not isinstance(discard, Mapping):
            raise ValueError("team.save.mirrorConfig.discard_systems must be an object")
        for key, value in mirror.items():
            if key in {"discard_systems", "second_system_fuse_IV", "second_system_buy", "second_system_select_reward", "second_system_power_up"}:
                continue
            if hasattr(setting, key):
                setattr(setting, key, copy.deepcopy(value))
        if isinstance(discard, Mapping):
            for system in SYSTEM_NAMES:
                if system in discard:
                    setattr(setting, f"system_{system}", BackendApplication._require_bool(
                        discard[system], f"team.save.mirrorConfig.discard_systems.{system}"
                    ))
        if "ignore_shop" in mirror:
            ignore_shop = mirror["ignore_shop"]
            if not isinstance(ignore_shop, list) or not all(isinstance(value, bool) for value in ignore_shop):
                raise ValueError("team.save.mirrorConfig.ignore_shop must be a boolean list")
            setting.ignore_shop = [1 if value else 0 for value in ignore_shop[:5]]
            setting.ignore_shop += [0] * (5 - len(setting.ignore_shop))
        if any(
            key in mirror
            for key in (
                "second_system_fuse_IV",
                "second_system_buy",
                "second_system_select_reward",
                "second_system_power_up",
            )
        ):
            actions = list(getattr(setting, "second_system_action", [0, 0, 0, 0]) or [])[:4]
            actions += [0] * (4 - len(actions))
            keys = (
                "second_system_fuse_IV",
                "second_system_buy",
                "second_system_select_reward",
                "second_system_power_up",
            )
            for index, key in enumerate(keys):
                if key in mirror:
                    actions[index] = 1 if BackendApplication._require_bool(
                        mirror[key], f"team.save.mirrorConfig.{key}"
                    ) else 0
            setting.second_system_action = actions

    @staticmethod
    def _require_mapping(value: Any, name: str) -> dict[str, Any]:
        if not isinstance(value, Mapping):
            raise ValueError(f"{name} requires an object")
        return dict(value)

    @staticmethod
    def _require_string(value: Any, name: str) -> str:
        if not isinstance(value, str):
            raise ValueError(f"{name} requires a string")
        return value

    @staticmethod
    def _require_bool(value: Any, name: str) -> bool:
        if not isinstance(value, bool):
            raise ValueError(f"{name} requires a boolean")
        return value

    @staticmethod
    def _tool_id(params: Any) -> str:
        values = BackendApplication._require_mapping(params, "tool call")
        tool_id = values.get("id")
        if tool_id not in {"infinite_battle", "enkephalin", "screenshot"}:
            raise ValueError("tool call requires a valid id")
        return str(tool_id)

    def _refresh_hotkeys(self) -> None:
        self._stop_hotkeys()
        if not self._hotkey_enabled or os.name != "nt":
            return
        try:
            from module.hotkey_listener import ExactGlobalHotKeys

            start_stop = self._config_value("shutdown_hotkey", "")
            pause = self._config_value("pause_hotkey", "")
            resume = self._config_value("resume_hotkey", "")
            hotkeys = {}
            if start_stop:
                hotkeys[start_stop] = self._hotkey_start_stop
            if pause:
                hotkeys[pause] = self._hotkey_pause_resume
            if resume and resume != pause:
                hotkeys[resume] = self._hotkey_pause_resume
            if hotkeys:
                listener = ExactGlobalHotKeys(hotkeys)
                listener.start()
                self._hotkey_listener = listener
        except (ImportError, OSError, ValueError) as error:
            log.warning("全局快捷键未启动：%s", error)

    def _stop_hotkeys(self) -> None:
        listener = self._hotkey_listener
        self._hotkey_listener = None
        if listener is not None:
            try:
                listener.stop()
            except Exception:
                log.debug("停止全局快捷键失败", exc_info=True)

    def _hotkey_start_stop(self) -> None:
        if self.is_busy():
            self.execution_stop()
        else:
            self.execution_start()

    def _hotkey_pause_resume(self) -> None:
        try:
            if self._execution_state == "running":
                self.execution_pause()
            elif self._execution_state == "paused":
                self.execution_resume()
        except Exception:
            log.exception("快捷键切换任务状态失败")


__all__ = ["BackendApplication", "BackendEventBus", "SCHEMA_VERSION"]
