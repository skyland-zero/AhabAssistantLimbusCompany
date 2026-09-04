"""Shared PyInstaller collection rules for the sidecar and task runner.

The sidecar and one-shot runner intentionally use different entry points, but
they execute the same lazy-loaded automation/task modules.  Keeping the
collection rules here prevents a module or native provider from silently
drifting out of one of the two bundles.

This module is also importable by the release builder without importing
PyInstaller.  The PyInstaller hook helpers are loaded lazily so manifest and
staging tests can run in a normal Python process.
"""

from __future__ import annotations

from pathlib import Path
from typing import Iterable

COMMON_PACKAGES = ("core", "module", "tasks", "utils")

# Imports which are reached through task/plugin registries or other lazy
# boundaries and therefore cannot be trusted to PyInstaller's static walker.
COMMON_LAZY_IMPORTS = (
    "module.automation",
    "module.automation.automation",
    "module.automation.screenshot",
    "module.automation.input_handlers.simulator.mumu_control",
    "module.automation.input_handlers.simulator.scrcpy_control",
    "module.automation.input_handlers.simulator.simulator_control",
    "module.resource_sync.service",
    "module.update.checker",
    "tasks.base.script_task_scheme",
    "tasks.battle.battle",
    "tasks.base.back_init_menu",
    "tasks.base.make_enkephalin_module",
)

RAPIDOCR_MODEL_NAMES = frozenset(
    {
        "PP-OCRv6_det_small.onnx",
        "PP-OCRv6_rec_small.onnx",
        "ch_ppocr_mobile_v2.0_cls_mobile.onnx",
    }
)


def collect_common_hiddenimports(extra: Iterable[str] = ()) -> list[str]:
    """Collect package submodules plus known lazy imports.

    ``extra`` is copied into the result and duplicates are removed by the
    caller/spec in a deterministic way.  Returning a list keeps this helper
    directly usable in a PyInstaller ``Analysis`` declaration.
    """

    from PyInstaller.utils.hooks import collect_submodules

    hiddenimports: list[str] = []
    for package in COMMON_PACKAGES:
        hiddenimports.extend(collect_submodules(package))
    hiddenimports.extend(COMMON_LAZY_IMPORTS)
    hiddenimports.extend(str(item) for item in extra)
    return sorted(set(hiddenimports))


def collect_common_datas(root: Path) -> list[tuple[str, str]]:
    """Collect data shared by the sidecar and Runner bundles.

    Keep the existing RapidOCR model filter and scrcpy server destination used
    by ``main_backend.spec``.  Project assets are staged beside the release
    executables by ``build.py``; duplicating the full assets tree into each
    PyInstaller bundle would make the release unnecessarily large.
    """

    from PyInstaller.utils.hooks import collect_data_files

    datas = [
        (source, destination)
        for source, destination in collect_data_files("rapidocr")
        if Path(source).parent.name != "models" or Path(source).name in RAPIDOCR_MODEL_NAMES
    ]
    datas.append((str(root / "assets" / "binary" / "scrcpy-server.jar"), "assets/binary"))
    return datas


def collect_common_binaries() -> list[tuple[str, str, str]]:
    """Collect the ONNX Runtime providers required by both executables."""

    from PyInstaller.utils.hooks import collect_dynamic_libs

    return collect_dynamic_libs("onnxruntime")


__all__ = [
    "COMMON_LAZY_IMPORTS",
    "COMMON_PACKAGES",
    "RAPIDOCR_MODEL_NAMES",
    "collect_common_binaries",
    "collect_common_datas",
    "collect_common_hiddenimports",
]
