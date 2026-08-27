"""Report pixel differences between Tauri and GPUI screenshot directories.

The two native clients can differ by one physical column because WebView2
rounds a 900/800 CSS viewport at non-100% DPI. The comparison crops both
images to their common top-left rectangle and records that normalization in
the report; it does not silently resize either image.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image


def image_pairs(reference: Path, gpui: Path) -> list[tuple[Path, Path]]:
    pairs: list[tuple[Path, Path]] = []
    for ref in sorted(reference.glob("tauri-*.png")):
        gpui_name = ref.name.replace("tauri-", "gpui-", 1)
        candidate = gpui / gpui_name
        if candidate.is_file():
            pairs.append((ref, candidate))
    return pairs


def compare(reference: Path, candidate: Path) -> dict[str, object]:
    with Image.open(reference) as ref_image:
        ref = np.asarray(ref_image.convert("RGB"), dtype=np.int16)
    with Image.open(candidate) as gpui_image:
        gpui = np.asarray(gpui_image.convert("RGB"), dtype=np.int16)

    height = min(ref.shape[0], gpui.shape[0])
    width = min(ref.shape[1], gpui.shape[1])
    delta = np.abs(ref[:height, :width] - gpui[:height, :width])
    changed = np.max(delta, axis=2) > 16
    return {
        "reference": reference.as_posix(),
        "gpui": candidate.as_posix(),
        "referenceSize": list(map(int, ref.shape[:2][::-1])),
        "gpuiSize": list(map(int, gpui.shape[:2][::-1])),
        "comparedSize": [width, height],
        "croppedPixels": int(ref.shape[0] * ref.shape[1] - height * width),
        "meanAbsoluteChannelDelta": round(float(delta.mean()), 3),
        "p95AbsoluteChannelDelta": round(float(np.percentile(delta, 95)), 3),
        "changedPixelRatioAt16": round(float(changed.mean()), 6),
        "maxChannelDelta": int(delta.max()) if delta.size else 0,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--gpui", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    pairs = image_pairs(args.reference, args.gpui)
    if not pairs:
        raise SystemExit("no matching tauri-*/gpui-* PNG pairs found")
    report = {
        "referenceDirectory": args.reference.as_posix(),
        "gpuiDirectory": args.gpui.as_posix(),
        "normalization": "top-left crop to common physical-pixel rectangle",
        "pairs": [compare(reference, gpui) for reference, gpui in pairs],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"compared {len(pairs)} screenshot pairs -> {args.output}")


if __name__ == "__main__":
    main()
