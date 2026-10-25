from __future__ import annotations

from pathlib import Path

import pytest

from module.backend_services.notifications import NotificationUpdateService
from module.backend_services.theme_packs import ThemePackService


class _ThemeStore:
    def __init__(self, path: Path) -> None:
        self.theme_pack_list_path = path
        self.config = {
            "theme_pack_list": {"pk-1": 3, "pk-2": -2},
            "theme_pack_list_hard": {},
        }
        self.saved = None

    def save_config(self, *, path, config_data) -> None:
        self.saved = (path, config_data)


def test_theme_pack_service_owns_validation_and_persistence(tmp_path: Path) -> None:
    store = _ThemeStore(tmp_path / "theme_pack_list.yaml")
    service = ThemePackService(lambda: store, lambda: True)

    listing = service.list_packs()
    assert listing["hardMirrorActive"] is True
    assert [(pack["id"], pack["weight"], pack["enabled"]) for pack in listing["packs"]] == [
        ("pk-1", 3, True),
        ("pk-2", 2, False),
    ]

    assert service.update_all(
        {"packs": [{"id": "pk-1", "weight": 5, "enabled": False}]}
    )
    assert store.config["theme_pack_list"]["pk-1"] == -5
    assert store.saved is not None

    with pytest.raises(ValueError, match="未知主题包"):
        service.update_all({"packs": [{"id": "missing", "weight": 1, "enabled": True}]})


class _Notifications:
    def __init__(self) -> None:
        self.spt: list[str] = []

    def send_test(self, spt: str) -> None:
        self.spt.append(spt)


def test_notification_service_validates_and_redacts_before_sending() -> None:
    notifications = _Notifications()
    service = NotificationUpdateService(
        notifications,
        lambda key, default: default,
        lambda: "1.0.0",
    )

    assert service.test_notification({"spt": " SPT_valid "}) == {"accepted": True}
    assert notifications.spt == ["SPT_valid"]

    with pytest.raises(ValueError, match="SPT 未配置"):
        service.test_notification({"spt": "   "})
    with pytest.raises(ValueError, match="SPT 格式无效"):
        service.test_notification({"spt": "bad"})
