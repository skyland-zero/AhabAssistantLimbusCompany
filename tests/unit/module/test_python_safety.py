from __future__ import annotations

import os
import subprocess
import sys
import threading
import time
from pathlib import Path

from PIL import Image

from module.automation.input_handlers import AbstractInput
from module.preview_capture import PreviewCapture

REPO_ROOT = Path(__file__).resolve().parents[3]


def test_config_path_environment_is_read_before_singleton_creation(tmp_path: Path) -> None:
    """A runner override must be observed by the first Config singleton."""

    config_path = tmp_path / "runner" / "config.yaml"
    env = os.environ.copy()
    env["AALC_CONFIG_PATH"] = str(config_path)
    result = subprocess.run(
        [
            sys.executable,
            "-c",
            "from module import CONFIG_PATH; from module.config import cfg; "
            "print(CONFIG_PATH); print(cfg.config_path); print(cfg.config_path.is_absolute())",
        ],
        cwd=REPO_ROOT,
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )

    lines = result.stdout.strip().splitlines()
    assert lines == [str(config_path.resolve()), str(config_path.resolve()), "True"]
    assert config_path.is_file()


def test_preview_stop_and_wait_retains_worker_on_timeout() -> None:
    entered = threading.Event()
    release = threading.Event()

    def capture() -> Image.Image:
        entered.set()
        release.wait()
        return Image.new("RGB", (16, 9))

    preview = PreviewCapture(lambda *_args: None, capture=capture, interval=0.05)
    assert preview.start("pc:limbus")
    assert entered.wait(1.0)

    assert not preview.stop_and_wait(time.monotonic() + 0.02)
    assert preview.running
    # The retained worker can be joined once the native/capture operation has
    # returned; stop_and_wait does not create a replacement worker.
    release.set()
    assert preview.stop_and_wait(time.monotonic() + 1.0)
    assert not preview.running


def test_preview_drops_frame_from_stopped_generation() -> None:
    entered = threading.Event()
    release = threading.Event()
    frames: list[dict] = []

    def capture() -> Image.Image:
        entered.set()
        release.wait()
        return Image.new("RGB", (16, 9))

    def emit(event: str, payload: dict) -> None:
        if event == "screenshot.frame":
            frames.append(payload)

    preview = PreviewCapture(emit, capture=capture, interval=0.05)
    assert preview.start("pc:limbus")
    assert entered.wait(1.0)
    assert not preview.stop_and_wait(time.monotonic() + 0.02)
    release.set()
    assert preview.stop_and_wait(time.monotonic() + 1.0)
    assert frames == []


def test_input_pause_and_stop_are_condition_wakeable() -> None:
    input_handler = AbstractInput()
    input_handler.set_paused(True)
    result: list[bool] = []
    waiter = threading.Thread(target=lambda: result.append(input_handler.wait_pause()), daemon=True)
    waiter.start()
    time.sleep(0.02)

    started = time.monotonic()
    input_handler.request_stop()
    waiter.join(0.5)

    assert not waiter.is_alive()
    assert result == [False]
    assert time.monotonic() - started < 0.5

    input_handler.reset_control()
    assert input_handler.is_pause is False
    assert input_handler.cancellation_requested() is False
    input_handler.set_paused(True)
    input_handler.set_paused(True)
    assert input_handler.is_pause is True
    input_handler.set_paused(False)
    assert input_handler.wait_pause() is True
