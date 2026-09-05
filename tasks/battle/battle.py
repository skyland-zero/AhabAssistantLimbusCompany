import random
import re
import time
from dataclasses import dataclass
from typing import Callable, Optional

import cv2
import numpy as np

from core.execution_control import interruptible_sleep as sleep
from core.pseudo_solo import PseudoSoloDefenseState
from module.automation import auto
from module.config import cfg
from module.decorator.decorator import begin_and_finish_time_log
from module.device_manager import is_simulator_runtime
from module.logger import log
from module.ocr import ocr
from tasks import sins
from tasks.base.retry import retry
from tasks.event import event_handling
from tasks.mirror.vision_regions import mirror_ego_gift_card_crop
from utils.image_utils import ImageUtils
from utils.utils import find_skill3

DEFENSE_FOR_SOLO_TURN_LIMIT = 5
# The battle gear is partially covered by the lower command row in some
# resolutions.  Keep the existing crop/operation, but accept the observed
# 0.72-0.73 match range so pseudo-solo defense is not blocked before it runs.
DEFENSE_GEAR_THRESHOLD = 0.70
# Once the pause marker is visible, polling more often than this is enough to
# catch the command row returning without waiting the old 2 * adaptive delay.
BATTLE_ANIMATION_POLL_MAX = 1.5
# A missed battle-state frame is normally a short transition.  Keep the
# adaptive value for diagnostics, but do not sleep longer than this before the
# next fresh frame.
BATTLE_MISS_WAIT_MAX = 1.0


def _battle_ui_crops() -> dict[str, tuple[int, int, int, int]]:
    """计算战斗界面的稍微放宽裁剪区域，避免全屏模板匹配的高额CPU消耗。"""
    height = int(getattr(cfg, "set_win_size", 1080) or 1080)
    width = int(height * 16 / 9)
    return {
        "win_rate": (int(width * 0.58), int(height * 0.65), int(width * 0.82), height),
        "gear_right": (int(width * 0.52), int(height * 0.62), int(width * 0.82), int(height * 0.98)),
        "gear_left": (int(width * 0.15), int(height * 0.62), int(width * 0.42), int(height * 0.98)),
        "in_mirror": (int(width * 0.72), int(height * 0.04), int(width * 0.92), int(height * 0.28)),
        "dead": (int(width * 0.08), int(height * 0.50), int(width * 0.92), int(height * 0.95)),
        "dead_all": (int(width * 0.20), int(height * 0.18), int(width * 0.80), int(height * 0.82)),
        "acquire_gift_card": mirror_ego_gift_card_crop(height),
    }


@dataclass
class DefenseForSoloState:
    """一次镜牢任务内共享的连续防御回合状态。"""

    remaining_turns: int = DEFENSE_FOR_SOLO_TURN_LIMIT

    def consume_turn(self) -> None:
        if self.remaining_turns > 0:
            self.remaining_turns -= 1


class Battle:
    def __init__(self, is_tool: bool = False):
        self.first_battle = False
        self.identify_keyword_turn = True
        self.mouse_click_rate = False
        self.INIT_CHANCE = 16
        self.running = True  # 用于外部打断战斗逻辑执行
        self.defense_all_time = False
        self.fail_times = 0
        self.cur_turn = 1
        self.is_tool = is_tool
        """是否由小工具初始化"""

    @staticmethod
    def to_battle():
        loop_count = 15
        auto.model = "clam"
        click = False
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue
            if click and (
                auto.find_element("battle/normal_to_battle_assets.png")
                or auto.find_element("battle/chaim_to_battle_assets.png")
            ):
                click = False
                from tasks.teams.team_formation import deal_with_spills

                deal_with_spills()
            if auto.click_element("battle/normal_to_battle_assets.png"):
                click = True
                sleep(2)
                continue
            if auto.click_element("battle/chaim_to_battle_assets.png"):
                click = True
                sleep(2)
                continue
            loop_count -= 1
            if loop_count < 10:
                auto.model = "normal"
                log.debug("识别模式切换到正常模式")
            if loop_count < 5:
                auto.model = "aggressive"
                log.debug("识别模式切换到激进模式")
            if loop_count < 0:
                msg = "超出最大尝试次数,未能进入战斗"
                log.error(msg)
                return False
            if click:
                break

    @staticmethod
    def _update_wait_time(time: float = None, fail_flag: bool = False, total_count: int = 1):
        MAX_WAITING = 3.0  # 最大等待时间
        MIN_WAITING = 0.5  # 最小等待时间
        INIT_WAITING = 1.5  # 初始等待时间
        fail_adjust = 0.5
        success_adjust = -0.2
        if time is None:
            return INIT_WAITING

        total_count = total_count if total_count > 0 else 1  # 防止除0
        adjust = fail_adjust if fail_flag else success_adjust
        new_time = time + adjust / (total_count**0.5)  # 平方根调整

        new_time = min(new_time, MAX_WAITING)  # 防止超过最大等待时间
        new_time = max(new_time, MIN_WAITING)  # 防止低于最小等待时间
        if fail_flag:
            msg = f"匹配失败，等待时间从{time:.3f}调整为{new_time:.3f}"
            log.debug(msg)

        return new_time

    def _start_auto_p(
        self,
        *,
        use_damage_p: bool,
        crops: dict[str, tuple[int, int, int, int]],
    ) -> str:
        """Start automatic battle with the configured Win Rate/Damage mode."""

        mode_name = "伤害P" if use_damage_p else "胜率P"
        auto.key_press("p")
        if use_damage_p:
            # The game toggles the automatic result selector with a second P
            # before Enter.  Keep the key sequence separate so it remains
            # observable in diagnostics and easy to test.
            auto.key_press("p")
        sleep(0.5)
        auto.key_press("enter")

        if self.mouse_click_rate:
            my_scale = cfg.set_win_size / 1440
            if pos := auto.find_element("battle/win_rate_card.png", threshold=0.75, my_crop=crops["win_rate"]):
                option_offset_y = 50 * my_scale if use_damage_p else -50 * my_scale
                click_x = pos[0] + 50 * my_scale
                click_y = pos[1] + option_offset_y
                auto.mouse_click(click_x, click_y)
                auto.click_element("battle/gear_right.png", my_crop=crops["gear_right"])
                log.debug(
                    f"自动{mode_name}鼠标兜底: anchor={pos} click=({click_x:.1f},{click_y:.1f})"
                )
        else:
            sleep(1)
            pause_visible = bool(auto.find_element("battle/pause_assets.png", threshold=0.75))
            self.mouse_click_rate = not pause_visible
            log.debug(
                f"自动{mode_name}键盘结果确认: pause_visible={pause_visible} "
                f"mouse_fallback={self.mouse_click_rate}"
            )

        message = f"使用{mode_name}开始战斗（按键={'P+P+Enter' if use_damage_p else 'P+Enter'}）"
        log.info(message)
        return message

    def _battle_operation(
        self,
        first_turn: bool,
        defense_first_round: bool,
        avoid_skill_3: bool,
        prioritize_skill_3: bool = False,
        defense_for_solo_state: DefenseForSoloState | PseudoSoloDefenseState | None = None,
        defense_for_solo_used_this_turn: bool = False,
        use_damage_p: bool = False,
    ) -> bool:
        auto.mouse_click_blank()
        is_dynamic_pseudo_solo = defense_for_solo_state is not None and callable(
            getattr(defense_for_solo_state, "should_defend", None)
        )
        pseudo_solo_should_defend = False
        pseudo_solo_observation = None
        pseudo_solo_live_count = None
        if is_dynamic_pseudo_solo:
            pseudo_solo_should_defend = bool(defense_for_solo_state.should_defend())
            pseudo_solo_observation = getattr(defense_for_solo_state, "last_observation", None)
            pseudo_solo_live_count = getattr(defense_for_solo_state, "last_battle_live_count", None)
            # ``should_defend`` is the source of truth for the dynamic mode;
            # this flag only prevents submitting the same defense twice in one
            # turn, as the existing battle loop already does.
            pseudo_solo_should_defend = pseudo_solo_should_defend and not defense_for_solo_used_this_turn
        use_limited_defense = (
            defense_for_solo_state is not None
            and not is_dynamic_pseudo_solo
            and defense_for_solo_state.remaining_turns > 0
            and not defense_for_solo_used_this_turn
        )
        use_first_round_defense = (
            not is_dynamic_pseudo_solo and first_turn and defense_first_round and not defense_for_solo_used_this_turn
        )
        limited_defense_succeeded = False
        crops = _battle_ui_crops()
        auto_p_mode = "伤害P" if use_damage_p else "胜率P"
        defense_requested = pseudo_solo_should_defend or use_limited_defense or use_first_round_defense
        observation_name = getattr(pseudo_solo_observation, "value", pseudo_solo_observation)
        log.debug(
            "战斗操作判定: "
            f"dynamic_pseudo_solo={is_dynamic_pseudo_solo} observation={observation_name} "
            f"battle_live_count={pseudo_solo_live_count} should_defend={pseudo_solo_should_defend} "
            f"used_this_turn={defense_for_solo_used_this_turn} "
            f"limited_defense={use_limited_defense} first_round_defense={use_first_round_defense} "
            f"defense_requested={defense_requested}"
        )

        if defense_requested:
            if pseudo_solo_should_defend:
                msg = "小指良单通检测到多人存活，执行全员守备"
            elif use_limited_defense:
                msg = f"小指良单通连续防御（剩余{defense_for_solo_state.remaining_turns}回合），开始战斗"
            else:
                msg = "第一回合全员防御，开始战斗"
            if self._defense_this_round() is False:
                if pseudo_solo_should_defend or use_limited_defense:
                    msg = f"小指良单通连续防御失败，本回合改为{auto_p_mode}"
                else:
                    msg = f"第一回合全员防御失败，本场战斗改为{auto_p_mode}"
                self._start_auto_p(use_damage_p=use_damage_p, crops=crops)
            else:
                if use_limited_defense:
                    limited_defense_succeeded = True
                    defense_for_solo_state.consume_turn()
                    log.info(f"小指良单通连续防御已执行，剩余 {defense_for_solo_state.remaining_turns} 回合")
                    if defense_for_solo_state.remaining_turns == 0:
                        log.info("本次镜牢的连续防御已完成，后续回合恢复普通战斗操作")
                elif pseudo_solo_should_defend:
                    limited_defense_succeeded = True
                    consume_turn = getattr(defense_for_solo_state, "consume_turn", None)
                    if callable(consume_turn):
                        before_turns = getattr(defense_for_solo_state, "remaining_turns", None)
                        consume_turn()
                        after_turns = getattr(defense_for_solo_state, "remaining_turns", None)
                        log.info(
                            "小指良单通本场守备回合已执行: "
                            f"remaining_turns={before_turns}->{after_turns}"
                        )
            sleep(2)
            if not auto.find_element("battle/pause_assets.png", take_screenshot=True):
                self._start_auto_p(use_damage_p=use_damage_p, crops=crops)
        elif self.defense_all_time:
            if auto.find_element("battle/gear_left.png", threshold=DEFENSE_GEAR_THRESHOLD, my_crop=crops["gear_left"]):
                msg = "使用全员防御模式开始战斗"
                self._defense_this_round()
        elif (avoid_skill_3 or prioritize_skill_3) and auto.find_element(
            "battle/gear_left.png", threshold=DEFENSE_GEAR_THRESHOLD, my_crop=crops["gear_left"]
        ):
            use_prioritize_skill_3 = prioritize_skill_3 and not avoid_skill_3
            mode_name = "优先" if use_prioritize_skill_3 else "避免"
            msg = f"使用{mode_name}3技能模式开始战斗"
            if self._chain_battle(prioritize_skill_3=use_prioritize_skill_3) is False:
                msg = f"使用{mode_name}三技能的链接战失败，本场战斗改为{auto_p_mode}"
                self._start_auto_p(use_damage_p=use_damage_p, crops=crops)
            sleep(2)
            if not auto.find_element("battle/pause_assets.png", take_screenshot=True):
                self._start_auto_p(use_damage_p=use_damage_p, crops=crops)
        else:
            msg = self._start_auto_p(use_damage_p=use_damage_p, crops=crops)
        log.debug(msg)
        return limited_defense_succeeded

    @begin_and_finish_time_log(task_name="一次战斗")
    def fight(
        self,
        avoid_skill_3=False,
        defense_first_round=False,
        infinite_battle=False,
        defense_all_time=False,
        defense_on_turn1=False,
        choice_event_handling=True,
        combat_count=1,
        defense_for_solo_state: DefenseForSoloState | PseudoSoloDefenseState | None = None,
        prioritize_skill_3=False,
        use_damage_p=False,
    ):
        chance = self.INIT_CHANCE
        waiting = self._update_wait_time()
        total_count = 0
        fail_count = 0
        in_mirror = False
        first_battle_reward = None
        event_chance = 15
        if defense_all_time:
            self.defense_all_time = defense_all_time
        if defense_on_turn1:
            defense_first_round = True
            turn_ocr_bbox = ImageUtils.get_bbox(ImageUtils.load_image("battle/turn_ocr_assets.png"))

        begin_battle_observation = getattr(defense_for_solo_state, "begin_battle", None)
        if callable(begin_battle_observation):
            # A Battle.fight invocation owns one battle.  Reset the observer
            # here so a normal next battle gets one fresh count, while a task
            # restarted during an existing battle also gets exactly one first
            # observation when its command row becomes available.
            begin_battle_observation()

        first_turn = True
        defense_for_solo_used_this_turn = False
        start_time = time.time()

        def perform_battle_operation() -> None:
            nonlocal defense_for_solo_used_this_turn
            limited_defense_succeeded = self._battle_operation(
                first_turn=first_turn,
                defense_first_round=defense_first_round,
                avoid_skill_3=avoid_skill_3,
                prioritize_skill_3=prioritize_skill_3,
                defense_for_solo_state=defense_for_solo_state,
                defense_for_solo_used_this_turn=defense_for_solo_used_this_turn,
                use_damage_p=use_damage_p,
            )
            defense_for_solo_used_this_turn = defense_for_solo_used_this_turn or limited_defense_succeeded

        def find_reward_page() -> Optional[str]:
            """Return the visible mirror reward page before probing battle controls.

            ``battle/win_rate_assets.png`` is a very small, full-screen template.
            On the EGO gift page a static UI detail can match it, so reward-page
            detection must happen before any generic battle-operation trigger.
            """

            if auto.find_element("mirror/road_in_mir/select_encounter_reward_card_assets.png"):
                return "select_encounter_reward_card"
            if auto.find_element(
                "mirror/road_in_mir/acquire_ego_gift_card.png",
                my_crop=crops["acquire_gift_card"],
            ):
                return "acquire_ego_gift_card"
            return None

        self.fail_times = 0
        while self.running:
            from tasks.base.retry import check_times

            # 自动截图
            if auto.take_screenshot() is None:
                continue
            if auto.get_restore_time() is not None:
                start_time = max(start_time, auto.get_restore_time())
            if infinite_battle is False and check_times(start_time, timeout=900 + 300 * combat_count, logs=False):
                from tasks.base.back_init_menu import back_init_menu

                back_init_menu()
                return False

            total_count += 1

            if auto.find_element("mirror/road_in_mir/legend_assets.png"):
                if infinite_battle:
                    continue
                return False

            # 战斗开始前的加载
            if auto.find_element("base/waiting_assets.png"):
                sleep(0.5)
                continue

            # 判断是否为镜牢战斗
            crops = _battle_ui_crops()
            if in_mirror is False and auto.find_element("battle/in_mirror_assets.png", model="aggressive", my_crop=crops["in_mirror"]):
                in_mirror = True

            if view_status := auto.find_element("battle/view_status_assets.png", model="clam"):
                my_scale = cfg.set_win_size / 1440
                auto.mouse_click(view_status[0] + 100 * my_scale, view_status[1] - 500 * my_scale)
                continue

            # 如果正在交战过程
            if auto.find_element("battle/pause_assets.png"):
                sleep(min(2 * waiting, BATTLE_ANIMATION_POLL_MAX))  # 战斗播片中适度增大间隔
                chance = self.INIT_CHANCE
                first_turn = False
                defense_for_solo_used_this_turn = False
                continue

            # 战斗失败重启
            if auto.find_element("battle/dead_all.png", my_crop=crops["dead_all"]):
                dead_select = auto.find_element("battle/dead_all.png", find_type="image_with_multiple_targets", my_crop=crops["dead_all"])
                if len(dead_select) == 3:
                    dead_select = sorted(dead_select, key=lambda y: y[1])
                    auto.mouse_click(dead_select[1][0], dead_select[1][1])
                else:
                    confirm_button = auto.find_element("battle/dead_all_confirm_assets.png")
                    try:
                        my_scale = cfg.set_win_size / 1440
                        auto.mouse_click(
                            confirm_button[0] + 200 * my_scale,
                            confirm_button[1] - 350 * my_scale,
                        )
                    except Exception:
                        continue

                auto.click_element("battle/dead_all_confirm_assets.png")
                sleep(1)
                start_time = time.time()
                self.fail_times += 1
                defense_for_solo_used_this_turn = False
                if self.fail_times >= 5:
                    return False
                continue

            if in_mirror and not cfg.fight_to_last_man:
                if dead_position := auto.find_element("battle/dead.png", my_crop=crops["dead"]):
                    my_scale = cfg.set_win_size / 1440
                    dead_bbox = (
                        dead_position[0] - 100 * my_scale,
                        dead_position[1] - 30 * my_scale,
                        dead_position[0] + 100 * my_scale,
                        dead_position[1] + 30 * my_scale,
                    )
                    ocr_result = auto.find_language_text("阵亡", "dead", dead_bbox)
                    if ocr_result is not False:
                        while True:
                            auto.mouse_to_blank()
                            if auto.take_screenshot() is None:
                                continue
                            if auto.find_element("mirror/road_in_mir/legend_assets.png"):
                                return False
                            if auto.click_element("battle/give_up_assets.png"):
                                sleep(2)
                                return False
                            if auto.click_element("battle/setting_assets.png"):
                                continue

            # Reward/Gift pages must win over the loose win-rate template match
            # below.  Otherwise the stale battle loop can run pseudo-solo
            # recognition and press P repeatedly after combat has ended.
            if reward_page := find_reward_page():
                log.info(f"战斗已结束，检测到镜牢{reward_page}页面，停止人数识别并返回奖励流程")
                if infinite_battle:
                    continue
                break

            # 如果正在战斗待机界面
            # 更新回合数
            if infinite_battle and defense_on_turn1:
                try:
                    sc = ImageUtils.crop(np.array(auto.screenshot), turn_ocr_bbox)
                    result = ocr.run(sc)
                    ocr_result = [result.txts[i] for i in range(len(result.txts))]
                    ocr_result = "".join(ocr_result)
                    # 用正则匹配字符串里的数字
                    m = re.search(r"\d+", ocr_result)
                    if m:
                        self.cur_turn = int(m.group())
                    else:
                        self.cur_turn = -1
                    if self.cur_turn == 1:
                        first_turn = True
                except Exception:
                    self.cur_turn = -1  # 表示识别失败

            if fail_count >= 10 or self.identify_keyword_turn is False:
                # 如果多次识别不到战斗界面
                try:
                    turn_bbox = ImageUtils.get_bbox(ImageUtils.load_image("battle/turn_assets.png"))
                    sc = ImageUtils.crop(np.array(auto.screenshot), turn_bbox)
                    sc = cv2.inRange(sc, 50, 255)
                    result = ocr.run(sc)
                    ocr_result = [result.txts[i] for i in range(len(result.txts))]
                    ocr_result = "".join(ocr_result).lower()
                except Exception:
                    ocr_result = ""
                if "turn" in ocr_result:
                    perform_battle_operation()
                    chance = self.INIT_CHANCE
                    waiting = self._update_wait_time(waiting, False, total_count)
                    self.identify_keyword_turn = False
                    continue
            elif fail_count >= 5:
                if auto.click_element("battle/turn_assets.png") or auto.find_element("battle/win_rate_assets.png"):
                    perform_battle_operation()
                    chance = self.INIT_CHANCE
                    waiting = self._update_wait_time(waiting, False, total_count)
                    continue
            else:
                if auto.find_element("battle/more_information_assets.png") or auto.find_element(
                    "battle/win_rate_assets.png"
                ):
                    perform_battle_operation()
                    chance = self.INIT_CHANCE
                    waiting = self._update_wait_time(waiting, False, total_count)
                    continue
            if chance < 5:
                if not infinite_battle:
                    auto.mouse_to_blank()
                try:
                    turn_bbox = ImageUtils.get_bbox(ImageUtils.load_image("battle/turn_assets.png"))
                    sc = ImageUtils.crop(np.array(auto.screenshot), turn_bbox)
                    sc = cv2.inRange(sc, 50, 255)
                    result = ocr.run(sc)
                    ocr_result = [result.txts[i] for i in range(len(result.txts))]
                    ocr_result = "".join(ocr_result).lower()
                except Exception:
                    ocr_result = ""
                if (
                    "turn" in ocr_result
                    or auto.click_element("battle/turn_assets.png")
                    or auto.find_element("battle/win_rate_assets.png")
                    or auto.find_element("battle/win_rate_card.png", threshold=0.75, my_crop=crops["win_rate"])
                ):
                    perform_battle_operation()
                    chance = self.INIT_CHANCE
                    waiting = self._update_wait_time(waiting, False, total_count)
                    continue
            if chance == 1:
                if not infinite_battle:
                    auto.mouse_to_blank()
                if auto.find_language_text("胜率", "rate"):
                    perform_battle_operation()
                    chance = self.INIT_CHANCE
                    waiting = self._update_wait_time(waiting, False, total_count)
                    sleep(1)
                    if not auto.find_element("battle/pause_assets.png"):
                        self.mouse_click_rate = True
                    continue
            if self.mouse_click_rate:
                if auto.find_element("battle/win_rate_card.png", threshold=0.75, my_crop=crops["win_rate"]):
                    perform_battle_operation()
                    chance = self.INIT_CHANCE
                    waiting = self._update_wait_time(waiting, False, total_count)

            # 如果战斗中途出现事件
            if (
                choice_event_handling
                and auto.find_element("event/choices_assets.png")
                and auto.find_element("event/select_first_option_assets.png")
            ):
                if event_chance > 5:
                    auto.click_element("event/select_first_option_assets.png")
                    event_chance -= 1
                elif event_chance > 0:
                    auto.click_element(
                        "event/select_first_option_assets.png",
                        find_type="image_with_multiple_targets",
                    )
                    event_chance -= 1
                else:
                    auto.click_element(
                        "event/select_first_option_assets.png",
                        find_type="image_with_multiple_targets",
                    )
                    finishes_bbox = ImageUtils.get_bbox(ImageUtils.load_image("event/continue_assets.png"))
                    if auto.find_text_element(
                        [
                            "conti",
                            "proc",
                            "comme",
                            "choices",
                            "confirm",
                            "行判",
                            "始战",
                            "继续",
                        ],
                        finishes_bbox,
                    ):
                        auto.mouse_click(
                            (finishes_bbox[0] + finishes_bbox[2]) // 2,
                            (finishes_bbox[1] + finishes_bbox[3]) // 2,
                        )
                        if infinite_battle:
                            continue
                        break
                    else:
                        event_chance = -1

            if choice_event_handling and auto.find_element("event/perform_the_check_feature_assets.png"):
                event_handling.decision_event_handling()
            if choice_event_handling:
                if auto.click_element("event/continue_assets.png"):
                    continue
                if auto.click_element("event/proceed_assets.png"):
                    continue
                if auto.click_element("event/commence_assets.png"):
                    continue
                if auto.click_element("event/skip_assets.png", times=6):
                    continue
            if auto.find_element("mirror/road_in_mir/select_encounter_reward_card_assets.png"):
                if infinite_battle:
                    continue
                break
            if not self.is_tool:
                # 点击中心以跳过播报员播报加速结算动画
                random_number = random.randint(-10, 10)
                width = int(cfg.set_win_size * 16 / 9)
                height = cfg.set_win_size
                center_x = width // 2
                center_y = height // 2
                auto.mouse_click(center_x - random_number, center_y + random_number, times=1)
                sleep(0.15)

            # 战斗结束，进入结算页面
            if auto.click_element("battle/battle_finish_confirm_assets.png", click=False) or auto.find_element(
                "mirror/claim_reward/battle_statistics_assets.png"
            ):
                sleep(1)
                if auto.click_element("base/leave_up_assets.png"):
                    auto.click_element("base/leave_up_confirm_assets.png")
                    continue
                # 为某些人在副本战斗过程中启动脚本任务进行收尾
                if self.first_battle:
                    if (
                        auto.find_element("battle/clear_rewards_EXP_1_assets.png")
                        or auto.find_element("battle/clear_rewards_EXP_2_assets.png")
                        or auto.find_element("battle/clear_rewards_EXP_3_assets.png")
                    ):
                        first_battle_reward = "EXP"
                    if auto.find_element("battle/clear_rewards_thread_assets.png"):
                        first_battle_reward = "thread"
                auto.click_element("battle/battle_finish_confirm_assets.png")
                if infinite_battle:
                    continue
                break

            if auto.find_element("mirror/road_in_mir/legend_assets.png"):
                if infinite_battle:
                    continue
                break
            if auto.find_element("mirror/road_in_mir/acquire_ego_gift_card.png", my_crop=crops["acquire_gift_card"]):
                if infinite_battle:
                    continue
                break
            if chance <= (self.INIT_CHANCE // 2 + 1) and auto.find_element("teams/identify_assets.png"):
                if infinite_battle:
                    continue
                break

            # 如果交战过程误触，导致战斗暂停
            if auto.click_element("battle/continue_assets.png"):
                continue
                # 如果网络波动，需要点击重试
            # The current frame was already captured and no click happened on
            # this path, so let retry reuse it instead of taking a duplicate
            # screenshot before checking rare network dialogs.
            if retry(screenshot_ready=True) is False:
                return False

            chance -= 1
            sleep(min(waiting, BATTLE_MISS_WAIT_MAX))
            # 更新等待时间
            waiting = self._update_wait_time(waiting, True, total_count)
            # 统计失败次数
            fail_count += 1
            if chance < 0:
                if infinite_battle:
                    continue
                break

        self.defense_all_time = False

        if total_count == 0:
            match_success_rate = 100
        else:
            # 保留最多三位小数
            match_success_rate = (1 - fail_count / total_count) * 100
        msg = f"此次战斗匹配失败次数{fail_count} 匹配总次数{total_count} 匹配成功率{match_success_rate}%"
        log.debug(msg)
        if self.first_battle:
            return first_battle_reward
        else:
            return None

    @staticmethod
    def _calculate_skills_position(
        skill_positions: list[list],
        gear_left: tuple,
        skill_nums: int,
        custom_tune: Optional[Callable[[list, int, int, float], list]] = None,
    ) -> None:
        """计算划线的技能位置(也是守备的位置)
        专门写一个方便修改
        直接修改传入的`skill_positions`, 故不返回值
        """
        scale: float = cfg.set_win_size / 1440
        skill_size: float = 161 * scale
        x_offset: float = 220 - 4.5 * skill_nums
        y_offset: float = 250

        for index in range(skill_nums):
            pos = [
                gear_left[0] + x_offset * scale + skill_size * index,
                gear_left[1] + y_offset * scale,
            ]
            if custom_tune is not None:
                pos: list = custom_tune(pos, index, skill_nums, scale)
            skill_positions.append(pos)

    @staticmethod
    def _get_lower_row_skill_indexes(
        skill_3_indexes: set[int],
        skill_nums: int,
        prioritize_skill_3: bool,
    ) -> set[int]:
        """Return slots that should use the lower skill row.

        ``find_skill3`` identifies slots whose upper option is skill 3. Avoid mode
        switches those slots to the lower row; prioritize mode switches the rest.
        """
        valid_indexes = set(range(1, skill_nums + 1))
        skill_3_indexes = skill_3_indexes & valid_indexes
        if prioritize_skill_3:
            return valid_indexes - skill_3_indexes
        return skill_3_indexes

    @staticmethod
    def _chain_battle(prioritize_skill_3: bool = False) -> bool:
        try:
            scale = cfg.set_win_size / 1440

            gear_left = auto.find_element("battle/gear_left.png")
            gear_right = auto.find_element("battle/gear_right.png")
            if gear_left is None or gear_right is None:
                return False

            gear_1 = [gear_left[0] + 94 * scale, gear_left[1] - 37 * scale]
            gear_2 = [gear_right[0] - 100 * scale, gear_right[1]]

            bbox = (gear_1[0], gear_1[1] - 15 * scale, gear_2[0], gear_1[1])

            skill_nums = int((bbox[2] - bbox[0]) / (145 * scale))
            if skill_nums <= 0:
                return False

            if skill_nums >= 10:
                bbox = (bbox[0] + 50 * scale, bbox[1], bbox[2], bbox[3])

            sc = auto.get_screenshot_crop(bbox)

            skill_3_matches = []
            for sin_color in sins.values():
                skill_3_matches.extend(find_skill3(sc, sin_color))
            skill_3_indexes = {round(match[0] / (145 * scale)) for match in skill_3_matches}
            lower_row_indexes = Battle._get_lower_row_skill_indexes(
                skill_3_indexes,
                skill_nums,
                prioritize_skill_3,
            )

            skill_list = [gear_left]

            def custom_tune(
                pos: list,
                index: int,
                skill_nums: int,
                scale: float,
                *,
                reverse: int = 1,
            ) -> list:
                """针对上方的技能位置调整, 呈三角形偏移"""
                center = skill_nums // 2

                if index <= center:
                    pos[0] += (center - index) * 8 * scale * reverse
                else:
                    pos[0] -= (index - center) * 8 * scale * reverse
                return pos

            Battle._calculate_skills_position(
                skill_list,
                (gear_left[0], gear_left[1] - 125 * scale),
                skill_nums,
                custom_tune=custom_tune,
            )
            for index in sorted(lower_row_indexes):
                skill_list[index][1] += 125 * scale
                skill_list[index] = custom_tune(skill_list[index], index, skill_nums, scale, reverse=-1)

            skill_list.append([gear_right[0] + 5 * skill_nums * scale, gear_right[1] + 150 * scale])

            auto.mouse_drag_link(skill_list)

            auto.mouse_to_blank()

            auto.key_press("enter")

            sleep(1)
            return True
        except Exception:
            return False

    @staticmethod
    def _defense_this_round(move_back: bool = False) -> bool:
        try:
            scale = cfg.set_win_size / 1440

            crops = _battle_ui_crops()
            gear_left = auto.find_element("battle/gear_left.png", threshold=DEFENSE_GEAR_THRESHOLD, my_crop=crops["gear_left"])

            gear_1 = [gear_left[0] + 100 * scale, gear_left[1] - 35 * scale]
            gear_right = auto.find_element("battle/gear_right.png", threshold=DEFENSE_GEAR_THRESHOLD, my_crop=crops["gear_right"])
            gear_2 = [gear_right[0] - 100 * scale, gear_right[1]]

            bbox = (gear_1[0], gear_1[1] - 15 * scale, gear_2[0], gear_1[1])

            skill_nums = int((bbox[2] - bbox[0]) / (145 * scale))

            skill_list = []
            Battle._calculate_skills_position(skill_list, gear_left, skill_nums)

            for skill in skill_list:
                auto.mouse_click(skill[0], skill[1])
                if is_simulator_runtime():
                    sleep(cfg.mouse_action_interval)
                else:
                    sleep(cfg.mouse_action_interval // 1.5)

            skill_list.insert(0, gear_left)
            skill_list.append([gear_right[0] + 5 * skill_nums * scale, gear_right[1] + 150 * scale])

            auto.mouse_drag_link(skill_list)

            auto.mouse_to_blank(move_back=move_back)

            sleep(1)
            return True
        except Exception:
            return False
