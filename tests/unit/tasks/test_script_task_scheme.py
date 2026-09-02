from __future__ import annotations

import pytest

import tasks.base.script_task_scheme as script_task_scheme
from module.my_error.my_error import userStopError


def test_onetime_mirror_process_propagates_user_stop(monkeypatch) -> None:
    class StoppingMirror:
        def __init__(self, *_args, **_kwargs) -> None:
            pass

        def run(self) -> None:
            raise userStopError("用户已请求停止任务")

    monkeypatch.setattr(script_task_scheme.cfg, "auto_hard_mirror", False, raising=False)
    monkeypatch.setattr(script_task_scheme, "Mirror", StoppingMirror)

    with pytest.raises(userStopError, match="用户已请求停止任务"):
        script_task_scheme.onetime_mir_process(object(), 1)
