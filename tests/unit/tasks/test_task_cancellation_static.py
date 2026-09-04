from __future__ import annotations

import ast
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
TASKS_ROOT = REPO_ROOT / "tasks"

# ``my_script_task.run`` is owned by another migration and is intentionally
# outside this batch.  Its mutex is acquired before the task cancellation
# bridge is bound; keep the existing call visible in this allowlist until that
# owner can change its lifecycle guard without changing task startup semantics.
UNBOUNDED_WAIT_ALLOWLIST = {
    ("tasks/base/script_task_scheme.py", "my_script_task.run", "acquire"),
}


def _production_files() -> list[Path]:
    return sorted(path for path in TASKS_ROOT.rglob("*.py") if "__pycache__" not in path.parts)


def _relative(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def _call_attribute(call: ast.Call) -> str | None:
    if isinstance(call.func, ast.Attribute):
        return call.func.attr
    return None


def _has_finite_wait_argument(call: ast.Call) -> bool:
    """Return whether a wait/join/acquire call has an explicit bound."""

    timeout = next((keyword.value for keyword in call.keywords if keyword.arg == "timeout"), None)
    if timeout is not None:
        # ``None`` and the threading sentinel ``-1`` both mean unbounded.
        if isinstance(timeout, ast.Constant) and timeout.value is None:
            return False
        if isinstance(timeout, ast.UnaryOp) and isinstance(timeout.op, ast.USub):
            if isinstance(timeout.operand, ast.Constant) and timeout.operand.value == 1:
                return False
        return True
    # Event.wait(delay), Thread.join(delay), and Lock.acquire(False) all use
    # their first positional argument as a finite/non-blocking bound.
    return bool(call.args)


def _enclosing_function(tree: ast.AST, line: int) -> str | None:
    """Find a stable class.method label for the small explicit allowlist."""

    class Visitor(ast.NodeVisitor):
        def __init__(self) -> None:
            self.class_name: str | None = None
            self.function_name: str | None = None

        def visit_ClassDef(self, node: ast.ClassDef) -> None:
            previous = self.class_name
            self.class_name = node.name
            self.generic_visit(node)
            self.class_name = previous

        def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
            if node.lineno <= line <= getattr(node, "end_lineno", node.lineno):
                self.function_name = (
                    f"{self.class_name}.{node.name}" if self.class_name is not None else node.name
                )
            self.generic_visit(node)

        visit_AsyncFunctionDef = visit_FunctionDef

    visitor = Visitor()
    visitor.visit(tree)
    return visitor.function_name


def test_tasks_do_not_reintroduce_raw_sleep_or_os_system() -> None:
    violations: list[str] = []
    for path in _production_files():
        relative = _relative(path)
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=relative)
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Attribute):
                continue
            if isinstance(node.func.value, ast.Name) and node.func.value.id == "time" and node.func.attr == "sleep":
                violations.append(f"{relative}:{node.lineno} uses time.sleep")
            if isinstance(node.func.value, ast.Name) and node.func.value.id == "os" and node.func.attr == "system":
                violations.append(f"{relative}:{node.lineno} uses os.system")
    assert not violations, "\n".join(violations)


def test_task_wait_join_and_acquire_calls_are_bounded_or_allowlisted() -> None:
    violations: list[str] = []
    for path in _production_files():
        relative = _relative(path)
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=relative)
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue
            operation = _call_attribute(node)
            if operation not in {"wait", "join", "acquire"}:
                continue
            if _has_finite_wait_argument(node):
                continue
            key = (relative, _enclosing_function(tree, node.lineno) or "<module>", operation)
            if key not in UNBOUNDED_WAIT_ALLOWLIST:
                violations.append(f"{relative}:{node.lineno} {key[1]}.{operation} lacks a finite timeout")
    assert not violations, "\n".join(violations)


def test_subprocess_invocations_have_a_timeout() -> None:
    violations: list[str] = []
    for path in _production_files():
        relative = _relative(path)
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=relative)
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Attribute):
                continue
            if not isinstance(node.func.value, ast.Name) or node.func.value.id != "subprocess":
                continue
            if node.func.attr in {"run", "call", "check_call", "check_output"} and not any(
                keyword.arg == "timeout" for keyword in node.keywords
            ):
                violations.append(f"{relative}:{node.lineno} subprocess.{node.func.attr} lacks timeout")
    assert not violations, "\n".join(violations)
