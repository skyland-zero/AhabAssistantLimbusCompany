"""Fail when the fork adds an unreviewed edit to upstream-owned Python code."""

from __future__ import annotations

import argparse
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "upstream-boundary.toml"


def _git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    return result.stdout


def changed_paths(base: str) -> set[str]:
    outputs = (
        _git("diff", "--name-only", f"{base}...HEAD"),
        _git("diff", "--name-only"),
        _git("diff", "--cached", "--name-only"),
        _git("ls-files", "--others", "--exclude-standard"),
    )
    return {
        line.strip().replace("\\", "/")
        for output in outputs
        for line in output.splitlines()
        if line.strip()
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default=None)
    args = parser.parse_args()

    config = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    base = args.base or f"{config.get('upstream_remote', 'origin')}/{config.get('upstream_branch', 'main')}"
    protected = tuple(config["protected_prefixes"])
    allowed = set(config["allowed_protected_files"])
    expected_deleted = tuple(config.get("expected_deleted_prefixes", ()))
    violations = sorted(
        path for path in changed_paths(base) if path.startswith(protected) and path not in allowed
    )
    restored = [
        prefix
        for prefix in expected_deleted
        if (ROOT / prefix.rstrip("/")).exists()
    ]
    if not violations and not restored:
        sys.stdout.write(f"upstream boundary: OK against {base}\n")
        return 0

    if restored:
        sys.stderr.write("Product-owned deletions were unexpectedly restored:\n")
        for prefix in restored:
            sys.stderr.write(f"  - {prefix}\n")
    if violations:
        sys.stderr.write("Unreviewed edits were found in upstream-owned paths:\n")
    for path in violations:
        sys.stderr.write(f"  - {path}\n")
    sys.stderr.write(
        "Move the change behind a fork-owned adapter or document it in upstream-boundary.toml.\n"
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
