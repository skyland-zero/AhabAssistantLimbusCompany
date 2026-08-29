from __future__ import annotations

import base64
import hashlib
import json

import pytest
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from module.update.signature import (
    UpdateSignatureError,
    canonical_manifest_bytes,
    verify_signed_manifest,
)


def _signed_payload(archive_name: str, content: bytes):
    private_key = Ed25519PrivateKey.generate()
    public_key = private_key.public_key().public_bytes(
        serialization.Encoding.Raw,
        serialization.PublicFormat.Raw,
    )
    payload = {
        "artifact": archive_name,
        "keyId": "primary",
        "schemaVersion": 1,
        "sha256": hashlib.sha256(content).hexdigest(),
        "size": len(content),
        "version": "v1.2.3",
    }
    manifest = canonical_manifest_bytes(payload)
    return payload, manifest, private_key.sign(manifest), base64.b64encode(public_key).decode("ascii")


def test_signed_manifest_binds_archive_identity_and_hash(tmp_path) -> None:
    content = b"signed archive fixture"
    archive = tmp_path / "AALC_v1.2.3.7z"
    archive.write_bytes(content)
    _, manifest, signature, public_key = _signed_payload(archive.name, content)

    verified = verify_signed_manifest(manifest, signature, archive, public_key)

    assert verified.version == "v1.2.3"
    assert verified.sha256 == hashlib.sha256(content).hexdigest()


def test_signed_manifest_rejects_tampered_archive(tmp_path) -> None:
    archive = tmp_path / "AALC_v1.2.3.7z"
    content = b"original"
    archive.write_bytes(content)
    _, manifest, signature, public_key = _signed_payload(archive.name, content)
    archive.write_bytes(b"tampered")

    with pytest.raises(UpdateSignatureError, match="SHA-256"):
        verify_signed_manifest(manifest, signature, archive, public_key)


def test_signed_manifest_rejects_unknown_fields(tmp_path) -> None:
    content = b"archive"
    archive = tmp_path / "AALC.7z"
    archive.write_bytes(content)
    payload, _, signature, public_key = _signed_payload(archive.name, content)
    payload["unexpected"] = True
    manifest = json.dumps(payload, separators=(",", ":")).encode("utf-8")

    with pytest.raises(UpdateSignatureError, match="字段"):
        verify_signed_manifest(manifest, signature, archive, public_key)
