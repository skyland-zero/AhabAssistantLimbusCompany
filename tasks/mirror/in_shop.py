from time import sleep

from PIL import Image

from module.automation import auto
from module.config import TeamSetting, cfg
from module.logger import log
from module.mirror_routes import MirrorRouteDefinition, get_mirror_route, route_target_priority
from module.ocr import ocr
from tasks import all_sinners_name, all_sinners_name_zh, all_systems, system_cn_zh
from tasks.base.back_init_menu import back_init_menu
from tasks.base.retry import retry, wait_for_ui_state
from tasks.mirror import fusion_result, must_be_abandoned, must_purchase
from utils.image_utils import ImageUtils


class Shop:
    def __init__(self, team_setting: TeamSetting):
        self.system = all_systems[team_setting.team_system]  # 队伍体系
        self.sinner_team = team_setting.sinner_order  # 选择的罪人序列
        # 获取舍弃的饰品体系列表
        self.shop_sell_list = []
        for system in list(all_systems.values()):
            if system == self.system:
                continue
            if getattr(team_setting, f"system_{system}", False):
                self.shop_sell_list.append(system)
        self.fuse_switch = False if team_setting.shop_strategy == 0 else True  # 是否启动合成模式
        self.fuse_aggressive_switch = True if team_setting.shop_strategy == 2 else False  # 是否启动激进合成模式
        self.do_not_heal = team_setting.do_not_heal  # 是否不治疗
        self.do_not_buy = team_setting.do_not_buy  # 是否不购买
        self.do_not_fuse = team_setting.do_not_fuse  # 是否不合成
        self.do_not_sell = team_setting.do_not_sell  # 是否不出售
        self.do_not_enhance = team_setting.do_not_enhance  # 是否不升级
        self.aggressive_save_systems = team_setting.aggressive_save_systems  # 激进合成期间保护本体系饰品
        self.only_aggressive_fuse = team_setting.only_aggressive_fuse  # 是否只进行激进合成，不进行其他合成
        self.do_not_system_fuse = team_setting.do_not_system_fuse  # 是否不进行体系饰品合成
        self.only_system_fuse = team_setting.only_system_fuse  # 是否只进行体系饰品合成
        # 是否在合成四级后改变行动策略
        self.after_level_IV = team_setting.after_level_IV
        self.after_level_IV_select = team_setting.after_level_IV_select
        # 是否自定义商店购物策略
        self.shopping_strategy = team_setting.shopping_strategy
        self.shopping_strategy_select = team_setting.shopping_strategy_select
        # 是否启用第二体系
        self.second_system = team_setting.second_system
        self.second_system_select = all_systems[team_setting.second_system_select]
        self.second_system_setting = team_setting.second_system_setting
        self.second_system_action = team_setting.second_system_action
        # 技能替换
        self.skill_replacement = team_setting.skill_replacement
        self.skill_replacement_select = team_setting.skill_replacement_select
        self.skill_replacement_mode = team_setting.skill_replacement_mode
        self.max_keyword_refresh = team_setting.max_keyword_refresh
        self.max_normal_refresh = team_setting.max_normal_refresh
        self.ignore_shop = team_setting.ignore_shop  # 忽略的商店楼层
        self.route: MirrorRouteDefinition = get_mirror_route(team_setting.mirror_route_profile)
        self.skill_replacement_target_counts = self.route.skill_replacement_target_counts
        self._route_skill_target_logged = False
        self.current_layer = 0
        self.pseudo_solo_active = False
        self._route_fused_gift_ids = set()

        self.aggressive_also_enhance = team_setting.aggressive_also_enhance  # 激进合成期间也升级饰品

        self.fuse_IV = False
        self.fuse_second_IV = False
        self.fuse_system_gift_1 = False
        self.fuse_system_gift_2 = False
        self.the_first_line_position = None
        self.replacement = 0

        # 用于记录已升级的ego饰品
        self.enhance_gifts_list = []
        self.first_gift_enhance = False

    def set_layer(self, layer: int) -> None:
        """Set the current mirror layer used by the selected gift route."""

        self.current_layer = max(0, int(layer))

    def set_pseudo_solo_active(self, active: bool) -> None:
        """Tell route recipes when multi-identity effects are no longer valid."""

        self.pseudo_solo_active = bool(active)

    def _route_targets_for_matching(self, *, include_recipe_targets: bool = False):
        targets = list(self.route.gift_targets_for_floor(self.current_layer))
        if not include_recipe_targets:
            return targets
        # A target remains valuable after its acquisition window closes.  The
        # selection window is floor-scoped, but protection is run-scoped so a
        # floor-1 material cannot be consumed before a later fusion.
        protected_ids = {target.gift_id for target in self.route.gifts if target.protected}
        protected_ids.update(
            gift_id
            for recipe in self.route.recipes
            if recipe.end_floor >= max(1, self.current_layer)
            for gift_id in (recipe.result_gift_id, *recipe.material_gift_ids)
        )
        return [target for target in self.route.gifts if target.protected and target.gift_id in protected_ids]

    def _route_gift_id_for_crop(
        self,
        crop,
        gift_ids=None,
        *,
        include_recipe_targets: bool = False,
    ) -> str | None:
        allowed = set(gift_ids) if gift_ids is not None else None
        for target in self._route_targets_for_matching(include_recipe_targets=include_recipe_targets):
            if allowed is not None and target.gift_id not in allowed:
                continue
            if target.asset and auto.find_element(target.asset, my_crop=crop, threshold=0.75):
                return target.gift_id
            if auto.find_language_text(
                list(target.names_zh),
                list(target.names_en),
                crop,
            ):
                return target.gift_id
        return None

    def _route_priority_for_crop(self, crop) -> int | None:
        """Match the active route's gift names in one OCR crop."""

        return route_target_priority(
            self.route,
            self.current_layer,
            lambda target: self._route_gift_id_for_crop(crop, [target.gift_id]) is not None,
        )

    def route_gift_priority_for_crop(self, crop, layer: int | None = None) -> int | None:
        """Return the route priority for a visible reward/shop gift crop.

        The public wrapper lets the mirror reward selector pass its current
        floor without mutating the shop's state.  ``None`` means that the
        route is inactive for this floor or the gift name was not recognized.
        """

        previous_layer = self.current_layer
        if layer is not None:
            self.set_layer(layer)
        try:
            return self._route_priority_for_crop(crop)
        finally:
            self.current_layer = previous_layer

    def _route_priority_at_position(
        self,
        position,
        *,
        hover: bool = False,
    ) -> int | None:
        """Read a route gift at an inventory position.

        Gift names are often only exposed by the game's hover tooltip.  The
        fast crop check handles visible names; the optional hover path is used
        only by destructive fusion filtering so route targets can be removed
        from the candidate list before any click occurs.
        """

        gift_id = self._route_gift_id_at_position(
            position,
            hover=hover,
            include_recipe_targets=True,
        )
        if gift_id is None:
            return None
        target = self.route.gift_target(gift_id)
        return target.priority if target is not None else None

    def _route_gift_id_at_position(
        self,
        position,
        *,
        gift_ids=None,
        hover: bool = False,
        include_recipe_targets: bool = False,
    ) -> str | None:
        """Match a route gift at an inventory coordinate, optionally by hover."""

        if not self.route.gifts or not position or len(position) < 2:
            return None
        scale = cfg.set_win_size / 1440
        x, y = float(position[0]), float(position[1])
        screen_width, screen_height = self._route_screen_size()
        # Route fusion builds a fixed ten-slot grid, but the last slots can be
        # outside a narrower runtime window.  Never pass an empty crop to the
        # image/OCR backends: OpenCV's CLAHE may spin indefinitely on one.
        if not (0 <= x < screen_width and 0 <= y < screen_height):
            return None
        crop = (
            max(0.0, x - 150 * scale),
            max(0.0, y - 180 * scale),
            min(screen_width, x + 260 * scale),
            min(screen_height, y + 220 * scale),
        )
        if int(round(crop[2])) <= int(round(crop[0])) or int(round(crop[3])) <= int(round(crop[1])):
            return None
        gift_id = self._route_gift_id_for_crop(
            crop,
            gift_ids,
            include_recipe_targets=include_recipe_targets,
        )
        if gift_id is not None or not hover:
            return gift_id

        try:
            auto.mouse_move((x, y))
            sleep(0.2)
            if auto.take_screenshot() is None:
                return None
            return self._route_gift_id_for_crop(
                crop,
                gift_ids,
                include_recipe_targets=include_recipe_targets,
            )
        except Exception as error:
            log.debug(f"读取路线饰品悬浮提示失败: {error}")
            return None
        finally:
            auto.mouse_to_blank()

    @staticmethod
    def _route_screen_size() -> tuple[float, float]:
        screenshot_size = getattr(getattr(auto, "screenshot", None), "size", None)
        if isinstance(screenshot_size, tuple) and len(screenshot_size) == 2:
            return tuple(map(float, screenshot_size))
        return float(cfg.set_win_size * 16 / 9), float(cfg.set_win_size)

    def _remove_route_targets(self, positions, *, hover: bool = False):
        """Filter route targets out of a destructive inventory operation."""

        if not self.route.gifts or not positions:
            return positions
        protected = []
        for position in positions:
            if self._route_priority_at_position(position, hover=hover) is not None:
                protected.append(position)
        if protected:
            log.debug(f"保护{len(protected)}个当前路线饰品，跳过通用合成/出售")
        return [position for position in positions if position not in protected]

    class RestartGame(Exception):
        pass

    def ego_gift_to_power_up(self):
        loop_count = 30
        auto.model = "clam"
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue
            auto.mouse_to_blank()
            if auto.click_element("mirror/shop/power_up_assets.png"):
                auto.mouse_to_blank()
                sleep(0.5)
                if auto.click_element("mirror/shop/power_up_confirm_assets.png", take_screenshot=True) is False:
                    return True
                wait_for_ui_state(
                    lambda: bool(auto.find_element("mirror/shop/power_up_assets.png")),
                    timeout=3,
                )
                if retry(screenshot_ready=True) is False:
                    raise self.RestartGame()
            if auto.find_element("mirror/shop/power_up_confirm_assets.png"):
                return False
            loop_count -= 1
            if loop_count < 20:
                auto.model = "normal"
            if loop_count < 10:
                auto.model = "aggressive"
            if loop_count < 0:
                log.error("无法升级ego饰品")
                break

    def buy_gifts(self):
        def sort_points(points, complete=0, threshold=40):
            # 第一步：先按 X 坐标排好（左右顺序）
            points.sort(key=lambda p: p[0])
            # 第二步：按 Y 坐标“归一化”后排序（上下顺序）
            points.sort(key=lambda p: p[1] // threshold)

            return points[:-complete] if complete > 0 else points

        def re_sort_points(points):
            commodity_every_line = 4
            coins_point = auto.find_element("mirror/shop/shop_coins_assets.png", take_screenshot=True)
            scale = cfg.set_win_size / 1440
            if not coins_point or not points:
                auto.mouse_click_blank(times=3)
                coins_point = auto.find_element("mirror/shop/shop_coins_assets.png", take_screenshot=True)
                if not coins_point or not points:
                    return points
            step = 300 * scale
            first_grid_x = coins_point[0] + 150 * scale
            new_points = []
            for p in points:
                orig_x, orig_y = p[0], p[1]

                # 计算当前所在的列索引 (0, 1, 2, 3...)
                # 注意：这里只用 x 判定列，不需要 round 整个坐标，只需要知道它在哪一列
                col = round((orig_x - first_grid_x) / step)

                if col > 0:
                    # --- 情况 A: 还在同行 ---
                    # y 值绝对不变，x 值减去一个 step
                    new_x = orig_x - step
                    new_y = orig_y
                else:
                    # --- 情况 B: 跨行移动 (col == 0) ---
                    # x 值跳到最后一列：增加 (每行个数 - 1) 个 step
                    # y 值向上移动一个 step
                    new_x = orig_x + (commodity_every_line - 1) * step
                    new_y = orig_y - step

                new_points.append([new_x, new_y])

            return new_points

        log.debug("开始执行饰品购买模块")
        keyword_refresh_count = 0
        normal_refresh_count = 0
        complete_count = 0
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue
            if self.shopping_strategy is False or (
                self.shopping_strategy and self.shopping_strategy_select in (0, 1, 5)
            ):
                # 购买必买项（回血饰品）
                log.debug("开始购买必买项")
                for commodity in must_purchase:
                    if auto.click_element(commodity, threshold=0.85):
                        buy_chance = 10
                        while auto.click_element("mirror/shop/purchase_assets.png") is False:
                            while auto.take_screenshot() is None:
                                continue
                            if retry() is False:
                                raise self.RestartGame()
                            if auto.click_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png"):
                                complete_count += 1
                                break
                            buy_chance -= 1
                            if buy_chance <= 5:
                                auto.click_element(commodity, threshold=0.85)
                            if buy_chance <= 0:
                                auto.mouse_click_blank(times=3)
                                break
                        sleep(1)
                        auto.click_element(
                            "mirror/road_in_mir/ego_gift_get_confirm_assets.png",
                            take_screenshot=True,
                        )
                while auto.take_screenshot() is None:
                    continue

            # Route targets are evaluated before the generic system purchase
            # pass.  A route is data-driven, so adding another preset does not
            # require another branch in this shop workflow.
            if self.route.gifts:
                for target in self.route.gift_targets_for_floor(self.current_layer):
                    target_position = auto.find_language_text(
                        list(target.names_zh),
                        list(target.names_en),
                    )
                    if not target_position:
                        continue
                    auto.mouse_click(target_position[0], target_position[1])
                    sleep(1)
                    if auto.click_element("mirror/shop/purchase_assets.png", take_screenshot=True):
                        sleep(1)
                        auto.click_element(
                            "mirror/road_in_mir/ego_gift_get_confirm_assets.png",
                            take_screenshot=True,
                        )
                        complete_count += 1
                        log.info(f"按路线优先购买饰品：{target.gift_id}")
                    else:
                        auto.mouse_click_blank(times=2)

            if self.fuse_aggressive_switch:
                log.debug("开始购买强化素材")
                if self.shopping_strategy is False or (
                    self.shopping_strategy and self.shopping_strategy_select in (1, 3, 4)
                ):
                    if auto.click_element("mirror/shop/level_IV_to_buy.png", threshold=0.82):
                        sleep(1)
                        while auto.take_screenshot() is None:
                            continue
                        if auto.click_element("mirror/shop/purchase_assets.png"):
                            sleep(1)
                            while auto.take_screenshot() is None:
                                continue
                            if retry() is False:
                                raise self.RestartGame()
                            auto.click_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png")
                            complete_count += 1
                            continue
                        else:
                            auto.mouse_click_blank()

                    if auto.click_element("mirror/shop/level_III_to_buy.png", threshold=0.82):
                        sleep(1)
                        if auto.click_element("mirror/shop/purchase_assets.png", take_screenshot=True):
                            sleep(1)
                            if retry() is False:
                                raise self.RestartGame()
                            auto.click_element(
                                "mirror/road_in_mir/ego_gift_get_confirm_assets.png",
                                take_screenshot=True,
                            )
                            complete_count += 1
                            continue
                        else:
                            auto.mouse_click_blank()

            if self.shopping_strategy is False or (
                self.shopping_strategy and self.shopping_strategy_select in (2, 4, 5)
            ):
                log.debug("开始购买本体系饰品")
                # 购买体系饰品
                system_gift = auto.find_element(
                    f"mirror/shop/enhance_gifts/shop_{self.system}.png",
                    find_type="image_with_multiple_targets",
                    threshold=0.85,
                    take_screenshot=True,
                )
                system_gift = sort_points(system_gift, complete_count)
                while system_gift:
                    gift = system_gift.pop(0)
                    auto.mouse_action_with_pos((gift[0], gift[1]), offset=True)
                    sleep(1)
                    while auto.take_screenshot() is None:
                        continue
                    if self.system == "bleed" and not cfg.not_skip_whitegossypium:
                        if auto.find_language_text("白棉花", ["white", "gossypium"], all_text=True):
                            auto.mouse_click_blank(times=2)
                        sleep(1)
                    if auto.click_element("mirror/shop/purchase_assets.png", take_screenshot=True):
                        sleep(1)
                        auto.click_element(
                            "mirror/road_in_mir/ego_gift_get_confirm_assets.png",
                            take_screenshot=True,
                        )
                        complete_count += 1
                        system_gift = re_sort_points(system_gift)
                        auto.mouse_click_blank(times=3)
                        continue
                    else:
                        if auto.click_element(
                            "mirror/road_in_mir/ego_gift_get_confirm_assets.png", take_screenshot=True
                        ):
                            sleep(0.5)
                        auto.mouse_click_blank(times=3)
                        sleep(1)

            if self.second_system and self.second_system_action[1]:
                if self.second_system_setting == 1 or (self.second_system_setting == 0 and self.fuse_IV is True):
                    system_gift = auto.find_element(
                        f"mirror/shop/enhance_gifts/shop_{self.second_system_select}.png",
                        find_type="image_with_multiple_targets",
                        threshold=0.85,
                    )
                    system_gift = sort_points(system_gift, complete_count)
                    while system_gift:
                        gift = system_gift.pop(0)
                        auto.mouse_action_with_pos((gift[0], gift[1]), offset=True)
                        sleep(1)
                        while auto.take_screenshot() is None:
                            continue
                        if self.system == "bleed" and not cfg.not_skip_whitegossypium:
                            if auto.find_language_text("白棉花", ["white", "gossypium"], all_text=True):
                                auto.mouse_click_blank(times=2)
                            sleep(1)
                        if auto.click_element("mirror/shop/purchase_assets.png", take_screenshot=True):
                            sleep(1)
                            auto.click_element(
                                "mirror/road_in_mir/ego_gift_get_confirm_assets.png",
                                take_screenshot=True,
                            )
                            complete_count += 1
                            system_gift = re_sort_points(system_gift)
                            auto.mouse_click_blank(times=3)
                            continue
                        else:
                            if auto.click_element(
                                "mirror/road_in_mir/ego_gift_get_confirm_assets.png", take_screenshot=True
                            ):
                                sleep(0.5)
                            auto.mouse_click_blank(times=3)
                            sleep(1)

            if retry() is False:
                raise self.RestartGame()

            my_remaining_money = self._get_cost()

            if my_remaining_money < 0:
                log.warning("无法读取剩余金钱，跳过本次刷新")
            elif keyword_refresh_count < self.max_keyword_refresh and my_remaining_money >= 300:
                auto.mouse_click_blank(times=3)
                if auto.click_element("mirror/shop/refresh_keyword_assets.png"):
                    sleep(1)
                    auto.click_element(
                        f"mirror/shop/keyword/keyword_{self.system}.png",
                        take_screenshot=True,
                    )
                    sleep(0.5)
                    auto.click_element("mirror/shop/refresh_keyword_confirm_assets.png")
                    for _ in range(3):
                        if auto.find_element(
                            "mirror/shop/refresh_keyword_confirm_assets.png",
                            take_screenshot=True,
                        ):
                            log.debug("关键词刷新确认未生效，重试中")
                            sleep(0.5)
                            auto.click_element(
                                f"mirror/shop/keyword/keyword_{self.system}.png",
                                take_screenshot=True,
                            )
                            sleep(0.5)
                            auto.click_element(
                                "mirror/shop/refresh_keyword_confirm_assets.png",
                                take_screenshot=True,
                            )
                        else:
                            break
                    keyword_refresh_count += 1
                    auto.mouse_click_blank()
                    wait_for_ui_state(
                        lambda: bool(auto.find_element("mirror/shop/shop_coins_assets.png"))
                        and not auto.find_element("mirror/shop/refresh_keyword_confirm_assets.png"),
                        timeout=3,
                    )
                    if retry(screenshot_ready=True) is False:
                        raise self.RestartGame()
                    if self.skill_replacement and self.replacement < 3:
                        self.replacement_skill()
                    continue

            if normal_refresh_count < self.max_normal_refresh and my_remaining_money >= 200:
                auto.mouse_click_blank(times=3)
                if auto.click_element("mirror/shop/refresh_assets.png"):
                    normal_refresh_count += 1
                    sleep(3)
                    if retry() is False:
                        raise self.RestartGame()
                    if self.skill_replacement and self.replacement < 3:
                        self.replacement_skill()
                    continue

            break

    def _route_fusion_inventory_positions(self):
        """Return the current fusion inventory grid using the existing anchor."""

        points = auto.find_element(
            "mirror/shop/fuse_label.png",
            find_type="image_with_multiple_targets",
            threshold=0.85,
            take_screenshot=True,
        )
        if not points:
            return []
        anchor = max(points, key=lambda point: (point[1], point[0]))
        scale = cfg.set_win_size / 1440
        screen_width, screen_height = self._route_screen_size()
        first_gift = (anchor[0] + 95 * scale, anchor[1] + 135 * scale)
        positions = [
            (
                first_gift[0] + 190 * (index % 5) * scale,
                first_gift[1] + 190 * (index // 5) * scale,
            )
            for index in range(10)
        ]
        return [
            position
            for position in positions
            if 0 <= position[0] < screen_width and 0 <= position[1] < screen_height
        ]

    def _route_result_position(self, recipe, dividing_line):
        """Find a route recipe result without assuming a language."""

        position = auto.find_language_text(
            list(recipe.result_names_zh),
            list(recipe.result_names_en),
        )
        if not position:
            return None
        if dividing_line is not None and position[1] >= dividing_line:
            return None
        return position

    def _select_route_recipe_materials(self, recipe, positions) -> bool:
        """Select each known material once; fail closed when OCR is uncertain."""

        used_positions = []
        for material_id in recipe.material_gift_ids:
            for position in positions:
                if position in used_positions:
                    continue
                if (
                    self._route_gift_id_at_position(
                        position,
                        gift_ids=(material_id,),
                        hover=True,
                    )
                    == material_id
                ):
                    auto.mouse_click(position[0], position[1])
                    used_positions.append(position)
                    break
            else:
                return False
        return True

    def _complete_route_fusion(self) -> bool:
        """Complete one selected recipe with the existing bounded retry loop."""

        for _ in range(15):
            if auto.take_screenshot() is None:
                continue
            if auto.find_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png"):
                auto.click_element(
                    "mirror/road_in_mir/ego_gift_get_confirm_assets.png",
                    take_screenshot=True,
                )
                return True
            if auto.click_element(
                "mirror/shop/enhance_and_fuse_and_sell_confirm_assets.png",
                model="normal",
            ):
                continue
            if retry() is False:
                raise self.RestartGame()
        return False

    def fuse_route_gifts(self) -> bool:
        """Attempt the route's explicit recipes before generic fusion.

        Recipe execution is deliberately conservative: it requires an OCR
        result label, all material labels, and the existing fusion controls.
        If any part is missing, it cancels the selection and leaves the
        generic path to continue with those materials protected.
        """

        if not self.route.recipes or not self.fuse_switch:
            return False
        if self.only_aggressive_fuse or self.only_system_fuse:
            return False

        completed = False
        for recipe in self.route.fusion_recipes_for_floor(self.current_layer):
            if recipe.skip_if_pseudo_solo and self.pseudo_solo_active:
                log.debug(f"进入伪单通状态，跳过多人格路线配方：{recipe.result_gift_id}")
                continue
            if self.enter_fuse(recipe.keyword) is False:
                break
            inventory_positions = self._route_fusion_inventory_positions()
            if recipe.result_gift_id in self._route_fused_gift_ids or any(
                self._route_gift_id_at_position(
                    position,
                    gift_ids=(recipe.result_gift_id,),
                    hover=True,
                    include_recipe_targets=True,
                )
                == recipe.result_gift_id
                for position in inventory_positions
            ):
                log.debug(f"路线成品已存在，跳过重复合成：{recipe.result_gift_id}")
                auto.mouse_click_blank(times=3)
                continue
            points = auto.find_element(
                "mirror/shop/fuse_label.png",
                find_type="image_with_multiple_targets",
                threshold=0.85,
                take_screenshot=True,
            )
            dividing_line = max((point[1] for point in points), default=None)
            result_position = self._route_result_position(recipe, dividing_line)
            if not result_position:
                auto.mouse_click_blank(times=3)
                continue
            auto.mouse_click(result_position[0], result_position[1])
            positions = self._route_fusion_inventory_positions()
            if not self._select_route_recipe_materials(recipe, positions):
                auto.mouse_click_blank(times=3)
                log.debug(f"路线配方材料识别不完整，跳过：{recipe.result_gift_id}")
                continue
            if self._complete_route_fusion():
                completed = True
                self._route_fused_gift_ids.add(recipe.result_gift_id)
                log.info(f"按路线优先合成饰品：{recipe.result_gift_id}")
            else:
                auto.mouse_click_blank(times=3)
                log.debug(f"路线配方合成未完成，跳过：{recipe.result_gift_id}")
        return completed

    def _route_enhance_candidates(self):
        """Find visible route gifts before the generic system upgrade pass."""

        if not self.route.gifts:
            return []
        candidates = []
        seen = []
        for system in all_systems.values():
            points = auto.find_element(
                f"mirror/shop/enhance_gifts/{system}.png",
                find_type="image_with_multiple_targets",
            )
            for point in points or []:
                if not any(
                    abs(point[0] - existing[0]) <= 40 and abs(point[1] - existing[1]) <= 40 for existing in seen
                ):
                    seen.append(point)
        for point in seen:
            priority = self._route_priority_at_position(point, hover=True)
            if priority is not None:
                candidates.append((priority, point))
        candidates.sort(key=lambda item: (item[0], item[1][1], item[1][0]))
        return [point for _, point in candidates]

    def fuse_useless_gifts_aggressive(self):
        """合成无用饰品_激进版"""

        scale = cfg.set_win_size / 1440

        def processing_coordinates(my_gift_list, coordinates, threshold=50):
            """将需要保护的坐标移除出列表"""
            for position in my_gift_list:
                if (
                    abs(position[0] - coordinates[0]) <= threshold * scale
                    and abs(position[1] - coordinates[1]) <= threshold * scale
                ):
                    my_gift_list.pop(my_gift_list.index(position))

            return my_gift_list

        if auto.find_element(
            f"mirror/shop/level_IV_gifts/{self.system}_level_IV.png",
            take_screenshot=True,
            threshold=0.75,
        ):
            self.after_fuse_IV()
            if self.fuse_aggressive_switch is False:
                log.info("已有本体系四级饰品，切换到非激进模式")
                return

        if auto.find_element(
            f"mirror/shop/level_IV_gifts/{self.second_system_select}_level_IV.png",
            take_screenshot=True,
        ):
            if self.fuse_IV is True:
                self.fuse_switch = False
                self.fuse_aggressive_switch = False
                self.fuse_second_IV = True
                log.info("已有本体系、第二体系的四级饰品，切换到出售模式")
                return

        log.debug("开始执行激进合成模块")
        while True:
            auto.mouse_to_blank()
            if auto.take_screenshot() is None:
                continue

            fuse = False
            gift_list = []

            points = auto.find_element(
                "mirror/shop/fuse_label.png", find_type="image_with_multiple_targets", threshold=0.9
            )

            points.sort(key=lambda c: (c[1], c[0]))

            if len(points) >= 1:
                gift = points[-1]
            else:
                return
            if gift:
                first_gift = [gift[0] + 95 * scale, gift[1] + 135 * scale]
                if self.the_first_line_position is None:
                    self.the_first_line_position = first_gift[1] + 100 * scale
                for i in range(10):
                    gift_list.append(
                        [
                            first_gift[0] + 190 * (i % 5) * scale,
                            first_gift[1] + 190 * (i // 5) * scale,
                        ]
                    )
            else:
                return

            protect_list = [
                "mirror/shop/level_IV_gifts/lunar_memory.png",
                f"mirror/shop/level_IV_gifts/{self.system}_level_IV.png",
                "mirror/shop/must_be_abandoned/leave2_vestige2.png",
                "mirror/shop/must_be_abandoned/leave3_vestige.png",
            ]
            if self.second_system and self.second_system_action[0]:
                protect_list.append(f"mirror/shop/level_IV_gifts/{self.second_system_select}_level_IV.png")

            for protect_gift in protect_list:
                if protect_coordinates := auto.find_element(protect_gift, threshold=0.7):
                    gift_list = processing_coordinates(gift_list, protect_coordinates)

            if self.aggressive_save_systems:
                protect_coord = []
                for coord in gift_list:
                    system_bbox = [
                        coord[0],
                        coord[1],
                        coord[0] + 100 * scale,
                        coord[1] + 100 * scale,
                    ]
                    if auto.find_element(
                        f"mirror/shop/enhance_gifts/{self.system}.png",
                        my_crop=system_bbox,
                    ):
                        protect_coord.append(coord)
                for coord in protect_coord:
                    gift_list = processing_coordinates(gift_list, coord)

            # The aggressive path can otherwise select any visible inventory
            # gift.  Remove OCR-matched route targets before the first click.
            gift_list = self._remove_route_targets(gift_list, hover=True)

            # 直到合成概率90%
            for coord in gift_list:
                auto.mouse_click(coord[0], coord[1])
                if auto.find_element(
                    "mirror/shop/fuse_90%_assets.png",
                    threshold=0.97,
                    take_screenshot=True,
                ):
                    break

            # 如果无法合成四级，或可用饰品不足三个，则退出此次合成
            if not auto.find_element("mirror/shop/fuse_90%_assets.png", take_screenshot=True):
                return
            if not auto.find_element("mirror/shop/fusion_level_IV_gift_assets.png", threshold=0.9):
                return

            loop_times = 15
            fuse_use_starlight_chance = 5
            while True:
                auto.mouse_to_blank()
                if auto.take_screenshot() is None:
                    continue

                if ego_gift_get_confirm := auto.find_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png"):
                    if auto.find_language_text(["残片", "罪孽"], ["fragment", "corrosion", "resources"]):
                        fuse = True
                    else:
                        if auto.find_language_text(system_cn_zh[self.system], self.system):
                            self.after_fuse_IV()
                            fuse = False
                    auto.mouse_click(ego_gift_get_confirm[0], ego_gift_get_confirm[1])
                    sleep(0.5)
                    auto.click_element(
                        "mirror/road_in_mir/ego_gift_get_confirm_assets.png",
                        take_screenshot=True,
                    )
                    break

                if fuse_use_starlight_chance > 0 and auto.click_element("mirror/shop/fuse_use_starlight_assets.png"):
                    fuse_use_starlight_chance -= 1
                    auto.click_element(
                        "mirror/shop/fuse_use_starlight_confirm_assets.png",
                        model="normal",
                        take_screenshot=True,
                    )
                    continue

                if auto.click_element(
                    "mirror/shop/enhance_and_fuse_and_sell_confirm_assets.png",
                    model="normal",
                ):
                    sleep(2)
                    if loop_times <= 0:
                        break
                    loop_times -= 1
                    continue

                if retry() is False:
                    raise self.RestartGame()

            if fuse:
                sleep(2)
                auto.click_element("mirror/shop/fuse_ego_gift_assets.png", take_screenshot=True)
                continue

            break

        if retry() is False:
            raise self.RestartGame()

    def fuse_useless_gifts(self):
        """合成无用饰品"""
        scale = cfg.set_win_size / 1440

        def protect_coordinates(my_gift_list, coordinates, threshold=50):
            """将需要保护的坐标移除出列表"""
            for position in my_gift_list:
                if (
                    abs(position[0] - 50 - coordinates[0]) <= threshold * scale
                    and abs(position[1] - 50 - coordinates[1]) <= threshold * scale
                ):
                    my_gift_list.pop(my_gift_list.index(position))

            return my_gift_list

        def processing_coordinates(my_gift_list):
            """将列表从左上到右下排序，然后去重"""

            # 排序
            sorted_list = sorted(my_gift_list, key=lambda x: (x[1], x[0]))

            # 去除重复坐标
            unique_list = []
            for coord in sorted_list:
                if not any(
                    abs(coord[0] - x[0]) <= 40 * scale and abs(coord[1] - x[1]) <= 40 * scale for x in unique_list
                ):
                    unique_list.append(coord)

            # 如果激活激进模式，则过滤第一行的饰品
            if self.fuse_aggressive_switch:
                unique_list = [items for items in unique_list if items[1] >= self.the_first_line_position]

            return unique_list

        log.debug("开始执行普通合成模块")
        block = True
        while True:
            auto.mouse_to_blank()
            if auto.take_screenshot() is None:
                continue

            fuse = False
            select_gifts = False
            gift_list = []

            # 获取无用饰品列表
            for commodity in must_be_abandoned:
                item = auto.find_element(commodity, find_type="image_with_multiple_targets")
                if "white_gossypium" in commodity and "bleed" in self.shop_sell_list:
                    continue
                if item:
                    if isinstance(item, list):
                        gift_list.extend(list(g) for g in item)
                    elif isinstance(item, tuple):
                        gift_list.append(item)
            for sell_system in self.shop_sell_list:
                my_sell_system = f"mirror/shop/enhance_gifts/{sell_system}.png"
                gift = auto.find_element(my_sell_system, find_type="image_with_multiple_targets")
                if gift:
                    if isinstance(gift, list):
                        log.debug(f"识别到{len(gift)}个{sell_system}饰品")
                        gift_list.extend(list(g) for g in gift)
                    elif isinstance(gift, tuple):
                        log.debug(f"识别到1个{sell_system}饰品")
                        gift_list.append(gift)
            # 对饰品位置列表进行排序、去重处理
            my_list = processing_coordinates(gift_list)

            if self.second_system and self.second_system_action[0]:
                if protect_gift := auto.find_element(
                    f"mirror/shop/level_IV_gifts/{self.second_system_select}_level_IV.png"
                ):
                    my_list = protect_coordinates(my_list, protect_gift)

            # Do not consume a route target merely because it is not one of
            # the generic protected system/abandoned assets.
            my_list = self._remove_route_targets(my_list, hover=True)

            # 选择3样无用饰品，不足则退出合成
            if len(my_list) <= 2:
                if block is False:
                    msg = "饰品舍弃列表数量不足，结束合成"
                    log.debug(msg)
                    return
            else:
                for sequence in range(3):
                    try:
                        gift_position = my_list[sequence]
                        auto.mouse_click(gift_position[0], gift_position[1])
                        sleep(0.75)
                    except IndexError:
                        msg = f"饰品舍弃列表已经没有第{sequence + 1}项了"
                        log.debug(msg)
                select_gifts = True

            if select_gifts:
                loop_times = 15
                fuse_IV = False
                while True:
                    auto.mouse_to_blank()
                    if auto.take_screenshot() is None:
                        continue

                    if loop_times < 0:
                        break
                    loop_times -= 1

                    if auto.find_element(
                        "mirror/shop/fusion_level_IV_gift_assets.png", threshold=0.9
                    ) and auto.find_element("mirror/shop/fuse_90%_assets.png"):
                        fuse_IV = True

                    if auto.find_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png"):
                        if fuse_IV:
                            if auto.find_language_text(["残片", "罪孽"], ["fragment", "corrosion", "resources"]):
                                pass
                            else:
                                if auto.find_language_text(system_cn_zh[self.system], self.system):
                                    self.fuse_IV = True
                                    self.fuse_aggressive_switch = False
                                    log.info("合成四级，切换到非激进模式")
                            fuse = True
                            auto.click_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png")
                            sleep(1)
                            auto.click_element(
                                "mirror/road_in_mir/ego_gift_get_confirm_assets.png",
                                take_screenshot=True,
                            )
                            break
                        else:
                            if auto.click_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png"):
                                fuse = True
                                sleep(1)
                                auto.click_element(
                                    "mirror/road_in_mir/ego_gift_get_confirm_assets.png",
                                    take_screenshot=True,
                                )
                                break

                    if auto.click_element(
                        "mirror/shop/enhance_and_fuse_and_sell_confirm_assets.png",
                        model="normal",
                    ):
                        continue

                    if auto.click_element("mirror/shop/fuse_ego_gift_assets.png"):
                        continue

                    if retry() is False:
                        raise self.RestartGame()

            if fuse:
                auto.click_element("mirror/shop/fuse_ego_gift_assets.png", take_screenshot=True)
                continue

            while auto.take_screenshot() is None:
                continue
            list_block = auto.find_element("mirror/shop/gifts_list_block.png")
            if list_block is not None and block:
                block = False
                auto.mouse_drag(list_block[0], list_block[1], drag_time=1, dy=500)
                continue

            auto.mouse_click_blank(times=3)
            break

        if retry() is False:
            raise self.RestartGame()

    def fuse_system_gifts(self):
        """合成体系饰品"""
        auto.model = "clam"
        log.debug("开始执行体系饰品合成模块")

        # 获取合成结果
        system_gifts = []
        for gift in fusion_result:
            gift_name = gift.split("/")[-1]
            g = gift_name.split("_")
            if g[0] == self.system:
                system_gifts.append(gift)

        self.enter_fuse()

        while True:
            if list_block := auto.find_element("mirror/shop/gifts_list_block.png", take_screenshot=True):
                auto.mouse_drag(list_block[0], list_block[1], drag_time=1, dy=-500)
            points = auto.find_element(
                "mirror/shop/fuse_label.png",
                find_type="image_with_multiple_targets",
                threshold=0.85,
                take_screenshot=True,
            )
            if len(points) < 2:
                auto.mouse_click_blank(times=3)
                return

            points.sort(key=lambda c: (c[1], c[0]))

            dividing_line = points[-1][1]

            fusion = False
            fusion_gift = None
            # 如果找到能合成的公式饰品，则合成
            for gift in system_gifts:
                if pos := auto.find_element(gift):
                    if pos[1] > dividing_line:
                        continue
                    auto.mouse_click(pos[0], pos[1])
                    fusion = True
                    fusion_gift = gift.split("_")[-1]
                    break

            fusion_pos = []
            if not fusion:
                all_system_gift = auto.find_element(
                    f"mirror/shop/enhance_gifts/{self.system}.png",
                    find_type="image_with_multiple_targets",
                    take_screenshot=True,
                )
                all_system_gift = self._remove_route_targets(all_system_gift, hover=True)
                if len(all_system_gift) > 0:
                    for g in all_system_gift:
                        if g[1] < dividing_line:
                            fusion_pos.append(g)
                if len(fusion_pos) == 0:
                    auto.mouse_click_blank(times=3)
                    return
                else:
                    fusion = True

            if len(fusion_pos) > 0 and fusion_gift is None:
                pos = fusion_pos.pop(0)
                auto.mouse_click(pos[0], pos[1])

            chance = 5
            while fusion:
                if auto.take_screenshot() is None:
                    continue

                if chance < 0:
                    if len(fusion_pos) > 0:
                        pos = fusion_pos.pop(0)
                        auto.mouse_click(pos[0], pos[1])
                        chance = 5
                    else:
                        auto.mouse_click_blank(times=3)
                        return
                else:
                    retry()
                    chance -= 1

                auto.mouse_to_blank()

                if auto.click_element("mirror/road_in_mir/ego_gift_get_confirm_assets.png"):
                    break

                if auto.click_element(
                    "mirror/shop/enhance_and_fuse_and_sell_confirm_assets.png",
                    model="normal",
                ):
                    continue

                if retry() is False:
                    raise self.RestartGame()

            if fusion:
                msg = f"成功合成{self.system}体系的公式饰品{fusion_gift if fusion_gift else ''}"
                log.debug(msg)

    def sell_gifts(self):
        scale = cfg.set_win_size / 1440

        def protect_coordinates(my_gift, coordinates, threshold=50):
            """将需要保护的坐标移除出列表"""
            if (
                abs(my_gift[0] - coordinates[0]) <= threshold * scale
                and abs(my_gift[1] - coordinates[1]) <= threshold * scale
            ):
                return True

            return False

        list_block = False
        system_sell = True
        sell_chance = 20
        second = None
        log.debug("开始执行饰品出售模块")
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue

            gift_sell = False

            if retry() is False:
                raise self.RestartGame()

            if self.second_system and self.second_system_action[0] and second is None:
                if protect_gift := auto.find_element(
                    f"mirror/shop/level_IV_gifts/{self.second_system_select}_level_IV.png"
                ):
                    second = protect_gift

            if auto.click_element("mirror/shop/sell_gift_assets.png"):
                continue

            if auto.click_element("mirror/shop/sell_gift_confirm_assets.png"):
                sleep(1)
                continue

            if system_sell:
                for sell_system in self.shop_sell_list:
                    my_sell_system = f"mirror/shop/enhance_gifts/{sell_system}.png"
                    if sell_gift := auto.find_element(my_sell_system):
                        if second is not None and protect_coordinates(sell_gift, second):
                            continue
                        if self._route_priority_at_position(sell_gift, hover=True) is not None:
                            log.debug(f"保护当前路线饰品，跳过出售：{sell_system}")
                            continue
                        auto.mouse_click(sell_gift[0], sell_gift[1])
                        sleep(cfg.mouse_action_interval)
                        auto.click_element(
                            "mirror/shop/enhance_and_fuse_and_sell_confirm_assets.png",
                            model="normal",
                        )
                        sleep(1)
                        if retry() is False:
                            raise self.RestartGame()
                        gift_sell = True
                        sell_chance -= 1
                        break

            if gift_sell:
                if sell_chance < 0:
                    return
                continue

            if list_block is False and auto.find_element("mirror/shop/gifts_list_block.png"):
                block_position = auto.find_element("mirror/shop/gifts_list_block.png")
                auto.mouse_drag(block_position[0], block_position[1], drag_time=1, dy=500)
                list_block = True
                second = None
                continue

            system_sell = False

            for commodity in must_be_abandoned:
                if auto.click_element(commodity):
                    auto.click_element(
                        "mirror/shop/enhance_and_fuse_and_sell_confirm_assets.png",
                        model="normal",
                    )
                    sleep(1)
                    gift_sell = True
                    break

            if gift_sell:
                continue

            auto.mouse_click_blank(times=3)
            sleep(1)
            break

    def enter_fuse(self, keyword=None):
        loop_count = 15
        auto.model = "clam"
        auto.mouse_to_blank()
        log.debug("开始执行饰品合成前置模块")
        requested_keyword = keyword
        keyword = keyword or self.system
        while True:
            if auto.take_screenshot() is None:
                continue

            if requested_keyword is None and (
                self.fuse_IV is True
                and self.second_system is True
                and self.fuse_second_IV is False
                and self.second_system_action[0] is True
                and self.fuse_aggressive_switch is True
            ):
                if auto.click_element(f"mirror/shop/keyword/keyword_{self.second_system_select}.png"):
                    while auto.take_screenshot() is None:
                        continue
                    if auto.click_element("mirror/shop/fuse_gift_confirm_assets.png", model="normal"):
                        sleep(0.5)
                        break
            else:
                if auto.click_element(f"mirror/shop/keyword/keyword_{keyword}.png"):
                    while auto.take_screenshot() is None:
                        continue
                    if auto.click_element("mirror/shop/fuse_gift_confirm_assets.png", model="normal"):
                        sleep(0.5)
                        break
            if auto.click_element("mirror/shop/fuse_to_select_keyword_assets.png"):
                continue

            loop_count -= 1
            if loop_count < 10:
                auto.model = "normal"
            if loop_count < 5:
                auto.model = "aggressive"
            if loop_count < 0:
                log.error("无法合成ego饰品")
                return False

            # 进入合成页面
            if auto.click_element("mirror/shop/fuse_gift_assets.png"):
                continue

        return True

    def fuse_gift(self):
        # Route-specific recipes must get the first chance to consume their
        # own materials; the generic fusion passes protect those materials but
        # cannot reconstruct a themed recipe by themselves.
        self.fuse_route_gifts()

        # 激进合成
        if self.fuse_aggressive_switch and not self.only_system_fuse:
            log.debug("开始执行第一次激进合成")
            if self.enter_fuse() is False:
                return False
            self.fuse_useless_gifts_aggressive()
            auto.mouse_click_blank(times=3)
        if self.fuse_switch is False:
            if self.fuse_IV is True and self.after_level_IV is True and self.after_level_IV_select == 3:
                log.debug("合成四级后跳过商店，执行最后一次升级饰品")
                self.enhance_gifts()
            return
        # 普通合成
        if self.only_aggressive_fuse or self.only_system_fuse:
            pass
        else:
            log.debug("开始执行普通合成")
            if self.enter_fuse() is False:
                return False
            self.fuse_useless_gifts()
            auto.mouse_click_blank(times=3)

        # 再次激进合成
        if self.fuse_aggressive_switch and self.fuse_IV is not True and not self.only_system_fuse:
            log.debug("开始执行第二次激进合成")
            if self.enter_fuse() is False:
                return False
            self.fuse_useless_gifts_aggressive()
            auto.mouse_click_blank(times=3)
        if self.fuse_switch is False:
            if self.fuse_IV is True and self.after_level_IV is True and self.after_level_IV_select == 3:
                log.debug("合成四级后跳过商店，执行最后一次升级饰品")
                self.enhance_gifts()
            return
        # 合成体系饰品
        if not self.only_aggressive_fuse and not self.do_not_system_fuse:
            self.fuse_system_gifts()
            auto.mouse_click_blank(times=3)

    def heal_sinner(self):
        # 全体治疗
        loop_count = 10
        auto.model = "clam"
        log.debug("开始执行罪人治疗模块")
        sinner_be_heal = False
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue

            loop_count -= 1
            if loop_count < 5:
                auto.model = "normal"
            if loop_count < 2:
                auto.model = "aggressive"
            if loop_count < 0:
                log.error("治疗罪人失败")
                break

            if sinner_be_heal is True and auto.click_element("mirror/shop/heal_sinner/heal_sinner_return_assets.png"):
                if auto.find_element("mirror/shop/shop_coins_assets.png", take_screenshot=True):
                    break
                elif auto.find_element("mirror/shop/heal_sinner/heal_sinner_return_assets.png"):
                    continue
                elif auto.find_element("mirror/shop/heal_sinner/heal_sinner_return_assets.png"):
                    continue

            if auto.click_element("mirror/shop/heal_sinner/heal_all_sinner_assets.png"):
                sinner_be_heal = True
                continue

            if loop_count < 5:
                money = self._get_cost(in_heal=True)
                if money < 0:
                    log.warning("无法读取剩余金钱，跳过治疗")
                    continue
                if money < 100:
                    log.debug("金币不足，无法治疗罪人")
                    break

            if auto.click_element("mirror/shop/heal_sinner/heal_sinner_assets.png"):
                continue

            if retry() is False:
                raise self.RestartGame()

    def enhance_gifts(self):
        _ENHANCE_SCAN_REGION_REL = {
            "left": 0.5151,
            "top": 0.3380,
            "right": 0.8844,
            "bottom": 0.7130,
        }

        def _filter_enhance_gift_scan_points(points, screen_size):
            if not points or not screen_size:
                return points

            width, height = screen_size
            if not width or not height:
                return points

            left = int(round(_ENHANCE_SCAN_REGION_REL["left"] * width))
            top = int(round(_ENHANCE_SCAN_REGION_REL["top"] * height))
            right = int(round(_ENHANCE_SCAN_REGION_REL["right"] * width))
            bottom = int(round(_ENHANCE_SCAN_REGION_REL["bottom"] * height))

            return [point for point in points if left <= int(point[0]) <= right and top <= int(point[1]) <= bottom]

        def check_enhanced(pos):
            for p in self.enhance_gifts_list:
                if abs(pos[0] - p[0]) <= 10 and abs(pos[1] - p[1]) <= 10:
                    return True
            return False

        log.debug("开始执行饰品升级模块")

        my_scale = cfg.set_win_size / 1440
        loop_try_count = 10
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue
            if button := auto.find_element("mirror/shop/sort_button_assets.png"):
                auto.mouse_click(button[0], button[1])
                auto.mouse_click(button[0], button[1] + 200 * my_scale)
                break
            if auto.click_element("mirror/shop/enhance_gifts_assets.png"):
                sleep(1)
                continue
            auto.mouse_click_blank()
            loop_try_count -= 1
            if loop_try_count < 0:  # issue 171
                if retry() is False:
                    raise self.RestartGame()
                return False

            if loop_try_count < -50:
                log.error("不应该发生这样的问题，请提交issue")
                return False

        loop_count = 30
        auto.model = "clam"
        system_level_IV = False
        second_system_level_IV = False
        stale_count = 0
        while True:
            # 自动截图
            if auto.take_screenshot() is None:
                continue

            next_gift = True

            for route_gift in self._route_enhance_candidates():
                if check_enhanced(route_gift):
                    continue
                auto.mouse_click(route_gift[0], route_gift[1])
                if self.ego_gift_to_power_up() is False:
                    return False
                self.enhance_gifts_list.append(route_gift)
                log.info("按路线优先升级路线饰品")
                next_gift = False
                break

            if next_gift is False:
                continue

            # 升级本体系四级
            if not system_level_IV:
                if level_IV := auto.find_element(f"mirror/shop/level_IV_gifts/{self.system}_level_IV.png"):
                    if check_enhanced(level_IV) is False:
                        auto.mouse_click(level_IV[0], level_IV[1])
                        if self.ego_gift_to_power_up() is False:
                            break
                        else:
                            self.enhance_gifts_list.append(level_IV)
                    system_level_IV = True
                    continue

            # 升级第二体系四级
            if self.second_system and self.second_system_action[3] and second_system_level_IV is False:
                if self.second_system_setting == 1 or (self.second_system_setting == 0 and self.fuse_IV is True):
                    if level_IV_2 := auto.find_element(
                        f"mirror/shop/level_IV_gifts/{self.second_system_select}_level_IV.png"
                    ):
                        if check_enhanced(level_IV_2) is False:
                            auto.mouse_click(level_IV_2[0], level_IV_2[1])
                            if self.ego_gift_to_power_up() is False:
                                break
                            else:
                                self.enhance_gifts_list.append(level_IV_2)
                        second_system_level_IV = True
                        continue

            if self.first_gift_enhance is False and system_level_IV is False:
                if f_gift := auto.find_element(f"mirror/shop/enhance_gifts/big_{self.system}.png"):
                    if self.ego_gift_to_power_up() is False:
                        break
                    else:
                        self.enhance_gifts_list.append(f_gift)
                        self.first_gift_enhance = True
                        continue

            if gifts := auto.find_element(
                f"mirror/shop/enhance_gifts/{self.system}.png",
                find_type="image_with_multiple_targets",
            ):
                gifts = sorted(gifts, key=lambda x: (x[1], x[0]))
                raw_count = len(gifts)
                screen_size = auto.screenshot.size if auto.screenshot is not None else None
                gifts = _filter_enhance_gift_scan_points(gifts, screen_size)
                if len(gifts) != raw_count:
                    log.debug(f"升级扫描区域过滤：{raw_count} -> {len(gifts)}")
                for gift in gifts:
                    if check_enhanced(gift) is False:
                        auto.mouse_click(gift[0], gift[1])
                        if self.ego_gift_to_power_up() is False:
                            next_gift = False
                            break
                        else:
                            self.enhance_gifts_list.append(gift)
                    else:
                        continue
                    next_gift = False

            # if list_block is False and auto.find_element("mirror/shop/gifts_list_block.png"):
            #     block_position = auto.find_element("mirror/shop/gifts_list_block.png")
            #     auto.mouse_drag(block_position[0], block_position[1], drag_time=1, dy=500)
            #     list_block = True
            #     continue

            if next_gift is False:
                break

            stale_count += 1
            if stale_count > 2:
                log.debug("连续多轮未找到可升级饰品，退出升级模块")
                break

            loop_count -= 1
            if loop_count < 20:
                auto.model = "normal"
            if loop_count < 10:
                auto.model = "aggressive"
            if loop_count < 0:
                log.error("升级ego饰品失败")
                break

        if retry() is False:
            raise self.RestartGame()

    def after_fuse_IV(self):
        self.fuse_IV = True
        if self.after_level_IV:
            if self.after_level_IV_select == 0:
                self.fuse_switch = False
                self.fuse_aggressive_switch = False
                log.info("合成四级，切换到出售模式")
            elif self.after_level_IV_select == 1:
                self.fuse_aggressive_switch = False
                log.info("合成四级，切换到非激进模式")
            elif (
                self.after_level_IV_select == 2
                and self.second_system is True
                and self.fuse_second_IV is False
                and self.second_system_action[0] is True
            ):
                self.fuse_aggressive_switch = True
                log.debug("合成四级，切换到第二体系四级合成")
            elif self.after_level_IV_select == 3:
                log.info("合成四级，跳过之后商店")
                for i in range(5):
                    self.ignore_shop[i] = True
            else:
                self.fuse_switch = False
                self.fuse_aggressive_switch = False
                log.info("合成四级但设置出错，切换到出售模式")
            return
        self.fuse_aggressive_switch = False
        log.info("合成四级，切换到非激进模式")

    def id_skill_replacement(self, image_path):
        """
        处理普通技能替换选项。识别指定模块位置的技能替换区域，
        根据当前语言和队伍配置构建优先罪人列表，若匹配则执行替换点击流程。

        参数:
            image_path (str): 代表该技能替换模块的图片路径。
        """
        if module_position := auto.find_element(image_path):
            my_scale = cfg.set_win_size / 1440
            bbox = (
                module_position[0] - 150 * my_scale,
                module_position[1] + 20 * my_scale,
                module_position[0] + 150 * my_scale,
                module_position[1] + 100 * my_scale,
            )
            sinner_nums = {0: 1, 1: 3, 2: 7}.get(self.skill_replacement_select, 12)
            sinner_en = [
                all_sinners_name[self.sinner_team.index(i + 1)]
                for i in range(sinner_nums)
                if (i + 1) in self.sinner_team
            ]
            sinner_zh = [
                all_sinners_name_zh[self.sinner_team.index(i + 1)]
                for i in range(sinner_nums)
                if (i + 1) in self.sinner_team
            ]
            if auto.find_language_text(sinner_zh, sinner_en, my_crop=bbox):
                auto.mouse_click(module_position[0], module_position[1] - 100 * my_scale)
                sleep(0.5)
                coins = auto.find_element(
                    "mirror/shop/skill_replacement_coins.png",
                    find_type="image_with_multiple_targets",
                    take_screenshot=True,
                )
                if len(coins) != 3:
                    return
                coins = sorted(coins, key=lambda x: x[0])
                select_mode = 3 - self.skill_replacement_mode - 1
                auto.mouse_click(coins[select_mode][0], coins[select_mode][1])
                sleep(0.5)
                auto.click_element("mirror/shop/skill_replacement_confirm_assets.png")
                auto.click_element("mirror/shop/skill_replacement_confirm_assets.png")
                # 检测游戏是否异常，若异常则重启游戏
                if retry() is False:
                    raise self.RestartGame()

    def selected_id_skill_replacement(self, image_path):
        """
        处理超级商店中的 "ID 技能替换" 功能。
        点击"ID 技能搜索"按钮进入已选中角色选项界面，
        遍历优先罪人列表尝试替换操作。若当前罪人无技能可替换则跳过并尝试下一个。

        参数:
            image_path (str): 代表该技能替换模块的图片路径。
        """
        my_scale = cfg.set_win_size / 1440
        sinner_x = [
            x * my_scale
            for x in [
                198,
                353,
                510,
                663,
                818,
                968,
                1121,
                1277,
                1428,
                1581,
                1737,
                1889,
            ]
        ]  # 人格图像位置硬编码
        sinner_y = 1299 * my_scale
        sinner_nums = {0: 1, 1: 3, 2: 7}.get(self.skill_replacement_select, 12)
        sinner_indices = [i for i in range(sinner_nums) if (i + 1) in self.sinner_team]
        for i in sinner_indices:
            if auto.find_element("mirror/shop/ID_skill_replace_0_purchased_assets.png"):
                return
            auto.click_element(image_path)
            sleep(0.5)
            auto.mouse_click(sinner_x[i], sinner_y)
            sleep(0.5)
            coins = auto.find_element(
                "mirror/shop/skill_replacement_coins.png",
                find_type="image_with_multiple_targets",
                take_screenshot=True,
            )
            if len(coins) != 3:
                continue
            coins = sorted(coins, key=lambda x: x[0])
            select_mode = 3 - self.skill_replacement_mode - 1
            auto.mouse_click(coins[select_mode][0], coins[select_mode][1])
            sleep(0.5)
            auto.click_element("mirror/shop/skill_replacement_confirm_assets.png")
            auto.click_element("mirror/shop/skill_replacement_confirm_assets.png")
            if retry() is False:
                raise self.RestartGame()
        # 如果所有优先罪人都没有技能可替换，则点击返回按钮退出该界面
        auto.click_element("mirror/shop/ID_skill_replace_search_return_assets.png")

    def replacement_skill(self):
        """
        技能替换主流程管理方法。
        会依据当前身处的商店类型（普通商店 or 超级商店）决定执行的替换逻辑：
        如果是超级商店，则依次对两个普通替换选项以及已选中角色技能搜索选项（如果存在）执行替换操作；
        如果是普通商店，则对唯一的技能替换选项执行操作。
        """
        msg = "执行商店技能替换任务"
        log.debug(msg)
        if any(self.skill_replacement_target_counts) and not self._route_skill_target_logged:
            self._route_skill_target_logged = True
            target = "、".join(
                f"S{index}×{count}" for index, count in enumerate(self.skill_replacement_target_counts, 1)
            )
            log.warning(
                f"当前技能替换界面仅支持全局模式，攻略目标为{target}；"
                f"按配置的1→{2 if self.skill_replacement_mode == 0 else 3}兼容执行"
            )
        self.replacement = 0
        # 检查是否在超级商店
        is_super_shop = auto.find_element("mirror/shop/super_shop_assets.png")
        if is_super_shop:
            log.debug("超级商店")

            # 1. 超级商店存在两个普通技能替换
            if not auto.find_element("mirror/shop/ID_skill_replace_1_purchased_assets.png"):
                self.id_skill_replacement("mirror/shop/super_shop_skill_replacement_1_assets.png")
            else:
                self.replacement += 1
            if not auto.find_element("mirror/shop/ID_skill_replace_2_purchased_assets.png"):
                self.id_skill_replacement("mirror/shop/super_shop_skill_replacement_2_assets.png")
            else:
                self.replacement += 1

            if not auto.find_element("mirror/shop/ID_skill_replace_0_purchased_assets.png"):
                self.selected_id_skill_replacement("mirror/shop/ID_skill_search_assets.png")
            else:
                self.replacement += 1

        # 普通商店流程
        else:
            log.debug("普通商店")
            if not auto.find_element("mirror/shop/ID_skill_replace_0_purchased_assets.png"):
                self.id_skill_replacement("mirror/shop/skill_replacement_assets.png")
            else:
                self.replacement = 3

    # 在商店的处理
    def in_shop(self, layer):
        self.set_layer(layer)
        heal = False
        sell = False
        buy = False
        fuse = False
        enhance = False
        skill = False
        try:
            while True:
                # 忽略楼层商店的情况
                if layer <= 5 and self.ignore_shop[layer - 1]:
                    msg = f"第{layer}楼层商店被忽略"
                    log.info(msg)
                    break

                # 自动截图
                if auto.take_screenshot() is None:
                    continue

                auto.mouse_click_blank(times=3)
                auto.click_element("mirror/shop/return_assets.png")
                sleep(1)

                if self.skill_replacement and skill is False:
                    self.replacement_skill()
                    skill = True
                    continue

                if heal is False:
                    if self.do_not_heal:
                        heal = True
                    else:
                        self.heal_sinner()
                        heal = True
                        continue

                if sell is False:
                    if self.do_not_sell:
                        sell = True
                    else:
                        # 出售无用饰品
                        if self.fuse_switch is False:
                            self.sell_gifts()
                        sell = True
                        continue

                if buy is False:
                    if self.do_not_buy:
                        buy = True
                    else:
                        self.buy_gifts()
                        buy = True
                        continue

                if fuse is False:
                    if self.do_not_fuse:
                        fuse = True
                    else:
                        # 合成饰品
                        if self.fuse_switch:
                            self.fuse_gift()
                        fuse = True
                        continue

                if enhance is False:
                    if self.do_not_enhance:
                        enhance = True
                    elif self.fuse_aggressive_switch and self.aggressive_also_enhance is False:
                        enhance = True
                    else:
                        # 升级体系ego饰品
                        self.enhance_gifts()
                        enhance = True
                        continue

                break

            auto.mouse_click_blank(times=3)

            loop_count = 30
            auto.model = "clam"
            while True:
                # 自动截图
                if auto.take_screenshot() is None:
                    continue

                if retry(screenshot_ready=True) is False:
                    raise self.RestartGame()
                if auto.find_element("mirror/road_in_mir/legend_assets.png"):
                    break
                if auto.click_element("mirror/shop/leave_shop_confirm_assets.png"):
                    continue
                if auto.click_element("mirror/shop/leave_assets.png"):
                    sleep(1)
                    continue
                if auto.click_element("mirror/shop/heal_sinner/heal_sinner_return_assets.png"):
                    continue
                loop_count -= 1
                if loop_count < 20:
                    auto.model = "normal"
                if loop_count < 10:
                    auto.model = "aggressive"
                    auto.mouse_click_blank(times=3)
                if loop_count < 0:
                    log.error("无法退出商店,尝试回到初始界面")
                    back_init_menu()
                    break
        except self.RestartGame:
            log.error("执行商店操作期间出现错误，尝试重启游戏")
            return

    def _get_cost(self, in_heal=False) -> int:
        _MONEY_OCR_TRANSLATION = str.maketrans(
            {
                "O": "0",
                "o": "0",
                "D": "0",
                "Q": "0",
                "I": "1",
                "l": "1",
                "Z": "2",
                "S": "5",
                "G": "6",
                "B": "8",
                "h": "4",
            }
        )

        def _extract_money_from_ocr_texts(texts):
            if not texts:
                return None

            cleaned_texts = [text.strip().replace(" ", "").replace(",", "") for text in texts if text]
            for text in cleaned_texts:
                if text.isdigit():
                    return int(text)

            for text in cleaned_texts:
                normalized = text.translate(_MONEY_OCR_TRANSLATION)
                if normalized.isdigit():
                    return int(normalized)

            return None

        def _retry_money_ocr_with_scaled_crop(money_bbox, scale=2):
            if auto.screenshot is None or money_bbox is None:
                return []

            try:
                crop = auto.screenshot.crop(money_bbox)
                width, height = crop.size
                try:
                    resampling = Image.Resampling.BICUBIC
                except AttributeError:
                    resampling = Image.BICUBIC
                enlarged = crop.resize((width * scale, height * scale), resample=resampling)
                result = ocr.run(enlarged)
                return list(result.txts or [])
            except Exception:
                log.debug("OCR 放大裁剪识别失败", exc_info=True)
                return []

        """获取当前剩余的经费"""
        my_remaining_money = -1
        money_bbox = None
        my_money = None
        try:
            if in_heal:
                money_bbox = ImageUtils.get_bbox(ImageUtils.load_image("mirror/shop/my_money_heal_bbox.png"))
            else:
                money_bbox = ImageUtils.get_bbox(ImageUtils.load_image("mirror/shop/my_money_bbox.png"))
            my_money = auto.get_text_from_screenshot(money_bbox)
            parsed_money = _extract_money_from_ocr_texts(my_money)
            if parsed_money is None:
                scaled_money = _retry_money_ocr_with_scaled_crop(money_bbox, scale=2)
                if scaled_money:
                    log.debug(f"OCR 放大重试结果: {scaled_money}")
                    parsed_money = _extract_money_from_ocr_texts(scaled_money)
                    if parsed_money is not None:
                        my_money = scaled_money
            if parsed_money is not None:
                my_remaining_money = parsed_money
            if parsed_money is None or my_remaining_money < 0:
                log.error("获取剩余金钱失败")
            else:
                log.debug(f"剩余金钱：{my_remaining_money}")
        except Exception as e:
            log.error(f"获取剩余金钱失败：{e}")
        return my_remaining_money
