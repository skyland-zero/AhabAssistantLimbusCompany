# -*- mode: python ; coding: utf-8 -*-
"""PyInstaller definition for the headless Python sidecar.

The sidecar intentionally excludes every Qt package. Backend modules use a
few lazy imports because the automation stack is large, so the pure-Python
backend packages are collected explicitly here instead of relying only on
PyInstaller's static import walk.
"""

from pathlib import Path

from PyInstaller.utils.hooks import collect_data_files, collect_submodules


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
        "module.automation.input_handlers.simulator.simulator_control",
        "module.resource_sync.service",
        "module.update.checker",
        "tasks.base.script_task_scheme",
        "tasks.battle.battle",
        "tasks.base.back_init_menu",
        "tasks.base.make_enkephalin_module",
    ]
)

datas = collect_data_files("rapidocr")

a = Analysis(
    [str(root / "main_backend.py")],
    pathex=[str(root)],
    binaries=[],
    datas=datas,
    hiddenimports=sorted(set(hiddenimports)),
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=["PySide6", "qfluentwidgets", "qframelesswindow", "tkinter", "FixTk"],
    noarchive=False,
)

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
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=str(root / "assets" / "logo" / "my_icon_256X256.ico"),
)
