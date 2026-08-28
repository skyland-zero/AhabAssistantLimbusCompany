"""Build the Windows GPUI application, Python sidecar and updater."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIST = ROOT / "dist"
RELEASE = DIST / "AALC"


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


def copy_tree(source: Path, destination: Path) -> None:
    if not source.is_dir():
        raise FileNotFoundError(f"required release directory is missing: {source}")
    shutil.copytree(source, destination, dirs_exist_ok=True)


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
    copy_tree(ROOT / "assets", RELEASE / "assets")
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


def archive_release(version: str) -> Path:
    archive_base = DIST / f"AALC_{version}"
    seven_zip = shutil.which("7z") or shutil.which("7zz")
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
