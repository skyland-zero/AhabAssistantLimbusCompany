"""Build the Windows GPUI application, Python sidecar and updater."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIST = ROOT / "dist"
RELEASE = DIST / "AALC"
NATIVE_DECODER_MANIFEST = ROOT / "native" / "scrcpy_decoder" / "Cargo.toml"
NATIVE_DECODER_BINARY = ROOT / "native" / "scrcpy_decoder" / "target" / "release" / "scrcpy_decoder.dll"
SCRCPY_RUNTIME_DIR = ROOT / "assets" / "binary" / "scrcpy-ffmpeg"


def run(command: list[str], *, cwd: Path = ROOT) -> None:
    sys.stdout.write(f"+ {' '.join(command)}\n")
    sys.stdout.flush()
    subprocess.run(command, cwd=cwd, check=True)


def build_python_executable(spec: str) -> None:
    run(
        [
            sys.executable,
            "-m",
            "PyInstaller",
            spec,
            "--noconfirm",
            "--clean",
        ]
    )


def build_native_decoder() -> None:
    run(
        [
            "cargo",
            "+nightly",
            "build",
            "--release",
            "--manifest-path",
            str(NATIVE_DECODER_MANIFEST),
        ]
    )
    if not NATIVE_DECODER_BINARY.is_file():
        raise FileNotFoundError(f"native decoder build output is missing: {NATIVE_DECODER_BINARY}")
    SCRCPY_RUNTIME_DIR.mkdir(parents=True, exist_ok=True)
    target_dll = SCRCPY_RUNTIME_DIR / "scrcpy_decoder.dll"
    try:
        shutil.copy2(NATIVE_DECODER_BINARY, target_dll)
    except PermissionError:
        # 当本地有正在运行的实例占用动态库时，采用 Windows NTFS 重命名替换策略
        backup = SCRCPY_RUNTIME_DIR / "scrcpy_decoder.dll.old"
        try:
            if backup.is_file():
                backup.unlink(missing_ok=True)
            target_dll.rename(backup)
            shutil.copy2(NATIVE_DECODER_BINARY, target_dll)
        except Exception as error:
            sys.stdout.write(f"Warning: could not overwrite {target_dll.name} ({error}); using existing runtime\n")
    run(
        [
            sys.executable,
            str(ROOT / "scripts" / "fetch_scrcpy_ffmpeg_runtime.py"),
            "--output",
            str(SCRCPY_RUNTIME_DIR),
        ]
    )


def copy_tree(source: Path, destination: Path, ignore=None) -> None:
    if not source.is_dir():
        raise FileNotFoundError(f"required release directory is missing: {source}")
    shutil.copytree(source, destination, dirs_exist_ok=True, ignore=ignore)


def stage_release(version: str) -> None:
    RELEASE.mkdir(parents=True, exist_ok=True)

    gpui_binary = ROOT / "gpui-app" / "target" / "release" / "ahab-gpui-app.exe"
    backend_binary = DIST / "AALC Backend.exe"
    updater_binary = DIST / "AALC Updater.exe"
    for path in (gpui_binary, backend_binary, updater_binary):
        if not path.is_file():
            raise FileNotFoundError(f"required build output is missing: {path}")

    shutil.copy2(gpui_binary, RELEASE / "AALC.exe")
    shutil.copy2(backend_binary, RELEASE / "AALC Backend.exe")
    shutil.copy2(updater_binary, RELEASE / "AALC Updater.exe")
    shutil.copy2(ROOT / "README.md", RELEASE / "README.md")
    shutil.copy2(ROOT / "LICENSE", RELEASE / "LICENSE")
    copy_tree(
        ROOT / "assets",
        RELEASE / "assets",
        ignore=shutil.ignore_patterns("wheels", "*.whl"),
    )
    copy_tree(ROOT / "gpui-app" / "resources", RELEASE / "resources")

    version_file = RELEASE / "assets" / "config" / "version.txt"
    version_file.parent.mkdir(parents=True, exist_ok=True)
    version_file.write_text(version, encoding="utf-8")

    (RELEASE / "release.json").write_text(
        json.dumps(
            {
                "version": version,
                "frontend": "AALC.exe",
                "backend": "AALC Backend.exe",
                "updater": "AALC Updater.exe",
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )

    # Embed the public verification key set in the shipped client.  Private
    # signing material is never written to the release tree; CI supplies it
    # through protected environment secrets when publishing a release.
    public_keys = os.getenv("AALC_UPDATE_PUBLIC_KEYS", "").strip()
    if public_keys:
        try:
            parsed_keys = json.loads(public_keys)
        except json.JSONDecodeError as error:
            raise ValueError("AALC_UPDATE_PUBLIC_KEYS must be a JSON object") from error
        if not isinstance(parsed_keys, dict) or not parsed_keys:
            raise ValueError("AALC_UPDATE_PUBLIC_KEYS must be a non-empty JSON object")
        (RELEASE / "update_public_keys.json").write_text(
            json.dumps(parsed_keys, ensure_ascii=False, sort_keys=True, separators=(",", ":")),
            encoding="utf-8",
        )


def archive_release(version: str) -> Path:
    archive_base = DIST / f"AALC_{version}"
    bundled_7z = ROOT / "assets" / "binary" / "7za.exe"
    seven_zip = (
        shutil.which("7z")
        or shutil.which("7zz")
        or (str(bundled_7z) if bundled_7z.is_file() else None)
    )
    if seven_zip:
        run([seven_zip, "a", "-mx=7", f"{archive_base}.7z", "AALC/*"], cwd=DIST)
        return archive_base.with_suffix(".7z")

    # A zip fallback keeps local builds usable on machines without 7-Zip;
    # CI installs 7-Zip and therefore publishes the normal .7z artifact.
    archive = Path(shutil.make_archive(str(archive_base), "zip", root_dir=DIST, base_dir="AALC"))
    sys.stdout.write(f"7z/7zz not found; created fallback archive: {archive}\n")
    return archive


def build(version: str) -> Path:
    if DIST.exists():
        shutil.rmtree(DIST)

    run(
        [
            "cargo",
            "+nightly",
            "build",
            "--release",
            "--manifest-path",
            "gpui-app/Cargo.toml",
        ]
    )
    build_native_decoder()
    build_python_executable("main_backend.spec")
    build_python_executable("updater.spec")
    stage_release(version)
    archive = archive_release(version)
    sys.stdout.write(f"Release ready: {archive}\n")
    return archive


def main() -> None:
    parser = argparse.ArgumentParser(description="Build AALC GPUI + Python sidecar")
    parser.add_argument("--version", default="dev", help="release version")
    args = parser.parse_args()
    build(args.version)


if __name__ == "__main__":
    main()
