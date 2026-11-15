from __future__ import annotations

from typing import Any

from module.rpc_contract import SCHEMA_VERSION
from module.update import checker

_DIRECT = "https://github.com/skyland-zero/AhabAssistantLimbusCompany"
_DIRECT_ATOM = f"{_DIRECT}/releases.atom"
_DIRECT_LATEST = f"{_DIRECT}/releases/latest"
_MIRROR_ATOM = f"https://gh-proxy.com/{_DIRECT_ATOM}"

_ATOM = """<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <title>V0.0.5-beta.1</title>
    <link rel="alternate" href="https://github.com/skyland-zero/AhabAssistantLimbusCompany/releases/tag/V0.0.5-beta.1"/>
    <updated>2026-11-16T11:31:28Z</updated>
    <content type="html">&lt;p&gt;preview notes&lt;/p&gt;</content>
  </entry>
  <entry>
    <title>V0.0.4</title>
    <link rel="alternate" href="https://github.com/skyland-zero/AhabAssistantLimbusCompany/releases/tag/V0.0.4"/>
    <updated>2026-11-15T11:31:28Z</updated>
    <content type="html">&lt;p&gt;stable notes&lt;/p&gt;</content>
  </entry>
</feed>
"""


class _Response:
    def __init__(
        self,
        *,
        status_code: int = 200,
        headers: dict[str, str] | None = None,
        text: str = "",
        payload: Any = None,
    ) -> None:
        self.status_code = status_code
        self.headers = headers or {}
        self.text = text
        self.payload = payload

    def raise_for_status(self) -> None:
        return None

    def json(self) -> Any:
        return self.payload


def _release(tag: str, *, body: Any = "", assets: list[dict[str, str]] | None = None) -> dict[str, Any]:
    return {"tag_name": tag, "body": body, "assets": assets or []}


def _statuses(thread: checker.UpdateThread) -> list[checker.UpdateStatus]:
    seen: list[checker.UpdateStatus] = []
    thread.updateSignal.connect(seen.append)
    return seen


def _forget_mirror_override(monkeypatch) -> None:
    monkeypatch.delenv("AALC_UPDATE_MIRROR", raising=False)


def test_stable_check_prefers_the_china_mirror_feed(monkeypatch) -> None:
    _forget_mirror_override(monkeypatch)
    thread = checker.UpdateThread(timeout=1, flag=False, version="V0.0.3")
    seen: list[str] = []

    def fake_get(url: str, **kwargs: Any) -> _Response:
        seen.append(url)
        return _Response(text=_ATOM)

    monkeypatch.setattr(checker.cfg, "update_prerelease_enable", False)
    monkeypatch.setattr(checker.requests, "get", fake_get)

    release = thread.check_update_info_github()

    # 稳定版跳过第一条 V0.0.5-beta.1，取 V0.0.4。
    assert release["tag_name"] == "V0.0.4"
    assert seen[0] == _MIRROR_ATOM


def test_preview_check_keeps_the_newest_feed_entry(monkeypatch) -> None:
    _forget_mirror_override(monkeypatch)
    thread = checker.UpdateThread(timeout=1, flag=False, version="V0.0.4")
    seen: list[str] = []

    def fake_get(url: str, **kwargs: Any) -> _Response:
        seen.append(url)
        return _Response(text=_ATOM)

    monkeypatch.setattr(checker.cfg, "update_prerelease_enable", True)
    monkeypatch.setattr(checker.requests, "get", fake_get)

    release = thread.check_update_info_github()

    assert release["tag_name"] == "V0.0.5-beta.1"
    assert "preview notes" in release["body"]
    assert seen[0] == _MIRROR_ATOM


def test_mirror_failure_falls_back_to_the_github_feed(monkeypatch) -> None:
    _forget_mirror_override(monkeypatch)
    thread = checker.UpdateThread(timeout=1, flag=False, version="V0.0.3")
    seen: list[str] = []

    def fake_get(url: str, **kwargs: Any) -> _Response:
        seen.append(url)
        if url == _DIRECT_ATOM:
            return _Response(text=_ATOM)
        raise RuntimeError("mirror blocked")

    monkeypatch.setattr(checker.cfg, "update_prerelease_enable", False)
    monkeypatch.setattr(checker.requests, "get", fake_get)

    release = thread.check_update_info_github()

    assert release["tag_name"] == "V0.0.4"
    assert seen[-1] == _DIRECT_ATOM


def test_feed_failure_falls_back_to_the_latest_redirect(monkeypatch) -> None:
    _forget_mirror_override(monkeypatch)
    thread = checker.UpdateThread(timeout=1, flag=False, version="V0.0.3")
    seen: list[tuple[str, Any]] = []

    def fake_get(url: str, **kwargs: Any) -> _Response:
        seen.append((url, kwargs.get("allow_redirects")))
        if url == _DIRECT_LATEST:
            return _Response(
                status_code=302,
                headers={"Location": f"{_DIRECT}/releases/tag/V0.0.4"},
            )
        raise RuntimeError("feed blocked")

    monkeypatch.setattr(checker.cfg, "update_prerelease_enable", False)
    monkeypatch.setattr(checker.requests, "get", fake_get)

    release = thread.check_update_info_github()

    assert release["tag_name"] == "V0.0.4"
    assert seen[-1] == (_DIRECT_LATEST, False)


def test_update_mirror_env_override(monkeypatch) -> None:
    monkeypatch.setenv("AALC_UPDATE_MIRROR", "https://mirror.example/")
    thread = checker.UpdateThread(timeout=1, flag=False, version="V0.0.3")
    seen: list[str] = []

    def fake_get(url: str, **kwargs: Any) -> _Response:
        seen.append(url)
        return _Response(text=_ATOM)

    monkeypatch.setattr(checker.cfg, "update_prerelease_enable", False)
    monkeypatch.setattr(checker.requests, "get", fake_get)

    assert thread.check_update_info_github()["tag_name"] == "V0.0.4"
    assert seen[0] == f"https://mirror.example/{_DIRECT_ATOM}"


def test_run_reports_a_newer_release_without_api_or_assets(monkeypatch) -> None:
    thread = checker.UpdateThread(timeout=1, flag=False, version="V0.0.3")
    monkeypatch.setattr(
        thread,
        "check_update_info_github",
        lambda: _release("V0.0.4", body=None, assets=[]),
    )
    statuses = _statuses(thread)

    thread.run()

    assert statuses == [checker.UpdateStatus.UPDATE_AVAILABLE]
    assert thread.new_version == "V0.0.4"
    assert thread.is_current_version_latest is False
    assert thread.title.startswith("发现新版本")


def test_run_reports_up_to_date_for_the_same_release(monkeypatch) -> None:
    thread = checker.UpdateThread(timeout=1, flag=False, version="V0.0.4")
    monkeypatch.setattr(
        thread,
        "check_update_info_github",
        lambda: _release("V0.0.4"),
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
        lambda: _release("V0.0.4"),
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
            self.new_version = "V0.0.5"
            self.is_current_version_latest = False
            self.error_msg = ""

        def run(self) -> None:
            return None

    monkeypatch.setattr(checker, "UpdateThread", _Checker)

    service = NotificationUpdateService(
        None,
        lambda key, default: default,
        lambda: "V0.0.4",
    )

    assert service.check_update() == {
        "schemaVersion": SCHEMA_VERSION,
        "status": "available",
        "updateAvailable": True,
        "latest": "V0.0.5",
    }
    assert created["version"] == "V0.0.4"
