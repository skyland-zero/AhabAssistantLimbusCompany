"""Theme-pack RPC operations.

The service owns validation and persistence of the two theme-pack sections;
``BackendApplication`` only provides the lazily resolved compatibility store
and the current hard-mirror flag.
"""

from __future__ import annotations

import copy
from collections.abc import Callable, Mapping
from pathlib import Path
from typing import Any

from module.config.theme_pack_catalog import theme_pack_display_name
from module.rpc_contract import SCHEMA_VERSION

_SECTIONS = ("theme_pack_list", "theme_pack_list_hard")


def _require_mapping(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, Mapping):
        raise ValueError(f"{name} 必须是对象")
    return dict(value)


def _require_string(value: Any, name: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{name} 必须是非空字符串")
    return value


class ThemePackService:
    """Read and update the persisted theme-pack weights."""

    def __init__(
        self,
        store: Callable[[], Any],
        hard_mirror_active: Callable[[], bool],
    ) -> None:
        self._store = store
        self._hard_mirror_active = hard_mirror_active

    def list_packs(self) -> dict[str, Any]:
        config = getattr(self._store(), "config", {}) or {}
        packs: list[dict[str, Any]] = []
        seen: set[str] = set()
        for section_name in _SECTIONS:
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
            "hardMirrorActive": bool(self._hard_mirror_active()),
            "packs": packs,
        }

    def update_all(self, params: Any) -> bool:
        values = _require_mapping(params, "themePack.updateAll")
        packs = values.get("packs")
        if not isinstance(packs, list):
            raise ValueError("themePack.updateAll.packs must be a list")
        store = self._store()
        config = copy.deepcopy(getattr(store, "config", {}) or {})
        sections = [config.get(name, {}) for name in _SECTIONS]
        known = {
            str(key)
            for section in sections
            if isinstance(section, Mapping)
            for key in section
        }
        for pack in packs:
            pack = _require_mapping(pack, "themePack.updateAll.pack")
            pack_id = _require_string(pack.get("id"), "themePack.id")
            if pack_id not in known:
                raise ValueError(f"未知主题包：{pack_id}")
            try:
                weight = int(pack.get("weight", 0))
            except (TypeError, ValueError) as error:
                raise ValueError(f"主题包权重无效：{pack_id}") from error
            if not 0 <= weight <= 10 or not isinstance(pack.get("enabled"), bool):
                raise ValueError(f"主题包权重或启用状态无效：{pack_id}")
            raw_weight = weight if pack["enabled"] else -max(1, weight)
            for section_name in _SECTIONS:
                section = config.get(section_name)
                if isinstance(section, dict) and pack_id in section:
                    section[pack_id] = raw_weight
        store.config = config
        store.save_config(path=store.theme_pack_list_path, config_data=config)
        return True

    def reset_weights(self) -> dict[str, Any]:
        store = self._store()
        example_path = Path("assets/config/theme_pack_list.example.yaml")
        if example_path.is_file():
            default = store.yaml.load(example_path.read_text(encoding="utf-8")) or {}
            store.config = copy.deepcopy(default)
            store.save_config(path=store.theme_pack_list_path, config_data=store.config)
        return self.list_packs()


__all__ = ["ThemePackService"]
