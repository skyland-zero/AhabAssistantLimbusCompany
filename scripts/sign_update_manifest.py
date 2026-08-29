"""Create an Ed25519-signed manifest for a packaged AALC archive."""

from __future__ import annotations

import argparse
import base64
import hashlib
import os
from pathlib import Path

from module.update.signature import canonical_manifest_bytes


def _load_private_key(value: str):
    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

    raw = value.strip().encode("ascii")
    if raw.startswith(b"-----BEGIN"):
        key = serialization.load_pem_private_key(raw, password=None)
        if not isinstance(key, Ed25519PrivateKey):
            raise ValueError("signing key must be Ed25519")
        return key
    try:
        decoded = base64.b64decode(raw, validate=True)
    except Exception:
        decoded = bytes.fromhex(raw.decode("ascii"))
    if len(decoded) == 64:
        decoded = decoded[:32]
    if len(decoded) != 32:
        raise ValueError("raw Ed25519 private key must be 32 bytes")
    return Ed25519PrivateKey.from_private_bytes(decoded)


def sign_archive(
    archive_path: Path,
    manifest_path: Path,
    signature_path: Path,
    *,
    version: str,
    key_id: str,
    private_key: str,
) -> None:
    if not archive_path.is_file():
        raise FileNotFoundError(archive_path)
    digest = hashlib.sha256()
    with archive_path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    payload = {
        "artifact": archive_path.name,
        "keyId": key_id,
        "schemaVersion": 1,
        "sha256": digest.hexdigest(),
        "size": archive_path.stat().st_size,
        "version": version,
    }
    manifest_bytes = canonical_manifest_bytes(payload)
    signature = _load_private_key(private_key).sign(manifest_bytes)
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    signature_path.parent.mkdir(parents=True, exist_ok=True)
    # Keep the exact signed bytes on disk; adding a trailing newline would
    # change the Ed25519 message and make otherwise valid signatures fail.
    manifest_path.write_bytes(manifest_bytes)
    signature_path.write_text(base64.b64encode(signature).decode("ascii") + "\n", encoding="ascii")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--signature", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--key-id", default="primary")
    parser.add_argument("--private-key-env", default="AALC_UPDATE_SIGNING_KEY")
    args = parser.parse_args()
    private_key = os.getenv(args.private_key_env)
    if not private_key:
        raise SystemExit(f"missing signing key environment variable: {args.private_key_env}")
    sign_archive(
        args.archive,
        args.manifest,
        args.signature,
        version=args.version,
        key_id=args.key_id,
        private_key=private_key,
    )


if __name__ == "__main__":
    main()
