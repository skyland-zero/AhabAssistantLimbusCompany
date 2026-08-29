"""Crash-safe helpers for small runtime configuration files."""

from __future__ import annotations

import io
import os
import tempfile
from pathlib import Path
from typing import Any


def atomic_write_bytes(path: str | Path, data: bytes) -> None:
    """Replace ``path`` only after the complete payload is durable."""

    # Resolve once so the temporary file and replacement target use the same
    # absolute path.  On Windows ``NamedTemporaryFile`` returns an absolute
    # pathname while a relative target can make ``os.replace`` fail with
    # ``WinError 5`` during test collection or when the process changes its
    # working directory between reads and writes.
    target = Path(path).resolve()
    target.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb",
            prefix=f".{target.name}.",
            suffix=".tmp",
            dir=target.parent,
            delete=False,
        ) as stream:
            temporary = Path(stream.name)
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, target)
        temporary = None
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def atomic_write_text(path: str | Path, data: str, *, encoding: str = "utf-8") -> None:
    atomic_write_bytes(path, data.encode(encoding))


def atomic_dump_yaml(yaml: Any, path: str | Path, data: Any) -> None:
    """Render ruamel YAML in memory, then atomically replace the target."""

    stream = io.StringIO()
    yaml.dump(data, stream)
    atomic_write_text(path, stream.getvalue())


__all__ = ["atomic_dump_yaml", "atomic_write_bytes", "atomic_write_text"]
