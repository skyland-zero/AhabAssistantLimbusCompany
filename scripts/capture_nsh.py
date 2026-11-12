#!/usr/bin/env python3
"""Capture a PNG of the Native Shell app running in the GPUI development host.

Linux/CI helper: the GPUI host needs a window system even when the machine has
no GPU, so this runs it under Xvfb with Vulkan's software driver (Mesa's
lavapipe). NSH's own `app.screenshot()` needs a real platform host to draw, and
the test host is fixed at its own window size, so desktop-size captures go
through `nsh run --window-size` plus an X11 root-window grab.

    python3 scripts/capture_nsh.py                     # nsh-app/docs/shots/p0-desktop.png
    python3 scripts/capture_nsh.py --size 1400x900 --out /tmp/big.png
    python3 scripts/capture_nsh.py --serve-only        # leave the app running

Requirements: `nsh` on PATH, Xvfb, ImageMagick (`magick` or `import`), and the
host's build prerequisites (GTK3, WebKitGTK 4.1, GStreamer, clang) — see
nsh-app/README.md. The first run builds the host (several minutes); later runs
reuse nsh-app/target.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_APP = "nsh-app"
DEFAULT_OUT = ROOT / "nsh-app" / "docs" / "shots" / "p0-desktop.png"
LAVAPIPE = "/usr/share/vulkan/icd.d/lvp_icd.json"
# The host logs this once it has loaded the bundle and opened its window.
READY = re.compile(r"connected, build \d+|connected$", re.IGNORECASE)


def fail(message: str) -> "None":
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def which_import() -> list[str]:
    if shutil.which("magick"):
        return ["magick", "import"]
    if shutil.which("import"):
        return ["import"]
    fail("ImageMagick is required: install `imagemagick` (magick or import)")


def free_display() -> int:
    for number in range(99, 130):
        if not Path(f"/tmp/.X{number}-lock").exists():
            return number
    fail("no free X display between :99 and :129")


def wait_for(pattern: re.Pattern[str], stream: Path, deadline: float) -> bool:
    while time.monotonic() < deadline:
        text = stream.read_text(encoding="utf-8", errors="replace")
        if pattern.search(text):
            return True
        time.sleep(0.5)
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--app", default=DEFAULT_APP, help=f"the app directory (default: {DEFAULT_APP})")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT, help=f"where to write the PNG (default: {DEFAULT_OUT.relative_to(ROOT)})")
    parser.add_argument("--size", default="1100x720", help="the window's size in points, WxH (default: 1100x720)")
    parser.add_argument("--screen", default=None, help="the Xvfb screen, WxH (default: the window's size, so the grab is exactly the window)")
    parser.add_argument("--target", default="gpui", help="the nsh target (default: gpui)")
    parser.add_argument("--timeout", type=float, default=900.0, help="seconds to wait for the host to open its window (default: 900)")
    parser.add_argument("--serve-only", action="store_true", help="leave the app running and only print its display")
    args = parser.parse_args()

    if not shutil.which("nsh"):
        fail("`nsh` is not on PATH; install it first (see nsh-app/README.md)")
    if not shutil.which("Xvfb"):
        fail("Xvfb is required: install `xvfb`")

    app = (ROOT / args.app).resolve()
    if not (app / "nsh.json").is_file():
        fail(f"{args.app} is not an nsh app (no nsh.json)")

    display = free_display()
    log = Path("/tmp") / f"nsh-capture-{display}.log"
    log.write_text("", encoding="utf-8")

    env = dict(os.environ)
    if Path(LAVAPIPE).is_file():
        # Software Vulkan: a machine without a GPU still draws.
        env["VK_ICD_FILENAMES"] = LAVAPIPE
    env["DISPLAY"] = f":{display}"

    width, _, height = args.size.partition("x")
    if not (width.isdigit() and height.isdigit()):
        fail(f"--size must be WIDTHxHEIGHT, got {args.size!r}")
    # A screen the size of the window puts it at the origin, so the root grab
    # is the window itself (no desktop margins to crop).
    screen = args.screen or f"{width}x{height}"

    xvfb = subprocess.Popen(
        ["Xvfb", f":{display}", "-screen", "0", f"{screen}x24"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    print(f"xvfb on :{display} (pid {xvfb.pid})")

    app_log = log.open("ab")
    app_process = subprocess.Popen(
        ["nsh", "run", args.app, "--target", args.target, "--window-size", args.size],
        cwd=ROOT,
        env=env,
        stdout=app_log,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    print(f"nsh run {args.app} --target {args.target} --window-size {args.size} (pid {app_process.pid})")

    cleaned = False

    def cleanup(*_: object) -> None:
        nonlocal cleaned
        if cleaned:
            return
        cleaned = True
        for process in (app_process, xvfb):
            if process.poll() is None:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
        for process in (app_process, xvfb):
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass

    signal.signal(signal.SIGINT, lambda *a: (cleanup(), raise_exit()))
    signal.signal(signal.SIGTERM, lambda *a: (cleanup(), raise_exit()))

    try:
        deadline = time.monotonic() + args.timeout
        if not wait_for(READY, log, deadline):
            print(log.read_text(encoding="utf-8", errors="replace")[-4000:], file=sys.stderr)
            fail(f"the host did not open its window within {args.timeout:.0f}s (log: {log})")
        # The window exists; give the first frames a moment to paint.
        time.sleep(3.0)
        if app_process.poll() is not None:
            fail(f"the host exited with {app_process.returncode} (log: {log})")

        if args.serve_only:
            print(f"ready: DISPLAY=:{display} (leave this process running)")
            wait_for(re.compile(r"$^"), log, time.monotonic() + 10**9)
            return 0

        args.out.parent.mkdir(parents=True, exist_ok=True)
        grab = subprocess.run([*which_import(), "-window", "root", str(args.out)], env=env)
        if grab.returncode != 0:
            fail(f"the screenshot failed with {grab.returncode}")
        print(f"wrote {args.out.relative_to(ROOT) if args.out.is_relative_to(ROOT) else args.out}")
        return 0
    finally:
        cleanup()


def raise_exit() -> "None":
    raise SystemExit(130)


if __name__ == "__main__":
    sys.exit(main())
