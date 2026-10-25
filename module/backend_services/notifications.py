"""Notification testing and update checks.

Both operations are synchronous, externally bounded, and independent from the
execution state machine.  Keeping them here prevents the RPC composition root
from importing update internals or notification redaction rules.
"""

from __future__ import annotations

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
    """Own the notification-test and update-check RPC methods."""

    def __init__(
        self,
        notifications: Any,
        config_value: Callable[[str, Any], Any],
        version: Callable[[], str],
    ) -> None:
        self._notifications = notifications
        self._config_value = config_value
        self._version = version

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

            checker = UpdateThread(timeout=10, flag=False)
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
            log.warning("更新检查失败：%s", error)
            return {"schemaVersion": SCHEMA_VERSION, "status": "failed", "error": str(error)}


__all__ = ["NotificationUpdateService"]
