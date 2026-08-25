"""架构守护测试：核心业务层不得依赖 UI 框架或 UI 层。

核心层范围: core/, module/, tasks/, utils/
禁止项:
- 导入 PySide6 / qfluentwidgets / qframelesswindow 等 UI 框架；
- 导入 app 包（UI 层），方向必须是 app -> core，而不是反向。
"""

import ast
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CORE_DIRS = ("core", "module", "tasks", "utils")

FORBIDDEN_UI_MODULES = ("PySide6", "qfluentwidgets", "qframelesswindow")
FORBIDDEN_UI_PACKAGE = "app"


def _iter_core_py_files():
    for d in CORE_DIRS:
        yield from (REPO_ROOT / d).rglob("*.py")


def _imports_of(tree: ast.AST) -> list[str]:
    names = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            names.extend(alias.name for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module and node.level == 0:
            names.append(node.module)
    return names


def test_core_layers_do_not_import_ui_frameworks_or_ui_package():
    violations = []
    for py_file in _iter_core_py_files():
        tree = ast.parse(py_file.read_text(encoding="utf-8"), filename=str(py_file))
        for name in _imports_of(tree):
            root = name.split(".")[0]
            if root in FORBIDDEN_UI_MODULES or root == FORBIDDEN_UI_PACKAGE:
                rel = py_file.relative_to(REPO_ROOT)
                violations.append(f"{rel}: imports {name}")

    assert not violations, "核心层出现违规导入:\n" + "\n".join(violations)


def test_importing_core_event_bus_does_not_load_qt():
    import sys

    import core.events  # noqa: F401

    qt_loaded = [m for m in sys.modules if m.split(".")[0] in FORBIDDEN_UI_MODULES]
    assert not qt_loaded, f"core.events 导入后加载了 UI 模块: {qt_loaded}"
