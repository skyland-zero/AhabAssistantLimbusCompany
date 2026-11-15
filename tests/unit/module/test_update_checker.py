from __future__ import annotations

from typing import Any

from module.rpc_contract import SCHEMA_VERSION
from module.update import checker


class _Response:
    def __init__(self, payload: dict[str, Any]) -> None:
        self.payload = payload

    def raise_for_status(self) -> None:
        return None

    def json(self) -> dict[str, Any]:
        return self.payload


def _release(tag: str, *, body: Any = "", assets: list[dict[str, str]] | None = None) -> dict[str, Any]:
    return {"tag_name": tag, "body": body, "assets": assets or []}


def _asset(tag: str) -> list[dict[str, str]]:
    return [
        {
            "name": f"AALC.nsh_{tag}.7z",
            "browser_download_url": f"https://example.com/AALC.nsh_{tag}.7z",
        }
    ]


def _statuses(thread: checker.UpdateThread) -> list[checker.UpdateStatus]:
    seen: list[checker.UpdateStatus] = []
    thread.updateSignal.connect(seen.append)
    return seen


def test_github_check_reads_this_forks_latest_release(monkeypatch) -> None:
    thread = checker.UpdateThread(timeout=1, flag=False, version="V0.0.2")
    seen: dict[str, str] = {}

    def fake_get(url: str, *, timeout: float, headers: dict[str, str]) -> _Response:
        seen["url"] = url
        return _Response(_release("V0.0.2"))

    monkeypatch.setattr(checker.cfg, "update_prerelease_enable", False)
    monkeypatch.setattr(checker.requests, "get", fake_get)

    assert thread.check_update_info_github()["tag_name"] == "V0.0.2"
    assert seen["url"] == (
        "https://api.github.com/repos/skyland-zero/AhabAssistantLimbusCompany/releases/latest"
    )


def test_run_reports_a_newer_release_with_an_empty_body(monkeypatch) -> None:
    thread = checker.UpdateThread(timeout=1, flag=False, version="V0.0.2")
    monkeypatch.setattr(
        thread,
        "check_update_info_github",
        lambda: _release("V0.0.3", body=None, assets=_asset("V0.0.3")),
    )
    statuses = _statuses(thread)

    thread.run()

    assert statuses == [checker.UpdateStatus.UPDATE_AVAILABLE]
    assert thread.new_version == "V0.0.3"
    assert thread.is_current_version_latest is False
    assert thread.title.startswith("发现新版本")


def test_run_reports_up_to_date_for_the_same_release(monkeypatch) -> None:
    thread = checker.UpdateThread(timeout=1, flag=False, version="V0.0.3")
    monkeypatch.setattr(
        thread,
        "check_update_info_github",
        lambda: _release("V0.0.3", assets=_asset("V0.0.3")),
    )
    statuses = _statuses(thread)

    thread.run()

    assert statuses == [checker.UpdateStatus.SUCCESS]
    assert thread.is_current_version_latest is True


def test_run_rejects_a_malformed_current_version(monkeypatch) -> None:
    thread = checker.UpdateThread(timeout=1, flag=False, version="DEFAULT VERSION")
    monkeypatch.setattr(
        thread,
        "check_update_info_github",
        lambda: _release("V0.0.3", assets=_asset("V0.0.3")),
    )
    statuses = _statuses(thread)

    thread.run()

    # 版本号解析失败时不能把远端版本当成可用更新，否则旧冻结包会误报 1.x。
    assert statuses == [checker.UpdateStatus.FAILURE]
    assert thread.new_version == ""
    assert thread.error_msg


def test_update_service_passes_the_application_version(monkeypatch) -> None:
    from module.backend_services.notifications import NotificationUpdateService

    created: dict[str, Any] = {}

    class _Checker:
        def __init__(self, timeout, flag, version=None) -> None:
            created["timeout"] = timeout
            created["flag"] = flag
            created["version"] = version
            self.new_version = "V0.0.3"
            self.is_current_version_latest = False
            self.error_msg = ""

        def run(self) -> None:
            return None

    monkeypatch.setattr(checker, "UpdateThread", _Checker)

    service = NotificationUpdateService(
        None,
        lambda key, default: default,
        lambda: "V0.0.2",
    )

    assert service.check_update() == {
        "schemaVersion": SCHEMA_VERSION,
        "status": "available",
        "updateAvailable": True,
        "latest": "V0.0.3",
    }
    assert created["version"] == "V0.0.2"
