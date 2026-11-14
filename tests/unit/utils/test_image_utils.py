from __future__ import annotations

import numpy as np
import pytest

from module.config import cfg
from utils.image_utils import ImageUtils


@pytest.fixture
def window_size(monkeypatch):
    """Patch the configured window height without touching the real config."""

    def _set(value: int) -> None:
        monkeypatch.setattr(cfg, "set_win_size", value)

    return _set


def test_coarse_limits_follow_the_window_size(window_size) -> None:
    """The coarse thresholds must not be absolute pixels.

    A template whose 1440p min side is 34 px shrinks to 26 px at 1080p.  With
    absolute limits it stopped qualifying for the coarse pass, so the shop and
    skill-replacement loops ran full-resolution matches at lower resolutions
    and were measurably slower than at 1440p.
    """

    region = (1440, 2560)
    template = (34, 168)

    window_size(1440)
    assert ImageUtils._coarse_scale(region, template) > 1

    window_size(1080)
    scaled_region = (1080, 1920)
    scaled_template = (26, 126)
    assert ImageUtils._coarse_scale(scaled_region, scaled_template) > 1


@pytest.mark.parametrize("win_size", [720, 900, 1080, 1440, 1800, 2160])
@pytest.mark.parametrize("min_side", [22, 26, 28, 34, 49, 65])
def test_coarse_gate_is_resolution_consistent(window_size, win_size, min_side) -> None:
    """The same feature must be gated the same way at every window size."""

    height = win_size
    width = int(round(win_size * 16 / 9))
    scale = win_size / 1440
    template_min = int(round(min_side * scale))
    region = (height, width)
    template = (template_min, max(template_min, int(round(min_side * scale * 4))))

    window_size(win_size)
    enabled = ImageUtils._coarse_scale(region, template) > 1

    window_size(1440)
    reference = ImageUtils._coarse_scale(
        (1440, 2560),
        (min_side, max(min_side, min_side * 4)),
    )
    assert enabled is (reference > 1)


def test_coarse_limits_scale_area_quadratically(window_size) -> None:
    window_size(1440)
    min_area_1440, side_1440, scaled_1440 = ImageUtils._coarse_limits()
    window_size(1080)
    min_area_1080, side_1080, scaled_1080 = ImageUtils._coarse_limits()

    assert min_area_1440 == ImageUtils._COARSE_MIN_AREA
    assert min_area_1080 == pytest.approx(ImageUtils._COARSE_MIN_AREA * 0.75**2)
    assert side_1080 == pytest.approx(side_1440 * 0.75)
    assert scaled_1080 == pytest.approx(scaled_1440 * 0.75)


def test_coarse_parameters_reports_effective_limits(window_size) -> None:
    window_size(1080)
    parameters = ImageUtils.coarse_parameters()

    assert parameters["reference_win_size"] == 1440
    assert parameters["min_template_side"] == pytest.approx(ImageUtils._COARSE_MIN_TEMPLATE_SIDE * 0.75, abs=1e-3)


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


def test_template_matching_returns_no_match_when_region_is_smaller_than_template() -> None:
    screenshot = np.zeros((8, 8), dtype=np.uint8)
    template = np.zeros((12, 12), dtype=np.uint8)

    assert ImageUtils.match_template(screenshot, template, None) == (None, 0.0)
    assert ImageUtils.match_template_with_multiple_targets(screenshot, template, threshold=0.8) == []


def test_coarse_single_target_matches_full_resolution() -> None:
    rng = np.random.default_rng(5)
    template = rng.integers(0, 255, (80, 80), dtype=np.uint8)
    screenshot = rng.integers(0, 255, (900, 1200), dtype=np.uint8)
    screenshot[500:580, 700:780] = template

    coarse = ImageUtils.match_template(screenshot, template, None, "aggressive", threshold=0.8)
    full = ImageUtils.match_template(
        screenshot,
        template,
        None,
        "aggressive",
        threshold=0.8,
        coarse=False,
    )

    assert coarse[0] == full[0] == (740, 540)
    assert coarse[1] > 0.99
    assert abs(coarse[1] - full[1]) < 0.01


def test_coarse_single_target_preserves_bbox_offset() -> None:
    rng = np.random.default_rng(6)
    template = rng.integers(0, 255, (80, 80), dtype=np.uint8)
    screenshot = rng.integers(0, 255, (900, 1200), dtype=np.uint8)
    screenshot[500:580, 700:780] = template

    center, score = ImageUtils.match_template(
        screenshot,
        template,
        (690, 490, 790, 590),
        "clam",
        threshold=0.8,
    )

    assert center == (740, 540)
    assert score > 0.99


def test_coarse_multiple_targets_matches_full_resolution() -> None:
    rng = np.random.default_rng(11)
    template = rng.integers(0, 255, (80, 80), dtype=np.uint8)
    screenshot = rng.integers(0, 255, (900, 1200), dtype=np.uint8)
    positions = [(120, 100), (850, 650)]
    for x, y in positions:
        screenshot[y : y + 80, x : x + 80] = template

    coarse = ImageUtils.match_template_with_multiple_targets(screenshot, template, threshold=0.95)
    full = ImageUtils.match_template_with_multiple_targets(
        screenshot,
        template,
        threshold=0.95,
        coarse=False,
    )

    assert set(coarse) == set(full) == {(160, 140), (890, 690)}


def test_coarse_matching_rejects_random_negative() -> None:
    rng = np.random.default_rng(123)
    template = rng.integers(0, 255, (80, 80), dtype=np.uint8)
    screenshot = rng.integers(0, 255, (900, 1200), dtype=np.uint8)

    assert ImageUtils.match_template(
        screenshot,
        template,
        None,
        "aggressive",
        threshold=0.95,
    ) == (None, 0.0)
    assert ImageUtils.match_template_with_multiple_targets(
        screenshot,
        template,
        threshold=0.95,
    ) == []


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


def test_image_to_blob_formats_contiguous_nchw_tensor() -> None:
    rng = np.random.default_rng(42)
    image = rng.integers(0, 256, size=(30, 40, 3), dtype=np.uint8)

    blob = ImageUtils.image_to_blob(image)

    assert blob.shape == (1, 3, 30, 40)
    assert blob.dtype == np.float32
    assert blob.flags["C_CONTIGUOUS"]
    expected = (image.astype(np.float32) / 255.0).transpose(2, 0, 1)[np.newaxis, ...]
    assert np.allclose(blob, expected, atol=1e-7)


def test_non_max_suppression_filters_overlap_and_low_confidence() -> None:
    boxes = [
        [10.0, 10.0, 50.0, 50.0],  # Box 0: 置信度最高
        [12.0, 12.0, 50.0, 50.0],  # Box 1: 与 Box 0 高度重叠，应被抑制
        [100.0, 100.0, 50.0, 50.0],  # Box 2: 独立框，应保留
        [200.0, 200.0, 50.0, 50.0],  # Box 3: 置信度低于阈值，应被丢弃
    ]
    scores = [0.95, 0.85, 0.90, 0.40]

    kept = ImageUtils.non_max_suppression(
        boxes,
        scores,
        score_threshold=0.5,
        nms_threshold=0.4,
    )

    # 预期保留 Box 0 和 Box 2，且按分数降序排序为 [0, 2]
    assert kept == [0, 2]


def test_bbox_from_path_is_cached_until_cleared(monkeypatch) -> None:
    calls: list[str] = []
    image = np.zeros((10, 10), dtype=np.uint8)
    image[2:5, 3:7] = 255

    def fake_load(path, **_kwargs):
        calls.append(path)
        return image

    monkeypatch.setattr(ImageUtils, "load_image", fake_load)
    ImageUtils.clear_bbox_cache()
    try:
        assert ImageUtils.get_bbox_from_path("asset.png") == (3, 2, 7, 5)
        assert ImageUtils.get_bbox_from_path("asset.png") == (3, 2, 7, 5)
        assert calls == ["asset.png"]

        ImageUtils.clear_bbox_cache()
        assert ImageUtils.get_bbox_from_path("asset.png") == (3, 2, 7, 5)
        assert calls == ["asset.png", "asset.png"]
    finally:
        ImageUtils.clear_bbox_cache()


def test_non_max_suppression_handles_empty_inputs() -> None:
    assert ImageUtils.non_max_suppression([], [], score_threshold=0.5) == []

    boxes = [[10.0, 10.0, 20.0, 20.0]]
    scores = [0.2]
    assert ImageUtils.non_max_suppression(boxes, scores, score_threshold=0.5) == []

