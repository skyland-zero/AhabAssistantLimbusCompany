#!/usr/bin/env python3
"""Fail when anything outside the migration allowlist still points at `gpui-app/`.

`gpui-app/` is deleted at the end of the Native Shell migration, so every
reference to it — build scripts, CI, contract validation, docs — has to move to
`nsh-app/` first. This check keeps that from regressing between P4.5 and P5:

    python scripts/check_no_gpui_app_refs.py

Exit code 0 means the repository no longer depends on `gpui-app/` (the
allowlist below names the files that are still allowed to mention it while the
migration is in flight). Add a path to `ALLOWED` only with a reason.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

PATTERN = re.compile(r"gpui-app|gpui_app")

# Files that legitimately mention the old tree. Shrink this list as the
# migration advances; it must be empty at P5.
ALLOWED = {
    # The migration ledger documents where each file came from.
    "nsh-app/MIGRATION.md",
    # The one-time theme exporter reads the GPUI palette tables.
    "scripts/export_nsh_theme.py",
    # This checker.
    "scripts/check_no_gpui_app_refs.py",
}

SKIP_DIRS = {".git", "target", "node_modules", ".nsh", "dist", "build"}
SKIP_SUFFIXES = {
    ".png", ".jpg", ".jpeg", ".webp", ".gif", ".ico", ".icns", ".ttf", ".otf",
    ".woff", ".woff2", ".wav", ".mp4", ".bin", ".nshc", ".pyc", ".lock",
}
SKIP_FILES = {"Cargo.lock", "uv.lock"}


def iter_files() -> "list[Path]":
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(ROOT)
        if any(part in SKIP_DIRS for part in relative.parts):
            continue
        if path.suffix.lower() in SKIP_SUFFIXES or path.name in SKIP_FILES:
            continue
        # The old tree itself is not a reference to itself.
        if relative.parts and relative.parts[0] == "gpui-app":
            continue
        yield path


def main() -> int:
    hits: list[tuple[str, int, str]] = []
    for path in iter_files():
        relative = path.relative_to(ROOT).as_posix()
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        if not PATTERN.search(text):
            continue
        if relative in ALLOWED:
            continue
        for number, line in enumerate(text.splitlines(), start=1):
            if PATTERN.search(line):
                hits.append((relative, number, line.strip()))

    if not hits:
        allowed = ", ".join(sorted(ALLOWED)) or "nothing"
        print(f"ok: no dependency on gpui-app/ outside the allowlist ({allowed})")
        return 0

    print("error: these files still point at gpui-app/, which is going away:")
    for relative, number, line in hits:
        print(f"  {relative}:{number}: {line[:140]}")
    print(
        "\nRepoint them at nsh-app/ (see nsh-app/MIGRATION.md) or, if the mention"
        " is documentation of the migration itself, add the file to ALLOWED."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
