from __future__ import annotations

from types import SimpleNamespace
from unittest.mock import Mock

import numpy as np
from PIL import Image

import tasks.mirror.search_road as search_road


class FakeNodeSession:
    def run(self, _outputs, _inputs):
        output = np.zeros((1, 11, 20), dtype=np.float32)
        output[0, 0, 0] = 600
        output[0, 1, 0] = 272
        output[0, 2, 0] = 100
        output[0, 3, 0] = 100
        output[0, 4, 0] = 0.95
        return [output]


def test_identify_nodes_vectorizes_model_output(monkeypatch) -> None:
    monkeypatch.setattr(search_road, "_get_node_detector", lambda: (FakeNodeSession(), "images"))
    monkeypatch.setattr(search_road.cfg, "set_win_size", 1440)

    nodes = search_road.identify_nodes(0, screenshot=Image.new("RGB", (1920, 1080), (0, 0, 0)))

    assert nodes == [["battle", (1200, 540)]]


def _fake_auto_for_fresh_gate(monkeypatch, *, age, wait_result=True):
    fake_auto = SimpleNamespace(
        current_frame_age=lambda: age,
        wait_for_fresh_frame=Mock(return_value=wait_result),
    )
    monkeypatch.setattr(search_road, "auto", fake_auto)
    return fake_auto


def test_ensure_fresh_map_frame_skipped_when_switch_off(monkeypatch) -> None:
    monkeypatch.setattr(search_road, "cfg", SimpleNamespace(mirror_fresh_frame_wait=False))
    fake_auto = _fake_auto_for_fresh_gate(monkeypatch, age=99.0)

    assert search_road.ensure_fresh_map_frame() is True
    fake_auto.wait_for_fresh_frame.assert_not_called()


def test_ensure_fresh_map_frame_passes_through_on_fresh_frame(monkeypatch) -> None:
    monkeypatch.setattr(search_road, "cfg", SimpleNamespace(mirror_fresh_frame_wait=True))
    fake_auto = _fake_auto_for_fresh_gate(monkeypatch, age=0.05)

    assert search_road.ensure_fresh_map_frame() is True
    fake_auto.wait_for_fresh_frame.assert_not_called()


def test_ensure_fresh_map_frame_passes_through_without_seq_transport(monkeypatch) -> None:
    monkeypatch.setattr(search_road, "cfg", SimpleNamespace(mirror_fresh_frame_wait=True))
    fake_auto = _fake_auto_for_fresh_gate(monkeypatch, age=None)

    assert search_road.ensure_fresh_map_frame() is True
    fake_auto.wait_for_fresh_frame.assert_not_called()


def test_ensure_fresh_map_frame_waits_on_stale_frame(monkeypatch) -> None:
    monkeypatch.setattr(search_road, "cfg", SimpleNamespace(mirror_fresh_frame_wait=True))
    fake_auto = _fake_auto_for_fresh_gate(monkeypatch, age=2.5, wait_result=True)

    assert search_road.ensure_fresh_map_frame(timeout=1.5, settle=0.1, reason="单测") is True
    fake_auto.wait_for_fresh_frame.assert_called_once_with(timeout=1.5, settle=0.1)


def test_ensure_fresh_map_frame_propagates_wait_failure(monkeypatch) -> None:
    monkeypatch.setattr(search_road, "cfg", SimpleNamespace(mirror_fresh_frame_wait=True))
    fake_auto = _fake_auto_for_fresh_gate(monkeypatch, age=2.5, wait_result=False)

    assert search_road.ensure_fresh_map_frame() is False
    fake_auto.wait_for_fresh_frame.assert_called_once()
