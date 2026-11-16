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


class _UpdateChecker:
    def __init__(self, timeout, flag, version=None) -> None:
        self.new_version = "V9.9.9"
        self.is_current_version_latest = False
        self.error_msg = ""

    def run(self) -> None:
        return None

    def check_update_info_github_api(self) -> dict:
        return {
            "assets": [
                {
                    "name": "AALC.nsh_V9.9.9.7z",
                    "browser_download_url": "https://example.test/archive.7z",
                },
                {
                    "name": "AALC.nsh_V9.9.9.manifest.json",
                    "browser_download_url": "https://example.test/manifest.json",
                },
                {
                    "name": "AALC.nsh_V9.9.9.manifest.sig",
                    "browser_download_url": "https://example.test/manifest.sig",
                },
            ]
        }


def test_install_update_requires_github_source() -> None:
    notices: list[tuple[str, dict]] = []
    service = NotificationUpdateService(
        None,
        lambda key, default: "MirrorChyan",
        lambda: "V9.9.8",
        emit=lambda event, payload: notices.append((event, dict(payload))),
    )

    result = service.install_update()

    assert result["status"] == "unsupported_source"
    assert notices[0][0] == "app.notice"
    assert notices[0][1]["level"] == "warn"


def test_install_update_worker_downloads_and_launches(monkeypatch) -> None:
    import module.update.checker as checker

    notices: list[tuple[str, dict]] = []
    downloads: list[tuple[str, dict]] = []
    started: list[str] = []

    monkeypatch.setattr(checker, "UpdateThread", _UpdateChecker)
    monkeypatch.setattr(
        checker,
        "update",
        lambda url, **kwargs: downloads.append((url, kwargs)) or True,
    )
    monkeypatch.setattr(checker, "start_update", lambda name: started.append(name))

    service = NotificationUpdateService(
        None,
        lambda key, default: "GitHub",
        lambda: "V9.9.8",
        emit=lambda event, payload: notices.append((event, dict(payload))),
    )
    service._install_worker()

    assert downloads[0][0] == "https://example.test/archive.7z"
    assert downloads[0][1] == {
        "manifest_url": "https://example.test/manifest.json",
        "signature_url": "https://example.test/manifest.sig",
    }
    assert started == ["AALC.nsh_V9.9.9.7z"]
    assert any("更新包下载完成" in payload["message"] for _, payload in notices)


def test_install_update_worker_rejects_an_unsigned_release(monkeypatch) -> None:
    import module.update.checker as checker

    notices: list[tuple[str, dict]] = []
    started: list[object] = []

    class _Unsigned(_UpdateChecker):
        def check_update_info_github_api(self) -> dict:
            return {
                "assets": [
                    {
                        "name": "AALC.nsh_V9.9.9.7z",
                        "browser_download_url": "https://example.test/archive.7z",
                    }
                ]
            }

    monkeypatch.setattr(checker, "UpdateThread", _Unsigned)
    monkeypatch.setattr(checker, "update", lambda *args, **kwargs: started.append(args))
    monkeypatch.setattr(checker, "start_update", lambda name: started.append(name))

    service = NotificationUpdateService(
        None,
        lambda key, default: "GitHub",
        lambda: "V9.9.8",
        emit=lambda event, payload: notices.append((event, dict(payload))),
    )
    service._install_worker()

    assert started == []
    assert any("缺少签名清单" in payload["message"] for _, payload in notices)


def test_install_update_is_busy_while_running() -> None:
    service = NotificationUpdateService(
        None,
        lambda key, default: "GitHub",
        lambda: "V9.9.8",
    )
    service._installing = True

    assert service.install_update()["status"] == "busy"
