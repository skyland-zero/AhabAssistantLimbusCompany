#!/usr/bin/env python3
"""Capture reference PNGs of the legacy GPUI client (`gpui-app/`).

The migration's acceptance criterion is a 1:1 screenshot comparison against the
old client, so this drives the same visual overrides its Windows harness
(`gpui-app/scripts/capture_visual.ps1`) uses, on Linux:

    AHAB_BACKEND=mock            no Python sidecar; deterministic mock data
    AHAB_VISUAL_PAGE             home, teams, themes, toolbox, resources, help, settings
    AHAB_VISUAL_STATE            a named state: home-running, teams-editor, …
    AHAB_VISUAL_THEME            light, dark, system
    AHAB_VISUAL_LANGUAGE         zh-CN, en-US
    AHAB_VISUAL_SKIN             default, glass, archive, limbus, mist
    AHAB_VISUAL_ACCENT           crimson, blue, amber, emerald, violet, limbusBrass

`gpui-app/` is deleted at P5, so treat these as reference images to compare
against, not as something the new app ships.

    # build once (dev profile is enough and much faster than release)
    cargo build --manifest-path gpui-app/Cargo.toml

    python3 scripts/capture_gpui.py --all                 # 7 pages x light/dark
    python3 scripts/capture_gpui.py --page home --theme dark --state home-running
    python3 scripts/capture_gpui.py --all --size 900x680 --out-dir /tmp/ref

Requirements: Xvfb, ImageMagick, a built `gpui-app` binary, and the same
graphics/system libraries the app needs (see nsh-app/README.md).
"""

from __future__ import annotations

import argparse
import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "gpui-app" / "target" / "debug" / "ahab-gpui-app"
DEFAULT_OUT = ROOT / "artifacts" / "visual" / "gpui"
LAVAPIPE = "/usr/share/vulkan/icd.d/lvp_icd.json"

PAGES = ["home", "teams", "themes", "toolbox", "resources", "help", "settings"]
THEMES = ["light", "dark"]
# From gpui-app/scripts/capture_visual.ps1: each state belongs to one page.
STATE_PAGES = {
    "home-expanded": "home",
    "home-mirror-details": "home",
    "home-select": "home",
    "home-running": "home",
    "home-paused": "home",
    "home-after-completion": "home",
    "teams-editor": "teams",
    "teams-shop-editor": "teams",
    "teams-combat-editor": "teams",
    "teams-starlight-editor": "teams",
    "teams-advanced-editor": "teams",
    "teams-delete": "teams",
    "teams-stats-clear": "teams",
    "teams-preset-picker": "teams",
    "teams-preset-overwrite": "teams",
    "teams-select": "teams",
    "settings-hotkey": "settings",
    "settings-select": "settings",
    "settings-latest": "settings",
    "toolbox-running": "toolbox",
    "resources-syncing": "resources",
    "help-scrolled": "help",
}


def fail(message: str) -> "None":
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def grab_command() -> list[str]:
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


def kill(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is None:
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=15)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass


def capture(display: int, env: dict[str, str], out: Path) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    result = subprocess.run([*grab_command(), "-window", "root", str(out)], env=env)
    if result.returncode != 0:
        fail(f"the screenshot failed with {result.returncode}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--page", choices=PAGES, help="one page; default when nothing else is given: home")
    parser.add_argument("--state", choices=sorted(STATE_PAGES), help="a named state (implies its page)")
    parser.add_argument("--theme", choices=THEMES + ["system"], default="light")
    parser.add_argument("--language", default="zh-CN", help="zh-CN or en-US (default: zh-CN)")
    parser.add_argument("--skin", default="default", help="default, glass, archive, limbus, mist")
    parser.add_argument("--accent", default=None, help="crimson, blue, amber, emerald, violet, limbusBrass")
    parser.add_argument("--size", default="820x680", help="the window (= Xvfb screen) size in points (default: 820x680)")
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT, help=f"output directory (default: {DEFAULT_OUT.relative_to(ROOT)})")
    parser.add_argument("--all", action="store_true", help="every page x every theme")
    parser.add_argument("--wait", type=float, default=6.0, help="seconds to let a window paint before the grab (default: 6)")
    parser.add_argument("--binary", type=Path, default=BINARY, help=f"the app binary (default: {BINARY.relative_to(ROOT)})")
    args = parser.parse_args()

    if not args.binary.is_file():
        fail(f"{args.binary.relative_to(ROOT)} is missing; build it: cargo build --manifest-path gpui-app/Cargo.toml")
    if not shutil.which("Xvfb"):
        fail("Xvfb is required: install `xvfb`")
    width, _, height = args.size.partition("x")
    if not (width.isdigit() and height.isdigit()):
        fail(f"--size must be WIDTHxHEIGHT, got {args.size!r}")

    cases: list[tuple[str, str | None, str]] = []
    if args.all:
        for page in PAGES:
            for theme in THEMES:
                cases.append((page, None, theme))
    elif args.state:
        cases.append((STATE_PAGES[args.state], args.state, args.theme))
    else:
        cases.append((args.page or "home", None, args.theme))

    display = free_display()
    env = dict(os.environ)
    env["DISPLAY"] = f":{display}"
    env["AHAB_BACKEND"] = "mock"
    env["AHAB_VISUAL_LANGUAGE"] = args.language
    env["AHAB_VISUAL_SKIN"] = args.skin
    if args.accent:
        env["AHAB_VISUAL_ACCENT"] = args.accent
    if Path(LAVAPIPE).is_file():
        env["VK_ICD_FILENAMES"] = LAVAPIPE

    xvfb = subprocess.Popen(
        ["Xvfb", f":{display}", "-screen", "0", f"{width}x{height}x24"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    print(f"xvfb on :{display} (pid {xvfb.pid}, {width}x{height})")
    time.sleep(2)

    try:
        for page, state, theme in cases:
            case_env = dict(env)
            case_env["AHAB_VISUAL_PAGE"] = page
            case_env["AHAB_VISUAL_THEME"] = theme
            if state:
                case_env["AHAB_VISUAL_STATE"] = state
            log = Path("/tmp") / f"gpui-capture-{page}-{theme}.log"
            with log.open("wb") as stream:
                app = subprocess.Popen(
                    [str(args.binary)],
                    cwd=ROOT,
                    env=case_env,
                    stdout=stream,
                    stderr=subprocess.STDOUT,
                    start_new_session=True,
                )
            try:
                time.sleep(args.wait)
                if app.poll() is not None:
                    print(f"  {page}/{theme}: the app exited with {app.returncode} (log: {log})", file=sys.stderr)
                    continue
                stem = f"{state}-{theme}" if state else f"{page}-{theme}"
                out = args.out_dir / f"{stem}.png"
                capture(display, case_env, out)
                print(f"  {out.relative_to(ROOT) if out.is_relative_to(ROOT) else out}")
            finally:
                kill(app)
    finally:
        kill(xvfb)

    return 0


if __name__ == "__main__":
    sys.exit(main())
