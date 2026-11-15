from __future__ import annotations

import json
from pathlib import Path

import main_backend


def _layout(root: Path, release: dict) -> Path:
    (root / "release.json").write_text(json.dumps(release), encoding="utf-8")
    (root / "backend").mkdir()
    (root / "backend" / "AALC Backend.exe").write_bytes(b"backend")
    (root / "backend" / "AALCRunner.exe").write_bytes(b"runner")
    (root / "runner").mkdir()
    (root / "runner" / "AALCRunner.exe").write_bytes(b"runner")
    return root


def test_merged_release_removes_the_legacy_runner_layout(tmp_path: Path) -> None:
    root = _layout(
        tmp_path,
        {
            "runtime": "shared",
            "runner": {
                "entry": "backend/AALC Backend.exe",
                "args": ["--execution-runner"],
            },
        },
    )

    removed = main_backend._remove_legacy_runtime_layout(root)

    assert len(removed) == 2
    assert not (root / "backend" / "AALCRunner.exe").exists()
    assert not (root / "runner").exists()


def test_current_shared_release_keeps_the_backend_runner(tmp_path: Path) -> None:
    root = _layout(
        tmp_path,
        {"runtime": "shared", "runner": {"entry": "backend/AALCRunner.exe", "args": []}},
    )

    removed = main_backend._remove_legacy_runtime_layout(root)

    assert len(removed) == 1
    assert (root / "backend" / "AALCRunner.exe").exists()
    assert not (root / "runner").exists()


def test_separate_release_removes_only_the_stale_backend_runner(tmp_path: Path) -> None:
    root = _layout(
        tmp_path,
        {"runtime": "separate", "runner": {"entry": "runner/AALCRunner.exe", "args": []}},
    )

    removed = main_backend._remove_legacy_runtime_layout(root)

    assert len(removed) == 1
    assert not (root / "backend" / "AALCRunner.exe").exists()
    assert (root / "runner").exists()


def test_missing_or_malformed_manifest_keeps_the_layout(tmp_path: Path) -> None:
    (tmp_path / "backend").mkdir()
    (tmp_path / "backend" / "AALC Backend.exe").write_bytes(b"backend")
    (tmp_path / "backend" / "AALCRunner.exe").write_bytes(b"runner")

    assert main_backend._remove_legacy_runtime_layout(tmp_path) == []
    assert (tmp_path / "backend" / "AALCRunner.exe").exists()

    (tmp_path / "release.json").write_text("not json", encoding="utf-8")
    assert main_backend._remove_legacy_runtime_layout(tmp_path) == []
    assert (tmp_path / "backend" / "AALCRunner.exe").exists()
