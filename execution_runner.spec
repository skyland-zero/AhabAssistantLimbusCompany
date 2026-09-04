# -*- mode: python ; coding: utf-8 -*-
# ruff: noqa: E402,F821
"""PyInstaller definition for the one-shot task Runner.

The bootstrap is intentionally the only script entry point.  It configures
the run-specific environment before importing the collected business modules.
The explicit ``COLLECT`` step is required: this is an onedir bundle, never a
one-file executable, so the sidecar can supervise the direct Runner process
without a second extraction/bootloader process.
"""

from pathlib import Path

root = Path(SPECPATH).resolve()

import sys

sys.path.insert(0, str(root / "scripts"))

from pyinstaller_common import (
    collect_common_binaries,
    collect_common_datas,
    collect_common_hiddenimports,
)

hiddenimports = collect_common_hiddenimports(
    (
        # The bootstrap imports these lazily after the IPC handshake; keeping
        # the names explicit makes the intended runner boundary visible in
        # the spec even when the bootstrap implementation changes.
        "module.execution",
    )
)
datas = collect_common_datas(root)
# The Runner's frozen ``module.__init__`` resolves these defaults relative to
# its private ``_internal`` root.  They therefore cannot rely on the release
# application's external ``assets`` directory: production task startup imports
# ``module.config`` after receiving its run-specific config path, and that
# module still needs the version/default/theme templates locally.
datas.extend(
    (
        str(root / "assets" / "config" / name),
        "assets/config",
    )
    for name in (
        "version.txt",
        "config.example.yaml",
        "theme_pack_list.example.yaml",
        "default_rapidocr.yaml",
    )
)
binaries = collect_common_binaries()

a = Analysis(
    [str(root / "runner_bootstrap.py")],
    pathex=[str(root)],
    binaries=binaries,
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=["PySide6", "qfluentwidgets", "qframelesswindow", "tkinter", "FixTk"],
    noarchive=False,
)

pyz = PYZ(a.pure)

# Keep the executable's own binary list empty and hand all collected files to
# COLLECT below.  This is the canonical PyInstaller onedir arrangement.
exe = EXE(
    pyz,
    a.scripts,
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
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name="AALCRunner",
)
