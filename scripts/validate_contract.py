"""Validate the GPUI ↔ sidecar contract against both implementations.

``contract/contract.json`` is the single reviewable source for RPC methods,
mutating flags, events and error names.  This script deliberately avoids
importing the heavy Python backend: it parses the source files so it can run in
CI before dependencies/configuration are initialized.

Run directly (``python scripts/validate_contract.py``) or through
``tests/unit/test_contract_manifest.py``.
"""

from __future__ import annotations

import ast
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
CONTRACT_PATH = ROOT / "contract" / "contract.json"
CONTRACT_RS = ROOT / "gpui-app" / "src" / "ipc" / "contract.rs"
DISPATCHER_PY = ROOT / "module" / "rpc_dispatcher.py"
RPC_CONTRACT_PY = ROOT / "module" / "rpc_contract.py"
BACKEND_APPLICATION_PY = ROOT / "module" / "backend_application.py"
MOCK_DIR = ROOT / "gpui-app" / "src" / "ipc" / "mock"


def load_contract(path: Path = CONTRACT_PATH) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as stream:
        contract = json.load(stream)
    if contract.get("schemaVersion") != 3:
        raise ValueError("contract schemaVersion must be 3")
    return contract


def _rust_module_constants(text: str, module: str) -> dict[str, str]:
    match = re.search(rf"pub mod {module} \{{(.*?)\n\}}", text, re.DOTALL)
    if match is None:
        raise ValueError(f"missing Rust module: {module}")
    return dict(re.findall(r'pub const ([A-Z0-9_]+): &str = "([^"]+)";', match.group(1)))


def _resolve_literal(node: ast.AST, namespace: dict[str, Any]) -> Any:
    if isinstance(node, ast.Constant):
        return node.value
    if isinstance(node, ast.Name):
        if node.id not in namespace:
            raise ValueError(f"cannot resolve {node.id} at module scope")
        return namespace[node.id]
    if isinstance(node, ast.Tuple):
        return tuple(_resolve_literal(item, namespace) for item in node.elts)
    if isinstance(node, ast.List):
        return [_resolve_literal(item, namespace) for item in node.elts]
    if isinstance(node, ast.Set):
        return {_resolve_literal(item, namespace) for item in node.elts}
    if isinstance(node, ast.Dict):
        return {
            _resolve_literal(key, namespace): _resolve_literal(value, namespace)
            for key, value in zip(node.keys, node.values)
            if key is not None
        }
    if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.USub):
        return -_resolve_literal(node.operand, namespace)
    raise ValueError(f"unsupported literal node: {type(node).__name__}")


def _python_assignment(path: Path, name: str) -> Any:
    tree = ast.parse(path.read_text(encoding="utf-8"))
    namespace: dict[str, Any] = {}
    for node in tree.body:
        if not isinstance(node, ast.Assign):
            continue
        for target in node.targets:
            if isinstance(target, ast.Name):
                try:
                    namespace[target.id] = _resolve_literal(node.value, namespace)
                except ValueError:
                    continue
    if name not in namespace:
        raise ValueError(f"{path.name} does not define {name}")
    return namespace[name]


def _python_dispatch_tables(path: Path) -> tuple[dict[str, str], set[str], dict[int, str]]:
    tree = ast.parse(path.read_text(encoding="utf-8"))
    route_names: dict[str, str] = {}
    mutating: set[str] = set()
    error_names: dict[int, str] = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.FunctionDef) and node.name == "_call":
            for child in ast.walk(node):
                if isinstance(child, ast.Assign) and isinstance(child.value, ast.Dict):
                    try:
                        value = ast.literal_eval(child.value)
                    except ValueError:
                        continue
                    if isinstance(value, dict) and "app.ping" in value:
                        route_names = value
        if isinstance(node, ast.FunctionDef) and node.name == "is_mutating":
            for child in ast.walk(node):
                if isinstance(child, ast.Set):
                    try:
                        value = ast.literal_eval(child)
                    except ValueError:
                        continue
                    if "app.shutdown" in value:
                        mutating = value
    error_names = _python_assignment(path, "_ERROR_CODE_NAMES")
    return route_names, mutating, error_names


def _python_class_methods(path: Path, class_name: str) -> set[str]:
    tree = ast.parse(path.read_text(encoding="utf-8"))
    for node in tree.body:
        if isinstance(node, ast.ClassDef) and node.name == class_name:
            return {
                item.name
                for item in node.body
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef))
            }
    raise ValueError(f"{path.name} does not define {class_name}")


def validate(root: Path = ROOT) -> list[str]:
    contract = load_contract(root / "contract" / "contract.json")
    rust = (root / "gpui-app" / "src" / "ipc" / "contract.rs").read_text(encoding="utf-8")
    findings: list[str] = []

    methods: dict[str, dict[str, Any]] = contract["methods"]
    events: dict[str, dict[str, Any]] = contract["events"]
    errors: dict[str, dict[str, Any]] = contract["errors"]

    rust_methods = _rust_module_constants(rust, "method")
    rust_events = _rust_module_constants(rust, "event")
    rust_capabilities = _rust_module_constants(rust, "capability")

    if set(rust_methods.values()) != set(methods):
        findings.append(
            "Rust method constants differ: "
            f"missing={sorted(set(methods) - set(rust_methods.values()))}, "
            f"extra={sorted(set(rust_methods.values()) - set(methods))}"
        )
    if set(rust_events.values()) != set(events):
        findings.append(
            "Rust event constants differ: "
            f"missing={sorted(set(events) - set(rust_events.values()))}, "
            f"extra={sorted(set(rust_events.values()) - set(events))}"
        )

    expected_capabilities = set(contract["capabilities"])
    if set(rust_capabilities.values()) != expected_capabilities:
        findings.append("Rust capability constants differ from the manifest")

    python_routes, python_mutating, python_errors = _python_dispatch_tables(
        root / "module" / "rpc_dispatcher.py"
    )
    if set(python_routes) != set(methods):
        findings.append(
            "Python route table differs: "
            f"missing={sorted(set(methods) - set(python_routes))}, "
            f"extra={sorted(set(python_routes) - set(methods))}"
        )

    expected_mutating = {name for name, spec in methods.items() if spec["mutating"]}
    if python_mutating != expected_mutating:
        findings.append(
            "Python mutating methods differ: "
            f"missing={sorted(expected_mutating - python_mutating)}, "
            f"extra={sorted(python_mutating - expected_mutating)}"
        )

    error_codes = {int(code): spec["name"] for code, spec in errors.items()}
    if python_errors != error_codes:
        findings.append(
            "Python error-code names differ: "
            f"missing={sorted(set(error_codes) - set(python_errors))}, "
            f"extra={sorted(set(python_errors) - set(error_codes))}"
        )

    python_capabilities = set(_python_assignment(root / "module" / "rpc_contract.py", "CAPABILITIES"))
    if python_capabilities != expected_capabilities:
        findings.append("Python capabilities differ from the manifest")

    python_schema = _python_assignment(root / "module" / "rpc_contract.py", "SCHEMA_VERSION")
    rust_schema = re.search(r"pub const RPC_SCHEMA_VERSION: u32 = (\d+);", rust)
    if rust_schema is None or int(rust_schema.group(1)) != python_schema:
        findings.append("Rust/Python/manifest schema versions differ")
    if contract.get("schemaVersion") != python_schema:
        findings.append("Manifest schemaVersion differs from the runtime constants")

    # Every RPC method must be accepted by the deterministic Mock as well.
    mock_source = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted(MOCK_DIR.rglob("*.rs"))
        if path.name != "tests.rs"
    )
    method_const_by_value = {value: name for name, value in rust_methods.items()}
    for method in methods:
        const_name = method_const_by_value.get(method)
        if const_name is None:
            continue
        if f"method::{const_name}" not in mock_source:
            findings.append(f"Mock does not handle {method} ({const_name})")

    # Python handlers referenced by the route table must exist on the class.
    backend_methods = _python_class_methods(
        root / "module" / "backend_application.py", "BackendApplication"
    )
    for method, route in python_routes.items():
        if "." in route or route.startswith("__"):
            continue
        if route not in backend_methods:
            findings.append(f"Python route {method} references missing {route}")

    # Every wire event needs at least one Python producer/alias reference.
    python_sources = "\n".join(
        path.read_text(encoding="utf-8", errors="replace")
        for path in sorted((root / "module").rglob("*.py"))
    )
    for event in events:
        if f'"{event}"' not in python_sources:
            findings.append(f"Python source does not reference event {event}")

    return findings


def main() -> int:
    try:
        findings = validate(ROOT)
    except (OSError, ValueError, SyntaxError, json.JSONDecodeError) as error:
        sys.stderr.write(f"contract validation failed to run: {error}\n")
        return 2
    if findings:
        for finding in findings:
            sys.stderr.write(f"- {finding}\n")
        return 1
    sys.stdout.write("contract manifest matches Rust, Python and Mock\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
