from __future__ import annotations

import numpy as np

from utils.image_utils import ImageUtils


def test_multiple_template_matching_returns_one_peak_per_target() -> None:
    rng = np.random.default_rng(7)
    template = rng.integers(30, 255, size=(12, 12), dtype=np.uint8)
    screenshot = np.zeros((100, 140), dtype=np.uint8)
    positions = [(10, 15), (80, 60)]
    for x, y in positions:
        screenshot[y : y + template.shape[0], x : x + template.shape[1]] = template

    matches = ImageUtils.match_template_with_multiple_targets(screenshot, template, threshold=0.99)

    expected = {(x + template.shape[1] // 2, y + template.shape[0] // 2) for x, y in positions}
    assert set(matches) == expected


def test_feature_matching_accepts_precomputed_descriptors() -> None:
    rng = np.random.default_rng(11)
    template = rng.integers(0, 255, size=(80, 80), dtype=np.uint8)
    target = np.zeros((180, 180), dtype=np.uint8)
    target[40:120, 50:130] = template

    template_features = ImageUtils.feature_descriptors(template)
    target_features = ImageUtils.feature_descriptors(target)
    result_with_cache = ImageUtils.feature_matching(
        template,
        target,
        min_matches=8,
        template_features=template_features,
        target_features=target_features,
    )
    result_without_cache = ImageUtils.feature_matching(template, target, min_matches=8)

    assert result_with_cache[0] is True
    assert result_with_cache[1] > 0
    assert result_without_cache[0] is True
