r"""Small local benchmark for the backend's CPU vision hot paths.

Run from the repository root, for example:

    .venv\Scripts\python.exe scripts\benchmark_vision.py --runs 10

The benchmark is synthetic by default so it does not depend on a running
game window.  ``--ocr`` additionally measures the configured RapidOCR engine
on the same synthetic frame.
"""

from __future__ import annotations

import argparse
import statistics
import sys
import time
from collections.abc import Callable
from pathlib import Path

import numpy as np

# Make direct execution from the ``scripts`` directory behave like execution
# from the repository root.
PROJECT_ROOT = Path(__file__).resolve().parents[1]
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from utils.image_utils import ImageUtils  # noqa: E402


def _measure(action: Callable[[], object], runs: int) -> list[float]:
    for _ in range(2):
        action()
    samples = []
    for _ in range(runs):
        start = time.perf_counter()
        action()
        samples.append((time.perf_counter() - start) * 1000)
    return samples


def _format_samples(samples: list[float]) -> str:
    ordered = sorted(samples)
    p95_index = min(len(ordered) - 1, round((len(ordered) - 1) * 0.95))
    return f"median={statistics.median(samples):.2f}ms p95={ordered[p95_index]:.2f}ms"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=int, default=10, help="每个项目的测量次数")
    parser.add_argument("--ocr", action="store_true", help="额外测量 RapidOCR")
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs 必须大于 0")

    rng = np.random.default_rng(2026)
    template = rng.integers(0, 255, size=(80, 80), dtype=np.uint8)
    frame = np.zeros((1080, 1920), dtype=np.uint8)
    frame[420:500, 760:840] = template

    template_features = ImageUtils.feature_descriptors(template)
    frame_features = ImageUtils.feature_descriptors(frame)
    actions = {
        "template_full": lambda: ImageUtils.match_template(frame, template, None, "aggressive"),
        "template_multiple": lambda: ImageUtils.match_template_with_multiple_targets(
            frame, template, threshold=0.9
        ),
        "feature_cached": lambda: ImageUtils.feature_matching(
            template,
            frame,
            template_features=template_features,
            target_features=frame_features,
        ),
    }

    for name, action in actions.items():
        sys.stdout.write(f"{name}: {_format_samples(_measure(action, args.runs))}\n")

    if args.ocr:
        from module.ocr import ocr

        sys.stdout.write(f"ocr: {_format_samples(_measure(lambda: ocr.run(frame), args.runs))}\n")


if __name__ == "__main__":
    main()
