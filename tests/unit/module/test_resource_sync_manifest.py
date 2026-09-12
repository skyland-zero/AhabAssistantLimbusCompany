"""Remote resource manifest boundary tests.

The manifest is downloaded from a remote source and its ``path`` values are
joined to the local assets directory, so absolute paths, drive letters and
parent-directory segments must be rejected while deserializing.
"""

from __future__ import annotations

import pytest

from module.resource_sync.manifest import ResourceFileEntry, ResourcePackageEntry


def _entry(path: str) -> ResourceFileEntry:
    return ResourceFileEntry.from_dict({"path": path, "sha256": "0" * 64, "size": 1})


def test_relative_paths_are_accepted() -> None:
    assert _entry("status_effects/bleed.png").path == "status_effects/bleed.png"
    assert _entry("a/b/c.webp").path == "a/b/c.webp"


@pytest.mark.parametrize(
    "path",
    [
        "../secrets.png",
        "a/../../secrets.png",
        "/etc/passwd",
        "//server/share/file.png",
        "C:/Windows/system32/evil.png",
        "C:evil.png",
        r"..\..\evil.png",
        "a/b/../../../evil.png",
        "file.png:stream",
        "  leading.png",
        "trailing.png  ",
        "",
        "a//b.png",
        "./file.png",
    ],
)
def test_unsafe_paths_are_rejected(path: str) -> None:
    with pytest.raises(ValueError):
        _entry(path)


def test_package_path_uses_the_same_validation() -> None:
    good = ResourcePackageEntry.from_dict(
        {"path": "packages/images.7z", "sha256": "0" * 64, "size": 1, "format": "7z"}
    )
    assert good.path == "packages/images.7z"
    with pytest.raises(ValueError):
        ResourcePackageEntry.from_dict(
            {"path": "../images.7z", "sha256": "0" * 64, "size": 1, "format": "7z"}
        )
