from __future__ import annotations

import importlib.util
from pathlib import Path


def _load_validator():
    path = Path(__file__).resolve().parents[2] / "scripts" / "validate_contract.py"
    spec = importlib.util.spec_from_file_location("aalc_contract_validator", path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_contract_manifest_matches_all_implementations() -> None:
    validator = _load_validator()
    assert validator.validate() == []
