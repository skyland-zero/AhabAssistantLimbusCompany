# -*- mode: python ; coding: utf-8 -*-
# ruff: noqa: E402,F821
"""Shared onedir runtime for the sidecar and the one-shot Runner.

One executable and one PYZ serve both roles.  The entry point selects the
Runner when it is launched with ``--execution-runner``; otherwise it starts the
sidecar.  Keeping the role split at the entry point preserves the direct-process
contract (the launched PID is the Python PID) while removing the second copy of
the near-identical pure-Python archive.

``main_backend.spec`` and ``execution_runner.spec`` remain the explicit
rollback path behind ``scripts/build.py --separate-bundles``.
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

analysis = Analysis(
    [str(root / "entrypoint.py")],
    pathex=[str(root)],
    binaries=list(_COMMON_BINARIES),
    datas=list(_COMMON_DATAS),
    hiddenimports=sorted(
        collect_common_hiddenimports(
            (
                "module.execution",
                "main_backend",
                "runner_bootstrap",
            )
        )
    ),
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=list(_COMMON_EXCLUDES),
    noarchive=False,
)

# OpenCV's video decoder is unused: video frames are decoded by the bundled
# scrcpy/ffmpeg runtime.
analysis.binaries = [
    binary
    for binary in analysis.binaries
    if not Path(binary[0]).name.lower().startswith("opencv_videoio_ffmpeg")
]

pyz = PYZ(analysis.pure)

exe = EXE(
    pyz,
    analysis.scripts,
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

coll = COLLECT(
    exe,
    analysis.binaries,
    analysis.zipfiles,
    analysis.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name="AALCRuntime",
)
