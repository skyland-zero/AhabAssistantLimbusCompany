# -*- mode: python ; coding: utf-8 -*-
"""PyInstaller definition for the headless Python sidecar.

The sidecar intentionally excludes every Qt package. Backend modules use a
few lazy imports because the automation stack is large, so the pure-Python
backend packages are collected explicitly here instead of relying only on
PyInstaller's static import walk.
"""

from pathlib import Path

from PyInstaller.utils.hooks import (
    collect_data_files,
    collect_dynamic_libs,
    collect_submodules,
)

root = Path(SPECPATH).resolve()
hiddenimports = []
for package in ("core", "module", "tasks", "utils"):
    hiddenimports.extend(collect_submodules(package))

hiddenimports.extend(
    [
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
    ]
)

# RapidOCR 包内还带有旧版 PP-OCRv4 Det/Rec 模型。当前业务只使用 PP-OCRv6
# Det/Rec；Cls 暂时只保留模型文件用于回滚，待验证无误后再移除。
rapidocr_model_names = {
    "PP-OCRv6_det_small.onnx",
    "PP-OCRv6_rec_small.onnx",
    "ch_ppocr_mobile_v2.0_cls_mobile.onnx",
}
datas = [
    (source, destination)
    for source, destination in collect_data_files("rapidocr")
    if Path(source).parent.name != "models"
    or Path(source).name in rapidocr_model_names
]
datas.append((str(root / "assets" / "binary" / "scrcpy-server.jar"), "assets/binary"))
binaries = collect_dynamic_libs("onnxruntime")

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
