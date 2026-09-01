from __future__ import annotations

import logging
import sys
from unittest.mock import Mock, patch

import pytest
import requests

from module.config.redaction import REDACTED_VALUE, redact_mapping, redact_text, register_secret
from module.notification.wxpusher import (
    WXPUSHER_SIMPLE_PUSH_URL,
    NotificationMessage,
    NotificationQueue,
    NotificationService,
    WxPusherClient,
    WxPusherError,
    format_completion,
    format_failure,
    format_final_summary,
    normalize_spt,
)


class FakeResponse:
    def __init__(self, status_code: int, payload: object) -> None:
        self.status_code = status_code
        self.payload = payload

    def json(self) -> object:
        return self.payload


def test_wxpusher_success_uses_simple_push_payload() -> None:
    calls: list[tuple[str, dict, float]] = []

    def post(url: str, *, json: dict, timeout: float) -> FakeResponse:
        calls.append((url, json, timeout))
        return FakeResponse(200, {"code": 1000, "msg": "ok"})

    result = WxPusherClient(post=post, timeout=4).send_message(
        "SPT_test-value",
        "AALC done",
        "done",
    )

    assert result["code"] == 1000
    assert calls == [
        (
            WXPUSHER_SIMPLE_PUSH_URL,
            {
                "content": "AALC done",
                "summary": "done",
                "contentType": 1,
                "spt": "SPT_test-value",
            },
            4,
        )
    ]


def test_wxpusher_api_error_does_not_echo_spt() -> None:
    client = WxPusherClient(
        post=lambda *_args, **_kwargs: FakeResponse(200, {"code": 1001, "msg": "bad"}),
        sleeper=lambda _delay: None,
    )

    with pytest.raises(WxPusherError, match="API 返回错误") as raised:
        client.send_message("SPT_private-value", "content", "summary")
    assert "SPT_private-value" not in str(raised.value)


def test_wxpusher_http_error_retries_transient_status() -> None:
    responses = iter(
        [
            FakeResponse(500, {}),
            FakeResponse(503, {}),
            FakeResponse(200, {"code": 1000}),
        ]
    )
    sleeps: list[float] = []
    calls = 0

    def post(*_args, **_kwargs):
        nonlocal calls
        calls += 1
        return next(responses)

    result = WxPusherClient(post=post, sleeper=sleeps.append).send_message("SPT_http-value", "content", "summary")

    assert result["code"] == 1000
    assert calls == 3
    assert sleeps == [0.5, 1.0]


def test_wxpusher_timeout_retries_and_eventually_fails_without_spt() -> None:
    sleeps: list[float] = []

    def post(*_args, **_kwargs):
        raise requests.exceptions.Timeout()

    client = WxPusherClient(post=post, sleeper=sleeps.append)
    with pytest.raises(WxPusherError, match="请求超时") as raised:
        client.send_message("SPT_timeout-value", "content", "summary")

    assert sleeps == [0.5, 1.0]
    assert "SPT_timeout-value" not in str(raised.value)


def test_notification_queue_is_fifo_and_sender_failures_are_local() -> None:
    calls: list[tuple[str, str, str]] = []
    sender = Mock()
    sender.send_message.side_effect = lambda spt, content, summary: calls.append((spt, content, summary))
    queue = NotificationQueue(sender, min_interval=0)
    try:
        assert queue.enqueue(NotificationMessage("SPT_queue", "one", "1"))
        assert queue.enqueue(NotificationMessage("SPT_queue", "two", "2"))
        assert queue.flush(2)
        assert calls == [
            ("SPT_queue", "one", "1"),
            ("SPT_queue", "two", "2"),
        ]

        sender.send_message.side_effect = RuntimeError("failed SPT_queue")
        with patch("module.notification.wxpusher.log.warning") as warning:
            queue.enqueue(NotificationMessage("SPT_queue", "three", "3"))
            assert queue.flush(2)
            warning.assert_called_once()
            assert "SPT_queue" not in str(warning.call_args)
    finally:
        queue.close(wait=True)


def test_notification_service_skips_unconfigured_spt_and_formats_summaries() -> None:
    sender = Mock()
    service = NotificationService(sender, queue_=NotificationQueue(sender, min_interval=0))
    try:
        assert not service.enqueue_completion("", "mirror", 1)
        assert not service.enqueue_final("", {"completed": {}})
        assert service.enqueue_completion("SPT_service", "thread", 4)
        assert service.queue.flush(2)
        sender.send_message.assert_called_once()
        assert sender.send_message.call_args.args[1:] == ("AALC 任务进度\n纽本完成 4 次", "纽本完成 4 次")
    finally:
        service.close()


def test_notification_formatters_cover_batch_final_and_failure_messages() -> None:
    content, summary = format_completion("exp", 3)
    assert content.endswith("经验本完成 3 次")
    assert summary == "经验本完成 3 次"

    final_content, final_summary = format_final_summary({"completed": {"exp": 1, "thread": 2, "mirror": 3}})
    assert "经验本：1 次" in final_content
    assert "纽本：2 次" in final_content
    assert final_summary == "AALC 任务已完成"

    failure_content, failure_summary = format_failure("safe failure")
    assert "safe failure" in failure_content
    assert failure_summary == "AALC 任务执行失败"


def test_config_and_exception_redaction_never_returns_registered_spt() -> None:
    secret = "SPT_redaction-value"
    register_secret(secret)
    config = {"wxpusher_spt": secret, "nested": [{"mirrorchyan_cdk": "CDK"}]}

    redacted = redact_mapping(config)
    assert redacted["wxpusher_spt"] == REDACTED_VALUE
    assert redacted["nested"][0]["mirrorchyan_cdk"] == REDACTED_VALUE
    assert secret not in redact_text(f"exception contains {secret}")


def test_log_filter_redacts_formatted_messages_and_exception_text() -> None:
    from module.logger.my_log import SecretRedactionFilter

    secret = "SPT_filter-value"
    register_secret(secret)
    record = logging.LogRecord("AALC", logging.ERROR, __file__, 1, "value=%s", (secret,), None)
    assert SecretRedactionFilter().filter(record)
    assert secret not in record.getMessage()

    try:
        raise RuntimeError(f"failure {secret}")
    except RuntimeError:
        record.exc_info = sys.exc_info()
    assert SecretRedactionFilter().filter(record)
    assert secret not in logging.Formatter().format(record)


def test_spt_validation_is_strict_and_does_not_echo_invalid_values() -> None:
    assert normalize_spt(" SPT_valid ") == "SPT_valid"
    with pytest.raises(WxPusherError, match="SPT 未配置"):
        normalize_spt("")
    with pytest.raises(WxPusherError, match="SPT 格式无效") as raised:
        normalize_spt("private-value")
    assert "private-value" not in str(raised.value)
