"""Task configuration RPC projection and validation.

The service keeps the GPUI ``TasksConfig`` shape and the compatibility
normalization rules out of :mod:`module.backend_application`.  The application
still owns the underlying config store and the persistence/validation callback.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import Any

from module.rpc_contract import SCHEMA_VERSION

WINDOW_POSITIONS = frozenset(
    {"free", "left_top", "right_top", "left_bottom", "right_bottom", "center"}
)
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


def _json_bool(value: Any, default: bool) -> bool:
    if isinstance(value, bool):
        return value
    if isinstance(value, int):
        return value != 0
    return default


def _normalise_window_position(value: Any) -> str:
    if isinstance(value, str):
        if value in WINDOW_POSITIONS:
            return value
        if value in LEGACY_WINDOW_POSITIONS:
            return LEGACY_WINDOW_POSITIONS[value]
    elif value in LEGACY_WINDOW_POSITIONS:
        return LEGACY_WINDOW_POSITIONS[value]
    return "free"


def _require_mapping(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, Mapping):
        raise ValueError(f"{name} requires an object")
    return dict(value)


def _require_string(value: Any, name: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{name} requires a string")
    return value


def _require_bool(value: Any, name: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{name} requires a boolean")
    return value


class TaskConfigService:
    """Read and patch the flat task configuration contract."""

    def __init__(
        self,
        config_snapshot: Callable[[], dict[str, Any]],
        apply_updates: Callable[[Mapping[str, Any]], None],
    ) -> None:
        self._config_snapshot = config_snapshot
        self._apply_updates = apply_updates

    def get_config(self) -> dict[str, Any]:
        raw = self._config_snapshot()
        return {
            "schemaVersion": SCHEMA_VERSION,
            "enabledTasks": {
                "daily_task": _json_bool(raw.get("daily_task"), False),
                "get_reward": _json_bool(raw.get("get_reward"), False),
                "buy_enkephalin": _json_bool(raw.get("buy_enkephalin"), False),
                "mirror": _json_bool(raw.get("mirror"), False),
                "resonate_with_Ahab": _json_bool(raw.get("resonate_with_Ahab"), False),
            },
            "set_windows": {
                key: (
                    _normalise_window_position(raw.get(key, default))
                    if key == "set_win_position"
                    else _json_bool(raw.get(key), default)
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
                key: _json_bool(raw.get(key), default)
                if isinstance(default, bool)
                else raw.get(key, default)
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
                "Dr_Grandet_mode": _json_bool(raw.get("Dr_Grandet_mode"), False),
                "skip_enkephalin": _json_bool(raw.get("skip_enkephalin"), False),
            },
            "mirror": {
                key: _json_bool(raw.get(key), default)
                if isinstance(default, bool)
                else raw.get(key, default)
                for key, default in {
                    "set_mirror_count": 1,
                    "infinite_dungeons": False,
                    "hard_mirror": False,
                    "hard_mirror_target_floors": 5,
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
                    "mirror_prefer_hatred_and_despair": False,
                    "mirror_minimize_non_boss_combat": False,
                }.items()
            },
            "resonate_with_Ahab": {
                "enabled": _json_bool(raw.get("resonate_with_Ahab"), False)
            },
            "afterCompletion": {
                "actions": list(raw.get("after_completion_actions", []) or []),
                "powerAction": raw.get("after_completion_power_action", "none"),
                "keepAfterCompletion": _json_bool(
                    raw.get("keep_after_completion"), False
                ),
            },
        }

    def set_config(self, params: Any) -> bool:
        values = _require_mapping(params, "tasks.setConfig")
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
            "hard_mirror_target_floors",
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
            "mirror_prefer_hatred_and_despair",
            "mirror_minimize_non_boss_combat",
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
                    key: _require_bool(enabled[key], f"enabledTasks.{key}")
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
            "buy_enkephalin": (
                "set_lunacy_to_enkephalin",
                "Dr_Grandet_mode",
                "skip_enkephalin",
            ),
            "mirror": (
                "set_mirror_count",
                "infinite_dungeons",
                "hard_mirror",
                "hard_mirror_target_floors",
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
                "mirror_prefer_hatred_and_despair",
                "mirror_minimize_non_boss_combat",
            ),
        }
        for section, fields in sections.items():
            section_values = values.get(section)
            if section_values is None:
                continue
            section_values = _require_mapping(
                section_values, f"tasks.setConfig.{section}"
            )
            for field in fields:
                if field in section_values:
                    updates[field] = section_values[field]

        completion = values.get("afterCompletion")
        if completion is not None:
            completion = _require_mapping(
                completion, "tasks.setConfig.afterCompletion"
            )
            if "actions" in completion:
                actions = completion["actions"]
                if not isinstance(actions, list) or not all(
                    isinstance(item, str) for item in actions
                ):
                    raise ValueError("afterCompletion.actions must be a string list")
                updates["after_completion_actions"] = list(actions)
            if "powerAction" in completion:
                updates["after_completion_power_action"] = _require_string(
                    completion["powerAction"], "afterCompletion.powerAction"
                )
            if "keepAfterCompletion" in completion:
                updates["keep_after_completion"] = _require_bool(
                    completion["keepAfterCompletion"],
                    "afterCompletion.keepAfterCompletion",
                )

        resonate = values.get("resonate_with_Ahab")
        if isinstance(resonate, Mapping) and "enabled" in resonate:
            updates["resonate_with_Ahab"] = _require_bool(
                resonate["enabled"], "resonate_with_Ahab.enabled"
            )

        self._apply_updates(updates)
        return True


__all__ = ["LEGACY_WINDOW_POSITIONS", "TaskConfigService", "WINDOW_POSITIONS"]
