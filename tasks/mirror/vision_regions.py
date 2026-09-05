"""Shared fixed regions for the mirror UI.

The EGO gift selection page has several selectable cards across the middle of
the screen.  Keeping the region in one module prevents the battle,
reward-card, and mirror state machines from drifting back to unrelated ROIs
independently.
"""

from module.config import cfg

# The page can show four card markers across the row (observed centers range
# from roughly x=214 to x=1405 at 1920x1080).  The former profiler-derived
# point ROI only covered the third card and made the other cards invisible.
# Keep a generous central area: it still avoids the outer HUD, while covering
# all card positions and the slightly different battle/reward layouts.
_EGO_GIFT_CARD_ROI_RATIO = (0.08, 0.15, 0.92, 0.85)


def mirror_ego_gift_card_crop(win_size: int | float | None = None) -> tuple[int, int, int, int]:
    """Return the safe central ROI used to detect an EGO gift-card marker."""

    height = int(win_size or getattr(cfg, "set_win_size", 1080) or 1080)
    width = int(height * 16 / 9)
    x1, y1, x2, y2 = _EGO_GIFT_CARD_ROI_RATIO
    return (
        round(width * x1),
        round(height * y1),
        round(width * x2),
        round(height * y2),
    )
