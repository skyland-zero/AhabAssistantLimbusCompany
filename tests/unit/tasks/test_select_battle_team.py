from __future__ import annotations

import types
import pytest

from tasks.teams import team_formation as team_formation_module


class FakeAuto:
    def __init__(self, initial_find_result=None, after_scroll_find_result=None):
        self.initial_find_result = initial_find_result
        self.after_scroll_find_result = after_scroll_find_result
        self.call_count = 0
        self.swipes = []
        self.clicks = []
        self.actions_with_pos = []

    def take_screenshot(self):
        return object()

    def find_element(self, path, **_kwargs):
        if path == "teams/identify_assets.png":
            # 模拟在 1080p 下 (scale=0.75) 识别到锚点
            return (1800, 266)
        if path in ("home/first_prompt_assets.png", "home/back_assets.png"):
            return None
        return None

    def click_element(self, _path, **_kwargs):
        return False

    def mouse_click(self, x, y, *_args, **_kwargs):
        self.clicks.append((x, y))

    def mouse_action_with_pos(self, pos, offset=False):
        self.actions_with_pos.append((pos, offset))
        return True

    def mouse_swipe_for_scroll(self, x, y, dy, duration=0.3):
        self.swipes.append((x, y, dy, duration))

    def find_language_text(self, zh_text, en_text, my_crop=None):
        self.call_count += 1
        if self.call_count == 1:
            return self.initial_find_result
        return self.after_scroll_find_result


def test_fast_path_hits_immediately_skipping_resets(monkeypatch):
    """当初始屏幕已经能识别到目标队伍时，触发快速路径，直接点击并跳过 5 次滑动复位。"""
    # 1080p 下，position 约为 (187.5, 427.25)，安全 Y 范围为 [427.25 + 22.5, 427.25 + 435] 约 [450, 862]
    # 我们设置初始命中坐标为 (187, 500)
    fake_auto = FakeAuto(initial_find_result=[187, 500])
    monkeypatch.setattr(team_formation_module, "auto", fake_auto)
    monkeypatch.setattr(team_formation_module, "sleep", lambda _s: None)

    result = team_formation_module.select_battle_team(1)

    assert result is True
    # 关键断言：没有执行任何 mouse_swipe_for_scroll 滑动
    assert len(fake_auto.swipes) == 0
    # 直接在目标位置执行了点击选中
    assert fake_auto.actions_with_pos == [([187, 500], False)]


def test_fast_path_misses_falls_back_to_reset_and_scroll(monkeypatch):
    """当初始屏幕未识别到目标队伍时，平滑回退执行 5 次复位滑动，随后翻页找到队伍。"""
    fake_auto = FakeAuto(initial_find_result=False, after_scroll_find_result=[187, 600])
    monkeypatch.setattr(team_formation_module, "auto", fake_auto)
    monkeypatch.setattr(team_formation_module, "sleep", lambda _s: None)

    # 默认模式：select_team_by_order = False
    result = team_formation_module.select_battle_team(2)

    assert result is True
    # 执行了 5 次正向下拉复位
    assert len(fake_auto.swipes) >= 5
    for swipe in fake_auto.swipes[:5]:
        assert swipe[2] > 0  # dy > 0，为下拉复位
    assert fake_auto.actions_with_pos == [([187, 600], False)]


def test_fast_path_out_of_safe_bounds_falls_back(monkeypatch):
    """当初始识别结果处于非安全边界（如顶部过高或底部露出半截）时，跳过快速直选，走慢速复位。"""
    # Y 坐标为 400，低于 safe_top (约 450)
    fake_auto = FakeAuto(initial_find_result=[187, 400], after_scroll_find_result=[187, 600])
    monkeypatch.setattr(team_formation_module, "auto", fake_auto)
    monkeypatch.setattr(team_formation_module, "sleep", lambda _s: None)

    result = team_formation_module.select_battle_team(3)

    assert result is True
    # 确认回退到了 5 次下拉复位
    assert len(fake_auto.swipes) >= 5


def test_slow_path_ignores_out_of_safe_bounds_coordinates(monkeypatch):
    """当慢速复位识别到的坐标处于非安全区域（如点击会点到左上角返回键）时，慢速路径不点击，继续翻页重试。"""
    # 第一次返回顶部外的异常坐标 [98, 71]，第二次翻页后返回合法坐标 [187, 600]
    results = [[98, 71], [187, 600]]

    class MultiFindFakeAuto(FakeAuto):
        def find_language_text(self, zh_text, en_text, my_crop=None):
            self.call_count += 1
            if self.call_count == 1:
                return self.initial_find_result
            if results:
                return results.pop(0)
            return None

    fake_auto = MultiFindFakeAuto(initial_find_result=False)
    monkeypatch.setattr(team_formation_module, "auto", fake_auto)
    monkeypatch.setattr(team_formation_module, "sleep", lambda _s: None)

    result = team_formation_module.select_battle_team(4)

    assert result is True
    # 绝不能执行对 [98, 71] 的误点击，只点击安全区域内的 [187, 600]
    assert fake_auto.actions_with_pos == [([187, 600], False)]

