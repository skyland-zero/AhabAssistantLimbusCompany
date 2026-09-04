from __future__ import annotations

from types import SimpleNamespace
from unittest.mock import Mock

import numpy as np
import pytest
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


def test_node_detector_lock_wait_is_cancelable(monkeypatch) -> None:
    search_road._node_detector_lock.acquire()
    checks = 0

    def cancel_after_one_check() -> None:
        nonlocal checks
        checks += 1
        if checks >= 2:
            raise RuntimeError("cancelled")

    monkeypatch.setattr(search_road, "check_cancelled", cancel_after_one_check)
    try:
        with pytest.raises(RuntimeError, match="cancelled"):
            search_road._get_node_detector()
    finally:
        search_road._node_detector_lock.release()

    assert checks >= 2


def _build_route_graph_for_strategy_test() -> search_road.RouteGraph:
    graph = search_road.RouteGraph([], search_road.Row.MID, (0, 0), hard_mode=True)
    graph._add_new_column()
    graph._set_node(2, search_road.Row.TOP, "event", 1)
    graph._set_node(2, search_road.Row.MID, "shop", 8)
    graph._add_new_column()
    graph._set_node(3, search_road.Row.TOP, "battle", 4)
    graph._set_node(3, search_road.Row.MID, "event", 8)
    graph._add_new_column()
    graph._set_node(4, search_road.Row.TOP, "boss_battle", 1)
    graph._set_node(4, search_road.Row.MID, "boss_battle", 1)
    graph.columns["column1"][search_road.Row.MID].add_next_node(
        graph.columns["column2"][search_road.Row.TOP]
    )
    graph.columns["column1"][search_road.Row.MID].add_next_node(
        graph.columns["column2"][search_road.Row.MID]
    )
    graph.columns["column2"][search_road.Row.TOP].add_next_node(
        graph.columns["column3"][search_road.Row.TOP]
    )
    graph.columns["column2"][search_road.Row.MID].add_next_node(
        graph.columns["column3"][search_road.Row.MID]
    )
    graph.columns["column3"][search_road.Row.TOP].add_next_node(
        graph.columns["column4"][search_road.Row.TOP]
    )
    graph.columns["column3"][search_road.Row.MID].add_next_node(
        graph.columns["column4"][search_road.Row.MID]
    )
    return graph


def test_route_strategy_keeps_legacy_weight_selection_by_default() -> None:
    graph = _build_route_graph_for_strategy_test()

    _, path = graph.find_min_weight_route()

    assert [node.node_class for node in path] == ["bus", "event", "battle", "boss_battle"]


def test_route_strategy_can_prioritize_fewer_non_boss_combats() -> None:
    graph = _build_route_graph_for_strategy_test()

    _, path = graph.find_min_weight_route(minimize_non_boss_combat=True)
    statistics = graph.get_path_statistics(path)

    assert [node.node_class for node in path] == ["bus", "shop", "event", "boss_battle"]
    assert statistics["non_boss_combat"] == 0
    assert statistics["event"] == 1
