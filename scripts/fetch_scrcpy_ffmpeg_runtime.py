"""Fetch the official Scrcpy Windows FFmpeg runtime used by the native bridge."""

from __future__ import annotations

import argparse
import hashlib
import io
import urllib.request
import zipfile
from pathlib import Path

SCRCPY_VERSION = "v4.1"
ARCHIVE_URL = f"https://github.com/Genymobile/scrcpy/releases/download/{SCRCPY_VERSION}/scrcpy-win64-{SCRCPY_VERSION}.zip"
ARCHIVE_SHA256 = "5b12172b3264b2889f4583ee64752ce832e29bc8b1089dca81093459697165db"
ARCHIVE_ROOT = f"scrcpy-win64-{SCRCPY_VERSION}/"
RUNTIME_FILES = ("avcodec-62.dll", "avutil-60.dll", "swresample-6.dll", "LICENSE.txt")


def _download_archive() -> bytes:
    with urllib.request.urlopen(ARCHIVE_URL, timeout=120) as response:
        archive = response.read()
    digest = hashlib.sha256(archive).hexdigest()
    if digest != ARCHIVE_SHA256:
        raise RuntimeError(f"Scrcpy archive SHA-256 mismatch: expected {ARCHIVE_SHA256}, got {digest}")
    return archive


def ensure_runtime(output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)
    missing = [name for name in RUNTIME_FILES if not (output / name).is_file()]
    if not missing:
        return

    archive = _download_archive()
    with zipfile.ZipFile(io.BytesIO(archive)) as package:
        for name in missing:
            archive_name = ARCHIVE_ROOT + name
            try:
                content = package.read(archive_name)
            except KeyError as error:
                raise RuntimeError(f"Official Scrcpy archive is missing {archive_name}") from error
            (output / name).write_bytes(content)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    ensure_runtime(args.output.resolve())


if __name__ == "__main__":
    main()
