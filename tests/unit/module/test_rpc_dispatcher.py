from __future__ import annotations

from typing import Any

from module.rpc_dispatcher import RpcDispatcher


class _DeviceManager:
    def set_busy_checker(self, checker: Any) -> None:
        self.busy_checker = checker


class _ParamApplication:
    def __init__(self) -> None:
        self.device_manager = _DeviceManager()
        self.calls: list[tuple[str, Any]] = []

    def is_busy(self) -> bool:
        return False

    def set_busy_checker(self, checker: Any) -> None:
        self.device_manager.set_busy_checker(checker)

    def execution_start(self, params: Any = None) -> dict[str, Any]:
        self.calls.append(("start", params))
        return {"accepted": True, "runId": "run-1", "state": "starting", "stateRevision": 1}

    def execution_pause(self, params: Any = None) -> dict[str, Any]:
        self.calls.append(("pause", params))
        return {"accepted": True, "runId": params["runId"]}

    def execution_resume(self, params: Any = None) -> dict[str, Any]:
        self.calls.append(("resume", params))
        return {"accepted": True, "runId": params["runId"]}

    def execution_stop(self, params: Any = None) -> dict[str, Any]:
        self.calls.append(("stop", params))
        return {"accepted": True, "runId": params["runId"], "state": "stopping"}

    def execution_get_state(self, params: Any = None) -> dict[str, Any]:
        self.calls.append(("getState", params))
        return {"state": "running", "runId": params.get("runId") if params else None}


def _request(method: str, params: Any = None) -> dict[str, Any]:
    request: dict[str, Any] = {"jsonrpc": "2.0", "id": method, "method": method}
    if params is not None:
        request["params"] = params
    return request


def test_execution_params_are_forwarded_to_schema_three_application() -> None:
    application = _ParamApplication()
    dispatcher = RpcDispatcher(application=application)

    requests = [
        _request(
            "execution.start",
            {"clientRequestId": "client-1", "taskId": "mirror", "options": {"dryRun": True}},
        ),
        _request("execution.pause", {"runId": "run-1"}),
        _request("execution.resume", {"runId": "run-1"}),
        _request("execution.stop", {"runId": "run-1"}),
        _request("execution.getState", {"runId": "run-1"}),
    ]

    responses = [dispatcher.dispatch(request) for request in requests]

    assert all("error" not in response for response in responses)
    assert application.calls == [
        ("start", {"clientRequestId": "client-1", "taskId": "mirror", "options": {"dryRun": True}}),
        ("pause", {"runId": "run-1"}),
        ("resume", {"runId": "run-1"}),
        ("stop", {"runId": "run-1"}),
        ("getState", {"runId": "run-1"}),
    ]


def test_execution_params_keep_legacy_no_argument_application_compatible() -> None:
    class LegacyApplication(_ParamApplication):
        def execution_stop(self) -> dict[str, Any]:
            self.calls.append(("stop", None))
            return {"accepted": True, "state": "stopping"}

        def execution_pause(self) -> dict[str, Any]:
            self.calls.append(("pause", None))
            return {"accepted": True, "state": "paused"}

        def execution_resume(self) -> dict[str, Any]:
            self.calls.append(("resume", None))
            return {"accepted": True, "state": "running"}

    application = LegacyApplication()
    dispatcher = RpcDispatcher(application=application)

    assert "error" not in dispatcher.dispatch(_request("execution.stop", {"runId": "legacy"}))
    assert "error" not in dispatcher.dispatch(_request("execution.pause", {"runId": "legacy"}))
    assert "error" not in dispatcher.dispatch(_request("execution.resume", {"runId": "legacy"}))
    assert application.calls == [("stop", None), ("pause", None), ("resume", None)]


def test_execution_params_must_be_objects() -> None:
    application = _ParamApplication()
    dispatcher = RpcDispatcher(application=application)

    response = dispatcher.dispatch(_request("execution.stop", ["run-1"]))

    assert response["error"]["code"] == -32602
    assert response["error"]["data"]["code"] == "INVALID_PARAMS"
    assert response["error"]["data"]["retryable"] is False
    assert application.calls == []


def test_structured_dispatch_errors_expose_symbolic_code_and_retryability() -> None:
    class ErrorApplication(_ParamApplication):
        def execution_stop(self, params: Any = None) -> dict[str, Any]:
            from module.rpc_dispatcher import RpcDispatchError

            raise RpcDispatchError(
                -32013,
                "STALE_RUN",
                {"retryable": False, "runId": params["runId"]},
            )

    response = RpcDispatcher(application=ErrorApplication()).dispatch(
        _request("execution.stop", {"runId": "old-run"})
    )

    assert response["error"]["code"] == -32013
    assert response["error"]["message"] == "STALE_RUN"
    assert response["error"]["data"] == {
        "code": "STALE_RUN",
        "retryable": False,
        "userMessage": "STALE_RUN",
        "runId": "old-run",
    }
