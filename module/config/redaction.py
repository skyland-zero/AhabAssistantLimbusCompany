"""Shared secret redaction helpers for configuration and log boundaries."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from threading import Lock
from typing import Any, Iterable

REDACTED_VALUE = "[REDACTED]"

# Mirror-Chyan CDKs predate the GPUI settings page and are credentials too.
# Keeping both keys in one helper makes it harder for a future config snapshot
# or change log to accidentally expose either credential.
SENSITIVE_CONFIG_KEYS = frozenset({"mirrorchyan_cdk", "wxpusher_spt"})
_secret_values: set[str] = set()
_secret_lock = Lock()


def _is_sensitive_key(key: Any) -> bool:
    return isinstance(key, str) and key.lower() in SENSITIVE_CONFIG_KEYS


def redact_config_value(key: Any, value: Any) -> Any:
    """Return a log-safe representation of one config value."""

    if _is_sensitive_key(key):
        return "" if value in (None, "") else REDACTED_VALUE
    return redact_mapping(value)


def redact_mapping(value: Any) -> Any:
    """Recursively copy mappings/sequences while masking known secret keys."""

    if isinstance(value, Mapping):
        return {key: redact_config_value(key, nested) for key, nested in value.items()}
    if isinstance(value, Sequence) and not isinstance(value, (str, bytes, bytearray)):
        return [redact_mapping(item) for item in value]
    return value


def redact_text(value: Any, secrets: Iterable[Any] = ()) -> str:
    """Replace concrete secret values in user-facing/log text.

    Empty values are ignored so redaction never changes unrelated text.  The
    longest values are replaced first to handle a secret that is a prefix of a
    second secret in a test double or a legacy configuration.
    """

    result = str(value)
    with _secret_lock:
        registered = set(_secret_values)
    candidates = registered | {
        str(secret) for secret in secrets if secret not in (None, "") and isinstance(secret, str)
    }
    for secret in sorted(candidates, key=len, reverse=True):
        result = result.replace(secret, REDACTED_VALUE)
    return result


def register_secret(value: Any) -> None:
    """Remember a loaded credential so generic exception logs can redact it."""

    if not isinstance(value, str) or not value:
        return
    with _secret_lock:
        _secret_values.add(value)


def register_secrets(value: Any) -> None:
    """Register sensitive values found in a config-shaped object."""

    if isinstance(value, Mapping):
        for key, nested in value.items():
            if _is_sensitive_key(key):
                register_secret(nested)
            register_secrets(nested)
    elif isinstance(value, Sequence) and not isinstance(value, (str, bytes, bytearray)):
        for nested in value:
            register_secrets(nested)


__all__ = [
    "REDACTED_VALUE",
    "SENSITIVE_CONFIG_KEYS",
    "redact_config_value",
    "redact_mapping",
    "redact_text",
    "register_secret",
    "register_secrets",
]
