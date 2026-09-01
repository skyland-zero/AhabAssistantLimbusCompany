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

from core.team_squad import validate_pseudo_solo_selection
from module.config.redaction import redact_text
from module.config.theme_pack_catalog import theme_pack_display_name
from module.device_manager import DeviceError, DeviceManager, get_device_manager
from module.execution_stats import ExecutionStatsStore
from module.logger import log
from module.observe_ego_gift import normalize_observe_ego_gifts
from module.preview_capture import PreviewCapture, encode_screenshot_frame

SCHEMA_VERSION = 2
WINDOW_POSITIONS = frozenset({"free", "left_top", "right_top", "left_bottom", "right_bottom", "center"})
LEGACY_WINDOW_POSITIONS = {
    "0": "center",
    "1": "left_top",
    "2": "right_top",
    "3": "free",
    0: "center",
    1: "left_top",
    2: "right_top",
    3: "free",
}

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

TEAM_PURPOSES = frozenset({"mirror", "luxcavation", "general"})
TEAM_MIRROR_BOOL_FIELDS = frozenset(
    {
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
        "second_system",
        "avoid_skill_3",
        "prioritize_skill_3",
        "re_formation_each_floor",
        "defense_first_round",
        "defense_for_solo",
        "skill_replacement",
        "use_starlight",
        "fixed_team_use",
        "use_team_code",
        "use_custom_theme_pack_weight",
        "observe_ego_gift",
        "reward_cards",
        "shopping_strategy",
        "opening_items",
    }
)
TEAM_MIRROR_INT_LIMITS = {
    "team_system": (0, len(SYSTEM_NAMES) - 1),
    "shop_strategy": (0, 2),
    "after_level_IV_select": (0, 3),
    "max_keyword_refresh": (0, 10),
    "max_normal_refresh": (0, 10),
    "second_system_select": (0, len(SYSTEM_NAMES) - 1),
    "second_system_setting": (0, 1),
    "defense_for_solo_turns": (1, 5),
    "skill_replacement_select": (0, 255),
    "skill_replacement_mode": (0, 1),
    "fixed_team_use_select": (0, 2),
    "reward_cards_select": (0, 255),
    "shopping_strategy_select": (0, 255),
    "opening_items_select": (0, 255),
    "opening_items_system": (0, len(SYSTEM_NAMES) - 1),
}
TEAM_MIRROR_ACTION_FIELDS = (
    "second_system_fuse_IV",
    "second_system_buy",
    "second_system_select_reward",
    "second_system_power_up",
)
EXECUTABLE_TASK_IDS = ("daily_task", "get_reward", "buy_enkephalin", "mirror")


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
            # Listener callbacks are intentionally serialized with sequence
            # allocation so another emitter cannot publish seq N+1 first.
            for listener in listeners:
                try:
                    # A listener may enrich its local view; don't let that
                    # mutate the payload observed by later subscribers.
                    listener(event, dict(value), sequence)
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
        stats_path: str | Path | None = None,
        notifications: Any | None = None,
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
        self._preview_enabled = True
        self._preview_device_id: str | None = None
        self._closed = False
        if stats_path is None:
            from module import CONFIG_PATH

            stats_path = Path(CONFIG_PATH).with_name("runtime_stats.json")
        self.stats = ExecutionStatsStore(stats_path)
        if notifications is None:
            from module.notification.wxpusher import NotificationService

            notifications = NotificationService()
        self.notifications = notifications

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

    def preview_set_enabled(self, params: Any) -> dict[str, bool]:
        values = self._require_mapping(params, "preview.setEnabled")
        enabled = values.get("enabled")
        if not isinstance(enabled, bool):
            raise ValueError("preview.setEnabled.enabled must be a boolean")

        with self._lock:
            if enabled != self._preview_enabled:
                self._preview_enabled = enabled
                if not enabled:
                    self._preview_capture.stop()
                elif self._preview_device_id:
                    self._preview_capture.start(self._preview_device_id)

            fallback_running = self._preview_enabled and self._preview_device_id is not None
            running = bool(getattr(self._preview_capture, "running", fallback_running))
            return {"enabled": self._preview_enabled, "running": running}

    def stats_get_summary(self) -> dict[str, Any]:
        return self.stats.summary()

    def stats_get_daily_summary(self, params: Any = None) -> dict[str, Any]:
        values = params if isinstance(params, Mapping) else {}
        date_from = values.get("dateFrom")
        date_to = values.get("dateTo")
        if date_from is not None and not isinstance(date_from, str):
            raise ValueError("dateFrom must be a YYYY-MM-DD string")
        if date_to is not None and not isinstance(date_to, str):
            raise ValueError("dateTo must be a YYYY-MM-DD string")
        return self.stats.daily_summary(date_from=date_from, date_to=date_to)

    def tasks_get_config(self) -> dict[str, Any]:
        raw = self._config_snapshot()
        return {
            "schemaVersion": SCHEMA_VERSION,
            "enabledTasks": {
                "daily_task": self._json_bool(raw.get("daily_task"), False),
                "get_reward": self._json_bool(raw.get("get_reward"), False),
                "buy_enkephalin": self._json_bool(raw.get("buy_enkephalin"), False),
                "mirror": self._json_bool(raw.get("mirror"), False),
                "resonate_with_Ahab": self._json_bool(raw.get("resonate_with_Ahab"), False),
            },
            "set_windows": {
                key: (
                    self._normalise_window_position(raw.get(key, default))
                    if key == "set_win_position"
                    else self._json_bool(raw.get(key), default)
                    if isinstance(default, bool)
                    else raw.get(key, default)
                )
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
                key: self._json_bool(raw.get(key), default) if isinstance(default, bool) else raw.get(key, default)
                for key, default in {
                    "set_EXP_count": 1,
                    "set_thread_count": 3,
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
                "Dr_Grandet_mode": self._json_bool(raw.get("Dr_Grandet_mode"), False),
                "skip_enkephalin": self._json_bool(raw.get("skip_enkephalin"), False),
            },
            "mirror": {
                key: self._json_bool(raw.get(key), default) if isinstance(default, bool) else raw.get(key, default)
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
            "resonate_with_Ahab": {"enabled": self._json_bool(raw.get("resonate_with_Ahab"), False)},
            "afterCompletion": {
                "actions": list(raw.get("after_completion_actions", []) or []),
                "powerAction": raw.get("after_completion_power_action", "none"),
                "keepAfterCompletion": self._json_bool(raw.get("keep_after_completion"), False),
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
            {key: value for key, value in values.items() if key in known_flat_fields and not isinstance(value, Mapping)}
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
            updates["resonate_with_Ahab"] = self._require_bool(resonate["enabled"], "resonate_with_Ahab.enabled")

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
            self._require_active_runtime()
            run_id = uuid.uuid4().hex
            task_id = None
            if isinstance(params, Mapping) and isinstance(params.get("taskId"), str):
                task_id = params["taskId"]
            if task_id not in EXECUTABLE_TASK_IDS:
                task_id = next((name for name in EXECUTABLE_TASK_IDS if enabled.get(name)), None)
            self._execution_state = "running"
            self._execution_task_id = task_id
            self._execution_run_id = run_id
            self._execution_stop.clear()
            self.emit(
                "execution.stats",
                self.stats.start_run(
                    run_id,
                    self._execution_targets(tasks),
                    current_task_id=task_id,
                ),
            )
            self.emit("execution.status", self._execution_payload())
            worker = threading.Thread(target=self._run_execution, args=(run_id,), name="AALCExecution", daemon=True)
            self._execution_thread = worker
            worker.start()
            return {"accepted": True, "runId": run_id, "state": "running"}

    def execution_stop(self) -> dict[str, Any]:
        worker = None
        with self._lock:
            if self._execution_state == "idle":
                return {"accepted": False, "runId": None, "state": "idle"}
            if self._execution_state == "stopping":
                return {"accepted": True, "runId": self._execution_run_id, "state": "stopping"}
            self._execution_stop.set()
            worker = self._execution_worker
            run_id = self._execution_run_id
            self._execution_state = "stopping"
            self.emit("execution.status", self._execution_payload())
            self.emit("execution.stats", self.stats.set_state("stopping"))
        # The event is the primary cancellation mechanism.  Notify the legacy
        # worker as well so it can stop its retry monitor and wake any
        # worker-owned cancellation hooks without resorting to thread killing.
        if worker is not None:
            terminate = getattr(worker, "terminate", None)
            if callable(terminate):
                try:
                    terminate()
                except Exception:
                    log.debug("通知任务线程停止失败", exc_info=True)
        return {"accepted": True, "runId": run_id, "state": "stopping"}

    def execution_pause(self) -> dict[str, Any]:
        with self._lock:
            if self._execution_state != "running":
                raise DeviceError("任务当前不可暂停")
            self._toggle_automation_pause()
            self._execution_state = "paused"
            payload = self._execution_payload()
            self.emit("execution.status", payload)
            self.emit("execution.stats", self.stats.set_state("paused"))
            return {"accepted": True, "runId": self._execution_run_id}

    def execution_resume(self) -> dict[str, Any]:
        with self._lock:
            if self._execution_state != "paused":
                raise DeviceError("任务当前不可恢复")
            self._toggle_automation_pause()
            self._execution_state = "running"
            payload = self._execution_payload()
            self.emit("execution.status", payload)
            self.emit("execution.stats", self.stats.set_state("running"))
            return {"accepted": True, "runId": self._execution_run_id}

    def team_list(self) -> list[dict[str, Any]]:
        config = self.config
        teams = getattr(getattr(config, "config", None), "teams", {}) or {}
        queue = {
            int(number)
            for number in (config.get_value("teams_active_queue", []) or [])
            if isinstance(number, int) and not isinstance(number, bool)
        }
        details: list[dict[str, Any]] = []
        for raw_number, setting in sorted(teams.items(), key=self._team_sort_key):
            try:
                number = int(raw_number)
            except TypeError, ValueError:
                continue
            if number > 0:
                details.append(self._team_detail(number, setting, number in queue))
        return details

    def team_preset_list(self) -> list[dict[str, Any]]:
        """Return the read-only built-in team preset catalog.

        Presets are templates rather than persisted teams.  Their complete
        team payload is normalized through the same projection used by
        ``team.list`` so the GPUI picker can save it without knowing Python's
        legacy ``chosen_sinners``/``sinner_order`` representation.
        """

        from module.team_presets import builtin_team_presets

        return builtin_team_presets(self._team_detail)

    def team_save(self, params: Any) -> dict[str, Any]:
        values = self._require_mapping(params, "team.save")
        team_id = values.get("id", "")
        if not isinstance(team_id, str):
            raise ValueError("team.save.id must be a string")
        numbers = self._team_numbers()
        if team_id:
            match = re.fullmatch(r"team-(\d+)", team_id)
            if match is None:
                raise ValueError("team.save.id 无效")
            team_number = int(match.group(1))
            if "teamNumber" in values:
                raise ValueError("team.save.teamNumber 只允许用于新建队伍")
        else:
            if "teamNumber" in values:
                team_number = self._require_int(values["teamNumber"], "team.save.teamNumber", minimum=1)
                if team_number in numbers:
                    raise ValueError(f"编队编号 {team_number} 已被占用")
            else:
                team_number = max(numbers, default=0) + 1
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
        purpose = getattr(setting, "purpose", "mirror") or "mirror"
        if purpose not in TEAM_PURPOSES:
            purpose = "mirror"
        if "purpose" in values:
            purpose = self._require_string(values["purpose"], "team.save.purpose")
            if purpose not in TEAM_PURPOSES:
                raise ValueError("team.save.purpose 无效")
        setting.purpose = purpose

        # Luxcavation teams deliberately do not expose mirror settings in the
        # GPUI contract.  A null mirrorConfig means "preserve the stored
        # mirror settings" when an existing team is changed back later; it is
        # never a request to erase the compatibility fields in TeamSetting.
        if purpose != "luxcavation":
            if "mirrorConfig" in values and values["mirrorConfig"] is not None:
                mirror = self._require_mapping(values["mirrorConfig"], "team.save.mirrorConfig")
                self._write_mirror_setting(setting, mirror)
            elif "accessoryScheme" in values:
                self._write_accessory_scheme(setting, values["accessoryScheme"])
            if getattr(setting, "defense_for_solo", False):
                validate_pseudo_solo_selection(setting.chosen_sinners)

        if hasattr(setting, "team_number"):
            setting.team_number = team_number
        requested_enabled = values.get("enabled", self._team_enabled(team_number))
        if not isinstance(requested_enabled, bool):
            raise ValueError("team.save.enabled requires a boolean")
        self.config.config.teams[str(team_number)] = setting
        if purpose == "luxcavation":
            self._remove_team_from_queue(team_number)
            enabled = False
        else:
            self._set_team_enabled(team_number, requested_enabled)
            enabled = requested_enabled
        self._persist_config()
        return self._team_detail(team_number, setting, enabled)

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
                except TypeError, ValueError:
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
        except OSError, ValueError, TypeError:
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
        worker = threading.Thread(
            target=self._run_resource_sync, args=(run_id, scope), name="AALCResourceSync", daemon=True
        )
        worker.start()
        return {"accepted": True, "runId": run_id}

    def tool_start(self, params: Any) -> dict[str, Any]:
        tool_id = self._tool_id(params)
        self._require_active_runtime()
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
        active_session = self._require_active_runtime()
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
                encode_screenshot_frame(
                    image,
                    active_session.target.info.id,
                    max_width=None,
                    quality=80,
                ),
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
            "wxpusher_spt",
        )
        result = {key: self._config_value(key, None) for key in keys}
        # Old injected/config snapshots may not have the newly introduced
        # credential field; keep the Rust model's string contract stable.
        result["wxpusher_spt"] = self._config_value("wxpusher_spt", "") or ""
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
            "wxpusher_spt",
        }
        self._apply_config_updates({key: value for key, value in values.items() if key in allowed})
        return True

    def notification_test(self, params: Any) -> dict[str, Any]:
        values = self._require_mapping(params, "notification.test")
        spt = values.get("spt", self._config_value("wxpusher_spt", ""))
        if not isinstance(spt, str):
            raise ValueError("SPT 必须是字符串")
        normalized_spt = spt.strip()
        if not normalized_spt:
            raise ValueError("SPT 未配置")
        if len(normalized_spt) <= 4 or not normalized_spt.startswith("SPT_"):
            raise ValueError("SPT 格式无效")
        try:
            self.notifications.send_test(normalized_spt)
        except Exception as error:
            raise ValueError(redact_text(error, (spt,))) from error
        return {"accepted": True}

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
            if not checker.new_version:
                return {
                    "schemaVersion": SCHEMA_VERSION,
                    "status": "failed",
                    "error": checker.error_msg or "无法从更新源获取版本信息",
                }
            latest = checker.new_version
            update_available = bool(latest != self.version and not checker.is_current_version_latest)
            return {
                "schemaVersion": SCHEMA_VERSION,
                "status": "available" if update_available else "up_to_date",
                "updateAvailable": update_available,
                "latest": latest,
            }
        except Exception as error:
            log.warning("更新检查失败：%s", error)
            return {"schemaVersion": SCHEMA_VERSION, "status": "failed", "error": str(error)}

    def close(self) -> None:
        with self._lock:
            if self._closed:
                return
            self._closed = True
        self._preview_capture.close()
        self.execution_stop()
        execution_thread = self._execution_thread
        if execution_thread is not None and execution_thread is not threading.current_thread():
            # Give cooperative cancellation a short grace period before
            # closing device resources.  The thread is daemonized so process
            # shutdown remains bounded even if legacy code is blocked in an
            # external API; no interpreter-level hard kill is attempted.
            execution_thread.join(timeout=2.0)
        close_notifications = getattr(self.notifications, "close", None)
        if callable(close_notifications):
            try:
                close_notifications()
            except Exception:
                log.exception("关闭通知服务失败")
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
        flush = getattr(self.config, "flush", None)
        if callable(flush):
            flush()

    def is_busy(self) -> bool:
        with self._lock:
            return self._execution_state != "idle"

    def _require_active_runtime(self) -> Any:
        """Validate and rebind the device selected by the sidecar UI."""
        binder = getattr(self.device_manager, "bind_active_runtime", None)
        if callable(binder):
            return binder()

        session = getattr(self.device_manager, "active_session", None)
        if session is None:
            raise DeviceError("未连接设备，请先选择并连接设备")
        target = getattr(session, "target", None)
        if getattr(target, "kind", None) in ("mumu", "adb") and getattr(session, "controller", None) is None:
            raise DeviceError("当前设备会话缺少模拟器控制器")
        return session

    def _run_execution(self, run_id: str) -> None:
        worker = None
        completed_normally = False
        failure_message: str | None = None
        manual_stop = False
        try:
            from core.execution_control import bind_cancel_event
            from tasks.base.script_task_scheme import my_script_task

            bind_cancel_event(self._execution_stop)
            worker = my_script_task()
            with self._lock:
                if self._execution_run_id != run_id:
                    return
                self._execution_worker = worker
            worker.start()
            worker.join()
            worker_error = getattr(worker, "exception", None)
            if worker_error is not None:
                from module.my_error.my_error import userStopError

                manual_stop = isinstance(worker_error, userStopError)
                if not manual_stop:
                    raise worker_error
            manual_stop = manual_stop or self._execution_stop.is_set()
            if not manual_stop:
                completed_normally = True
                self.emit("app.notice", {"level": "info", "message": "所有任务已完成"})
        except Exception as error:
            manual_stop = manual_stop or self._execution_stop.is_set()
            if not manual_stop:
                failure_message = redact_text(
                    error,
                    (self._config_value("wxpusher_spt", ""),),
                )
                # The sanitized message is sufficient for the sidecar event;
                # avoid logging an unsanitized traceback that could contain a
                # credential supplied by a legacy task/plugin.
                log.error("任务执行失败：%s", failure_message)
                self.emit("app.notice", {"level": "error", "message": f"任务执行失败：{failure_message}"})
        finally:
            from core.execution_control import bind_cancel_event

            bind_cancel_event(None)
            if completed_normally:
                self._queue_run_notification(run_id, "final")
            elif failure_message is not None:
                self._queue_run_notification(run_id, "failure", failure_message)
            stats_payload = self.stats.finish_run(run_id)
            with self._lock:
                if self._execution_run_id == run_id:
                    self._execution_state = "idle"
                    self._execution_task_id = None
                    self._execution_run_id = None
                    self._execution_thread = None
                    self._execution_worker = None
                    self.emit("execution.stats", stats_payload)
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
            # Keep the GPUI progress indicator recoverable when a sync fails;
            # app.notice is intentionally routed to the log surface, while
            # this typed progress event lets the resources page clear its
            # in-flight state without parsing user-facing text.
            self.emit(
                "resource.sync.progress",
                {"scope": scope, "progress": 0, "runId": run_id, "error": str(error)},
            )
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

    @staticmethod
    def _json_bool(value: Any, default: bool) -> bool:
        if isinstance(value, bool):
            return value
        if isinstance(value, int):
            return value != 0
        return default

    def _apply_config_updates(self, updates: Mapping[str, Any]) -> None:
        if not updates:
            return
        with self._lock:
            for key, value in updates.items():
                if key == "set_win_position":
                    value = self._canonical_window_position(value)
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
        if key == "wxpusher_spt" and value.strip() and not value.strip().startswith("SPT_"):
            raise ValueError("SPT 格式无效")
        if key == "set_win_position" and value not in WINDOW_POSITIONS:
            raise ValueError(f"{key} contains an unknown position")

    @staticmethod
    def _canonical_window_position(value: Any) -> str:
        value = LEGACY_WINDOW_POSITIONS.get(value, value)
        if isinstance(value, str) and value in WINDOW_POSITIONS:
            return value
        raise ValueError("set_win_position contains an unknown position")

    @staticmethod
    def _normalise_window_position(value: Any) -> str:
        try:
            return BackendApplication._canonical_window_position(value)
        except ValueError:
            return "free"

    def _persist_config(self) -> None:
        self.config.save()

    def _execution_payload(self, *, run_id: str | None = None) -> dict[str, Any]:
        return {
            "state": self._execution_state,
            "currentTaskId": self._execution_task_id,
            "runId": run_id if run_id is not None else self._execution_run_id,
            "schemaVersion": SCHEMA_VERSION,
        }

    @staticmethod
    def _execution_targets(tasks: Mapping[str, Any]) -> dict[str, Any]:
        enabled = tasks.get("enabledTasks", {})
        daily = tasks.get("daily_task", {})
        mirror = tasks.get("mirror", {})
        infinite = bool(mirror.get("infinite_dungeons", False))
        return {
            "exp": int(daily.get("set_EXP_count", 0)) if enabled.get("daily_task") else 0,
            "thread": int(daily.get("set_thread_count", 0)) if enabled.get("daily_task") else 0,
            "mirror": int(mirror.get("set_mirror_count", 0)) if enabled.get("mirror") and not infinite else 0,
            "mirrorInfinite": infinite and bool(enabled.get("mirror")),
        }

    def _on_device_event(self, event: str, payload: dict[str, Any]) -> None:
        self.emit(event, payload)
        if event != "device.status":
            return

        status = payload.get("status")
        device_id = payload.get("deviceId")
        try:
            with self._lock:
                if status == "connected" and isinstance(device_id, str) and device_id:
                    self._preview_device_id = device_id
                    if self._preview_enabled:
                        self._preview_capture.start(device_id)
                elif status in {"connecting", "disconnected"}:
                    self._preview_device_id = None
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
                (mediator.task_started, self._on_task_started),
                (mediator.task_completed, self._on_task_completed),
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

    def _on_task_started(self, task_id: Any) -> None:
        if not isinstance(task_id, str) or task_id not in EXECUTABLE_TASK_IDS:
            return
        with self._lock:
            run_id = self._execution_run_id
            if self._execution_state != "running" or run_id is None:
                return
            if self._execution_task_id == task_id:
                return
            stats_payload = self.stats.set_current_task(task_id, run_id=run_id)
            if stats_payload is None:
                return
            self._execution_task_id = task_id
            self.emit("execution.stats", stats_payload)
            self.emit("execution.status", self._execution_payload())

    def _on_task_completed(self, kind: Any, count: Any = 1) -> None:
        run_id = self._execution_run_id
        if run_id is None:
            return
        try:
            amount = int(count)
            payload = self.stats.record_completion(str(kind), amount, run_id=run_id)
        except TypeError, ValueError:
            return
        if payload is not None:
            self.emit("execution.stats", payload)
            if self._execution_state == "running" and not self._execution_stop.is_set():
                self._queue_notification("enqueue_completion", str(kind), amount)

    def _queue_run_notification(self, run_id: str, outcome: str, error: str | None = None) -> None:
        current_run = self.stats.summary().get("currentRun", {})
        if current_run.get("runId") != run_id:
            return
        if outcome == "final":
            self._queue_notification("enqueue_final", current_run)
        elif outcome == "failure" and error is not None:
            self._queue_notification("enqueue_failure", error)

    def _queue_notification(self, operation: str, *args: Any) -> None:
        spt = self._config_value("wxpusher_spt", "")
        if not isinstance(spt, str) or not spt.strip():
            return
        try:
            getattr(self.notifications, operation)(spt, *args)
        except Exception as error:
            log.warning("任务通知入队失败：%s", redact_text(error, (spt,)))

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
            except TypeError, ValueError:
                continue
            if number > 0:
                numbers.append(number)
        return sorted(numbers)

    @staticmethod
    def _team_sort_key(item: tuple[Any, Any]) -> tuple[int, str]:
        try:
            return int(item[0]), str(item[0])
        except TypeError, ValueError:
            return 0, str(item[0])

    def _team_detail(self, number: int, setting: Any, enabled: bool) -> dict[str, Any]:
        values = setting.model_dump() if hasattr(setting, "model_dump") else dict(setting)
        defaults = self._team_default()
        purpose = values.get("purpose", "mirror")
        if purpose not in TEAM_PURPOSES:
            purpose = "mirror"
        chosen = values.get("chosen_sinners", []) or []
        order = values.get("sinner_order", []) or []
        selected_indexes = [
            index for index, selected in enumerate(chosen[: len(SINNER_IDS)]) if selected and index < len(SINNER_IDS)
        ]
        ordered_indexes: dict[int, int] = {}
        for index in selected_indexes:
            position = order[index] if index < len(order) else 0
            if (
                isinstance(position, int)
                and not isinstance(position, bool)
                and 1 <= position <= len(selected_indexes)
                and position not in ordered_indexes
            ):
                ordered_indexes[position] = index
        ordered_selected = [ordered_indexes[position] for position in sorted(ordered_indexes)]
        ordered_selected.extend(index for index in selected_indexes if index not in ordered_selected)
        sinners = [SINNER_IDS[index] for index in ordered_selected]

        team_system = self._team_int_value(
            values.get("team_system", getattr(defaults, "team_system", 0)),
            default=0,
            minimum=0,
            maximum=len(SYSTEM_NAMES) - 1,
        )
        mirror: dict[str, Any] = {
            "team_system": team_system,
            "shop_strategy": self._team_int_value(
                values.get("shop_strategy", getattr(defaults, "shop_strategy", 0)),
                default=0,
                minimum=0,
                maximum=2,
            ),
            "discard_systems": {
                name: self._team_bool_value(
                    values.get(f"system_{name}", False),
                    default=False,
                )
                for name in SYSTEM_NAMES
            },
        }

        for field in TEAM_MIRROR_BOOL_FIELDS:
            mirror[field] = self._team_bool_value(
                values.get(field, getattr(defaults, field, False)),
                default=False,
            )
        for field, (minimum, maximum) in TEAM_MIRROR_INT_LIMITS.items():
            mirror[field] = self._team_int_value(
                values.get(field, getattr(defaults, field, minimum)),
                default=minimum,
                minimum=minimum,
                maximum=maximum,
            )

        opening_bonus = values.get("opening_bonus", getattr(defaults, "opening_bonus", [])) or []
        if not isinstance(opening_bonus, list):
            opening_bonus = []
        mirror["opening_bonus"] = [
            self._team_int_value(value, default=0, minimum=0, maximum=3) for value in opening_bonus[:10]
        ]
        mirror["opening_bonus"] += [0] * (10 - len(mirror["opening_bonus"]))

        ignore_shop = values.get("ignore_shop", getattr(defaults, "ignore_shop", [])) or []
        if not isinstance(ignore_shop, list):
            ignore_shop = []
        mirror["ignore_shop"] = [self._team_bool_value(value, default=False) for value in ignore_shop[:5]]
        mirror["ignore_shop"] += [False] * (5 - len(mirror["ignore_shop"]))

        team_code = values.get("team_code", getattr(defaults, "team_code", ""))
        mirror["team_code"] = team_code if isinstance(team_code, str) else ""
        route_profile = values.get(
            "mirror_route_profile",
            getattr(defaults, "mirror_route_profile", ""),
        )
        mirror["mirror_route_profile"] = route_profile if isinstance(route_profile, str) else ""
        selected_gifts = (
            values.get(
                "observe_ego_gift_selected",
                getattr(defaults, "observe_ego_gift_selected", []),
            )
            or []
        )
        mirror["observe_ego_gift_selected"] = (
            normalize_observe_ego_gifts(selected_gifts) if isinstance(selected_gifts, list) else []
        )

        actions = values.get("second_system_action", getattr(defaults, "second_system_action", [])) or []
        if not isinstance(actions, list):
            actions = []
        default_actions = getattr(defaults, "second_system_action", [0, 0, 0, 0]) or [0, 0, 0, 0]
        for index, field in enumerate(TEAM_MIRROR_ACTION_FIELDS):
            raw_value = actions[index] if index < len(actions) else default_actions[index]
            mirror[field] = self._team_bool_value(raw_value, default=False)

        return {
            "schemaVersion": SCHEMA_VERSION,
            "id": f"team-{number}",
            "name": values.get("remark_name") or f"编队 {number}",
            "sinners": sinners,
            "purpose": purpose,
            "accessoryScheme": SYSTEM_NAMES[team_system],
            "enabled": False if purpose == "luxcavation" else bool(enabled),
            "mirrorConfig": None if purpose == "luxcavation" else mirror,
        }

    @staticmethod
    def _team_default() -> Any:
        from module.config import TeamSetting

        return TeamSetting()

    def _write_team_sinners(self, setting: Any, sinners: Any) -> None:
        if not isinstance(sinners, list) or not all(isinstance(item, str) for item in sinners):
            raise ValueError("team.save.sinners must be a string list")
        if len(sinners) > len(SINNER_IDS):
            raise ValueError(f"team.save.sinners 最多允许 {len(SINNER_IDS)} 名人格")
        if len(set(sinners)) != len(sinners):
            raise ValueError("team.save.sinners 不允许重复人格")
        selected = [0] * len(SINNER_IDS)
        order = [0] * len(SINNER_IDS)
        for position, sinner_id in enumerate(sinners, start=1):
            if sinner_id not in SINNER_IDS:
                raise ValueError(f"未知人格：{sinner_id}")
            index = SINNER_IDS.index(sinner_id)
            selected[index] = 1
            order[index] = position
        setting.chosen_sinners = selected
        setting.sinner_order = order
        setting.sinners_be_select = sum(selected)

    def _team_enabled(self, number: int) -> bool:
        return number in {
            int(value)
            for value in (self.config.get_value("teams_active_queue", []) or [])
            if isinstance(value, int) and not isinstance(value, bool)
        }

    def _set_team_enabled(self, number: int, enabled: bool) -> None:
        if hasattr(self.config, "set_team_enabled"):
            self.config.set_team_enabled(number, enabled)
            return
        queue = [
            int(value)
            for value in (self.config.get_value("teams_active_queue", []) or [])
            if isinstance(value, int) and not isinstance(value, bool) and int(value) > 0
        ]
        if enabled and number not in queue:
            queue.append(number)
        if not enabled:
            queue = [value for value in queue if value != number]
        self._write_team_queue(queue)

    def _remove_team_from_queue(self, number: int) -> None:
        if hasattr(self.config, "remove_team_from_queue"):
            self.config.remove_team_from_queue(number)
            return
        queue = [
            int(value)
            for value in (self.config.get_value("teams_active_queue", []) or [])
            if isinstance(value, int) and not isinstance(value, bool) and int(value) != number
        ]
        self._write_team_queue(queue)

    def _write_team_queue(self, queue: list[int]) -> None:
        normalized: list[int] = []
        for value in queue:
            if value > 0 and value not in normalized:
                normalized.append(value)
        if hasattr(self.config, "unsaved_set_value"):
            self.config.unsaved_set_value("teams_active_queue", normalized, stacklevel=3)
        else:
            setattr(self.config.config, "teams_active_queue", normalized)

    @staticmethod
    def _write_accessory_scheme(setting: Any, value: Any) -> None:
        scheme = BackendApplication._require_string(value, "team.save.accessoryScheme")
        if scheme not in SYSTEM_NAMES:
            raise ValueError("team.save.accessoryScheme 无效")
        setting.team_system = SYSTEM_NAMES.index(scheme)

    @staticmethod
    def _write_mirror_setting(setting: Any, mirror: Mapping[str, Any]) -> None:
        discard = mirror.get("discard_systems")
        if discard is not None and not isinstance(discard, Mapping):
            raise ValueError("team.save.mirrorConfig.discard_systems must be an object")
        for key, value in mirror.items():
            if key == "discard_systems" or key in TEAM_MIRROR_ACTION_FIELDS:
                continue
            if key in TEAM_MIRROR_BOOL_FIELDS:
                setattr(setting, key, BackendApplication._require_bool(value, f"team.save.mirrorConfig.{key}"))
                continue
            if key in TEAM_MIRROR_INT_LIMITS:
                minimum, maximum = TEAM_MIRROR_INT_LIMITS[key]
                setattr(
                    setting,
                    key,
                    BackendApplication._require_int(
                        value,
                        f"team.save.mirrorConfig.{key}",
                        minimum=minimum,
                        maximum=maximum,
                    ),
                )
                continue
            if key == "team_code":
                setattr(setting, key, BackendApplication._require_string(value, f"team.save.mirrorConfig.{key}"))
                continue
            if key == "mirror_route_profile":
                setattr(
                    setting,
                    key,
                    BackendApplication._require_string(value, f"team.save.mirrorConfig.{key}").strip(),
                )
                continue
            if key == "opening_bonus":
                values = BackendApplication._require_int_list(value, f"team.save.mirrorConfig.{key}")
                if len(values) > 10 or any(item < 0 or item > 3 for item in values):
                    raise ValueError(
                        "team.save.mirrorConfig.opening_bonus must contain 0-3 values and have at most 10 items"
                    )
                values += [0] * (10 - len(values))
                setattr(setting, key, values)
                continue
            if key == "observe_ego_gift_selected":
                values = BackendApplication._require_string_list(value, f"team.save.mirrorConfig.{key}")
                setattr(setting, key, normalize_observe_ego_gifts(values))
                continue
            if key == "ignore_shop":
                values = BackendApplication._require_bool_list(value, f"team.save.mirrorConfig.{key}")
                if len(values) > 5:
                    raise ValueError("team.save.mirrorConfig.ignore_shop must have at most 5 items")
                values += [False] * (5 - len(values))
                setattr(setting, key, [1 if item else 0 for item in values])
                continue
            if key == "second_system_action":
                values = BackendApplication._require_int_list(value, f"team.save.mirrorConfig.{key}")
                if len(values) > 4 or any(item not in {0, 1} for item in values):
                    raise ValueError(
                        "team.save.mirrorConfig.second_system_action must contain 0/1 values and have at most 4 items"
                    )
                values += [0] * (4 - len(values))
                setattr(setting, key, values)
        if isinstance(discard, Mapping):
            for system in SYSTEM_NAMES:
                if system in discard:
                    setattr(
                        setting,
                        f"system_{system}",
                        BackendApplication._require_bool(
                            discard[system], f"team.save.mirrorConfig.discard_systems.{system}"
                        ),
                    )
        if any(key in mirror for key in TEAM_MIRROR_ACTION_FIELDS):
            actions = list(getattr(setting, "second_system_action", [0, 0, 0, 0]) or [])[:4]
            actions += [0] * (4 - len(actions))
            for index, key in enumerate(TEAM_MIRROR_ACTION_FIELDS):
                if key in mirror:
                    actions[index] = (
                        1 if BackendApplication._require_bool(mirror[key], f"team.save.mirrorConfig.{key}") else 0
                    )
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
    def _require_int(value: Any, name: str, *, minimum: int = 0, maximum: int | None = None) -> int:
        if not isinstance(value, int) or isinstance(value, bool):
            raise ValueError(f"{name} requires an integer")
        if value < minimum or (maximum is not None and value > maximum):
            if maximum is None:
                raise ValueError(f"{name} must be at least {minimum}")
            raise ValueError(f"{name} must be between {minimum} and {maximum}")
        return value

    @staticmethod
    def _require_int_list(value: Any, name: str) -> list[int]:
        if not isinstance(value, list) or not all(
            isinstance(item, int) and not isinstance(item, bool) for item in value
        ):
            raise ValueError(f"{name} must be an integer list")
        return list(value)

    @staticmethod
    def _require_bool_list(value: Any, name: str) -> list[bool]:
        if not isinstance(value, list) or not all(isinstance(item, bool) for item in value):
            raise ValueError(f"{name} must be a boolean list")
        return list(value)

    @staticmethod
    def _require_string_list(value: Any, name: str) -> list[str]:
        if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
            raise ValueError(f"{name} must be a string list")
        return list(value)

    @staticmethod
    def _require_bool(value: Any, name: str) -> bool:
        if not isinstance(value, bool):
            raise ValueError(f"{name} requires a boolean")
        return value

    @staticmethod
    def _team_bool_value(value: Any, *, default: bool) -> bool:
        if isinstance(value, bool):
            return value
        if isinstance(value, int):
            return value != 0
        return default

    @staticmethod
    def _team_int_value(value: Any, *, default: int, minimum: int, maximum: int) -> int:
        if not isinstance(value, int) or isinstance(value, bool):
            return default
        return min(max(value, minimum), maximum)

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
