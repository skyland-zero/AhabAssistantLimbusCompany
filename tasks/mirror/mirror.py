import time
from datetime import datetime

import cv2
import numpy as np

from core.events import mediator
from core.execution_control import check_cancelled, request_cancellation
from core.execution_control import interruptible_sleep as sleep
from core.pseudo_solo import BattleRosterObserver, PseudoSoloDefenseState
from core.team_squad import validate_pseudo_solo_selection
from module.automation import auto
from module.config import TeamSetting, cfg
from module.decorator.decorator import begin_and_finish_time_log
from module.logger import log
from module.mirror_plan_runtime import (
    MIRROR_SEGMENT_LENGTH,
    MirrorPlanProgressError,
    MirrorPlanRuntime,
    extract_mirror_floor,
    select_mirror_floor_count,
)
from module.mirror_routes import MirrorRouteDefinition, get_mirror_route
from module.my_error.my_error import (
    InputAttributeError,
    backMainWinError,
    cannotOperateGameError,
    unableToFindTeamError,
    userStopError,
)
from module.observe_ego_gift import normalize_observe_ego_gifts, resolve_observe_ego_gift
from module.ocr import ocr
from tasks import all_systems, observe_system, start_gift
from tasks.base.back_init_menu import back_init_menu
from tasks.base.make_enkephalin_module import make_enkephalin_module
from tasks.base.retry import retry
from tasks.battle import battle
from tasks.battle.battle import DefenseForSoloState
from tasks.event import event_handling
from tasks.mirror.in_shop import Shop
from tasks.mirror.reward_card import get_reward_card
from tasks.mirror.search_road import (
    MirrorMap,
    search_road_default_distance,
    search_road_farthest_distance,
    search_road_simple_keyboard,
)
from tasks.mirror.select_theme_pack import select_theme_pack
from tasks.teams.team_formation import check_team, load_team_code_in_game, select_battle_team, team_formation
from utils.image_utils import ImageUtils
from module.vision_profiler import vision_profiler

MIRROR_SINNER_IDS = (
    "yi_sang",
    "faust",
    "don_quixote",
    "ryoshu",
    "meursault",
    "hong_lu",
    "heathcliff",
    "ishmael",
    "rodion",
    "sinclair",
    "outis",
    "gregor",
)
MIRROR_SINNER_NAMES_ZH = (
    "李箱",
    "浮士德",
    "堂吉诃德",
    "良秀",
    "默尔索",
    "鸿璐",
    "希斯克利夫",
    "以实玛利",
    "罗佳",
    "辛克莱",
    "奥提斯",
    "格雷戈尔",
)
MIRROR_SINNER_NAMES_EN = (
    "Yi Sang",
    "Faust",
    "Don Quixote",
    "Ryoshu",
    "Meursault",
    "Hong Lu",
    "Heathcliff",
    "Ishmael",
    "Rodion",
    "Sinclair",
    "Outis",
    "Gregor",
)


# 输出时间统计
def to_log_with_time(msg, elapsed_time):
    # 将总秒数转换为小时、分钟和秒
    hours, remainder = divmod(elapsed_time, 3600)
    minutes, seconds = divmod(remainder, 60)
    time_string = f"{int(hours):02}:{int(minutes):02}:{int(seconds):02}"
    log.info(f"{msg} 总耗时:{time_string}")


class Mirror:
    def __init__(self, team_setting: TeamSetting, team_num: int):
        team_setting = team_setting.model_copy(deep=True)  # 避免修改原始配置
        if team_setting.defense_for_solo:
            validate_pseudo_solo_selection(team_setting.chosen_sinners)
        self.logger = log
        self.team_order = team_num
        self.sinner_team = team_setting.sinner_order  # 选择的罪人序列
        self.chosen_sinners = team_setting.chosen_sinners
        self.team_number = team_setting.team_number  # 选择的编队名
        configured_name = getattr(team_setting, "remark_name", None)
        self.team_name = (
            configured_name.strip()
            if isinstance(configured_name, str) and configured_name.strip()
            else f"编队 {self.team_order}"
        )
        self.team_sinners = self._ordered_team_sinners(self.chosen_sinners, self.sinner_team)
        self.team_code_loaded = False
        self.mirror_route: MirrorRouteDefinition = get_mirror_route(team_setting.mirror_route_profile)
        self.hard_switch = bool(cfg.hard_mirror)
        hard_target = getattr(cfg, "hard_mirror_target_floors", 5)
        selected_floor_count = select_mirror_floor_count(
            self.mirror_route.floor_counts, self.hard_switch, hard_target
        )
        self.plan_runtime = MirrorPlanRuntime(self.mirror_route.floor_counts, floor_count=selected_floor_count)
        log.info(
            f"镜牢路线{self.mirror_route.route_id or 'default'}任务模式={'困难' if self.hard_switch else '普通'}，"
            f"目标{selected_floor_count}层"
        )
        self.shop = Shop(team_setting)
        self.system = all_systems[team_setting.team_system]  # 选择的体系
        self.avoid_skill_3 = team_setting.avoid_skill_3  # 是否避免使用3技能
        self.prioritize_skill_3 = team_setting.prioritize_skill_3  # 是否优先使用3技能
        # 开局星光加成
        self.opening_bonus = team_setting.opening_bonus
        self.use_starlight = team_setting.use_starlight
        # 自选奖励卡优先度
        self.reward_cards = team_setting.reward_cards
        self.reward_cards_select = team_setting.reward_cards_select
        # 自选开局饰品
        self.opening_items = team_setting.opening_items
        self.opening_items_select = team_setting.opening_items_select
        self.opening_items_system = team_setting.opening_items_system
        self.re_formation_each_floor = team_setting.re_formation_each_floor  # 是否每层重新配队
        # 第二体系
        self.second_system = team_setting.second_system  # 启用第二体系
        self.second_system_select = team_setting.second_system_select  # 选择的第二体系
        self.second_system_setting = team_setting.second_system_setting  # 第二体系策略

        self.observe_ego_gift = team_setting.observe_ego_gift  # 是否需要观测EGO饰品
        self.observe_ego_gift_selected = normalize_observe_ego_gifts(
            team_setting.observe_ego_gift_selected
        )  # 用户选择的观测EGO饰品列表
        self.observe_ego_gift_done = False

        self.defense_first_round = team_setting.defense_first_round  # 是否第一回合全员防御
        self.defense_for_solo = team_setting.defense_for_solo  # 是否小指良单通连续防御
        if team_setting.defense_for_solo:
            base_defense_state = DefenseForSoloState(team_setting.defense_for_solo_turns)
            self.defense_for_solo_state = PseudoSoloDefenseState(
                base_defense_state,
                BattleRosterObserver(team_setting.chosen_sinners),
            )
        else:
            self.defense_for_solo_state = None

        self.start_time = time.time()
        self.first_battle = True  # 判断是否首次进入战斗，如果是则重新配队
        self.use_custom_theme_pack_weight = team_setting.use_custom_theme_pack_weight  # 是否启用自定义主题包权重
        # 统计时间
        self.find_road_total_time = 0
        self.battle_total_time = 0
        self.shop_total_time = 0
        self.event_total_time = 0
        self.event_times = 0
        self.theme_pack_total_time = 0
        self.reward_card_total_time = 0
        self.ego_gift_total_time = 0
        self.settlement_total_time = 0
        self.other_total_time = 0
        self.last_completion_stats: dict[str, object] | None = None

        self.floor = 0
        self.get_floor_num = True
        # Keep the legacy attribute as a compatibility view for integrations;
        # the route runtime owns its size and completion rule.
        self.floor_times = self.plan_runtime.floor_times
        self.LOOP_COUNT = 250

        self.mirror_map = MirrorMap(hard_mode=self.hard_switch)

        self.pass_coins = None

        self.bequest_from_the_previous_game = False
        self.resumed_from_existing_game = False
        self.parallel_mode_enable_attempts = 0
        self.parallel_mode_confirmed = False

    def _time_call(self, fn, *args, **kwargs):
        """调用 fn 并返回 (result, elapsed_time)，用于显式计时替代装饰器返回值。"""
        start = time.time()
        result = fn(*args, **kwargs)
        return result, time.time() - start

    @staticmethod
    def _ordered_team_sinners(chosen_sinners, sinner_order) -> list[dict[str, str]]:
        """Snapshot the configured team in the same order used by the UI."""

        selected_indexes = [
            index
            for index, selected in enumerate((chosen_sinners or [])[: len(MIRROR_SINNER_IDS)])
            if selected
        ]
        ordered_indexes: dict[int, int] = {}
        for index in selected_indexes:
            position = (sinner_order or [])[index] if index < len(sinner_order or []) else 0
            if (
                isinstance(position, int)
                and not isinstance(position, bool)
                and 1 <= position <= len(selected_indexes)
                and position not in ordered_indexes
            ):
                ordered_indexes[position] = index
        ordered = [ordered_indexes[position] for position in sorted(ordered_indexes)]
        ordered.extend(index for index in selected_indexes if index not in ordered)
        return [
            {
                "id": MIRROR_SINNER_IDS[index],
                "name": MIRROR_SINNER_NAMES_ZH[index],
                "nameEn": MIRROR_SINNER_NAMES_EN[index],
            }
            for index in ordered
        ]

    def _mirror_team_details(self) -> dict[str, object]:
        """Return an immutable-at-completion snapshot of the selected team."""

        return {
            "id": f"team-{self.team_order}",
            "name": self.team_name,
            "number": max(0, int(self.team_number)),
            "sinners": [sinner["id"] for sinner in self.team_sinners],
            "sinnerNames": [sinner["name"] for sinner in self.team_sinners],
            "sinnerNamesEn": [sinner["nameEn"] for sinner in self.team_sinners],
            "system": self.system,
            "accessoryScheme": self.system,
        }

    def _build_completion_stats(
        self,
        elapsed_time: float,
        *,
        failed: bool = False,
        failure_reason: str | None = None,
    ) -> dict[str, object]:
        """Build the persisted timing and run-context snapshot once per run."""

        other_time = max(
            0.0,
            elapsed_time
            - (
                self.battle_total_time
                + self.event_total_time
                + self.shop_total_time
                + self.find_road_total_time
                + self.theme_pack_total_time
                + self.reward_card_total_time
                + self.ego_gift_total_time
                + self.settlement_total_time
            ),
        )
        self.other_total_time = other_time
        details: dict[str, object] = {
            "completedAt": datetime.now().astimezone().isoformat(timespec="seconds"),
            "totalSeconds": max(0.0, elapsed_time),
            "battleSeconds": max(0.0, self.battle_total_time),
            "eventSeconds": max(0.0, self.event_total_time),
            "shopSeconds": max(0.0, self.shop_total_time),
            "findRoadSeconds": max(0.0, self.find_road_total_time),
            "themePackSeconds": max(0.0, self.theme_pack_total_time),
            "rewardCardSeconds": max(0.0, self.reward_card_total_time),
            "egoGiftSeconds": max(0.0, self.ego_gift_total_time),
            "settlementSeconds": max(0.0, self.settlement_total_time),
            "otherSeconds": max(0.0, other_time),
            "eventCount": max(0, int(self.event_times)),
            "team": self._mirror_team_details(),
            "hardMode": bool(self.hard_switch),
            "mode": "hard" if self.hard_switch else "normal",
            "floorCount": max(0, int(self.plan_runtime.floor_count or 0)),
            "routeId": self.mirror_route.route_id or "default",
            "routeName": self.mirror_route.name_zh,
            "routeNameEn": self.mirror_route.name_en,
            "failed": failed,
        }
        if failure_reason:
            details["failureReason"] = failure_reason
        return details

    def _fight(self) -> None:
        _, elapsed = self._time_call(
            battle.fight,
            avoid_skill_3=self.avoid_skill_3,
            prioritize_skill_3=self.prioritize_skill_3,
            defense_first_round=self.defense_first_round,
            defense_for_solo_state=self.defense_for_solo_state,
        )
        self.battle_total_time += elapsed

    def _form_team_for_battle(self) -> bool:
        """Apply the selected formation without overwriting a loaded team code."""

        if self.team_code_loaded:
            self.team_code_loaded = False
            log.info("编队码已加载，跳过手动编队")
            return True
        return team_formation(self.sinner_team, self.chosen_sinners)

    def _parallel_mode_bbox(self):
        bbox = ImageUtils.get_bbox(ImageUtils.load_image("mirror/road_to_mir/infinity_mirror_bbox.png"))
        return (
            bbox[2] - 70,
            bbox[1],
            bbox[2] + 100,
            bbox[3],
        )

    def _read_parallel_mode(self, *, take_screenshot: bool = False) -> bool | None:
        """Read the parallel-superposition switch from its small OCR region."""

        if take_screenshot:
            auto.take_screenshot()
        ocr_result = auto.find_text_element(None, self._parallel_mode_bbox(), only_text=True)
        if not ocr_result:
            return None
        normalized = "".join(str(text).casefold().replace(" ", "") for text in ocr_result)
        if "off" in normalized or "关闭" in normalized:
            return False
        if "on" in normalized or "开启" in normalized:
            return True
        return None

    def _read_absolute_floor(self) -> int | None:
        """Read the absolute 1-based floor title when entering/resuming a map."""

        floor_image = ImageUtils.load_image("mirror/road_in_mir/get_floor_bbox.png")
        if floor_image is None:
            log.warning("无法加载镜牢楼层标题识别区域")
            return None
        floor_bbox = ImageUtils.get_bbox(floor_image)
        # The tight non-zero bounding box makes RapidOCR split the title into
        # fragments and frequently drop the trailing floor number.  Keep a
        # small amount of surrounding UI background so the detector can see
        # the complete title line (the same title asset works reliably with
        # this padding at 1080p and 1440p).
        padding = max(12, int(round((floor_bbox[3] - floor_bbox[1]) * 0.5)))
        padded_floor_bbox = (
            floor_bbox[0] - padding,
            floor_bbox[1] - padding,
            floor_bbox[2] + padding,
            floor_bbox[3] + padding,
        )
        for attempt in range(3):
            screenshot = auto.take_screenshot(gray=False)
            if screenshot is None:
                continue
            crop = ImageUtils.crop(np.array(screenshot), padded_floor_bbox)
            candidates = (crop, cv2.bitwise_not(crop))
            for candidate in candidates:
                try:
                    result = ocr.run(candidate)
                except Exception as error:
                    check_cancelled()
                    log.debug(f"楼层标题OCR失败：{error}")
                    continue
                ocr_text = "".join(getattr(result, "txts", ()) or ())
                floor = extract_mirror_floor(ocr_text)
                if floor is not None:
                    log.debug(f"楼层标题OCR[{attempt + 1}]得到：{ocr_text}，绝对楼层：{floor}")
                    return floor
            if attempt < 2:
                sleep(0.3)
        return None

    def _handle_mirror_extension(self) -> bool:
        """Accept or reject the five-floor extension prompt for this run."""

        if not auto.find_element("mirror/infinity_mirror_assets.png"):
            return False

        if not self.plan_runtime.can_advance_segment:
            auto.click_element("mirror/infinity_mirror_close_assets.png")
            return True

        if self.plan_runtime.floor_count != 15:
            auto.click_element("mirror/infinity_mirror_close_assets.png")
            return True

        confirmed = auto.click_element(
            "mirror/road_to_mir/infinity_mirror_enter_assets.png",
            take_screenshot=True,
        )
        if not confirmed:
            confirmed = auto.click_element(
                "mirror/road_to_mir/enter_mirror_confirm.png",
                take_screenshot=True,
            )
        if not confirmed:
            message = "困难镜牢需要继续进入下一段，但未找到平行叠加确认按钮"
            self.plan_runtime.record_deviation(message)
            log.error(message)
            raise cannotOperateGameError(message)

        sleep(0.5)
        if auto.find_element("mirror/infinity_mirror_assets.png", take_screenshot=True):
            message = "困难镜牢平行叠加续层确认未生效，已停止本轮路线"
            self.plan_runtime.record_deviation(message)
            log.error(message)
            raise cannotOperateGameError(message)
        try:
            next_floor = self.plan_runtime.advance_segment()
        except MirrorPlanProgressError as error:
            self.plan_runtime.record_deviation(str(error))
            log.error(str(error))
            raise cannotOperateGameError(str(error)) from error
        self.get_floor_num = True
        log.info(f"困难镜牢已确认继续，下一段从实际第{next_floor + 1}层开始")
        return True

    def road_to_mir(self):
        loop_count = 30
        auto.model = "clam"
        self.first_battle = True
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue
            auto.mouse_to_blank()
            if retry() is False:
                return False
            if auto.find_element("home/first_prompt_assets.png", model="clam") and auto.find_element(
                "home/back_assets.png", model="normal"
            ):
                auto.click_element("home/back_assets.png")
                continue
            if auto.find_element("mirror/claim_reward/clear_assets.png"):
                self.bequest_from_the_previous_game = True
                return True
            if auto.find_element("mirror/shop/shop_coins_assets.png"):  # 防止卡死在商店
                self.resumed_from_existing_game = True
                break
            if auto.find_element("mirror/road_in_mir/legend_assets.png"):
                self.resumed_from_existing_game = True
                break
            if auto.click_element("mirror/road_to_mir/resume_assets.png"):
                self.resumed_from_existing_game = True
                break
            parallel_mode = self._read_parallel_mode()
            if self.plan_runtime.floor_count == 15 and not self.parallel_mode_confirmed:
                if parallel_mode is False:
                    if self.parallel_mode_enable_attempts >= 2:
                        message = "困难镜牢目标为15层，但无法自动开启平行叠加模式"
                        self.plan_runtime.record_deviation(message)
                        log.error(message)
                        raise cannotOperateGameError(message)
                    if auto.click_element("mirror/road_to_mir/enter_mirror_assets.png", threshold=0.78):
                        self.parallel_mode_enable_attempts += 1
                        sleep(0.5)
                        continue
                    message = "困难镜牢目标为15层，但未找到开启平行叠加模式的控件"
                    self.plan_runtime.record_deviation(message)
                    log.error(message)
                    raise cannotOperateGameError(message)
                if parallel_mode is True:
                    if auto.click_element("mirror/road_to_mir/infinity_mirror_enter_assets.png"):
                        self.parallel_mode_confirmed = True
                        sleep(0.5)
                        continue
                    self.parallel_mode_confirmed = True
                else:
                    if self.parallel_mode_enable_attempts < 2 and auto.click_element(
                        "mirror/road_to_mir/enter_mirror_assets.png", threshold=0.78
                    ):
                        self.parallel_mode_enable_attempts += 1
                        sleep(0.5)
                        continue
                    if auto.find_element("mirror/road_to_mir/enter_assets.png"):
                        message = "困难镜牢目标为15层，但无法确认平行叠加模式已开启"
                        self.plan_runtime.record_deviation(message)
                        log.error(message)
                        raise cannotOperateGameError(message)

            if self.plan_runtime.floor_count != 15:
                if auto.click_element("mirror/road_to_mir/enter_mirror_assets.png", threshold=0.78):
                    break

            if self.plan_runtime.floor_count != 15:
                infinity_bbox = self._parallel_mode_bbox()
                if not auto.find_text_element(["off", "ff"], infinity_bbox):
                    auto.click_element("mirror/road_to_mir/infinity_mirror_enter_assets.png")
            if self.plan_runtime.floor_count == 15 and not self.parallel_mode_confirmed:
                if auto.find_element("mirror/road_to_mir/enter_assets.png"):
                    message = "困难镜牢目标为15层，但无法确认平行叠加模式已开启"
                    self.plan_runtime.record_deviation(message)
                    log.error(message)
                    raise cannotOperateGameError(message)
            if auto.click_element("mirror/road_to_mir/enter_assets.png"):
                sleep(0.5)
                continue
            if auto.click_element("home/mirror_dungeons_assets.png"):
                continue
            if auto.find_element("home/inferno_bus_assets.png") and not auto.find_element(
                "home/mirror_dungeons_assets.png"
            ):
                sleep(1)
                if not auto.find_element("home/mirror_dungeons_assets.png"):
                    auto.click_element("home/window_assets.png")
                    continue
            if auto.find_element("base/renew_confirm_assets.png", model="clam") and auto.find_element(
                "home/drive_assets.png", model="normal"
            ):
                auto.click_element("base/renew_confirm_assets.png")
                back_init_menu()
                continue
            # bug已修复，从这里进可能会进轨道线，先注释掉
            # if auto.click_element("mirror/road_to_mir/quick_start_assets.png"):
            #     continue
            if auto.click_element("home/drive_assets.png", model="normal"):
                sleep(0.5)
                continue
            if auto.find_element("mirror/road_to_mir/select_team_stars_assets.png"):
                break
            if auto.find_element("mirror/road_to_mir/dreaming_star/coins_assets.png"):
                # 防止卡在星光选择
                break
            if auto.find_element("mirror/theme_pack/feature_theme_pack_assets.png"):
                # 防止卡死在主题包页面
                break
            loop_count -= 1
            if loop_count < 20:
                auto.model = "normal"
            if loop_count < 10:
                auto.model = "aggressive"
            if loop_count < 0:
                log.error("无法进入镜牢,尝试回到初始界面")
                back_init_menu()
                break

    def run(self):
        # 计时开始
        start_time = time.time()
        vision_profiler.reset()

        if auto.click_element("home/drive_assets.png") or auto.find_element("home/window_assets.png"):
            sleep(0.5)
            make_enkephalin_module()

        main_loop_count = self.LOOP_COUNT
        back_menu_count = 0
        # 未到达奖励页不会停止
        while True:
            check_cancelled()
            if main_loop_count >= 50:
                auto.model = "clam"  # 防止函数内修改后未还原
            # 自动截图
            if auto.take_screenshot() is None:
                continue

            retry(screenshot_ready=True)

            if cfg.floor_3_exit and self.floor >= 4:
                if auto.click_element("mirror/road_in_mir/towindow&forfeit_confirm_assets.png"):
                    break
                if auto.click_element("mirror/road_in_mir/forfeit_assets.png"):
                    continue
                if auto.click_element("mirror/road_in_mir/setting_assets.png"):
                    continue

            # 镜牢结束领取奖励
            if auto.find_element("mirror/claim_reward/battle_statistics_assets.png"):
                if auto.click_element("mirror/claim_reward/claim_rewards_assets.png") is False:
                    claim_rewards_bbox = ImageUtils.get_bbox(
                        ImageUtils.load_image("mirror/claim_reward/claim_rewards_assets.png")
                    )
                    auto.mouse_click(
                        (claim_rewards_bbox[0] + claim_rewards_bbox[2]) / 2,
                        (claim_rewards_bbox[1] + claim_rewards_bbox[3]) / 2,
                    )
                break
            if auto.find_element("mirror/claim_reward/claim_rewards_assets.png") and auto.find_element(
                "mirror/claim_reward/complete_mirror_100%_assets.png"
            ):
                break
            if auto.find_element(
                "mirror/claim_reward/use_enkephalin_assets.png",
                threshold=0.9,
                model="clam",
            ):
                break

            # 离开镜牢的设置页面
            if to_window_position := auto.find_element("mirror/road_in_mir/to_window_assets.png"):
                auto.mouse_click(to_window_position[0] - 200 * cfg.set_win_size / 1440, to_window_position[1])
                continue

            # 选择楼层主题包的情况
            if auto.find_element("mirror/theme_pack/feature_theme_pack_assets.png"):
                sleep(2)
                _, elapsed = self._time_call(
                    select_theme_pack,
                    self.hard_switch,
                    self.floor,
                    self.team_order,
                    self.use_custom_theme_pack_weight,
                    route=self.mirror_route,
                )
                self.theme_pack_total_time += elapsed
                if self.re_formation_each_floor:
                    self.first_battle = True
                try:
                    floor_num = self.floor
                    now = time.time()
                    previous_floor_start = self.plan_runtime.record_floor_start(floor_num, now)
                    if floor_num != 0:
                        if previous_floor_start is not None:
                            floor_time = now - previous_floor_start
                            msg = f"启动后第{self.floor}层卡包"
                        else:
                            floor_time = now - self.floor_times[0]
                            msg = f"启动后第{self.floor}层卡包，该楼层时间不完整"
                        to_log_with_time(msg, floor_time)
                        log.debug(vision_profiler.get_summary_text())
                except MirrorPlanProgressError as error:
                    self.plan_runtime.record_deviation(str(error))
                    log.info(f"楼层时间记录跳过：{error}")
                self.get_floor_num = True
                main_loop_count += 50
                continue

            # 遇到选择增益事件（少见）
            if auto.click_element("mirror/road_in_mir/event_effect_button.png", threshold=0.75):
                auto.click_element("mirror/road_in_mir/select_event_effect_confirm.png")
                continue

            # 在镜牢中寻路
            if auto.find_element("mirror/road_in_mir/legend_assets.png"):
                auto.mouse_to_blank()
                while auto.take_screenshot() is None:
                    continue
                if auto.click_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png"):
                    continue
                if auto.find_element("teams/identify_assets.png"):
                    continue
                if auto.find_element("mirror/shop/shop_coins_assets.png", model="normal"):
                    continue
                if auto.find_element("mirror/claim_reward/claim_rewards_assets.png") and auto.find_element(
                    "mirror/claim_reward/complete_mirror_100%_assets.png"
                ):
                    break
                retry(screenshot_ready=True)
                if self.get_floor_num:
                    self.get_which_floor()

                if cfg.floor_3_exit and self.floor >= 4:
                    continue

                while auto.take_screenshot() is None:
                    continue
                if auto.find_element("mirror/road_in_mir/legend_assets.png"):
                    _, elapsed = self._time_call(self.search_road)
                    self.find_road_total_time += elapsed
                continue

            # 进入节点
            if auto.click_element("mirror/road_in_mir/enter_assets.png"):
                continue

            # 选择镜牢队伍
            if auto.find_element("mirror/road_to_mir/select_team_stars_assets.png"):
                self.select_mirror_team()
                continue

            if battle.fail_times >= 5:
                battle.fail_times = 0
                self.re_start()
                continue

            # 战斗配队的情况
            if auto.find_element("teams/identify_assets.png"):
                if self.defense_for_solo_state is not None:
                    self.defense_for_solo_state.observe_team_page(self.floor)
                    self.shop.set_pseudo_solo_active(self.defense_for_solo_state.live_count == 1)
                # 如果第一次启动脚本，还没进行编队，就先编队
                if self.first_battle:
                    if self._form_team_for_battle():
                        self.first_battle = False
                    else:
                        log.warning("罪人编队失败，将在下一轮识别后重试")
                    continue
                # 战斗配队失败的情况
                if auto.click_element("teams/none_sinner_assets.png", model="clam"):
                    self.first_battle = True
                    continue
                # 如果未开启战斗直至全灭，则检测罪人幸存人数是否少于10人
                if (
                    not cfg.fight_to_last_man
                    and not self.defense_for_solo
                    and not (
                        auto.find_element("teams/12_sinner_live_assets.png")
                        or auto.find_element("teams/11_sinner_live_assets.png")
                        or auto.find_element("teams/10_sinner_live_assets.png")
                    )
                ):
                    continue_mirror = check_team()
                    # 如果还有至少5人能战斗就继续，不然就退出重开
                    if continue_mirror is False and self.first_battle is False:
                        self.re_start()
                if auto.click_element("battle/chaim_to_battle_assets.png") or auto.click_element(
                    "battle/normal_to_battle_assets.png"
                ):
                    retry()
                    continue

            # 没有配队的情况
            if auto.find_element("battle/select_none_assets.png"):
                auto.mouse_click_blank()
                self.first_battle = True
                continue

            # 在战斗中
            if (
                auto.find_element("battle/more_information_assets.png")
                or auto.find_element("battle/in_mirror_assets.png")
                or auto.find_element("battle/turn_assets.png")
                or (auto.find_element("battle/win_rate_card.png") and auto.find_element("battle/gear_right.png"))
            ):
                self._fight()
                continue
            elif main_loop_count < 10:
                # 连续未能识别界面（濒临超时卡死）时的 OCR 终极兜底，保证极端情况防卡死
                turn_bbox = ImageUtils.get_bbox(ImageUtils.load_image("battle/turn_assets.png"))
                turn_ocr_result = auto.find_text_element("turn", turn_bbox)
                if turn_ocr_result is not False:
                    self._fight()
                    continue

            # 镜牢星光
            if auto.find_element("mirror/road_to_mir/dreaming_star/coins_assets.png", threshold=0.9):
                self.enter_mir_with_star()
                continue

            # 如果遇到选择ego饰品的情况
            if auto.find_element("mirror/road_in_mir/acquire_ego_gift_card.png"):
                _, elapsed = self._time_call(self.acquire_ego_gift)
                self.ego_gift_total_time += elapsed
                continue
            if (
                main_loop_count < 50
                and auto.find_element("mirror/road_in_mir/acquire_ego_gift_box_assets.png", model="clam")
                and auto.find_element(
                    "mirror/road_in_mir/acquire_ego_gift_refuse_assets.png",
                    model="clam",
                )
            ):
                _, elapsed = self._time_call(self.acquire_ego_gift, type=2)
                self.ego_gift_total_time += elapsed
                continue
            if main_loop_count < 30 and auto.find_language_text("拒绝饰品", "refuse"):
                _, elapsed = self._time_call(self.acquire_ego_gift, type=2)
                self.ego_gift_total_time += elapsed
                continue

            # 如果遇到获取ego饰品的情况
            if auto.click_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png"):
                continue

            # 遇到事件
            if auto.click_element("event/skip_assets.png", times=6):
                self.event_handling()
                continue

            # 商店事件
            if auto.find_element("mirror/shop/shop_coins_assets.png"):
                _, elapsed = self._time_call(self.in_shop)
                self.shop_total_time += elapsed
                continue

            # 选择奖励卡
            if auto.find_element("mirror/road_in_mir/select_encounter_reward_card_assets.png"):
                if self.reward_cards:
                    _, elapsed = self._time_call(get_reward_card, self.reward_cards_select)
                else:
                    _, elapsed = self._time_call(get_reward_card)
                self.reward_card_total_time += elapsed
                continue

            # 在主界面时，开始进入镜牢
            if auto.click_element("home/drive_assets.png") or auto.find_element("home/window_assets.png"):
                sleep(0.5)
                if self.road_to_mir() and self.bequest_from_the_previous_game:
                    break
                continue
            # 在镜牢界面，进入镜牢
            if auto.click_element("mirror/road_to_mir/enter_assets.png"):
                if self.road_to_mir() and self.bequest_from_the_previous_game:
                    break
                continue

            # 初始饰品选择
            if auto.find_element("mirror/road_to_mir/activate_gift_search_on_assets.png") or auto.find_element(
                "mirror/road_to_mir/activate_gift_search_off_assets.png"
            ):
                self.select_init_ego_gift()
                continue

            if not self.observe_ego_gift_done and (
                auto.find_element("mirror/road_to_mir/observe_ego_gift/observe_bleed_assets.png", model="clam")
                or auto.find_element("mirror/road_to_mir/observe_ego_gift/observe_burn_assets.png", model="clam")
            ):
                self.select_observe_ego_gift()
                self.observe_ego_gift_done = True
                continue

            # 5层结束后，普通流程关闭续层，困难15层流程自动确认续层。
            if self._handle_mirror_extension():
                continue

            if auto.find_element("home/first_prompt_assets.png", model="clam") and auto.find_element(
                "home/back_assets.png", model="normal"
            ):
                auto.click_element("home/back_assets.png")
                continue

            # 防卡死
            auto.mouse_click_blank()
            retry()
            main_loop_count -= 1
            if main_loop_count % 10 == 0:
                log.debug(f"镜牢道中识别次数剩余{main_loop_count}次")
            if main_loop_count < 75:
                auto.model = "normal"
                auto.mouse_to_blank(move_back=False)
                log.debug("识别模式切换到正常模式")
            if main_loop_count < 15:
                auto.model = "aggressive"
                log.debug("识别模式切换到激进模式，警告，道中识别可能会出错")
            if main_loop_count < 0:
                if back_menu_count > 5:
                    raise backMainWinError("镜牢道中出错,请手动操作重试")
                log.error("镜牢道中识别失败次数达到最大值,正在返回主界面")
                back_init_menu()
                back_menu_count += 1
                main_loop_count = 250

        msg = "开始进行镜牢奖励领取"
        log.info(msg)

        if self.bequest_from_the_previous_game:
            settlement_start = time.time()
            self.get_reward_in_road()
            self.settlement_total_time = time.time() - settlement_start
            self.last_completion_stats = self._build_completion_stats(time.time() - start_time)
            return True

        settlement_start = time.time()
        auto.model = "clam"
        failed = None
        settlement_attempt = 0
        MAX_SETTLEMENT_ATTEMPTS = 5
        BASE_BACKOFF = 1.5
        settlement_success = False
        # 用于稳健判断是否真正100%：连续2次未找到才视为非100%，避免首帧过渡误判
        not_found_100_count = 0
        while settlement_attempt < MAX_SETTLEMENT_ATTEMPTS:
            check_cancelled()
            # 自动截图
            if auto.take_screenshot() is None:
                auto.mouse_to_blank()
                continue
            has_100 = auto.find_element(
                "mirror/claim_reward/complete_mirror_100%_assets.png"
            ) or auto.find_element("mirror/claim_reward/clear_assets.png")
            if has_100:
                failed = False
                not_found_100_count = 0
                log.debug("镜牢完成度100%，能够正常领取奖励")
            elif failed is None and cfg.floor_3_exit is False:
                not_found_100_count += 1
                # 连续2次截图都未找到100%才标记为未完成，避免单帧误判导致误走forfeit流程
                if not_found_100_count >= 2:
                    failed = True
            # 如果回到主界面，退出循环（兼容多种主界面标识）
            if auto.find_element("home/drive_assets.png") or auto.find_element(
                "home/window_assets.png"
            ) or auto.find_element("home/drive_words_assets.png"):
                settlement_success = True
                break
            if auto.click_element("battle/battle_finish_confirm_assets.png"):
                continue
            if auto.click_element("mirror/claim_reward/rewards_acquired_assets.png"):
                continue
            if auto.click_element(
                "mirror/claim_reward/claim_rewards_confirm_assets.png",
                threshold=0.75,
                model="clam",
                take_screenshot=True,
            ):
                continue
            if failed:
                auto.mouse_click_blank()
                sleep(0.5)
                complete_mirror_bbox = ImageUtils.get_bbox(
                    ImageUtils.load_image("mirror/claim_reward/complete_mirror_100%_assets.png")
                )
                if auto.find_text_element("100", complete_mirror_bbox):
                    failed = False
                    continue
                if auto.click_element("mirror/claim_reward/claim_rewards_assets.png"):
                    sleep(1)
                if auto.click_element(
                    "mirror/claim_reward/claim_forfeit_assets.png", model="normal", take_screenshot=True
                ):
                    continue
            else:
                if self.hard_switch and cfg.save_rewards:
                    auto.click_element("mirror/claim_reward/claim_rewards_assets.png")
                    sleep(1)
                    pos = auto.find_element(
                        "mirror/claim_reward/use_enkephalin_assets.png",
                        take_screenshot=True,
                    )
                    if pos:
                        auto.mouse_click(pos[0] - 300 * (cfg.set_win_size / 1440), pos[1])
                        sleep(1)
                    continue
                elif auto.click_element("mirror/claim_reward/claim_rewards_assets.png"):
                    sleep(1)
                    if cfg.no_weekly_bonuses:
                        bonuses = auto.find_element(
                            "mirror/claim_reward/weekly_bonuses.png",
                            find_type="image_with_multiple_targets",
                            take_screenshot=True,
                        )
                        if len(bonuses) >= 1:
                            for _ in range(len(bonuses)):
                                position = bonuses.pop(-1)
                                auto.mouse_click(position[0], position[1])
                    if cfg.hard_mirror_single_bonuses:
                        log.debug("开启了困牢单次领取奖励，如果存在多次奖励，则将单次领取")
                        sleep(1)
                        bonuses = auto.find_element(
                            "mirror/claim_reward/weekly_bonuses.png",
                            find_type="image_with_multiple_targets",
                            take_screenshot=True,
                        )
                        bonuses = sorted(bonuses, key=lambda x: x[0])
                        if len(bonuses) > 1:
                            for _ in range(len(bonuses) - 1):
                                position = bonuses.pop(-1)
                                auto.mouse_click(position[0], position[1])

                    auto.take_screenshot()
                    coins_bbox = ImageUtils.get_bbox(ImageUtils.load_image("mirror/claim_reward/coins_bbox.png"))
                    for _ in range(5):
                        try:
                            sc = ImageUtils.crop(np.array(auto.screenshot), coins_bbox)
                            result = ocr.run(sc)
                            ocr_result = [result.txts[i] for i in range(len(result.txts))]
                            ocr_result = "".join(ocr_result)
                            ocr_result = ocr_result.lower()
                            if "x" in ocr_result:
                                ocr_result = ocr_result.split("x")
                                self.pass_coins = int(ocr_result[-1])
                                break
                        except Exception:
                            check_cancelled()
                            continue
                    if self.pass_coins is None:
                        for _ in range(5):
                            try:
                                scale = cfg.set_win_size / 1440
                                if coins_pos := auto.find_element("mirror/claim_reward/coins.png"):
                                    coins_bbox = [
                                        coins_pos[0],
                                        coins_pos[1] - 40 * scale,
                                        coins_pos[0] + 100 * scale,
                                        coins_pos[1] + 40 * scale,
                                    ]
                                    sc = ImageUtils.crop(np.array(auto.screenshot), coins_bbox)
                                    result = ocr.run(sc)
                                    ocr_result = [result.txts[i] for i in range(len(result.txts))]
                                    ocr_result = "".join(ocr_result)
                                    ocr_result = ocr_result.lower()
                                    if "x" in ocr_result:
                                        ocr_result = ocr_result.split("x")
                                        self.pass_coins = int(ocr_result[-1])
                                        break
                            except Exception:
                                check_cancelled()
                                continue
                    if self.pass_coins:
                        msg = f"本次镜牢领取{self.pass_coins}个通行证经验"
                        log.info(msg)
                    else:
                        msg = "无法识别通行证经验数量，可能是UI发生变化"
                        log.warning(msg)
                    if auto.click_element(
                        "mirror/claim_reward/use_enkephalin_assets.png",
                        take_screenshot=True,
                    ):
                        sleep(1)
                    retry()
                    continue
            if auto.click_element("mirror/claim_reward/use_enkephalin_assets.png", threshold=0.75):  # 降低识别阈值
                sleep(1)
                continue
            # 处理周年活动弹出的窗口
            if auto.click_element("home/close_anniversary_event_assets.png"):
                continue
            retry()
            settlement_attempt += 1
            if settlement_attempt < MAX_SETTLEMENT_ATTEMPTS:
                delay = BASE_BACKOFF * (2 ** (settlement_attempt - 1))
                # 5次上限时延迟序列 1.5/3/6/12/24，上限12s避免过长
                delay = min(delay, 12.0)
                log.warning(f"结算重试 {settlement_attempt}/{MAX_SETTLEMENT_ATTEMPTS} 失败，{delay:.1f}s后指数退避重试")
                if settlement_attempt == 1:
                    auto.model = "normal"
                    auto.mouse_to_blank(move_back=False)
                elif settlement_attempt == 2:
                    auto.model = "aggressive"
                sleep(delay)
            log.debug(f"镜牢奖励结算剩余尝试次数{MAX_SETTLEMENT_ATTEMPTS - settlement_attempt}次")
        self.settlement_total_time = time.time() - settlement_start
        # 纠正误判：已回到主界面但 failed 仍为 True 时，若已成功读取通行证经验或再次检测到100%，则视为成功
        if settlement_success and failed:
            if self.pass_coins is not None:
                failed = False
                log.debug("结算已回到主界面且已读取通行证经验，撤销失败标记")
            else:
                try:
                    complete_mirror_bbox = ImageUtils.get_bbox(
                        ImageUtils.load_image("mirror/claim_reward/complete_mirror_100%_assets.png")
                    )
                    if auto.find_text_element("100", complete_mirror_bbox):
                        failed = False
                        log.debug("结算已回到主界面且检测到100%文字，撤销失败标记")
                except Exception:
                    pass
        if not settlement_success:
            # 3次仍未回到主界面，视为领取超时
            end_time = time.time()
            elapsed_time = end_time - start_time
            self.last_completion_stats = self._build_completion_stats(
                elapsed_time,
                failed=True,
                failure_reason="settlement_timeout",
            )
            other_time = self.other_total_time
            # 输出全部时间统计
            for msg, t in [
                ("此次镜牢在战斗", self.battle_total_time),
                ("此次镜牢在事件", self.event_total_time),
                ("此次镜牢中在商店", self.shop_total_time),
                ("此次镜牢中在寻路", self.find_road_total_time),
                ("此次镜牢在主题包", self.theme_pack_total_time),
                ("此次镜牢在奖励卡", self.reward_card_total_time),
                ("此次镜牢在饰品选择", self.ego_gift_total_time),
                ("此次镜牢在结算", self.settlement_total_time),
                ("此次镜牢其他耗时", other_time),
            ]:
                to_log_with_time(msg, t)
            log.debug(f"战斗时间:{self.battle_total_time} 事件时间:{self.event_total_time} 商店时间:{self.shop_total_time} 寻路时间:{self.find_road_total_time} 主题包时间:{self.theme_pack_total_time} 奖励卡时间:{self.reward_card_total_time} 饰品时间:{self.ego_gift_total_time} 结算时间:{self.settlement_total_time} 其他时间:{other_time} 总时间:{elapsed_time}")
            to_log_with_time(f"此次镜牢使用{self.system}体系队伍", elapsed_time)
            log.error(f"奖励领取失败：已重试{MAX_SETTLEMENT_ATTEMPTS}次(1.5s/3s/6s/12s)仍未回到主界面，大概率饼不足，已自动停止任务")
            try:
                mediator.task_completed.emit("mirror", 1, dict(self.last_completion_stats))
            except Exception:
                pass
            request_cancellation()
            raise userStopError(f"镜牢结算领取超时，已自动停止任务（已重试{MAX_SETTLEMENT_ATTEMPTS}次 1.5s/3s/6s/12s）")
        if failed:
            # 非超时但判定为未完成（floor_3_exit 等）仍按原逻辑返回
            end_time = time.time()
            elapsed_time = end_time - start_time
            self.last_completion_stats = self._build_completion_stats(elapsed_time, failed=True)
            try:
                mediator.task_completed.emit("mirror", 1, dict(self.last_completion_stats))
            except Exception:
                pass
            return False
        # 计时结束 - 成功路径，补齐所有时间
        end_time = time.time()
        elapsed_time = end_time - start_time
        # settlement已在上面计时，other为补齐项
        self.last_completion_stats = self._build_completion_stats(elapsed_time)
        other_time = self.other_total_time

        if self.plan_runtime.complete:
            team = cfg.config.teams.get(f"{self.team_order}")
            if team:
                team_history = {
                    "total_mirror_time_hard": team.total_mirror_time_hard,
                    "mirror_hard_count": team.mirror_hard_count,
                    "total_mirror_time_normal": team.total_mirror_time_normal,
                    "mirror_normal_count": team.mirror_normal_count,
                }
            else:
                team_history = {}

            def calculate_time(time_list, elapsed_time, count):
                """计算新的平均时间"""
                total_avr = time_list[0] if len(time_list) > 0 else 0
                last_five = time_list[1] if len(time_list) > 1 else 0
                last_ten = time_list[2] if len(time_list) > 2 else 0

                total_time = total_avr * count + elapsed_time
                total_five = last_five * min(count, 4) + elapsed_time
                total_ten = last_ten * min(count, 9) + elapsed_time

                total_avr = total_time / (count + 1)
                last_five = total_five / min(count + 1, 5)
                last_ten = total_ten / min(count + 1, 10)
                return [total_avr, last_five, last_ten]

            if self.hard_switch:
                team_total_battle_time_hard = team_history.get("total_mirror_time_hard", [0.0, 0.0, 0.0])
                team_total_battle_count = team_history.get("mirror_hard_count", 0)
                team_history["total_mirror_time_hard"] = calculate_time(
                    team_total_battle_time_hard, elapsed_time, team_total_battle_count
                )
                team_total_battle_count += 1
                team_history["mirror_hard_count"] = team_total_battle_count
            else:
                team_total_battle_time_normal = team_history.get("total_mirror_time_normal", [0.0, 0.0, 0.0])
                team_total_battle_count = team_history.get("mirror_normal_count", 0)
                team_history["total_mirror_time_normal"] = calculate_time(
                    team_total_battle_time_normal, elapsed_time, team_total_battle_count
                )
                team_total_battle_count += 1
                team_history["mirror_normal_count"] = team_total_battle_count
            if team:
                team.total_mirror_time_hard = team_history.get("total_mirror_time_hard", [0.0, 0.0, 0.0])
                team.mirror_hard_count = team_history.get("mirror_hard_count", 0)
                team.total_mirror_time_normal = team_history.get("total_mirror_time_normal", [0.0, 0.0, 0.0])
                team.mirror_normal_count = team_history.get("mirror_normal_count", 0)
            else:
                log.warning(f"无法找到编队{self.team_number}的历史记录，无法更新数据")
            log.debug(team_history)

        last_floor_start = self.plan_runtime.last_floor_start(self.floor)
        if last_floor_start is not None:
            last_floor_time = time.time() - last_floor_start
            msg = f"启动后第{self.floor}层卡包"
            to_log_with_time(msg, last_floor_time)
        else:
            log.info("楼层异常，可能是OCR识别错误，本轮镜牢层间的时间记录无效")

        # 输出战斗总时间
        msg = "此次镜牢在战斗"
        to_log_with_time(msg, self.battle_total_time)

        # 输出事件总时间
        msg = f"此次镜牢走的事件次数{self.event_times}"
        log.info(msg)
        # 输出事件总时间
        msg = "此次镜牢在事件"
        to_log_with_time(msg, self.event_total_time)

        # 输出商店总时间
        msg = "此次镜牢中在商店"
        to_log_with_time(msg, self.shop_total_time)

        # 输出寻路总时间
        msg = "此次镜牢中在寻路"
        to_log_with_time(msg, self.find_road_total_time)

        # 新增细项
        to_log_with_time("此次镜牢在主题包", self.theme_pack_total_time)
        to_log_with_time("此次镜牢在奖励卡", self.reward_card_total_time)
        to_log_with_time("此次镜牢在饰品选择", self.ego_gift_total_time)
        to_log_with_time("此次镜牢在结算", self.settlement_total_time)
        to_log_with_time("此次镜牢其他耗时", self.other_total_time)

        # debug输出时间
        log.debug(
            f"战斗时间:{self.battle_total_time} 事件时间:{self.event_total_time} 商店时间:{self.shop_total_time} 寻路时间:{self.find_road_total_time} 主题包时间:{self.theme_pack_total_time} 奖励卡时间:{self.reward_card_total_time} 饰品时间:{self.ego_gift_total_time} 结算时间:{self.settlement_total_time} 其他时间:{self.other_total_time} 总时间:{elapsed_time}"
        )

        # 输出镜牢总时间
        msg = f"此次镜牢使用{self.system}体系队伍"
        to_log_with_time(msg, elapsed_time)
        vision_profiler.log_summary()

        return True

    def enter_mir_with_star(self):
        coins = auto.find_element("mirror/road_to_mir/dreaming_star/coins_assets.png", threshold=0.9)
        scale = cfg.set_win_size / 1440
        first_starlight = [coins[0] - 1800 * scale, coins[1] + 300 * scale]
        starlights_X = [first_starlight[0] + (i % 5) * 400 * scale for i in range(10)]
        starlights_Y = [first_starlight[1] + (i // 5) * 480 * scale for i in range(10)]

        loop_count = 30
        auto.model = "clam"
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue

            if auto.find_element("mirror/road_to_mir/bleed_gift_assets.png"):
                break

            if self.use_starlight and auto.click_element(
                "mirror/road_to_mir/dreaming_star/no_convert_star_to_cost_assets.png"
            ):
                continue
            if not self.use_starlight and auto.click_element(
                "mirror/road_to_mir/dreaming_star/convert_star_to_cost_assets.png"
            ):
                continue

            if auto.click_element(
                "mirror/road_to_mir/dreaming_star/select_star_confirm_assets.png",
                model="normal",
            ):
                break

            bonus_num = len(self.opening_bonus)
            level_count = self.opening_bonus.count(1)
            level_plus_count = self.opening_bonus.count(2)
            level_plusplus_count = self.opening_bonus.count(3)
            if level_count == bonus_num:
                auto.click_element("mirror/road_to_mir/select_all_stars_assets.png")
            elif level_plus_count == bonus_num:
                auto.click_element("mirror/road_to_mir/select_all_stars_assets.png")
                auto.click_element("mirror/road_to_mir/dreaming_star/level_one_bonus_assets.png")
            elif level_plusplus_count == bonus_num:
                auto.click_element("mirror/road_to_mir/select_all_stars_assets.png")
                auto.click_element("mirror/road_to_mir/dreaming_star/level_two_bonus_assets.png")
            else:
                for i in range(bonus_num):
                    if self.opening_bonus[i] >= 1:
                        auto.mouse_action_with_pos((starlights_X[i], starlights_Y[i]))
                    if self.opening_bonus[i] == 2:
                        auto.mouse_action_with_pos(
                            (starlights_X[i] - 80 * scale, starlights_Y[i] + 320 * scale),
                        )
                    elif self.opening_bonus[i] == 3:
                        auto.mouse_action_with_pos(
                            (starlights_X[i] + 80 * scale, starlights_Y[i] + 320 * scale),
                        )

            if auto.click_element("mirror/road_to_mir/dreaming_star/dreaming_star_enter_assets.png"):
                sleep(0.5)
                continue

            if retry() is False:
                return False
            loop_count -= 1
            if loop_count % 5 == 0:
                log.debug(f"进入镜牢识别次数剩余{loop_count}次")
            if loop_count < 20:
                auto.model = "normal"
                auto.mouse_to_blank(move_back=False)
                log.debug("识别模式切换到正常模式")
            if loop_count < 10:
                auto.model = "aggressive"
                log.debug("识别模式切换到激进模式")
            if loop_count < 0:
                raise cannotOperateGameError("无法进入镜牢，不能进行下一步,请手动操作重试")

    def select_init_ego_gift(self):
        scroll = False
        select_system = False
        loop_count = 30
        auto.model = "clam"

        team_system = self.system
        if self.opening_items:
            team_system = all_systems[self.opening_items_system]
        log.debug("开始选择初始EGO")
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue

            if auto.click_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png"):
                auto.mouse_to_blank()
                continue

            # 如果未启用观测或启用了但未选择饰品，关闭观测饰品按钮
            if not self.observe_ego_gift or len(self.observe_ego_gift_selected) == 0:
                auto.click_element("mirror/road_to_mir/activate_gift_search_on_assets.png")
            # 如果已经进入观测饰品页面,则跳过初始EGO选择
            if (
                auto.find_element("mirror/theme_pack/feature_theme_pack_assets.png")
                or auto.find_element("mirror/road_to_mir/observe_ego_gift/observe_bleed_assets.png")
                or auto.find_element("mirror/road_to_mir/observe_ego_gift/observe_burn_assets.png")
            ):
                break

            if (team_system == "slash" or team_system == "pierce" or team_system == "blunt") and not scroll:
                while slash_button := auto.find_element("mirror/road_to_mir/slash_gift_1.png"):
                    auto.mouse_drag(slash_button[0], slash_button[1], drag_time=0.2, dx=0, dy=-400)
                    sleep(0.5)
                    if auto.find_element(
                        "mirror/road_to_mir/blunt_gift_1_assets.png",
                        take_screenshot=True,
                    ):
                        scroll = True
                        break

            if auto.click_element(f"mirror/road_to_mir/{team_system}_gift_assets.png") and not select_system:
                select_system = True
                continue

            if self.opening_items:
                start_gift_order = start_gift[self.opening_items_select]
                auto.click_element(
                    f"mirror/road_to_mir/select_init_gift/{team_system}_ego_gift_{start_gift_order[0]}.png"
                )
                auto.click_element(
                    f"mirror/road_to_mir/select_init_gift/{team_system}_ego_gift_{start_gift_order[1]}.png"
                )
                auto.click_element(
                    f"mirror/road_to_mir/select_init_gift/{team_system}_ego_gift_{start_gift_order[2]}.png"
                )
            else:
                auto.click_element(f"mirror/road_to_mir/select_init_gift/{team_system}_ego_gift_1.png")
                auto.click_element(f"mirror/road_to_mir/select_init_gift/{team_system}_ego_gift_2.png")
                auto.click_element(f"mirror/road_to_mir/select_init_gift/{team_system}_ego_gift_3.png")

            # 如果选择观测饰品且选择了饰品，则开启观测饰品按钮
            if self.observe_ego_gift and self.observe_ego_gift_selected:
                auto.click_element("mirror/road_to_mir/activate_gift_search_off_assets.png")

            if auto.click_element("mirror/road_to_mir/select_init_ego_gifts_confirm_assets.png"):
                sleep(1)
                continue

            if retry() is False:
                return False
            loop_count -= 1
            if loop_count % 5 == 0:
                log.debug(f"选择藏品识别次数剩余{loop_count}次")
            if loop_count < 20:
                auto.model = "normal"
                auto.mouse_to_blank(move_back=False)
                log.debug("识别模式切换到正常模式")
            if loop_count < 10:
                auto.model = "aggressive"
                log.debug("识别模式切换到激进模式")
            if loop_count < 0:
                log.error("无法进入镜牢,尝试回到初始界面")
                back_init_menu()
                break

    def select_observe_ego_gift(self):
        """
        观测EGO饰品选择
        """

        has_scrolled = False

        def _select_gift(level_p, gift_row, gift_col):
            nonlocal has_scrolled
            first_gift = (level_p[0], level_p[1] + 80 * my_scale)
            select_gift_point = (
                first_gift[0] + (gift_col - 1) * 165 * my_scale,
                first_gift[1] + (gift_row - 1) * 160 * my_scale,
            )
            if select_gift_point[1] < gift_box[-1]:
                auto.mouse_click(select_gift_point[0], select_gift_point[1])
            else:
                step = 300 * my_scale
                height = step
                if select_gift_point[1] - gift_box[-1] > step:
                    height = select_gift_point[1] - gift_box[-1]
                    result = [step] * int(height // step) + ([int(height % step)] if int(height % step) > 0 else [])
                else:
                    result = [step]
                for s in result:
                    auto.mouse_drag(
                        gift_box[-2] - 100 * my_scale,
                        gift_box[-1] - 100 * my_scale,
                        dy=-s,
                        drag_time=1.5,
                    )
                    sleep(1)
                    has_scrolled = True
                auto.mouse_click(select_gift_point[0], select_gift_point[1] - height)

        def _scroll_gift_list(delta_y):
            nonlocal has_scrolled
            auto.mouse_drag(
                gift_box[-2] - 100 * my_scale,
                gift_box[-1] - 100 * my_scale,
                dy=delta_y,
                drag_time=1.5,
            )
            sleep(1)
            has_scrolled = True

        def _select_gift_by_asset(target):
            nonlocal has_scrolled
            if not target.asset or not ImageUtils.existing_image_paths(target.asset):
                return False
            for attempt in range(5):
                if point := auto.find_element(target.asset, model="clam", take_screenshot=True):
                    auto.mouse_click(point[0], point[1])
                    log.info(f"已选择观测饰品：{target.key}")
                    return True
                if attempt < 4:
                    _scroll_gift_list(-300 * my_scale)
            # Restore the list position so the coordinate fallback starts from
            # the same state as the normal selector.
            for _ in range(4):
                _scroll_gift_list(300 * my_scale)
            has_scrolled = False
            return False

        log.debug("开始选择观测EGO饰品")
        auto.model = "clam"

        my_scale = cfg.set_win_size / 1440
        benchmark_point = None
        if point := auto.find_element(
            "mirror/road_to_mir/observe_ego_gift/observe_burn_assets.png", model="clam", take_screenshot=True
        ):
            benchmark_point = point
        elif point := auto.find_element("mirror/road_to_mir/observe_ego_gift/observe_bleed_assets.png", model="clam"):
            benchmark_point = (point[0] - 110 * my_scale, point[1])

        if not benchmark_point:
            return

        gift_box = ImageUtils.get_bbox(ImageUtils.load_image("mirror/road_to_mir/observe_ego_gift/gift_box_bbox.png"))

        current_system_index = None

        # 选择观测饰品
        for gift_id in self.observe_ego_gift_selected:
            target = resolve_observe_ego_gift(gift_id)
            if target is None:
                log.warning(f"无法解析观测饰品：{gift_id}，跳过")
                continue
            file_system = target.system
            gift_level = target.level
            gift_row = target.row
            gift_col = target.col

            # 选择体系
            if file_system == "general":
                system_index = 10
            else:
                system_index = [k for k, v in observe_system.items() if v == file_system][0]

            # 状态机：选择体系或按需重置
            if current_system_index is None or current_system_index != system_index:
                # 首次进入或跨体系切换：直接点击目标分类（游戏自动从新分类列表顶部加载）
                auto.mouse_click(benchmark_point[0] + 110 * system_index * my_scale, benchmark_point[1])
                sleep(0.3)
                current_system_index = system_index
                has_scrolled = False
            elif has_scrolled:
                # 连续在同一个体系下选择下一个饰品，且此前页面曾向下滚动过：
                # 借切一次其他体系重置回顶部
                reset_temp_index = 0 if system_index != 0 else 1
                auto.mouse_click(benchmark_point[0] + 110 * reset_temp_index * my_scale, benchmark_point[1])
                sleep(0.2)
                auto.mouse_click(benchmark_point[0] + 110 * system_index * my_scale, benchmark_point[1])
                sleep(0.3)
                has_scrolled = False

            if _select_gift_by_asset(target):
                continue

            if level_point := auto.find_element(
                f"mirror/road_to_mir/observe_ego_gift/Level_{'I' * gift_level}.png", take_screenshot=True
            ):
                _select_gift(level_point, gift_row, gift_col)
            else:
                level_point = None
                for _ in range(5):
                    _scroll_gift_list(-(gift_box[-1] - gift_box[1]) / 2)
                    if p := auto.find_element(
                        f"mirror/road_to_mir/observe_ego_gift/Level_{'I' * gift_level}.png", take_screenshot=True
                    ):
                        level_point = p
                        break
                if level_point is None:
                    continue
                _select_gift(level_point, gift_row, gift_col)
                sleep(0.5)

        # 观测饰品选择完毕
        for _ in range(5):
            bbox = ImageUtils.get_bbox(
                ImageUtils.load_image("mirror/road_to_mir/observe_ego_gift/select_gift_bbox.png")
            )
            ocr_result = auto.find_language_text("选择", "select", bbox)
            if ocr_result:
                auto.mouse_click((bbox[0] + bbox[2]) / 2, (bbox[1] + bbox[3]) / 2)
                sleep(1)
                if auto.click_element("mirror/shop/leave_shop_confirm_assets.png", take_screenshot=True):
                    break
        for _ in range(5):
            auto.click_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png", take_screenshot=True)

        for _ in range(3):
            bbox = ImageUtils.get_bbox(
                ImageUtils.load_image("mirror/road_to_mir/observe_ego_gift/reject_gift_bbox.png")
            )
            ocr_result = auto.find_language_text("拒绝", "reject", bbox)
            if ocr_result:
                auto.mouse_click((bbox[0] + bbox[2]) / 2, (bbox[1] + bbox[3]) / 2)
                sleep(1)
                if auto.click_element("mirror/shop/leave_shop_confirm_assets.png", take_screenshot=True):
                    self.observe_ego_gift_done = True
                    return
        self.observe_ego_gift_done = True

    def select_mirror_team(self):
        self.team_code_loaded = False
        chance_to_select_team = 5
        while not select_battle_team(self.team_number):
            chance_to_select_team -= 1
            if chance_to_select_team < 0:
                log.error("无法寻得队伍")
                raise unableToFindTeamError("无法寻得队伍，请检查队伍名称是否为默认名称")
        # 加载编队码（如果启用）
        team_setting = cfg.config.teams.get(str(self.team_order))
        if team_setting and team_setting.use_team_code and team_setting.team_code:
            self.team_code_loaded = load_team_code_in_game(team_setting.team_code)
            if not self.team_code_loaded:
                message = "攻略编队码加载失败，已停止本轮镜牢，避免使用错误编队"
                log.error(message)
                raise cannotOperateGameError(message)
        loop_count = 30
        auto.model = "clam"
        while auto.find_element("mirror/road_to_mir/dreaming_star/coins_assets.png") is None:
            if auto.take_screenshot() is None:
                continue
            loop_count -= 1
            if loop_count % 5 == 0:
                log.debug(f"选择队伍识别次数剩余{loop_count}次")
            if loop_count < 20:
                auto.model = "normal"
                auto.mouse_to_blank(move_back=False)
                log.debug("识别模式切换到正常模式")
            if loop_count < 10:
                auto.model = "aggressive"
                log.debug("识别模式切换到激进模式")
            if loop_count < 5:
                if auto.find_language_text("平均", "current"):
                    auto.click_element("mirror/road_to_mir/enter_mirror_confirm.png")
            if loop_count < 0:
                log.error("无法进入镜牢,尝试回到初始界面")
                back_init_menu()
                break
            if retry() is False:
                return False
            if auto.click_element("mirror/road_to_mir/level_confirm_assets.png"):
                continue
            if auto.click_element("mirror/road_to_mir/select_team_confirm_assets.png"):
                sleep(0.5)
                loop_count -= 1
                continue

        time.sleep(3)

    @begin_and_finish_time_log(task_name="镜牢寻路")
    def search_road(self):
        if cfg.mirror_keyboard_simple_pathfinding:
            if search_road_simple_keyboard():
                return True
            log.debug("简单键盘寻路失败，回退到常规寻路")

        try:
            if next_node := self.mirror_map.get_next_step():
                if next_node is True:
                    return True
                if self.mirror_map.enter_next_node(next_node):
                    return True
            log.debug("未能构建路线图，尝试使用最近节点法重新寻路")
        except Exception as e:
            check_cancelled()
            log.debug(f"使用onnx模型寻路出错:{e}")
        finally:
            auto.mouse_to_blank()
        try:
            for _ in range(3):
                while auto.take_screenshot() is None:
                    continue
                if search_road_default_distance():
                    sleep(1)
                    return True
                if auto.click_element("mirror/road_in_mir/enter_assets.png"):
                    return True
                if retry() is False:
                    return False
            for _ in range(3):
                if cfg.background_click:
                    continue
                while auto.take_screenshot() is None:
                    continue
                if search_road_farthest_distance():
                    sleep(1)
                    return True
                if retry() is False:
                    return False
        except InputAttributeError as e:
            log.error(f"寻路出错:{e}, 尝试重进镜牢")
            pass
        except Exception as e:
            check_cancelled()
            log.error(f"寻路出错:{e}")
            return False
        if auto.click_element("mirror/road_in_mir/enter_assets.png", take_screenshot=True):
            return True
        start_time = time.time()
        log.info("寻路出错, 尝试重进镜牢")
        while True:
            from tasks.base.retry import check_times

            # 自动截图
            if auto.take_screenshot() is None:
                continue
            if auto.get_restore_time() is not None:
                start_time = max(start_time, auto.get_restore_time())
            if check_times(start_time):
                back_init_menu()
                return False
            auto.mouse_to_blank()
            if auto.click_element("mirror/road_in_mir/enter_assets.png"):
                return True
            if auto.click_element("home/drive_assets.png") or auto.find_element("home/window_assets.png"):
                sleep(0.5)
                break
            if auto.click_element("mirror/road_in_mir/towindow&forfeit_confirm_assets.png"):
                break
            if auto.click_element("mirror/road_in_mir/to_window_assets.png"):
                continue
            if auto.click_element("mirror/road_in_mir/setting_assets.png"):
                sleep(1)
                continue
            if retry() is False:
                return False

    def re_start(self):
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue
            if auto.click_element("mirror/road_in_mir/towindow&forfeit_confirm_assets.png"):
                break
            if auto.click_element("mirror/road_in_mir/forfeit_assets.png"):
                continue
            if auto.click_element("battle/give_up_assets.png"):
                continue
            if auto.click_element("mirror/road_in_mir/setting_assets.png"):
                continue
            auto.key_press("esc")
            time.sleep(1)
            if retry() is False:
                return False
        # TODO耗时
        msg = f"满 身 疮 痍 ！ 重 开 ！此次战败耗时{time.time() - self.start_time}"
        log.info(msg)
        self.first_battle = True
        if self.defense_for_solo_state is not None:
            self.defense_for_solo_state.reset_for_run()
            self.shop.set_pseudo_solo_active(False)
        self.start_time = time.time()

    def event_handling(self):
        # 遇到有SKIP的情况
        event_start_time = time.time()
        loop_count = 30
        auto.model = "clam"
        event_chance = 15
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue

            if retry() is False:
                return False

            # 如果在战斗中或回到镜牢路线图中，则跳出循环
            if auto.find_element("battle/turn_assets.png"):
                break
            if auto.find_element("mirror/road_in_mir/legend_assets.png"):
                break

            if event_chance == 0:
                if key_word_position := auto.find_language_text("判定", "check"):
                    auto.mouse_action_with_pos(key_word_position, offset=False)
                    event_chance += 5
            if 5 <= event_chance < 10:
                auto.click_element("event/select_first_option_assets.png")
                event_chance -= 1
            elif 5 > event_chance > 0:
                if coordinates := auto.find_element(
                    "event/select_first_option_assets.png", find_type="image_with_multiple_targets", threshold=0.75
                ):
                    for coordinate in coordinates:
                        auto.mouse_click(coordinate[0], coordinate[1])
                    retry()
                event_chance -= 1
            if event_chance < 0:
                finishes_bbox = ImageUtils.get_bbox(ImageUtils.load_image("event/continue_assets.png"))
                if auto.find_text_element(
                    [
                        "continue",
                        "proceed",
                        "commence",
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
                    break
                elif coordinates := auto.find_element(
                    "event/select_first_option_assets.png", find_type="image_with_multiple_targets", threshold=0.75
                ):
                    for coordinate in coordinates:
                        auto.mouse_click(coordinate[0], coordinate[1])
                    retry()
                else:
                    msg = "事件卡死，尝试返回主界面"
                    log.error(msg)
                    back_init_menu()
                    return

            # 针对不同事件进行处理，优先选???与直接获取的，再选需要判定的，再选后续事件的，最后第一个事项
            if auto.click_element("event/unknown_event.png"):
                event_chance -= 1
                continue
            if positions_list := auto.find_element(
                "event/select_to_gain_ego.png",
                find_type="image_with_multiple_targets",
                threshold=0.75,
            ):
                positions_list = sorted(positions_list, key=lambda x: (x[1], x[0]))
                auto.mouse_click(positions_list[0][0], positions_list[0][1])
                event_chance -= 1
                continue
            if auto.click_element("event/advantage_check.png"):
                event_chance -= 1
                continue
            if auto.click_element("event/gain_a_ego_depending_on_result.png"):
                event_chance -= 1
                continue

            # 如果需要罪人判定
            if auto.find_element("event/choices_assets.png") and auto.find_element(
                "event/select_first_option_assets.png"
            ):
                auto.click_element("event/select_first_option_assets.png")
                event_chance -= 1
            if (
                auto.find_element(
                    "event/perform_the_check_feature_assets.png",
                    threshold=0.75,
                )
                and event_handling.decision_event_handling()
            ):
                # 输入后立即刷新画面，避免继续在已失效的判定帧上匹配其它按钮。
                continue
            if auto.click_element("event/continue_assets.png"):
                continue
            if auto.click_element("event/proceed_assets.png"):
                continue
            if auto.click_element("event/commence_assets.png"):
                continue
            if auto.click_element("event/commence_battle_assets.png"):
                continue

            if auto.click_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png"):
                continue

            if auto.click_element("event/skip_assets.png", times=6):
                continue

            loop_count -= 1
            if loop_count % 3 == 0:
                log.debug(f"事件处理识别次数剩余{loop_count}次")
            if loop_count < 20:
                auto.model = "normal"
                auto.mouse_to_blank(move_back=False)
                log.debug("识别模式切换到正常模式")
            if loop_count < 10:
                auto.model = "aggressive"
                log.debug("识别模式切换到激进模式")
            if loop_count < 0:
                log.error("无法解决事件,尝试回到初始界面")
                back_init_menu()
                break

        # 统计事件处理时间
        event_end_time = time.time()
        event_elapsed_time = event_end_time - event_start_time
        self.event_total_time += event_elapsed_time
        self.event_times += 1

    def acquire_ego_gift(self, type: int = 1):
        my_scale = cfg.set_win_size / 1440
        auto.model = "clam"
        auto.mouse_to_blank()

        def route_priority(bbox):
            return self.shop.route_gift_priority_for_crop(bbox, self.floor)

        route_fallback_priority = 10**6
        if type == 2:
            pos = auto.find_element("mirror/road_in_mir/acquire_ego_gift_refuse_assets.png")
            auto.mouse_click(pos[0] - 500 * my_scale, pos[1] - 500 * my_scale)
            sleep(cfg.mouse_action_interval)
            auto.mouse_click(pos[0], pos[1] - 500 * my_scale)
            sleep(cfg.mouse_action_interval)
            auto.click_element("mirror/road_in_mir/acquire_ego_gift_select_assets.png", model="normal")
            time.sleep(2)
            if retry() is False:
                return False
            return
        while True:
            if auto.take_screenshot() is None:
                continue

            if auto.click_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png"):
                break
            try:
                acquire_card = auto.find_element(
                    "mirror/road_in_mir/acquire_ego_gift_card.png",
                    find_type="image_with_multiple_targets",
                )
                my_list = []
                if len(acquire_card) == 2:
                    gift_candidates = []
                    for button in acquire_card:
                        bbox = (
                            button[0] - 50 * my_scale,
                            button[1] - 300 * my_scale,
                            button[0] + 450 * my_scale,
                            button[1] + 350 * my_scale,
                        )
                        if not cfg.not_skip_whitegossypium:
                            is_white_gossypium = bool(
                                auto.find_element(
                                    "mirror/ego_gifts/white_gossypium.png",
                                    my_crop=bbox,
                                    threshold=0.75,
                                )
                            )
                            if not is_white_gossypium:
                                ocr_result = auto.find_language_text("白棉花", ["white", "gossypium"], bbox)
                                if isinstance(ocr_result, list) and len(ocr_result) >= 2:
                                    is_white_gossypium = True
                            if is_white_gossypium:
                                continue
                        is_owned = bool(auto.find_language_text("已持有", "Owned", bbox))
                        priority = route_priority(bbox)
                        gift_candidates.append(
                            (
                                is_owned,
                                priority if priority is not None else route_fallback_priority,
                                button,
                            )
                        )

                    if gift_candidates:
                        gift_candidates.sort(key=lambda gift: (gift[0], gift[1]))
                        button = gift_candidates[0][2]
                        auto.mouse_click(button[0], button[1])
                        auto.click_element(
                            "mirror/road_in_mir/acquire_ego_gift_select_assets.png",
                            model="normal",
                        )
                        time.sleep(2)
                        if retry() is False:
                            return False
                        return
                elif len(acquire_card) == 1:
                    for button in acquire_card:
                        bbox = (
                            button[0] - 50 * my_scale,
                            button[1] - 300 * my_scale,
                            button[0] + 450 * my_scale,
                            button[1] + 350 * my_scale,
                        )
                        if not cfg.not_skip_whitegossypium:
                            is_white_gossypium = bool(
                                auto.find_element(
                                    "mirror/ego_gifts/white_gossypium.png",
                                    my_crop=bbox,
                                    threshold=0.75,
                                )
                            )
                            if not is_white_gossypium:
                                ocr_result = auto.find_language_text("白棉花", ["white", "gossypium"], bbox)
                                if isinstance(ocr_result, list) and len(ocr_result) >= 2:
                                    is_white_gossypium = True
                            if is_white_gossypium:
                                time.sleep(1)
                                auto.click_element(
                                    "mirror/road_in_mir/refuse_gift_assets.png",
                                    take_screenshot=True,
                                )
                                sleep(1)
                                auto.click_element(
                                    "mirror/road_in_mir/refuse_gift_confirm_assets.png",
                                    take_screenshot=True,
                                )
                                time.sleep(2)
                                if retry() is False:
                                    return False
                                return
                        auto.mouse_click(button[0], button[1])
                        time.sleep(1)
                        auto.click_element(
                            "mirror/road_in_mir/acquire_ego_gift_select_assets.png",
                            model="normal",
                        )
                        time.sleep(2)
                        if retry() is False:
                            return False
                        return
                else:
                    for button in acquire_card:
                        bbox = (
                            button[0] - 50 * my_scale,
                            button[1] - 300 * my_scale,
                            button[0] + 450 * my_scale,
                            button[1] + 350 * my_scale,
                        )
                        if not cfg.not_skip_whitegossypium:
                            ocr_result = auto.find_language_text("白棉花", ["white", "gossypium"], bbox)
                            if ocr_result:
                                continue
                        is_owned = bool(auto.find_language_text("已持有", "Owned", bbox))
                        priority = route_priority(bbox)
                        gift_candidate = (
                            is_owned,
                            priority if priority is not None else route_fallback_priority,
                            2,
                            button,
                        )
                        if priority is not None:
                            log.debug(f"识别到当前阶段路线饰品，优先级为{priority}")
                        if auto.find_element(
                            f"mirror/road_in_mir/acquire_ego_gift/{self.system}.png",
                            my_crop=bbox,
                            threshold=0.85,
                        ):
                            gift_candidate = (*gift_candidate[:2], 0, gift_candidate[3])
                            my_list.append(gift_candidate)
                        else:
                            if self.second_system and (
                                self.second_system_setting == 0
                                or (self.second_system_setting == 1 and self.shop.fuse_IV)
                            ):
                                if auto.find_element(
                                    f"mirror/road_in_mir/acquire_ego_gift/{all_systems[self.second_system_select]}.png",
                                    my_crop=bbox,
                                    threshold=0.85,
                                ):
                                    gift_candidate = (*gift_candidate[:2], 1, gift_candidate[3])
                                    my_list.append(gift_candidate)
                                    continue
                            my_list.append(gift_candidate)
                    my_list.sort(key=lambda gift: (gift[0], gift[1], gift[2]))
                    owned_gifts = sum(gift[0] for gift in my_list)
                    my_list = [gift[3] for gift in my_list]
                    if owned_gifts:
                        log.debug(f"检测到{owned_gifts}个已持有EGO饰品，已降低选择优先级")
                select_bbox = ImageUtils.get_bbox(ImageUtils.load_image("mirror/road_in_mir/ego_gift_get_bbox.png"))
                if select_bbox:
                    select_bbox = (
                        max(select_bbox[0] - 100, 0),  # 确保左上角 x 坐标不小于 0
                        max(select_bbox[1] - 100, 0),  # 确保左上角 y 坐标不小于 0
                        min(select_bbox[2] + 100, cfg.set_win_size * 16 / 9),  # 确保右下角 x 坐标不大于 图片宽
                        min(select_bbox[3] + 100, cfg.set_win_size),  # 确保右下角 y 坐标不大于 图片高
                    )
                if auto.find_text_element(["0/1", "01", "1/1", "11", "1/2", "12", "/1"], my_crop=select_bbox):
                    for gift in my_list[:1]:
                        auto.mouse_click(gift[0], gift[1])
                        sleep(cfg.mouse_action_interval)
                    auto.click_element(
                        "mirror/road_in_mir/acquire_ego_gift_select_assets.png",
                        model="normal",
                    )
                    time.sleep(2)
                    if retry() is False:
                        return False
                    return
                elif auto.find_text_element(["0/2", "02", "2/2", "22", "/2"], my_crop=select_bbox):
                    for gift in my_list[:2]:
                        auto.mouse_click(gift[0], gift[1])
                        sleep(cfg.mouse_action_interval)
                    auto.click_element(
                        "mirror/road_in_mir/acquire_ego_gift_select_assets.png",
                        model="normal",
                    )
                    time.sleep(2)
                    if retry() is False:
                        return False
                    return
                else:
                    for gift in my_list:
                        auto.mouse_click(gift[0], gift[1])
                        sleep(cfg.mouse_action_interval)
                    auto.click_element(
                        "mirror/road_in_mir/acquire_ego_gift_select_assets.png",
                        model="normal",
                    )
                    time.sleep(2)
                    if retry() is False:
                        return False
                    return

            except Exception as e:
                check_cancelled()
                log.error(e)
                continue

    def get_reward_in_road(self):
        main_loop_count = 20
        auto.model = "clam"
        while True:
            check_cancelled()
            if auto.take_screenshot() is None:
                auto.mouse_to_blank()
                continue
            # 如果回到主界面，退出循环
            if auto.click_element("mirror/claim_reward/rewards_acquired_assets.png"):
                return True
            if cfg.no_weekly_bonuses:
                bonuses = auto.find_element(
                    "mirror/claim_reward/weekly_bonuses.png",
                    find_type="image_with_multiple_targets",
                )
                if len(bonuses) >= 1:
                    for _ in range(len(bonuses)):
                        position = bonuses.pop(-1)
                        auto.mouse_click(position[0], position[1])
            if cfg.hard_mirror_single_bonuses:
                bonuses = auto.find_element(
                    "mirror/claim_reward/weekly_bonuses.png",
                    find_type="image_with_multiple_targets",
                )
                bonuses = sorted(bonuses, key=lambda x: x[0])
                if len(bonuses) > 1:
                    for _ in range(len(bonuses) - 1):
                        position = bonuses.pop(-1)
                        auto.mouse_click(position[0], position[1])
            if auto.click_element(
                "mirror/claim_reward/claim_rewards_confirm_assets.png",
                threshold=0.75,
                model="clam",
            ):
                continue
            if self.hard_switch and cfg.save_rewards:
                auto.click_element("mirror/claim_reward/claim_rewards_assets.png")
                sleep(1)
                pos = auto.find_element(
                    "mirror/claim_reward/use_enkephalin_assets.png",
                    take_screenshot=True,
                )
                if pos:
                    auto.mouse_click(pos[0] - 300 * (cfg.set_win_size / 1440), pos[1])
                    sleep(1)
                continue
            elif auto.click_element("mirror/claim_reward/claim_rewards_assets.png"):
                sleep(1)
                if auto.click_element(
                    "mirror/claim_reward/use_enkephalin_assets.png",
                    take_screenshot=True,
                ):
                    sleep(1)
                # TODO: 统计获取的coins
                continue
            if auto.click_element("mirror/claim_reward/use_enkephalin_assets.png", threshold=0.75):  # 降低识别阈值
                sleep(1)
                continue
            # 处理周年活动弹出的窗口
            if auto.click_element("home/close_anniversary_event_assets.png"):
                continue
            retry()
            main_loop_count -= 1
            if main_loop_count % 3 == 0:
                log.debug(f"镜牢奖励识别次数剩余{main_loop_count}次")
            if main_loop_count < 10:
                auto.model = "normal"
                auto.mouse_to_blank(move_back=False)
                log.debug("识别模式切换到正常模式")
            if main_loop_count < 5:
                auto.model = "aggressive"
                log.debug("识别模式切换到激进模式")
            if main_loop_count < 0:
                raise cannotOperateGameError("镜牢奖励领取出错,请手动操作重试")

    @begin_and_finish_time_log(task_name="镜牢商店")
    def in_shop(self):
        self.shop.in_shop(self.floor)

    def get_which_floor(self):
        absolute_floor = None
        if self.resumed_from_existing_game and not self.plan_runtime.progress_observed:
            absolute_floor = self._read_absolute_floor()
            if absolute_floor is None:
                if self.plan_runtime.floor_count is not None and self.plan_runtime.floor_count > MIRROR_SEGMENT_LENGTH:
                    message = "无法从恢复的15层镜牢地图读取绝对楼层，不能安全套用饰品路线"
                    self.plan_runtime.record_deviation(message)
                    log.error(message)
                    raise cannotOperateGameError(message)
                log.warning("恢复的5层镜牢标题楼层读取失败，将使用未通过标记计算当前具体楼层")

        auto.click_element("mirror/road_in_mir/setting_assets.png", take_screenshot=True)
        sleep(1)

        scale = cfg.set_win_size / 1440
        floor_progress_crop = (
            900 * scale,
            650 * scale,
            1700 * scale,
            720 * scale,
        )
        if to_window_position := auto.find_element("mirror/road_in_mir/to_window_assets.png", take_screenshot=True):
            not_passed_floors = (
                auto.find_element(
                    "mirror/road_in_mir/not_passed_floor.png",
                    find_type="image_with_multiple_targets",
                    my_crop=floor_progress_crop,
                    take_screenshot=True,
                    min_dist=80 * scale,
                )
                or []
            )
            not_passed_floor_count = len(not_passed_floors)
            try:
                self.floor = self.plan_runtime.detect_floor(
                    not_passed_floor_count,
                    initial=not self.plan_runtime.progress_observed,
                    absolute_floor=absolute_floor - 1 if absolute_floor is not None else None,
                )
            except MirrorPlanProgressError as error:
                self.plan_runtime.record_deviation(str(error))
                log.error(str(error))
                auto.mouse_action_with_pos(
                    (to_window_position[0] - 200 * cfg.set_win_size / 1440, to_window_position[1])
                )
                raise cannotOperateGameError(str(error)) from error
            log.debug(
                f"当前镜牢层数: 内部{self.floor}（实际第{self.floor + 1}层，目标{self.plan_runtime.floor_count}层，"
                f"当前段剩余标记{not_passed_floor_count}个）"
            )
            self.get_floor_num = False
            auto.mouse_action_with_pos((to_window_position[0] - 200 * cfg.set_win_size / 1440, to_window_position[1]))
            self.mirror_map.refresh_floor(self.floor)
