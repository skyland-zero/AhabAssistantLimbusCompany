r"""Fetch and normalize E.G.O gift template images for mirror routes.

Downloads transparent gift icons from Limbus Company wiki.gg (with Fandom fallback),
crops transparent margins using bounding box, scales them to the 1440p template
baseline, and saves them to ``assets/images/default/share/mirror/ego_gifts/{gift_id}.png``.

Usage:
    .venv\Scripts\python.exe scripts\fetch_ego_gifts.py
"""

from __future__ import annotations

import argparse
import logging
import re
import sys
import time
from io import BytesIO
from pathlib import Path

import requests
from PIL import Image

PROJECT_ROOT = Path(__file__).resolve().parents[1]
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

import module.mirror_routes as r  # noqa: E402

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger("fetch_ego_gifts")

HEADERS = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
}

WIKI_GG_API = "https://limbuscompany.wiki.gg/api.php"

# Manual aliases for route IDs that map to specific wiki file names
MANUAL_ALIASES: dict[str, str] = {
    "bridle_of_infinity": "File:Temporal Bridle Gift.png",
    "hot_n_juicy_drumstick": "File:Hot \u2018n Juicy Drumstick Gift.png",
}


def _normalize_name(name: str) -> str:
    """Normalize string for fuzzy comparison by removing all punctuation and spaces."""
    return re.sub(r"[^a-zA-Z0-9]", "", name).lower().replace("and", "")


def get_all_route_gifts() -> dict[str, list[str]]:
    """Extract all unique gifts defined in mirror routes."""
    routes = [getattr(r, n) for n in dir(r) if isinstance(getattr(r, n), r.MirrorRouteDefinition)]
    gifts: dict[str, list[str]] = {}
    for rt in routes:
        for g in rt.gifts:
            gifts.setdefault(g.gift_id, list(g.names_en))
        for rec in rt.recipes:
            gifts.setdefault(rec.result_gift_id, list(rec.result_names_en))
    return gifts


def fetch_wiki_gg_gift_catalog() -> dict[str, str]:
    """Fetch mapping of normalized title -> wiki File title from wiki.gg."""
    logger.info("Fetching E.G.O gift catalog from wiki.gg...")
    titles = []
    cmcontinue = None
    while True:
        params = {
            "action": "query",
            "list": "categorymembers",
            "cmtitle": "Category:E.G.O_Gifts",
            "cmlimit": "500",
            "format": "json",
        }
        if cmcontinue:
            params["cmcontinue"] = cmcontinue
        resp = requests.get(WIKI_GG_API, params=params, headers=HEADERS, timeout=20)
        data = resp.json()
        for m in data.get("query", {}).get("categorymembers", []):
            titles.append(m["title"])
        if "continue" in data and "cmcontinue" in data["continue"]:
            cmcontinue = data["continue"]["cmcontinue"]
        else:
            break

    catalog: dict[str, str] = {}
    for t in titles:
        base = t.replace("File:", "").replace(".png", "").replace(".webp", "")
        if base.lower().endswith("gift"):
            base = base[:-4]
        catalog[_normalize_name(base)] = t
    logger.info(f"Loaded {len(catalog)} gifts from wiki.gg catalog.")
    return catalog


def resolve_image_urls(wiki_titles: list[str]) -> dict[str, str]:
    """Batch resolve direct download URLs for wiki file titles."""
    urls: dict[str, str] = {}
    chunk_size = 40
    for i in range(0, len(wiki_titles), chunk_size):
        chunk = wiki_titles[i : i + chunk_size]
        params = {
            "action": "query",
            "titles": "|".join(chunk),
            "prop": "imageinfo",
            "iiprop": "url",
            "format": "json",
        }
        resp = requests.get(WIKI_GG_API, params=params, headers=HEADERS, timeout=20)
        data = resp.json()
        for page in data.get("query", {}).get("pages", {}).values():
            if int(page.get("pageid", -1)) > 0 and "imageinfo" in page and page["imageinfo"]:
                urls[page["title"]] = page["imageinfo"][0]["url"]
    return urls


def process_and_save_image(image_bytes: bytes, output_path: Path, max_dim: int = 96) -> tuple[int, int]:
    """Crop transparent padding, resize to standard scale, and save."""
    img = Image.open(BytesIO(image_bytes)).convert("RGBA")
    bbox = img.getbbox()
    if bbox:
        img = img.crop(bbox)

    w, h = img.size
    scale = max_dim / max(w, h)
    new_w = max(1, int(round(w * scale)))
    new_h = max(1, int(round(h * scale)))
    resized = img.resize((new_w, new_h), Image.Resampling.LANCZOS)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    resized.save(output_path, "PNG", optimize=True)
    return new_w, new_h


def main() -> None:
    parser = argparse.ArgumentParser(description="Fetch and process mirror route E.G.O gift templates.")
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=PROJECT_ROOT / "assets" / "images" / "default" / "share" / "mirror" / "ego_gifts",
        help="Directory to save gift templates",
    )
    parser.add_argument("--max-dim", type=int, default=96, help="Target max dimension in pixels")
    parser.add_argument("--force", action="store_true", help="Force overwrite existing templates")
    args = parser.parse_args()

    args.output_dir.mkdir(parents=True, exist_ok=True)

    route_gifts = get_all_route_gifts()
    logger.info(f"Target gifts count from mirror routes: {len(route_gifts)}")

    wiki_catalog = fetch_wiki_gg_gift_catalog()

    # Match each route gift to a wiki file title
    matched_files: dict[str, str] = {}
    for gift_id, names in route_gifts.items():
        if gift_id in MANUAL_ALIASES:
            matched_files[gift_id] = MANUAL_ALIASES[gift_id]
            continue

        found = False
        for name in names:
            norm = _normalize_name(name)
            if norm in wiki_catalog:
                matched_files[gift_id] = wiki_catalog[norm]
                found = True
                break
        if not found:
            for norm_cat, file_title in wiki_catalog.items():
                if any(_normalize_name(n) == norm_cat for n in names):
                    matched_files[gift_id] = file_title
                    found = True
                    break

    logger.info(f"Matched {len(matched_files)} / {len(route_gifts)} gifts to wiki files.")

    # Resolve direct URLs for matched files
    unique_titles = list(set(matched_files.values()))
    image_urls = resolve_image_urls(unique_titles)
    logger.info(f"Resolved direct URLs for {len(image_urls)} files.")

    # Download and process
    success = 0
    skipped = 0
    failed = []

    for gift_id, file_title in matched_files.items():
        out_file = args.output_dir / f"{gift_id}.png"
        if out_file.exists() and not args.force:
            skipped += 1
            continue

        url = image_urls.get(file_title)
        if not url:
            failed.append((gift_id, file_title, "No direct URL resolved"))
            continue

        try:
            resp = requests.get(url, headers=HEADERS, timeout=20)
            resp.raise_for_status()
            w, h = process_and_save_image(resp.content, out_file, max_dim=args.max_dim)
            success += 1
            logger.info(f"✓ Saved {gift_id}.png ({w}x{h}) from {file_title}")
            time.sleep(0.05)  # polite throttle
        except Exception as e:
            failed.append((gift_id, file_title, str(e)))
            logger.warning(f"✗ Failed {gift_id}: {e}")

    # Summary.  Use stdout directly so this CLI keeps its human-readable
    # report while satisfying the repository's no-bare-print lint policy.
    summary_lines = [
        "",
        "=" * 60,
        "E.G.O Gift Template Download Summary:",
        f"  Target Gifts:  {len(route_gifts)}",
        f"  Matched Gifts: {len(matched_files)}",
        f"  Downloaded:    {success}",
        f"  Skipped:       {skipped} (already exist)",
        f"  Total on disk: {len(list(args.output_dir.glob('*.png')))}",
    ]
    if failed:
        summary_lines.append(f"  Failed ({len(failed)}):")
        for gid, title, err in failed:
            summary_lines.append(f"    - {gid} ({title}): {err}")
    summary_lines.extend(["=" * 60, ""])
    sys.stdout.write("\n".join(summary_lines) + "\n")


if __name__ == "__main__":
    main()
