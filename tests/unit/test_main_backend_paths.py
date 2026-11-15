from __future__ import annotations

import os
from pathlib import Path

import main_backend


def test_version_reads_the_staged_release_file(tmp_path: Path, monkeypatch) -> None:
    names = (
        "AALC_CONFIG_PATH",
        "AALC_THEME_PACK_LIST_PATH",
        "AALC_THEME_PACK_WEIGHT_PATH",
        "AALC_VERSION_PATH",
    )
    for name in names:
        monkeypatch.delenv(name, raising=False)

    version = tmp_path / "assets" / "config" / "version.txt"
    version.parent.mkdir(parents=True)
    version.write_text("V0.0.2\n", encoding="utf-8")

    try:
        main_backend._configure_runtime_paths(tmp_path)
        assert os.environ["AALC_VERSION_PATH"] == str(version)
        assert main_backend._version() == "V0.0.2"
    finally:
        for name in names:
            os.environ.pop(name, None)
