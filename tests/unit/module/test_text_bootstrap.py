import os

import numpy as np
import pytest

from module.automation import auto
from module.automation.text_bootstrap import TextBootstrapManager
from utils.path_manager import path_manager


@pytest.fixture(autouse=True)
def setup_teardown_bootstrap():
    path_manager.initialize_paths()
    path_manager.current_theme = "default"
    path_manager.current_language = "zh_cn"
    test_key = "__unit_test_bootstrap_key__"
    yield test_key
    # 清理测试生成的图片
    save_path = TextBootstrapManager.get_save_abs_path(test_key)
    if os.path.exists(save_path):
        os.remove(save_path)


def test_harvest_from_frame_creates_valid_png(setup_teardown_bootstrap):
    test_key = setup_teardown_bootstrap
    # 创建一个 1080x1920 假图像 (带有特定图案)
    fake_frame = np.zeros((1080, 1920, 3), dtype=np.uint8)
    fake_frame[100:150, 200:300] = 255  # 白色方块
    bbox = (200.0, 100.0, 300.0, 150.0)

    success = TextBootstrapManager.harvest_from_frame(fake_frame, bbox, test_key, padding=4)
    assert success is True

    save_path = TextBootstrapManager.get_save_abs_path(test_key)
    assert os.path.exists(save_path)
    assert os.path.getsize(save_path) > 0

    # 验证已存在模板的检查函数
    assert TextBootstrapManager.has_template(test_key) is True


def test_find_or_bootstrap_text_uses_existing_template(setup_teardown_bootstrap, monkeypatch):
    test_key = setup_teardown_bootstrap
    fake_frame = np.zeros((1080, 1920, 3), dtype=np.uint8)
    fake_frame[200:260, 400:500] = 200
    bbox = (400.0, 200.0, 500.0, 260.0)

    TextBootstrapManager.harvest_from_frame(fake_frame, bbox, test_key)
    assert TextBootstrapManager.has_template(test_key) is True

    # 模拟 find_element 成功返回
    monkeypatch.setattr(auto, "find_element", lambda target, **kwargs: (450, 230))
    # 模拟 OCR，如果走 OCR 则抛出异常确保它没走 OCR
    def fail_ocr(*args, **kwargs):
        raise AssertionError("不应调用 OCR，应优先命中模板")
    monkeypatch.setattr(auto, "find_language_text", fail_ocr)

    pos = auto.find_or_bootstrap_text("测试", "test", test_key)
    assert pos == (450, 230)
