from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path


def _load_build_module():
    path = Path(__file__).resolve().parents[2] / "scripts" / "build.py"
    spec = importlib.util.spec_from_file_location("aalc_build_script", path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


build = _load_build_module()


def test_stage_runner_bundle_copies_complete_onedir_tree(tmp_path: Path) -> None:
    source = tmp_path / "AALCRunner"
    (source / "_internal" / "onnxruntime").mkdir(parents=True)
    (source / "AALCRunner.exe").write_bytes(b"runner")
    (source / "_internal" / "python.dll").write_bytes(b"runtime")
    (source / "_internal" / "onnxruntime" / "provider.dll").write_bytes(b"provider")

    destination = tmp_path / "release" / "runner"
    destination.mkdir(parents=True)
    (destination / "stale.dll").write_bytes(b"stale")
    assert build.stage_runner_bundle(source, destination) == destination
    assert (destination / "AALCRunner.exe").read_bytes() == b"runner"
    assert (destination / "_internal" / "onnxruntime" / "provider.dll").is_file()
    assert not (destination / "stale.dll").exists()


def test_runner_bundle_entries_are_relative_sorted_and_hashed(tmp_path: Path) -> None:
    bundle = tmp_path / "runner"
    (bundle / "z").mkdir(parents=True)
    (bundle / "a.txt").write_bytes(b"alpha")
    (bundle / "z" / "b.bin").write_bytes(b"bravo")

    entries = build.build_bundle_file_entries(bundle)
    assert [entry["path"] for entry in entries] == ["runner/a.txt", "runner/z/b.bin"]
    assert entries[0] == {
        "path": "runner/a.txt",
        "sha256": hashlib.sha256(b"alpha").hexdigest(),
        "size": 5,
    }
    assert all(not Path(str(entry["path"])).is_absolute() for entry in entries)


def test_release_manifest_records_runner_entry_and_all_bundle_metadata(tmp_path: Path) -> None:
    release = tmp_path / "release"
    runner = release / "runner"
    runner.mkdir(parents=True)
    (runner / "AALCRunner.exe").write_bytes(b"runner")
    (runner / "_internal.dll").write_bytes(b"runtime")

    manifest = build.build_release_manifest("1.2.3", release_root=release)
    assert manifest["runner"]["entry"] == "runner/AALCRunner.exe"
    assert manifest["runner"]["files"] == [
        {
            "path": "runner/AALCRunner.exe",
            "sha256": hashlib.sha256(b"runner").hexdigest(),
            "size": 6,
        },
        {
            "path": "runner/_internal.dll",
            "sha256": hashlib.sha256(b"runtime").hexdigest(),
            "size": 7,
        },
    ]

    manifest_path = build.write_release_manifest("1.2.3", release_root=release)
    assert json.loads(manifest_path.read_text(encoding="utf-8")) == manifest
