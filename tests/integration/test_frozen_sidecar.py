"""Black-box checks for a packaged onedir sidecar.

Set ``AALC_FROZEN_BACKEND`` to the built ``AALC Backend.exe`` to enable this
test.  The normal unit/integration suite runs from source and does not require
a PyInstaller build.
"""

from __future__ import annotations

import importlib.util
import os
from pathlib import Path

import pytest

_EXECUTABLE = os.environ.get("AALC_FROZEN_BACKEND")


def _load_smoke_module():
    path = Path(__file__).resolve().parents[2] / "scripts" / "smoke_frozen_sidecar.py"
    spec = importlib.util.spec_from_file_location("aalc_frozen_smoke", path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@pytest.mark.skipif(
    not _EXECUTABLE or not Path(_EXECUTABLE).is_file(),
    reason="set AALC_FROZEN_BACKEND to a built AALC Backend.exe",
)
def test_frozen_onedir_sidecar_reports_its_own_pid_and_exits_cleanly() -> None:
    smoke = _load_smoke_module()
    ready = smoke.smoke(Path(_EXECUTABLE))
    assert ready["ready"] is True
    assert ready["port"] > 0
    assert ready["pid"] == ready["supervisedPid"]
