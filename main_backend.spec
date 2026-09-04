# -*- mode: python ; coding: utf-8 -*-
# ruff: noqa: E402,F821
"""PyInstaller definition for the headless Python sidecar.

The sidecar intentionally excludes every Qt package. Backend modules use a
few lazy imports because the automation stack is large, so the pure-Python
backend packages are collected explicitly here instead of relying only on
PyInstaller's static import walk.
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

hiddenimports = collect_common_hiddenimports()
datas = collect_common_datas(root)
# The frozen ``module.__init__`` resolves these defaults relative to the
# private one-file extraction root.  Keep the sidecar self-contained for
# startup before it can read the external release ``assets`` directory.
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
    [str(root / "main_backend.py")],
    pathex=[str(root)],
    binaries=binaries,
    datas=datas,
    hiddenimports=sorted(set(hiddenimports)),
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=["PySide6", "qfluentwidgets", "qframelesswindow", "tkinter", "FixTk"],
    noarchive=False,
)

# 过滤掉 OpenCV 的冗余视频编解码动态库（~30MB）。
# 项目视频流由 assets/binary/scrcpy-ffmpeg/ 原生运行时解码，不使用 cv2.VideoCapture。
a.binaries = [
    binary for binary in a.binaries
    if not Path(binary[0]).name.lower().startswith("opencv_videoio_ffmpeg")
]

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
    a.datas,
    [],
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
