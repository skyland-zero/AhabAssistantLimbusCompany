"""Team configuration and statistics RPC service.

The service owns the legacy ``TeamSetting`` projection, validation and queue
updates.  ``BackendApplication`` supplies the configuration/theme stores, the
persistence callback and the application lock, so this module never imports the
sidecar composition root.
"""

from __future__ import annotations

import math
import re
from collections.abc import Callable, Mapping
from typing import Any

from core.team_squad import validate_pseudo_solo_selection
from module.observe_ego_gift import normalize_observe_ego_gifts
from module.rpc_contract import SCHEMA_VERSION

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
        "use_damage_p",
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
    "skill_replacement_select": (0, 3),
    "skill_replacement_mode": (0, 4),
    "fixed_team_use_select": (0, 2),
    "reward_cards_select": (0, 3),
    "shopping_strategy_select": (0, 5),
    "opening_items_select": (0, 5),
    "opening_items_system": (0, len(SYSTEM_NAMES) - 1),
}
TEAM_MIRROR_ACTION_FIELDS = (
    "second_system_fuse_IV",
    "second_system_buy",
    "second_system_select_reward",
    "second_system_power_up",
)


def _require_mapping(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, Mapping):
        raise ValueError(f"{name} requires an object")
    return dict(value)


def _require_string(value: Any, name: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{name} requires a string")
    return value


def _require_int(value: Any, name: str, *, minimum: int = 0, maximum: int | None = None) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise ValueError(f"{name} requires an integer")
    if value < minimum or (maximum is not None and value > maximum):
        if maximum is None:
            raise ValueError(f"{name} must be at least {minimum}")
        raise ValueError(f"{name} must be between {minimum} and {maximum}")
    return value


def _require_int_list(value: Any, name: str) -> list[int]:
    if not isinstance(value, list) or not all(
        isinstance(item, int) and not isinstance(item, bool) for item in value
    ):
        raise ValueError(f"{name} must be an integer list")
    return list(value)


def _require_bool_list(value: Any, name: str) -> list[bool]:
    if not isinstance(value, list) or not all(isinstance(item, bool) for item in value):
        raise ValueError(f"{name} must be a boolean list")
    return list(value)


def _require_string_list(value: Any, name: str) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise ValueError(f"{name} must be a string list")
    return list(value)


def _require_bool(value: Any, name: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{name} requires a boolean")
    return value


def _team_bool_value(value: Any, *, default: bool) -> bool:
    if isinstance(value, bool):
        return value
    if isinstance(value, int):
        return value != 0
    return default


def _team_int_value(value: Any, *, default: int, minimum: int, maximum: int) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        return default
    return min(max(value, minimum), maximum)


def _team_number_from_id(team_id: str, name: str) -> int:
    match = re.fullmatch(r"team-(\d+)", team_id)
    if match is None or not 1 <= int(match.group(1)) <= 0xFFFFFFFF:
        raise ValueError(f"{name} 无效")
    return int(match.group(1))


def _team_stats_bucket(setting: Any, mode: str) -> dict[str, Any]:
    time_values = getattr(setting, f"total_mirror_time_{mode}", [])
    if not isinstance(time_values, list):
        time_values = []
    averages: list[float] = []
    for value in time_values[:3]:
        if not isinstance(value, (int, float)) or isinstance(value, bool):
            averages.append(0.0)
            continue
        try:
            numeric = float(value)
        except (TypeError, ValueError, OverflowError):
            averages.append(0.0)
            continue
        averages.append(max(0.0, numeric) if math.isfinite(numeric) else 0.0)
    averages.extend([0.0] * (3 - len(averages)))
    count = getattr(setting, f"mirror_{mode}_count", 0)
    if not isinstance(count, int) or isinstance(count, bool):
        count = 0
    return {
        "count": min(0xFFFFFFFF, max(0, count)),
        "averageSeconds": averages[0],
        "last5AverageSeconds": averages[1],
        "last10AverageSeconds": averages[2],
    }


class TeamService:
    """Project and mutate teams through the existing configuration store."""

    def __init__(
        self,
        *,
        config: Callable[[], Any],
        theme_store: Callable[[], Any],
        persist_config: Callable[[], None],
        lock: Any,
    ) -> None:
        self._config = config
        self._theme_store = theme_store
        self._persist_config = persist_config
        self._lock = lock

    def list_teams(self) -> list[dict[str, Any]]:
        config = self._config()
        teams = getattr(getattr(config, "config", None), "teams", {}) or {}
        queue = {
            int(number)
            for number in (config.get_value("teams_active_queue", []) or [])
            if isinstance(number, int) and not isinstance(number, bool)
        }
        details: list[dict[str, Any]] = []
        for raw_number, setting in sorted(teams.items(), key=self._sort_key):
            try:
                number = int(raw_number)
            except (TypeError, ValueError):
                continue
            if number > 0:
                details.append(self.detail(number, setting, number in queue))
        return details

    def stats_get(self, params: Any) -> dict[str, Any]:
        values = _require_mapping(params, "team.stats.get")
        team_id = _require_string(values.get("id"), "team.stats.get.id")
        team_number = _team_number_from_id(team_id, "team.stats.get.id")
        with self._lock:
            setting = (getattr(self._config().config, "teams", {}) or {}).get(
                str(team_number)
            )
            if setting is None:
                raise ValueError("team.stats.get.id 对应的队伍不存在")
            return self._stats_detail(team_id, team_number, setting)

    def stats_clear(self, params: Any) -> dict[str, Any]:
        values = _require_mapping(params, "team.stats.clear")
        team_id = _require_string(values.get("id"), "team.stats.clear.id")
        team_number = _team_number_from_id(team_id, "team.stats.clear.id")
        with self._lock:
            setting = (getattr(self._config().config, "teams", {}) or {}).get(
                str(team_number)
            )
            if setting is None:
                raise ValueError("team.stats.clear.id 对应的队伍不存在")
            setting.total_mirror_time_hard = [0.0, 0.0, 0.0]
            setting.mirror_hard_count = 0
            setting.total_mirror_time_normal = [0.0, 0.0, 0.0]
            setting.mirror_normal_count = 0
            self._persist_config()
            return self._stats_detail(team_id, team_number, setting)

    def preset_list(self) -> list[dict[str, Any]]:
        """Return built-in presets projected through the same team contract."""

        from module.team_presets import builtin_team_presets

        return builtin_team_presets(self.detail)

    def save(self, params: Any) -> dict[str, Any]:
        config = self._config()
        values = _require_mapping(params, "team.save")
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
                team_number = _require_int(
                    values["teamNumber"], "team.save.teamNumber", minimum=1
                )
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
        current = getattr(config.config, "teams", {}).get(str(team_number))
        setting = (
            current.model_copy(deep=True)
            if current is not None
            else TeamSetting(team_number=team_number)
        )
        current_name = getattr(setting, "remark_name", None) or f"编队 {team_number}"
        name = _require_string(values.get("name", current_name), "team.save.name").strip()
        if not name:
            raise ValueError("队伍名称不能为空")
        setting.remark_name = name
        if "sinners" in values:
            self._write_team_sinners(setting, values["sinners"])
        purpose = getattr(setting, "purpose", "mirror") or "mirror"
        if purpose not in TEAM_PURPOSES:
            purpose = "mirror"
        if "purpose" in values:
            purpose = _require_string(values["purpose"], "team.save.purpose")
            if purpose not in TEAM_PURPOSES:
                raise ValueError("team.save.purpose 无效")
        setting.purpose = purpose

        if purpose == "luxcavation":
            if "teamCode" in values:
                team_code = _require_string(values["teamCode"], "team.save.teamCode").strip()
                setting.team_code = team_code
                setting.use_team_code = bool(team_code)
            if "accessoryScheme" in values:
                self._write_accessory_scheme(setting, values["accessoryScheme"])

        # Luxcavation teams deliberately do not expose mirror settings in the
        # GPUI contract.  A null mirrorConfig means "preserve the stored
        # mirror settings" when an existing team is changed back later; it is
        # never a request to erase the compatibility fields in TeamSetting.
        if purpose != "luxcavation":
            if "mirrorConfig" in values and values["mirrorConfig"] is not None:
                mirror = _require_mapping(
                    values["mirrorConfig"], "team.save.mirrorConfig"
                )
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
        config.config.teams[str(team_number)] = setting
        if purpose == "luxcavation":
            self._remove_team_from_queue(team_number)
            enabled = False
        else:
            self._set_team_enabled(team_number, requested_enabled)
            enabled = requested_enabled
        self._persist_config()
        return self.detail(team_number, setting, enabled)

    def delete(self, params: Any) -> bool:
        config = self._config()
        values = _require_mapping(params, "team.delete")
        team_id = _require_string(values.get("id"), "team.delete.id")
        match = re.fullmatch(r"team-(\d+)", team_id)
        if match is None:
            raise ValueError("team.delete.id 无效")
        number = int(match.group(1))
        config.config.teams.pop(str(number), None)
        if hasattr(config, "remove_team_from_queue"):
            config.remove_team_from_queue(number)
        theme_store = self._theme_store()
        if hasattr(theme_store, "delete_team_weight_config"):
            theme_store.delete_team_weight_config(number)
        self._persist_config()
        return True

    @staticmethod
    def sinners() -> list[dict[str, str]]:
        return [
            {"id": sinner_id, "name": name}
            for sinner_id, name in zip(SINNER_IDS, SINNER_NAMES, strict=True)
        ]

    def detail(self, number: int, setting: Any, enabled: bool) -> dict[str, Any]:
        values = (
            setting.model_dump() if hasattr(setting, "model_dump") else dict(setting)
        )
        defaults = self._team_default()
        purpose = values.get("purpose", "mirror")
        if purpose not in TEAM_PURPOSES:
            purpose = "mirror"
        chosen = values.get("chosen_sinners", []) or []
        order = values.get("sinner_order", []) or []
        selected_indexes = [
            index
            for index, selected in enumerate(chosen[: len(SINNER_IDS)])
            if selected and index < len(SINNER_IDS)
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
        ordered_selected = [
            ordered_indexes[position] for position in sorted(ordered_indexes)
        ]
        ordered_selected.extend(
            index for index in selected_indexes if index not in ordered_selected
        )
        sinners = [SINNER_IDS[index] for index in ordered_selected]

        team_system = _team_int_value(
            values.get("team_system", getattr(defaults, "team_system", 0)),
            default=0,
            minimum=0,
            maximum=len(SYSTEM_NAMES) - 1,
        )
        team_code = values.get("team_code", "")
        if not isinstance(team_code, str):
            team_code = ""
        mirror: dict[str, Any] = {
            "team_system": team_system,
            "shop_strategy": _team_int_value(
                values.get("shop_strategy", getattr(defaults, "shop_strategy", 0)),
                default=0,
                minimum=0,
                maximum=2,
            ),
            "discard_systems": {
                name: _team_bool_value(values.get(f"system_{name}", False), default=False)
                for name in SYSTEM_NAMES
            },
        }
        for field in TEAM_MIRROR_BOOL_FIELDS:
            mirror[field] = _team_bool_value(
                values.get(field, getattr(defaults, field, False)), default=False
            )
        for field, (minimum, maximum) in TEAM_MIRROR_INT_LIMITS.items():
            mirror[field] = _team_int_value(
                values.get(field, getattr(defaults, field, minimum)),
                default=minimum,
                minimum=minimum,
                maximum=maximum,
            )

        opening_bonus = (
            values.get("opening_bonus", getattr(defaults, "opening_bonus", [])) or []
        )
        if not isinstance(opening_bonus, list):
            opening_bonus = []
        mirror["opening_bonus"] = [
            _team_int_value(value, default=0, minimum=0, maximum=3)
            for value in opening_bonus[:10]
        ]
        mirror["opening_bonus"] += [0] * (10 - len(mirror["opening_bonus"]))

        ignore_shop = (
            values.get("ignore_shop", getattr(defaults, "ignore_shop", [])) or []
        )
        if not isinstance(ignore_shop, list):
            ignore_shop = []
        mirror["ignore_shop"] = [
            _team_bool_value(value, default=False) for value in ignore_shop[:5]
        ]
        mirror["ignore_shop"] += [False] * (5 - len(mirror["ignore_shop"]))

        team_code = values.get("team_code", getattr(defaults, "team_code", ""))
        mirror["team_code"] = team_code if isinstance(team_code, str) else ""
        route_profile = values.get(
            "mirror_route_profile",
            getattr(defaults, "mirror_route_profile", ""),
        )
        mirror["mirror_route_profile"] = (
            route_profile if isinstance(route_profile, str) else ""
        )
        selected_gifts = (
            values.get(
                "observe_ego_gift_selected",
                getattr(defaults, "observe_ego_gift_selected", []),
            )
            or []
        )
        mirror["observe_ego_gift_selected"] = (
            normalize_observe_ego_gifts(selected_gifts)
            if isinstance(selected_gifts, list)
            else []
        )

        actions = (
            values.get(
                "second_system_action",
                getattr(defaults, "second_system_action", []),
            )
            or []
        )
        if not isinstance(actions, list):
            actions = []
        default_actions = getattr(defaults, "second_system_action", [0, 0, 0, 0]) or [
            0,
            0,
            0,
            0,
        ]
        for index, field in enumerate(TEAM_MIRROR_ACTION_FIELDS):
            raw_value = actions[index] if index < len(actions) else default_actions[index]
            mirror[field] = _team_bool_value(raw_value, default=False)

        return {
            "schemaVersion": SCHEMA_VERSION,
            "id": f"team-{number}",
            "name": values.get("remark_name") or f"编队 {number}",
            "sinners": sinners,
            "purpose": purpose,
            "accessoryScheme": SYSTEM_NAMES[team_system],
            "enabled": False if purpose == "luxcavation" else bool(enabled),
            "mirrorConfig": None if purpose == "luxcavation" else mirror,
            # Luxcavation teams hide mirrorConfig, so their formation code is
            # carried at the top level instead.  Mirror teams keep using
            # mirrorConfig.team_code to preserve the existing contract.
            "teamCode": team_code if purpose == "luxcavation" else "",
        }

    def _team_numbers(self) -> list[int]:
        teams = getattr(self._config().config, "teams", {}) or {}
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
    def _sort_key(item: tuple[Any, Any]) -> tuple[int, str]:
        try:
            return int(item[0]), str(item[0])
        except (TypeError, ValueError):
            return 0, str(item[0])

    @staticmethod
    def _team_default() -> Any:
        from module.config import TeamSetting

        return TeamSetting()

    @staticmethod
    def _stats_detail(team_id: str, team_number: int, setting: Any) -> dict[str, Any]:
        hard = _team_stats_bucket(setting, "hard")
        normal = _team_stats_bucket(setting, "normal")
        return {
            "schemaVersion": 1,
            "teamId": team_id,
            "teamNumber": team_number,
            "totalCount": min(0xFFFFFFFF, hard["count"] + normal["count"]),
            "hard": hard,
            "normal": normal,
        }

    @staticmethod
    def _write_team_sinners(setting: Any, sinners: Any) -> None:
        if not isinstance(sinners, list) or not all(
            isinstance(item, str) for item in sinners
        ):
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
            for value in (self._config().get_value("teams_active_queue", []) or [])
            if isinstance(value, int) and not isinstance(value, bool)
        }

    def _set_team_enabled(self, number: int, enabled: bool) -> None:
        config = self._config()
        if hasattr(config, "set_team_enabled"):
            config.set_team_enabled(number, enabled)
            return
        queue = [
            int(value)
            for value in (config.get_value("teams_active_queue", []) or [])
            if isinstance(value, int) and not isinstance(value, bool) and int(value) > 0
        ]
        if enabled and number not in queue:
            queue.append(number)
        if not enabled:
            queue = [value for value in queue if value != number]
        self._write_team_queue(queue)

    def _remove_team_from_queue(self, number: int) -> None:
        config = self._config()
        if hasattr(config, "remove_team_from_queue"):
            config.remove_team_from_queue(number)
            return
        queue = [
            int(value)
            for value in (config.get_value("teams_active_queue", []) or [])
            if isinstance(value, int) and not isinstance(value, bool) and int(value) != number
        ]
        self._write_team_queue(queue)

    def _write_team_queue(self, queue: list[int]) -> None:
        normalized: list[int] = []
        for value in queue:
            if value > 0 and value not in normalized:
                normalized.append(value)
        config = self._config()
        if hasattr(config, "unsaved_set_value"):
            config.unsaved_set_value("teams_active_queue", normalized, stacklevel=3)
        else:
            setattr(config.config, "teams_active_queue", normalized)

    @staticmethod
    def _write_accessory_scheme(setting: Any, value: Any) -> None:
        scheme = _require_string(value, "team.save.accessoryScheme")
        if scheme not in SYSTEM_NAMES:
            raise ValueError("team.save.accessoryScheme 无效")
        setting.team_system = SYSTEM_NAMES.index(scheme)

    @staticmethod
    def _write_mirror_setting(setting: Any, mirror: Mapping[str, Any]) -> None:
        discard = mirror.get("discard_systems")
        if discard is not None and not isinstance(discard, Mapping):
            raise ValueError(
                "team.save.mirrorConfig.discard_systems must be an object"
            )
        for key, value in mirror.items():
            if key == "discard_systems" or key in TEAM_MIRROR_ACTION_FIELDS:
                continue
            if key in TEAM_MIRROR_BOOL_FIELDS:
                setattr(
                    setting,
                    key,
                    _require_bool(value, f"team.save.mirrorConfig.{key}"),
                )
                continue
            if key in TEAM_MIRROR_INT_LIMITS:
                minimum, maximum = TEAM_MIRROR_INT_LIMITS[key]
                setattr(
                    setting,
                    key,
                    _require_int(
                        value,
                        f"team.save.mirrorConfig.{key}",
                        minimum=minimum,
                        maximum=maximum,
                    ),
                )
                continue
            if key == "team_code":
                setattr(
                    setting,
                    key,
                    _require_string(value, f"team.save.mirrorConfig.{key}"),
                )
                continue
            if key == "mirror_route_profile":
                setattr(
                    setting,
                    key,
                    _require_string(
                        value, f"team.save.mirrorConfig.{key}"
                    ).strip(),
                )
                continue
            if key == "opening_bonus":
                values = _require_int_list(value, f"team.save.mirrorConfig.{key}")
                if len(values) > 10 or any(item < 0 or item > 3 for item in values):
                    raise ValueError(
                        "team.save.mirrorConfig.opening_bonus must contain 0-3 values and have at most 10 items"
                    )
                values += [0] * (10 - len(values))
                setattr(setting, key, values)
                continue
            if key == "observe_ego_gift_selected":
                values = _require_string_list(value, f"team.save.mirrorConfig.{key}")
                setattr(setting, key, normalize_observe_ego_gifts(values))
                continue
            if key == "ignore_shop":
                values = _require_bool_list(value, f"team.save.mirrorConfig.{key}")
                if len(values) > 5:
                    raise ValueError(
                        "team.save.mirrorConfig.ignore_shop must have at most 5 items"
                    )
                values += [False] * (5 - len(values))
                setattr(setting, key, [1 if item else 0 for item in values])
                continue
            if key == "second_system_action":
                values = _require_int_list(value, f"team.save.mirrorConfig.{key}")
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
                        _require_bool(
                            discard[system],
                            f"team.save.mirrorConfig.discard_systems.{system}",
                        ),
                    )
        if any(key in mirror for key in TEAM_MIRROR_ACTION_FIELDS):
            actions = list(getattr(setting, "second_system_action", [0, 0, 0, 0]) or [])[
                :4
            ]
            actions += [0] * (4 - len(actions))
            for index, key in enumerate(TEAM_MIRROR_ACTION_FIELDS):
                if key in mirror:
                    actions[index] = (
                        1
                        if _require_bool(
                            mirror[key], f"team.save.mirrorConfig.{key}"
                        )
                        else 0
                    )
            setting.second_system_action = actions


__all__ = [
    "SINNER_IDS",
    "SINNER_NAMES",
    "SYSTEM_NAMES",
    "TeamService",
]
