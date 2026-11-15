# ruff: noqa: E402,T201
"""End-to-end smoke test for the native Rust updater.

The fixture uses the same Ed25519 manifest format as release automation and
the same bundled 7-Zip runtime as the application, so it exercises the real
contract instead of a mocked one.

Usage:
    uv run python scripts/smoke_native_updater.py
    uv run python scripts/smoke_native_updater.py --updater native/updater/target/release/aalc-updater.exe
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from module.update.signature import canonical_manifest_bytes

SEVEN_ZIP = ROOT / "assets" / "binary" / "7za.exe"
DEFAULT_UPDATER = ROOT / "native" / "updater" / "target" / "release" / "aalc-updater.exe"
WAIT_SECONDS = 60.0


class SmokeFailure(RuntimeError):
    pass


def _write(path: Path, content: bytes | str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if isinstance(content, str):
        path.write_text(content, encoding="utf-8")
    else:
        path.write_bytes(content)


def _signed_archive(
    payload_root: Path,
    archive_path: Path,
    *,
    extra_payload: dict[str, str] | None = None,
) -> tuple[Path, Path, str]:
    """Create `AALC.nsh/` payload, archive it and sign the manifest."""

    tree = payload_root / "AALC.nsh"
    _write(tree / "AALC.nsh.exe", b"new frontend")
    _write(tree / "backend" / "AALC Backend.exe", b"new backend")
    _write(tree / "assets" / "images" / "new.png", b"new image")
    _write(tree / "README.md", b"new readme")
    _write(
        tree / "release.json",
        json.dumps(
            {
                "runtime": "shared",
                "runner": {"entry": "backend/AALCRunner.exe", "args": []},
            }
        ),
    )
    for relative, content in (extra_payload or {}).items():
        _write(tree / relative, content)

    subprocess.run(
        [str(SEVEN_ZIP), "a", "-mx=1", str(archive_path), "AALC.nsh/*"],
        cwd=payload_root,
        check=True,
        capture_output=True,
    )

    private_key = Ed25519PrivateKey.generate()
    public_key = private_key.public_key().public_bytes(
        serialization.Encoding.Raw,
        serialization.PublicFormat.Raw,
    )
    manifest = canonical_manifest_bytes(
        {
            "artifact": archive_path.name,
            "keyId": "primary",
            "schemaVersion": 1,
            "sha256": hashlib.sha256(archive_path.read_bytes()).hexdigest(),
            "size": archive_path.stat().st_size,
            "version": "v0.0.0-smoke",
        }
    )
    signature = private_key.sign(manifest)
    manifest_path = archive_path.with_suffix(".manifest.json")
    signature_path = archive_path.with_suffix(".manifest.sig")
    manifest_path.write_bytes(manifest)
    signature_path.write_text(base64.b64encode(signature).decode("ascii") + "\n", encoding="ascii")
    return manifest_path, signature_path, base64.b64encode(public_key).decode("ascii")


def _install_root(base: Path, updater: Path, public_key: str) -> Path:
    root = base / "install"
    root.mkdir(parents=True)
    _write(root / "AALC.nsh.exe", b"old frontend")
    _write(root / "backend" / "AALC Backend.exe", b"old backend")
    _write(root / "backend" / "AALCRunner.exe", b"old runner")
    _write(root / "assets" / "images" / "old.png", b"old image")
    _write(root / "config.yaml", b"user config")
    _write(root / "update_public_keys.json", json.dumps({"primary": public_key}))
    _write(root / "assets" / "binary" / "7za.exe", SEVEN_ZIP.read_bytes())
    shutil.copy2(updater, root / "AALC Updater.exe")
    return root


def _prepare_update(root: Path, manifest: Path, signature: Path, archive: Path) -> None:
    temp = root / "update_temp"
    temp.mkdir(exist_ok=True)
    shutil.copy2(archive, temp / archive.name)
    shutil.copy2(manifest, temp / manifest.name)
    shutil.copy2(signature, temp / signature.name)


def _run_updater(root: Path, archive_name: str, *, timeout: float = WAIT_SECONDS) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment["AALC_UPDATER_NO_RESTART"] = "1"
    environment["AALC_UPDATER_NO_MESSAGE_BOX"] = "1"
    environment["AALC_UPDATE_REQUIRE_SIGNATURE"] = "1"
    return subprocess.run(
        [str(root / "AALC Updater.exe"), archive_name],
        cwd=root,
        env=environment,
        # Do not hand the launcher's pipes to the detached updater: the
        # smoke harness would otherwise wait for the grandchild to close them.
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        text=True,
        timeout=timeout,
    )


def _wait_for(predicate, *, timeout: float = WAIT_SECONDS, message: str) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.1)
    raise SmokeFailure(message)


def _log(root: Path) -> str:
    path = root / "update_temp" / "updater.log"
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def _assert_file(root: Path, relative: str, expected: bytes) -> None:
    path = root / relative
    if not path.is_file() or path.read_bytes() != expected:
        raise SmokeFailure(f"{relative} was not updated (expected {expected!r})")


def _assert_missing(root: Path, relative: str) -> None:
    if (root / relative).exists():
        raise SmokeFailure(f"{relative} should have been removed")


def full_cover(updater: Path, base: Path) -> None:
    payload = base / "payload"
    archive = base / "AALC.nsh_smoke.7z"
    manifest, signature, public_key = _signed_archive(payload, archive)
    root = _install_root(base, updater, public_key)
    _prepare_update(root, manifest, signature, archive)

    result = _run_updater(root, archive.name)
    if result.returncode != 0:
        raise SmokeFailure(f"updater launcher failed: {result.stderr}")
    _wait_for(
        lambda: (root / "README.md").is_file() and not (root / "update_temp" / archive.name).exists(),
        message=f"full cover did not complete\nlog:\n{_log(root)}",
    )

    _assert_file(root, "AALC.nsh.exe", b"new frontend")
    _assert_file(root, "backend/AALC Backend.exe", b"new backend")
    _assert_file(root, "assets/images/new.png", b"new image")
    _assert_file(root, "config.yaml", b"user config")
    _assert_missing(root, "assets/images/old.png")
    # The current release still launches a standalone Runner, so it is kept.
    _assert_file(root, "backend/AALCRunner.exe", b"old runner")
    if (root / "assets" / "images" / "old.png").exists():
        raise SmokeFailure("assets/images was not replaced")
    if "signature verified" not in _log(root):
        raise SmokeFailure("signature verification was not logged")


def merged_layout_migration(updater: Path, base: Path) -> None:
    payload = base / "payload"
    archive = base / "AALC.nsh_smoke.7z"
    manifest, signature, public_key = _signed_archive(payload, archive)
    tree = payload / "AALC.nsh"
    _write(
        tree / "release.json",
        json.dumps(
            {
                "runtime": "shared",
                "runner": {
                    "entry": "backend/AALC Backend.exe",
                    "args": ["--execution-runner"],
                },
            }
        ),
    )
    # Re-create the archive and manifest after rewriting release.json.
    subprocess.run(
        [str(SEVEN_ZIP), "a", "-mx=1", str(archive), "AALC.nsh/*"],
        cwd=payload,
        check=True,
        capture_output=True,
    )
    manifest.write_bytes(
        canonical_manifest_bytes(
            {
                "artifact": archive.name,
                "keyId": "primary",
                "schemaVersion": 1,
                "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
                "size": archive.stat().st_size,
                "version": "v0.0.0-smoke",
            }
        )
    )
    # The signature from _signed_archive no longer matches; sign the new bytes.
    private_key = Ed25519PrivateKey.generate()
    # Replace the public key too so the fixture stays self-consistent.
    public_key = base64.b64encode(
        private_key.public_key().public_bytes(
            serialization.Encoding.Raw,
            serialization.PublicFormat.Raw,
        )
    ).decode("ascii")
    signature.write_text(
        base64.b64encode(private_key.sign(manifest.read_bytes())).decode("ascii") + "\n",
        encoding="ascii",
    )

    root = _install_root(base, updater, public_key)
    _write(root / "runner" / "AALCRunner.exe", b"old separate runner")
    _prepare_update(root, manifest, signature, archive)

    result = _run_updater(root, archive.name)
    if result.returncode != 0:
        raise SmokeFailure(f"updater launcher failed: {result.stderr}")
    _wait_for(
        lambda: (root / "README.md").is_file() and not (root / "update_temp" / archive.name).exists(),
        message=f"merged migration did not complete\nlog:\n{_log(root)}",
    )

    _assert_missing(root, "backend/AALCRunner.exe")
    _assert_missing(root, "runner")
    _assert_file(root, "backend/AALC Backend.exe", b"new backend")


def tampered_archive(updater: Path, base: Path) -> None:
    payload = base / "payload"
    archive = base / "AALC.nsh_smoke.7z"
    manifest, signature, public_key = _signed_archive(payload, archive)
    root = _install_root(base, updater, public_key)
    _prepare_update(root, manifest, signature, archive)
    archive_in_temp = root / "update_temp" / archive.name
    content = bytearray(archive_in_temp.read_bytes())
    content[-1] ^= 0xFF
    archive_in_temp.write_bytes(content)

    result = _run_updater(root, archive.name)
    if result.returncode != 0:
        raise SmokeFailure(f"updater launcher failed: {result.stderr}")
    _wait_for(
        lambda: "SHA-256" in _log(root) or "签名" in _log(root),
        message=f"tampered archive was not rejected\nlog:\n{_log(root)}",
    )
    _assert_file(root, "AALC.nsh.exe", b"old frontend")
    if (root / "README.md").exists():
        raise SmokeFailure("tampered archive must not install files")


def incremental_update(updater: Path, base: Path) -> None:
    payload = base / "payload"
    archive = base / "AALC.nsh_smoke.7z"
    manifest, signature, public_key = _signed_archive(
        payload,
        archive,
        extra_payload={
            "version.txt": "new version",
            "new.txt": "new file",
            "changes.json": json.dumps(
                {
                    "deleted": ["old.txt"],
                    "deleted_dir": ["old_dir"],
                    "added": ["new.txt"],
                    "modified": ["version.txt"],
                }
            ),
        },
    )
    root = _install_root(base, updater, public_key)
    _write(root / "old.txt", b"old file")
    _write(root / "old_dir" / "nested.txt", b"old directory")
    _write(root / "version.txt", b"old version")
    _prepare_update(root, manifest, signature, archive)

    result = _run_updater(root, archive.name)
    if result.returncode != 0:
        raise SmokeFailure(f"updater launcher failed: {result.stderr}")
    _wait_for(
        lambda: (root / "new.txt").is_file() and not (root / "update_temp" / archive.name).exists(),
        message=f"incremental update did not complete\nlog:\n{_log(root)}",
    )

    _assert_file(root, "new.txt", b"new file")
    _assert_file(root, "version.txt", b"new version")
    _assert_missing(root, "old.txt")
    _assert_missing(root, "old_dir")
    # Incremental updates do not delete unrelated user files.
    _assert_file(root, "config.yaml", b"user config")
    # release.json describes the runtime layout and is refreshed even when an
    # incremental plan does not list it.
    _assert_file(
        root,
        "release.json",
        (payload / "AALC.nsh" / "release.json").read_bytes(),
    )


def handoff(updater: Path, base: Path) -> None:
    payload = base / "payload"
    archive = base / "AALC.nsh_smoke.7z"
    manifest, signature, public_key = _signed_archive(
        payload,
        archive,
        extra_payload={"AALC Updater.exe": updater.read_bytes()},
    )
    root = _install_root(base, updater, public_key)
    _prepare_update(root, manifest, signature, archive)

    result = _run_updater(root, archive.name)
    if result.returncode != 0:
        raise SmokeFailure(f"updater launcher failed: {result.stderr}")
    _wait_for(
        lambda: (root / "README.md").is_file()
        and "handed off to bundled updater" in _log(root)
        and not (root / "update_temp" / archive.name).exists(),
        message=f"handoff did not complete\nlog:\n{_log(root)}",
    )
    _assert_file(root, "AALC.nsh.exe", b"new frontend")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--updater", type=Path, default=DEFAULT_UPDATER)
    parser.add_argument("--keep", action="store_true", help="keep temporary fixtures")
    parser.add_argument(
        "--test",
        action="append",
        choices=[
            "full-cover",
            "merged-migration",
            "tampered-archive",
            "incremental-update",
            "handoff",
        ],
        help="run only the named fixture (repeatable)",
    )
    args = parser.parse_args()

    updater = args.updater.resolve()
    if not updater.is_file():
        raise SystemExit(f"native updater is missing: {updater}")
    if not SEVEN_ZIP.is_file():
        raise SystemExit(f"bundled 7za is missing: {SEVEN_ZIP}")

    base = Path(tempfile.mkdtemp(prefix="aalc-native-updater-smoke-"))
    failures: list[str] = []
    try:
        tests = [
            ("full-cover", full_cover),
            ("merged-migration", merged_layout_migration),
            ("tampered-archive", tampered_archive),
            ("incremental-update", incremental_update),
            ("handoff", handoff),
        ]
        if args.test:
            tests = [item for item in tests if item[0] in set(args.test)]
        for label, test in tests:
            directory = base / label
            directory.mkdir(parents=True)
            try:
                test(updater, directory)
            except Exception as error:  # noqa: BLE001 - report every fixture failure
                failures.append(f"{label}: {error}")
                print(f"FAIL {label}: {error}", file=sys.stderr, flush=True)
            else:
                print(f"PASS {label}", flush=True)
    finally:
        if args.keep:
            print(f"fixtures kept at {base}")
        else:
            shutil.rmtree(base, ignore_errors=True)

    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        return 1
    print("native updater smoke tests passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
