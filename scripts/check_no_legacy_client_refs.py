#!/usr/bin/env python3
"""Fail when source, scripts, CI or docs still point at the deleted old tree.

The old GPUI client tree was removed at P5 of the Native Shell migration. Every
reference — build scripts, CI, contract validation, docs — now belongs to
`nsh-app/`. This check keeps that from regressing:

    python scripts/check_no_legacy_client_refs.py

Exit code 0 means the repository no longer points at the old tree.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHECKER = Path(__file__).resolve()

PATTERN = re.compile(r"gpui-app")

SKIP_DIRS = {
    ".git", "target", "node_modules", ".nsh", "dist", "build",
    # Generated caches and logs are not source dependencies.
    ".tgrep", "artifacts",
}
SKIP_SUFFIXES = {
    ".png", ".jpg", ".jpeg", ".webp", ".gif", ".ico", ".icns", ".ttf", ".otf",
    ".woff", ".woff2", ".wav", ".mp4", ".bin", ".nshc", ".pyc", ".lock",
}
SKIP_FILES = {"Cargo.lock", "uv.lock"}


def iter_files() -> "list[Path]":
    for path in ROOT.rglob("*"):
        if not path.is_file() or path.resolve() == CHECKER:
            continue
        relative = path.relative_to(ROOT)
        if any(part in SKIP_DIRS for part in relative.parts):
            continue
        if path.suffix.lower() in SKIP_SUFFIXES or path.name in SKIP_FILES:
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
        for number, line in enumerate(text.splitlines(), start=1):
            if PATTERN.search(line):
                hits.append((relative, number, line.strip()))

    if not hits:
        print("ok: no dependency on the deleted old client tree")
        return 0

    print("error: these files still point at the deleted old client tree:")
    for relative, number, line in hits:
        print(f"  {relative}:{number}: {line[:140]}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
