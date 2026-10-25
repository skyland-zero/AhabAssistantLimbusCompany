"""Image-resource status, update checks and background sync."""

from __future__ import annotations

import json
import threading
import uuid
from collections.abc import Callable, Mapping
from pathlib import Path
from typing import Any

from module.device_manager import DeviceError
from module.logger import log


class ResourceService:
    """Own the resource synchronization lifecycle and RPC projections."""

    def __init__(
        self,
        *,
        service_factory: Callable[[], Any],
        config_value: Callable[[str, Any], Any],
        emit: Callable[[str, Mapping[str, Any]], int],
        lock: Any,
    ) -> None:
        self._service_factory = service_factory
        self._config_value = config_value
        self._emit = emit
        self._lock = lock
        self._check: Any | None = None
        self._plan: Any | None = None
        self._run_id: str | None = None

    def status(self) -> list[dict[str, Any]]:
        state_path = Path("assets/config/image_resource_state.json")
        local_version = "unknown"
        try:
            value = json.loads(state_path.read_text(encoding="utf-8"))
            local_version = str(value.get("last_applied_manifest_id") or "unknown")
        except (OSError, ValueError, TypeError):
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

    def check_update(self) -> list[dict[str, Any]]:
        service = self._service_factory()
        result = service.check_for_updates(
            self._config_value("image_resource_source", "Auto")
        )
        self._check = result
        groups = self.status()
        if result.remote_manifest is not None:
            remote_version = result.remote_manifest.manifest_id
            for group in groups:
                group["remoteVersion"] = remote_version
        if result.status.value == "error":
            raise RuntimeError(result.message or "资源更新检查失败")
        return groups

    def sync_start(self, params: Any = None) -> dict[str, Any]:
        with self._lock:
            if self._run_id is not None:
                raise DeviceError("资源同步已经在运行")
            run_id = uuid.uuid4().hex
            self._run_id = run_id
        scope = "all"
        if isinstance(params, Mapping) and isinstance(params.get("scope"), str):
            scope = params["scope"]
        worker = threading.Thread(
            target=self._run,
            args=(run_id, scope),
            name="AALCResourceSync",
            daemon=True,
        )
        worker.start()
        return {"accepted": True, "runId": run_id}

    def _run(self, run_id: str, scope: str) -> None:
        try:
            service = self._service_factory()
            result = service.check_for_updates(
                self._config_value("image_resource_source", "Auto")
            )
            self._check = result
            if result.remote_manifest is None:
                raise RuntimeError(result.message or "远端资源清单不可用")
            plan = service.build_sync_plan(result.remote_manifest)
            self._plan = plan
            if not plan.has_changes:
                service.accept_remote_state(result)
                self._emit(
                    "resource.sync.progress",
                    {"scope": scope, "progress": 100, "runId": run_id},
                )
                return

            def progress(current: int, total: int) -> None:
                value = (
                    100
                    if total <= 0
                    else max(0, min(100, round(current * 100 / total)))
                )
                self._emit(
                    "resource.sync.progress",
                    {"scope": scope, "progress": value, "runId": run_id},
                )

            service.apply_sync_plan(
                check_result=result,
                sync_plan=plan,
                progress_callback=progress,
            )
            self._emit(
                "resource.sync.progress",
                {"scope": scope, "progress": 100, "runId": run_id},
            )
        except Exception as error:
            self._emit(
                "resource.sync.progress",
                {"scope": scope, "progress": 0, "runId": run_id, "error": str(error)},
            )
            self._emit(
                "app.notice",
                {
                    "level": "error",
                    "message": f"资源同步失败：{error}",
                    "runId": run_id,
                },
            )
            log.exception("资源同步失败")
        finally:
            with self._lock:
                if self._run_id == run_id:
                    self._run_id = None


__all__ = ["ResourceService"]
