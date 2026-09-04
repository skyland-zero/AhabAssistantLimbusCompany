"""Safety policy shared by the simulator input handlers.

The sidecar predates the one-run-per-process Runner and is intentionally kept
permissive by default.  A Runner, however, must never infer permission to
start or restart a shared emulator from a lost ADB connection.  This module
keeps that decision in one small, dependency-free object so the concrete
controllers can fail closed before invoking MuMu or an automatic recovery
path.
"""

from __future__ import annotations

import os
from dataclasses import dataclass


class RunnerDevicePolicyError(RuntimeError):
    """A simulator action is not safe under the current Runner policy."""

    code = "RUNNER_DEVICE_POLICY"

    def __init__(self, message: str, *, action: str | None = None) -> None:
        self.action = action
        self.message = message
        super().__init__(message)


def _env_bool(value: str | None, *, default: bool) -> bool:
    if value is None:
        return default
    normalized = value.strip().lower()
    if normalized in {"1", "true", "yes", "on", "y"}:
        return True
    if normalized in {"0", "false", "no", "off", "n", ""}:
        return False
    return default


@dataclass(frozen=True, slots=True)
class RunnerPolicy:
    """Policy controlling emulator launch/recovery for one controller.

    ``AALC_RUNNER_MODE`` defaults to sidecar mode.  The launch flag defaults
    to ``True`` in sidecar mode for backwards compatibility and to ``False``
    in Runner mode.  An explicit ``AALC_ALLOW_EMULATOR_LAUNCH`` always wins.
    Environment values are read when a controller is created rather than at
    module import time, which keeps tests and one-shot Runner processes
    deterministic.
    """

    runner_mode: bool = False
    allow_emulator_launch: bool | None = None

    def __post_init__(self) -> None:
        # Constructing ``RunnerPolicy(runner_mode=True)`` should have the
        # same safe default as ``from_env()`` rather than accidentally
        # inheriting the sidecar launch permission.
        if self.allow_emulator_launch is None:
            object.__setattr__(self, "allow_emulator_launch", not self.runner_mode)
        else:
            object.__setattr__(self, "allow_emulator_launch", bool(self.allow_emulator_launch))

    @classmethod
    def from_env(cls) -> RunnerPolicy:
        runner_mode = _env_bool(os.environ.get("AALC_RUNNER_MODE"), default=False)
        allow_emulator_launch = _env_bool(
            os.environ.get("AALC_ALLOW_EMULATOR_LAUNCH"),
            default=not runner_mode,
        )
        return cls(runner_mode=runner_mode, allow_emulator_launch=allow_emulator_launch)

    @property
    def forbid_emulator_launch(self) -> bool:
        """Whether launch/restart operations must fail closed."""

        return not self.allow_emulator_launch

    @property
    def is_runner(self) -> bool:
        """Compatibility spelling useful at integration boundaries."""

        return self.runner_mode

    def assert_emulator_launch_allowed(self, action: str) -> None:
        """Reject an operation that may start, stop, or restart MuMu."""

        if self.forbid_emulator_launch:
            raise RunnerDevicePolicyError(
                f"Runner policy forbids emulator launch/restart during {action}; "
                "attach to an already-running instance or fail the run",
                action=action,
            )

    def assert_recovery_allowed(self, action: str) -> None:
        """Reject automatic recovery that could alter emulator lifecycle."""

        self.assert_emulator_launch_allowed(action)

    def assert_explicit_exit_allowed(self, *, task_succeeded: bool, lease_active: bool) -> None:
        """Allow the separately requested successful-task emulator exit.

        ``exit_emulator`` is deliberately not implemented in terms of the
        launch/recovery permission above.  In Runner mode it is an explicit
        completion action and is valid only while the Runner still owns the
        device lease and the task has reported success.
        """

        if not self.runner_mode:
            return
        if not task_succeeded or not lease_active:
            raise RunnerDevicePolicyError(
                "Runner may exit the emulator only after a successful task "
                "while its device lease is still active",
                action="exit_emulator",
            )


def get_runner_policy() -> RunnerPolicy:
    """Read the current process policy without caching environment state."""

    return RunnerPolicy.from_env()


__all__ = ["RunnerDevicePolicyError", "RunnerPolicy", "get_runner_policy"]
