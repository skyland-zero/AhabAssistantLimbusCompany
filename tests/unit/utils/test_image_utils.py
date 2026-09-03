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


def test_non_max_suppression_handles_empty_inputs() -> None:
    assert ImageUtils.non_max_suppression([], [], score_threshold=0.5) == []

    boxes = [[10.0, 10.0, 20.0, 20.0]]
    scores = [0.2]
    assert ImageUtils.non_max_suppression(boxes, scores, score_threshold=0.5) == []

