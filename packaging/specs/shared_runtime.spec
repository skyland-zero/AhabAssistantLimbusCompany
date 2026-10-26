# -*- mode: python ; coding: utf-8 -*-
# ruff: noqa: E402,F821
"""Shared onedir runtime for the sidecar and the one-shot Runner.

The two entry points have independent ``Analysis`` graphs, but every collected
binary/data file is staged into one directory.  Each executable keeps its own
embedded PYZ/scripts, so both start directly with the same PID they report,
while Python DLLs, ONNX Runtime providers and the OCR models exist only once.

This spec is the packaging-evaluation counterpart to the standalone
``main_backend.spec`` and ``execution_runner.spec``.  ``scripts/build.py``
selects it behind ``--shared-runtime`` so the established two-bundle layout
remains a rollback path.
"""

from pathlib import Path

root = Path(SPECPATH).resolve().parents[1]

import sys

sys.path.insert(0, str(root / "scripts"))

from pyinstaller_common import (
    collect_common_binaries,
    collect_common_datas,
    collect_common_hiddenimports,
)

_COMMON_EXCLUDES = ["PySide6", "qfluentwidgets", "qframelesswindow", "tkinter", "FixTk"]
_CONFIG_DATAS = tuple(
    (str(root / "assets" / "config" / name), "assets/config")
    for name in (
        "version.txt",
        "config.example.yaml",
        "theme_pack_list.example.yaml",
        "default_rapidocr.yaml",
    )
)
_COMMON_DATAS = collect_common_datas(root) + list(_CONFIG_DATAS)
_COMMON_BINARIES = collect_common_binaries()


def _analysis(entry: str, hiddenimports: list[str]):
    return Analysis(
        [str(root / entry)],
        pathex=[str(root)],
        binaries=list(_COMMON_BINARIES),
        datas=list(_COMMON_DATAS),
        hiddenimports=hiddenimports,
        hookspath=[],
        hooksconfig={},
        runtime_hooks=[],
        excludes=list(_COMMON_EXCLUDES),
        noarchive=False,
    )


backend = _analysis(
    "main_backend.py",
    sorted(collect_common_hiddenimports()),
)
runner = _analysis(
    "runner_bootstrap.py",
    sorted(collect_common_hiddenimports(("module.execution",))),
)

# OpenCV's video decoder is unused: video frames are decoded by the bundled
# scrcpy/ffmpeg runtime.  Filter both graphs before COLLECT merges them.
for analysis in (backend, runner):
    analysis.binaries = [
        binary
        for binary in analysis.binaries
        if not Path(binary[0]).name.lower().startswith("opencv_videoio_ffmpeg")
    ]

backend_pyz = PYZ(backend.pure)
runner_pyz = PYZ(runner.pure)

backend_exe = EXE(
    backend_pyz,
    backend.scripts,
    [],
    exclude_binaries=True,
    name="AALC Backend",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=str(root / "assets" / "logo" / "my_icon_256X256.ico"),
)

runner_exe = EXE(
    runner_pyz,
    runner.scripts,
    [],
    exclude_binaries=True,
    name="AALCRunner",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=str(root / "assets" / "logo" / "my_icon_256X256.ico"),
)

coll = COLLECT(
    backend_exe,
    runner_exe,
    backend.binaries,
    backend.zipfiles,
    backend.datas,
    runner.binaries,
    runner.zipfiles,
    runner.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name="AALCRuntime",
)
