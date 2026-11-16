"""Notification testing and update checks.

Both operations are synchronous, externally bounded, and independent from the
execution state machine.  Keeping them here prevents the RPC composition root
from importing update internals or notification redaction rules.
"""

from __future__ import annotations

import threading
from collections.abc import Callable, Mapping
from typing import Any

from module.config.redaction import redact_text
from module.logger import log
from module.rpc_contract import SCHEMA_VERSION


def _require_mapping(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, Mapping):
        raise ValueError(f"{name} 必须是对象")
    return dict(value)


class NotificationUpdateService:
    """Own the notification-test and update RPC methods."""

    def __init__(
        self,
        notifications: Any,
        config_value: Callable[[str, Any], Any],
        version: Callable[[], str],
        emit: Callable[[str, Mapping[str, Any]], Any] | None = None,
    ) -> None:
        self._notifications = notifications
        self._config_value = config_value
        self._version = version
        self._emit = emit
        self._install_lock = threading.Lock()
        self._installing = False

    def test_notification(self, params: Any) -> dict[str, Any]:
        values = _require_mapping(params, "notification.test")
        spt = values.get("spt", self._config_value("wxpusher_spt", ""))
        if not isinstance(spt, str):
            raise ValueError("SPT 必须是字符串")
        normalized_spt = spt.strip()
        if not normalized_spt:
            raise ValueError("SPT 未配置")
        if len(normalized_spt) <= 4 or not normalized_spt.startswith("SPT_"):
            raise ValueError("SPT 格式无效")
        try:
            self._notifications.send_test(normalized_spt)
        except Exception as error:
            raise ValueError(redact_text(error, (spt,))) from error
        return {"accepted": True}

    def check_update(self) -> dict[str, Any]:
        try:
            from module.update.checker import UpdateThread

            checker = UpdateThread(timeout=10, flag=False, version=self._version())
            checker.run()
            if not checker.new_version:
                return {
                    "schemaVersion": SCHEMA_VERSION,
                    "status": "failed",
                    "error": checker.error_msg or "无法从更新源获取版本信息",
                }
            latest = checker.new_version
            update_available = bool(
                latest != self._version() and not checker.is_current_version_latest
            )
            return {
                "schemaVersion": SCHEMA_VERSION,
                "status": "available" if update_available else "up_to_date",
                "updateAvailable": update_available,
                "latest": latest,
            }
        except Exception as error:
            log.warning("检查更新失败：%s", error)
            return {"schemaVersion": SCHEMA_VERSION, "status": "failed", "error": str(error)}

    def install_update(self) -> dict[str, Any]:
        """Start a signed in-app update in the background.

        The sidecar is replaced by the updater, so this method only starts the
        worker and returns immediately.  Download progress and failures reach
        the UI through ``app.notice``.
        """

        source = str(self._config_value("update_source", "GitHub") or "GitHub").casefold()
        if source != "github":
            self._notice(
                "warn",
                "自动安装暂仅支持 GitHub 更新源；Mirror 酱源可用于检查或手动下载",
            )
            return {
                "schemaVersion": SCHEMA_VERSION,
                "status": "unsupported_source",
                "error": "自动安装暂仅支持 GitHub 更新源",
            }

        with self._install_lock:
            if self._installing:
                return {"schemaVersion": SCHEMA_VERSION, "status": "busy"}
            self._installing = True

        try:
            thread = threading.Thread(
                target=self._install_worker,
                name="AALCUpdateInstall",
                daemon=True,
            )
            thread.start()
        except Exception as error:
            self._reset_installing()
            log.warning("启动自动更新线程失败：%s", error)
            return {
                "schemaVersion": SCHEMA_VERSION,
                "status": "failed",
                "error": str(error),
            }
        return {"schemaVersion": SCHEMA_VERSION, "status": "checking"}

    def _reset_installing(self) -> None:
        with self._install_lock:
            self._installing = False

    def _notice(self, level: str, message: str) -> None:
        if self._emit is None:
            return
        try:
            self._emit("app.notice", {"level": level, "message": message})
        except Exception:
            log.debug("发送更新通知失败", exc_info=True)

    def _install_worker(self) -> None:
        try:
            from module.update.checker import UpdateThread, start_update, update

            checker = UpdateThread(timeout=10, flag=False, version=self._version())
            checker.run()
            latest = checker.new_version
            if not latest:
                self._notice("error", checker.error_msg or "无法获取最新版本信息")
                return
            if checker.is_current_version_latest:
                self._notice("info", f"当前已是最新版本：{latest}")
                return

            # The signed archive and its detached metadata live in the same
            # GitHub release.  Using the release assets directly avoids relying
            # on the mirror's URL layout, which does not host the signature.
            release = checker.check_update_info_github_api()
            assets = release.get("assets") if isinstance(release, Mapping) else None
            if not isinstance(assets, list):
                self._notice("error", "无法读取 GitHub Release 资产列表")
                return

            archive = next(
                (
                    asset
                    for asset in assets
                    if isinstance(asset, Mapping)
                    and str(asset.get("name", "")).endswith(".7z")
                ),
                None,
            )
            manifest = next(
                (
                    asset
                    for asset in assets
                    if isinstance(asset, Mapping)
                    and str(asset.get("name", "")).endswith(".manifest.json")
                ),
                None,
            )
            signature = next(
                (
                    asset
                    for asset in assets
                    if isinstance(asset, Mapping)
                    and str(asset.get("name", "")).endswith(".manifest.sig")
                ),
                None,
            )
            if archive is None or manifest is None or signature is None:
                self._notice("error", "该版本缺少签名清单，无法自动安装")
                return

            archive_name = str(archive.get("name", ""))
            archive_url = str(archive.get("browser_download_url", ""))
            manifest_url = str(manifest.get("browser_download_url", ""))
            signature_url = str(signature.get("browser_download_url", ""))
            if not archive_name or not archive_url or not manifest_url or not signature_url:
                self._notice("error", "该版本的更新资产不完整")
                return

            self._notice("info", f"正在下载更新：{latest}")
            if not update(
                archive_url,
                manifest_url=manifest_url,
                signature_url=signature_url,
            ):
                self._notice("error", "更新下载失败，请查看日志")
                return

            self._notice("info", "更新包下载完成，正在启动安装程序...")
            start_update(archive_name)
        except Exception as error:
            log.warning("自动更新失败：%s", error)
            self._notice("error", f"自动更新失败：{error}")
        finally:
            self._reset_installing()


__all__ = ["NotificationUpdateService"]
