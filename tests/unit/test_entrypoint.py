from __future__ import annotations

import sys

import entrypoint
import main_backend
import runner_bootstrap


def test_backend_is_the_default(monkeypatch) -> None:
    calls: list[object] = []
    monkeypatch.setattr(main_backend, "main", lambda: calls.append("backend"))
    monkeypatch.setattr(
        runner_bootstrap,
        "main",
        lambda argv: calls.append(("runner", argv)) or 0,
    )
    monkeypatch.setattr(sys, "argv", ["AALC Backend.exe", "--health"])

    assert entrypoint.main() == 0
    assert calls == ["backend"]


def test_runner_flag_dispatches_to_the_runner_bootstrap(monkeypatch) -> None:
    calls: list[object] = []
    monkeypatch.setattr(main_backend, "main", lambda: calls.append("backend"))
    monkeypatch.setattr(
        runner_bootstrap,
        "main",
        lambda argv: calls.append(("runner", argv)) or 42,
    )
    monkeypatch.setattr(
        sys,
        "argv",
        ["AALC Backend.exe", "--execution-runner", "--run-id", "run-1"],
    )

    assert entrypoint.main() == 42
    assert calls == [("runner", ["--run-id", "run-1"])]


def test_entrypoint_flag_matches_the_runtime() -> None:
    from module.execution.supervisor import RUNNER_ROLE_FLAG

    assert entrypoint.RUNNER_FLAG == RUNNER_ROLE_FLAG


def test_the_runner_flag_must_be_the_first_argument(monkeypatch) -> None:
    calls: list[object] = []
    monkeypatch.setattr(main_backend, "main", lambda: calls.append("backend"))
    monkeypatch.setattr(
        runner_bootstrap,
        "main",
        lambda argv: calls.append(("runner", argv)) or 0,
    )
    monkeypatch.setattr(
        sys,
        "argv",
        ["AALC Backend.exe", "--token", "--execution-runner"],
    )

    assert entrypoint.main() == 0
    assert calls == ["backend"]
