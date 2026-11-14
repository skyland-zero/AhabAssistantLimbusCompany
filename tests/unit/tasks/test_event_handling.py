from __future__ import annotations

import numpy as np

from tasks.event.event_handling import EventHandling, ImageUtils, auto


def test_decision_event_handling_accepts_ndarray_bbox(monkeypatch) -> None:
    """The bbox asset is loaded as a numpy array, not a PIL image."""

    handler = EventHandling()
    frame = np.zeros((1080, 1920, 3), dtype=np.uint8)
    bbox_image = np.zeros((1080, 1920), dtype=np.uint8)
    bbox_image[100:200, 100:200] = 255

    def fake_take_screenshot(**_kwargs):
        auto.screenshot = frame
        return frame

    monkeypatch.setattr(auto, "take_screenshot", fake_take_screenshot)
    monkeypatch.setattr(auto, "screenshot", frame, raising=False)
    monkeypatch.setattr(ImageUtils, "load_image", lambda *_args, **_kwargs: bbox_image)
    monkeypatch.setattr(handler, "_click_first_hit", lambda *_args, **_kwargs: True)

    assert handler.decision_event_handling() is True
