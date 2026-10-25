from __future__ import annotations

from pathlib import Path

from updater import Updater


def _updater_for(root: Path) -> Updater:
    updater = Updater.__new__(Updater)
    updater.cover_folder_path = str(root)
    return updater


def test_legacy_onefile_backend_is_removed_after_onedir_upgrade(tmp_path: Path) -> None:
    (tmp_path / "AALC Backend.exe").write_bytes(b"legacy")
    staged = tmp_path / "backend" / "AALC Backend.exe"
    staged.parent.mkdir()
    staged.write_bytes(b"staged")

    _updater_for(tmp_path).remove_legacy_sidecar()

    assert not (tmp_path / "AALC Backend.exe").exists()
    assert staged.is_file()


def test_legacy_backend_is_kept_when_new_bundle_is_missing(tmp_path: Path) -> None:
    legacy = tmp_path / "AALC Backend.exe"
    legacy.write_bytes(b"legacy")

    _updater_for(tmp_path).remove_legacy_sidecar()

    assert legacy.is_file()
