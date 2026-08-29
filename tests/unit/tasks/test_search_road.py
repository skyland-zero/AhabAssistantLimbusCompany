from __future__ import annotations

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
