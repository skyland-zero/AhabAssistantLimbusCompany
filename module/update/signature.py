"""Ed25519 verification for downloadable application archives.

The updater intentionally verifies the bytes it received before extracting or
terminating any running application.  Release automation signs the compact,
sorted JSON representation produced by :func:`canonical_manifest_bytes`.
Public keys are supplied by the packaged configuration or the
``AALC_UPDATE_PUBLIC_KEYS`` environment variable (a JSON ``keyId -> base64``
mapping), which keeps private signing material out of the client repository.
"""

from __future__ import annotations

import base64
import binascii
import hashlib
import json
import os
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping


class UpdateSignatureError(ValueError):
    """Raised when a downloaded update cannot be authenticated or matched."""


@dataclass(frozen=True)
class SignedUpdateManifest:
    """Validated metadata bound to one update archive."""

    schema_version: int
    version: str
    artifact: str
    size: int
    sha256: str
    key_id: str


def canonical_manifest_bytes(manifest: Mapping[str, Any]) -> bytes:
    """Return the deterministic bytes used by CI when signing a manifest."""

    return json.dumps(
        dict(manifest),
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")


def _decode_signature(value: bytes | str) -> bytes:
    if isinstance(value, bytes):
        # A detached Ed25519 signature is binary.  Do not strip it before
        # checking its length: valid signatures may end in whitespace bytes.
        if len(value) == 64:
            return value
        raw = value.strip()
    else:
        raw = value.strip().encode("ascii")
    if not raw:
        raise UpdateSignatureError("更新签名为空")
    try:
        decoded = base64.b64decode(raw, validate=True)
    except (ValueError, binascii.Error) as error:
        try:
            decoded = bytes.fromhex(raw.decode("ascii"))
        except (ValueError, UnicodeDecodeError) as hex_error:
            raise UpdateSignatureError("更新签名不是有效的 base64 或十六进制数据") from hex_error
        if not decoded:
            raise UpdateSignatureError("更新签名为空") from error
    if len(decoded) != 64:
        raise UpdateSignatureError("Ed25519 签名长度无效")
    return decoded


def _decode_public_key(value: bytes | str):
    try:
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
    except ImportError as error:  # pragma: no cover - dependency is locked in pyproject
        raise UpdateSignatureError("当前环境缺少 Ed25519 验证依赖") from error

    if isinstance(value, bytes):
        # Raw public keys are exactly 32 bytes and may legitimately contain
        # leading/trailing whitespace values.
        if len(value) == 32:
            return Ed25519PublicKey.from_public_bytes(value)
        raw = value.strip()
        if len(raw) == 32:
            return Ed25519PublicKey.from_public_bytes(raw)
    else:
        text = value.strip()
        if text.startswith("-----BEGIN"):
            try:
                from cryptography.hazmat.primitives import serialization
                from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

                key = serialization.load_pem_public_key(text.encode("ascii"))
                if not isinstance(key, Ed25519PublicKey):
                    raise UpdateSignatureError("PEM 公钥不是 Ed25519")
                return key
            except (ValueError, TypeError, UnicodeEncodeError) as error:
                raise UpdateSignatureError("Ed25519 公钥 PEM 无效") from error
        raw = text.encode("ascii")
    try:
        decoded = base64.b64decode(raw, validate=True)
    except ValueError, binascii.Error:
        try:
            decoded = bytes.fromhex(raw.decode("ascii"))
        except (ValueError, UnicodeDecodeError) as error:
            raise UpdateSignatureError("Ed25519 公钥不是有效的 base64 或十六进制数据") from error
    if len(decoded) != 32:
        raise UpdateSignatureError("Ed25519 公钥长度无效")
    return Ed25519PublicKey.from_public_bytes(decoded)


def _public_key_mapping(public_keys: Mapping[str, bytes | str] | bytes | str | None) -> dict[str, bytes | str]:
    if public_keys is None:
        configured = os.getenv("AALC_UPDATE_PUBLIC_KEYS", "").strip()
        if not configured:
            configured = os.getenv("AALC_UPDATE_PUBLIC_KEY", "").strip()
        if not configured:
            raise UpdateSignatureError("未配置更新公钥")
        public_keys = configured
    if isinstance(public_keys, Mapping):
        return {str(key): value for key, value in public_keys.items()}
    if isinstance(public_keys, bytes):
        return {"primary": public_keys}
    text = public_keys.strip()
    if text.startswith("{"):
        try:
            parsed = json.loads(text)
        except json.JSONDecodeError as error:
            raise UpdateSignatureError("更新公钥映射不是有效 JSON") from error
        if not isinstance(parsed, dict):
            raise UpdateSignatureError("更新公钥映射必须是对象")
        return {str(key): value for key, value in parsed.items()}
    return {"primary": text}


def _parse_manifest(manifest_bytes: bytes) -> dict[str, Any]:
    try:
        payload = json.loads(manifest_bytes)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise UpdateSignatureError("更新清单不是有效 JSON") from error
    if not isinstance(payload, dict):
        raise UpdateSignatureError("更新清单必须是对象")
    expected = {"schemaVersion", "version", "artifact", "size", "sha256", "keyId"}
    if set(payload) != expected:
        raise UpdateSignatureError("更新清单字段不完整或包含未知字段")
    if payload.get("schemaVersion") != 1:
        raise UpdateSignatureError("更新清单协议版本不兼容")
    if not isinstance(payload.get("version"), str) or not payload["version"].strip():
        raise UpdateSignatureError("更新清单版本无效")
    artifact = payload.get("artifact")
    if not isinstance(artifact, str) or not artifact or Path(artifact).name != artifact:
        raise UpdateSignatureError("更新清单 artifact 必须是文件名")
    size = payload.get("size")
    if not isinstance(size, int) or isinstance(size, bool) or size < 0:
        raise UpdateSignatureError("更新清单文件大小无效")
    digest = payload.get("sha256")
    if not isinstance(digest, str) or len(digest) != 64:
        raise UpdateSignatureError("更新清单 SHA-256 无效")
    try:
        int(digest, 16)
    except ValueError as error:
        raise UpdateSignatureError("更新清单 SHA-256 无效") from error
    key_id = payload.get("keyId")
    if not isinstance(key_id, str) or not key_id:
        raise UpdateSignatureError("更新清单 keyId 无效")
    return payload


def verify_signed_manifest(
    manifest_bytes: bytes,
    signature: bytes | str,
    archive_path: str | Path,
    public_keys: Mapping[str, bytes | str] | bytes | str | None = None,
) -> SignedUpdateManifest:
    """Verify signature, archive identity, size, and SHA-256 before install."""

    payload = _parse_manifest(manifest_bytes)
    keys = _public_key_mapping(public_keys)
    key_id = payload["keyId"]
    if key_id not in keys:
        raise UpdateSignatureError(f"更新清单引用了未知公钥：{key_id}")
    try:
        _decode_public_key(keys[key_id]).verify(_decode_signature(signature), manifest_bytes)
    except UpdateSignatureError:
        raise
    except Exception as error:
        raise UpdateSignatureError("更新清单签名校验失败") from error

    archive = Path(archive_path)
    if not archive.is_file():
        raise UpdateSignatureError("更新包不存在")
    if archive.name != payload["artifact"]:
        raise UpdateSignatureError("更新清单与下载文件名不匹配")
    actual_size = archive.stat().st_size
    if actual_size != payload["size"]:
        raise UpdateSignatureError("更新包大小与清单不匹配")
    digest = hashlib.sha256()
    with archive.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    if digest.hexdigest().lower() != payload["sha256"].lower():
        raise UpdateSignatureError("更新包 SHA-256 与清单不匹配")
    return SignedUpdateManifest(
        schema_version=payload["schemaVersion"],
        version=payload["version"],
        artifact=payload["artifact"],
        size=payload["size"],
        sha256=payload["sha256"].lower(),
        key_id=key_id,
    )


def verify_signed_manifest_files(
    manifest_path: str | Path,
    signature_path: str | Path,
    archive_path: str | Path,
    public_keys: Mapping[str, bytes | str] | bytes | str | None = None,
) -> SignedUpdateManifest:
    """Read and verify a manifest/signature pair from disk."""

    try:
        manifest_bytes = Path(manifest_path).read_bytes()
        signature = Path(signature_path).read_bytes()
    except OSError as error:
        raise UpdateSignatureError("无法读取更新清单或签名") from error
    return verify_signed_manifest(manifest_bytes, signature, archive_path, public_keys)


__all__ = [
    "SignedUpdateManifest",
    "UpdateSignatureError",
    "canonical_manifest_bytes",
    "verify_signed_manifest",
    "verify_signed_manifest_files",
]
