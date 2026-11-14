# 上游合并台账

记录**每一次**审视过的 `origin/main` 上游提交及其处置结论，避免重复评估、重复合并或误合已放弃的改动。

## 维护规则

1. 每次同步（流程见 [`upstream-sync.md`](./upstream-sync.md)）新增一节，标题写明 merge-base 与目标 `origin/main` 的日期/提交。
2. 每个**非 merge** 提交必须占一行。merge 提交只是发布动作，不记录。
3. 状态取值：
   - `已合并` —— 上游改动已落到本分支，写明批次与本分支文件。
   - `部分合并` —— 只取其中一部分，写明取了什么、放弃什么。
   - `已存在` —— 本分支已有等价或更强的实现，写明自有锚点（文件/提交），便于下次核对。
   - `放弃` —— 明确不合并，**必须写理由**；除非理由中的前提变化，不再重复评估。
   - `待定` —— 尚未决策，应尽快消灭。
4. 短哈希在本仓库可直接 `git cat-file -e <hash>` 校验。
5. 落地改动请按批次单独提交，提交信息带上游短哈希，便于台账 ↔ `git revert` 双向定位。

## 2026-11 同步（merge-base `d7a6b376` 2026-08-28 → `origin/main` 2026-11-08 `2fcb1b46`）

共 102 个提交，其中非 merge 68 个（含 4 个纯 `app/`+`i18n/`）。实施方案：[`upstream-sync-2026-11-plan.md`](./upstream-sync-2026-11-plan.md)。

| 提交 | 日期 | 标题 | 状态 | 说明 |
| --- | --- | --- | --- | --- |
| `8a0e08f0` | 07-27 | feat(simulator): support remote adb endpoints | 放弃 | 属 BlueStacks/远程 ADB 旧架构整套；本分支 `device_manager` 已用 ADB/scrcpy 统一接管 |
| `0fdcba12` | 08-07 | fix(simulator): bound adb game startup | 放弃 | 同上 |
| `b7bd4bb9` | 08-10 | feat(simulator): add BlueStacks automation | 放弃 | 同上；`device_manager.py:1810` 已识别蓝叠 |
| `4a827f51` | 08-11 | fix(bluestacks): prevent team list swipe reordering | 放弃 | 同上 |
| `4f586968` | 08-14 | fix(bluestacks): harden ADB recovery and connection lifecycle | 已存在 | 本方为 Runner 私有 session + 显式 `ConnectionError`（`screenshot.py:353-410`、`retry.py:117-135`） |
| `1d5fe025` | 08-14 | chore: keep simulator behavior tests local | 已存在 | `tests/unit/module/test_scrcpy_control.py`、`test_mumu_control.py`、`test_runner_policy.py` 已覆盖 |
| `094d75a1` | 08-22 | ⚙️ chore(tools): 重置安全检查器状态 | 放弃 | `tasks/tools/*` 是 Qt 小工具，侧车不走 |
| `cd7acce3` | 08-28 | chore: 恢复auto接口函数的签名 | 已存在 | `automation.py:99-120` 显式包装 `mouse_*`/`key_*`/`input_*` |
| `ddc8022d` | 08-30 | ⚙️ chore(mirror): 优化楼层检测流程 | 放弃 | 本方 `get_which_floor()`（`mirror.py:2471`）流程不同，上游"点设置→点退出坐标"不适用 |
| `f804f807` | 08-30 | ⚙️ chore(battle): 优化镜像战斗中的死亡判定逻辑 fix #858 | 已合并 | 批次 B4，`tasks/battle/battle.py` 阵亡分支补 `not infinite_battle` |
| `567f2cc4` | 08-30 | ⚙️ chore(base): 添加困牢执行次数限制的日志提示 fix #882 | 放弃 | `tasks/base/script_task_scheme.py` 已被 Runner 重写，日志点不存在 |
| `7779efad` | 08-30 | ⚙️ chore(tasks): 更新自动换体小工具图片初始化 fix #864 | 放弃 | Qt 小工具（`tasks/tools/production_module.py`） |
| `4669454f` | 08-30 | 🐛 fix(make_enkephalin_module): 修复OCR解析的鲁棒性 fix #851 | 已合并 | 批次 B3，OCR 结果先过滤非数字 |
| `44717602` | 08-30 | 🐛 fix(mumu-control): 修复无标准流环境下Nemu连接异常 fix #846 | 已存在 | `mumu_control.py` 自有连接重建与 `get_mumu_adb_port` 重试链 |
| `1da957dd` | 09-01 | 🐛 fix(simulator): 改进 BlueStacks 设备连接错误处理 | 放弃 | 同 `b7bd4bb9` |
| `19ac7029` | 09-02 | 🎨 style(setting_interface): 优化模拟器设置界面交互与文案 | 放弃 | Qt 界面 |
| `ece221b6` | 09-03 | fix: 修复迁移星光时由于对应值不存在导致的报错 | 已合并 | 批次 B2，`migrate_legacy_team_setting_data` 去 `np.array` |
| `ddacf3ff` | 09-03 | feat(config): add hard-mode floor setting | 已存在 | `auto_hard_mirror` + `hard_mirror_target_floors`（`config_typing.py:265/517`） |
| `e4d634cc` | 09-03 | feat(mirror): switch to hard mode from selected floor | 已存在 | 同上；`select_theme_pack.py:233` 已按 `hard_switch` 走 |
| `1033d5e6` | 09-06 | 🐛 fix(battle): 修复战斗流程中按键按下时机过早的问题 | 已合并 | 批次 B5，`key_press("enter")` 前 `sleep(0.5)` |
| `1d7516c6` | 09-06 | ⚙️ chore(mirror): 调整 12 罪人存活状态图片识别阈值 | 已合并 | 批次 B8，`threshold=0.75` |
| `cae50bfb` | 09-06 | fix(mumu): 修正 MuMu 编队列表起拖误触、惯性漂移及队伍定位偏差 | 已存在 | 本方 `ca72c0b6` 编队安全点击保护 + `team_formation.py` 自有实现 |
| `40283e0c` | 09-09 | ✨ fix(mirror): 添加等待加载检测以避免结算记录丢失 | 已存在 | `battle.py:461`、`back_init_menu.py:100` 已有 `waiting_assets`；结算走自有重试阶梯 `62dc1ce8` |
| `ca28d722` | 09-09 | ⚙️ chore(setting_interface): 更新模拟器 ADB 配置提示文案 | 放弃 | Qt 界面 |
| `fe2f875d` | 09-09 | 🌐 i18n: 同步英文翻译文件 | 放弃 | i18n |
| `a77f9b8e` | 09-09 | ⚙️ chore(mirror): 调整镜牢离开窗口按钮的识别阈值 fix #893 | 已合并 | 批次 B7，`to_window_assets` 三处 `threshold=0.75` |
| `f9d4d38e` | 09-10 | 🐛 fix(mirror): 修复镜牢楼层识别失败的问题，新增兜底逻辑 fix #897 | 已存在 | `_read_absolute_floor()` 多帧融合投票 + 弹窗标记兜底 + `MIRROR_FLOOR_DETECT_MAX_ATTEMPTS` |
| `983c4132` | 09-14 | fix(automation): guard empty feature matching inputs | 已存在 | `_unavailable_feature_templates`（`automation.py:75/1620`）；本方 `87db7484` |
| `f2590e5d` | 09-14 | fix(task): queue automatic starts and skip unattended update prompts | 放弃 | 队列在 `nsh-app`，Runner 侧无该逻辑点 |
| `c9894e58` | 09-19 | 🐛 fix(automation): 修复截图裁剪后尺寸为空导致的崩溃 | 已存在 | 同 `983c4132`，`shape` 校验已含 |
| `2ab8592e` | 09-22 | fix: 修复赛季更新无法领取日常和周常的问题 | 已合并 | 批次 A1，换 `home/season_assets.png` |
| `812c70d0` | 09-22 | ⚙️ chore(daily): 扩展赛季文本识别关键词以兼容OCR误识别 | 已存在 | `get_prize.py` 已含 `["season", "seasun"]` |
| `54e94e5b` | 09-22 | ✨ feat(daily): 增加收取奖励的备用重试机制 | 已存在 | `get_prize.py` 的 `last_try` + `_mail_fallback_position()` |
| `50d23593` | 09-23 | ⚙️ chore(config): 优化 rapidocr 字体配置 | 放弃 | 本分支锁定的 rapidocr 3.9.2 中 `font_path` 只用于 `VisRes` 绘制，且 `get_font_path()` 为 lazy + `lru_cache` 属性；不绘制结果就不打开字体，`return_word_box=false` 也不会触发字典下载。硬编码 Windows 字体路径反而增加脆性 |
| `438cd158` | 09-26 | feat: add selectable priority for all-to-3 skill replacement | 已存在 | `skill_replacement_mode_sequence()` + `_replace_selected_skill()`（`in_shop.py:17/1779`） |
| `d1044dfd` | 09-30 | fix(mumu): NemuIpc 截图通道序修正为 RGBA | 已存在 | 本方 `9150db90` |
| `998c751e` | 09-30 | feat(mirror): 楼层识别改为 CLEAR 红字母颜色计数 fix #927 | 部分合并 | 批次 D3：取 `ImageUtils.match_color_regions()`，仅在设置弹窗“裁剪区/全屏未通过标记数不一致”时用 CLEAR 红字计数仲裁；**不换**识别主路径 |
| `b89442ce` | 09-30 | chore: keep skill replacement tests local only | 已存在 | `tests/unit/tasks/test_in_shop.py` 已覆盖 |
| `e922260d` | 09-30 | fix(mirror): 修复遗留奖励单次领取 | 已存在 | 已一致（奖励加成选择在点击领取之后） |
| `ac36d70c` | 10-01 | fix(mirror): restore unknown floor detection and distinguish matching errors | 放弃 | 控制流由 `MirrorPlanRuntime` 承担，见 `998c751e` |
| `68d1697a` | 10-02 | fix(mirror): decouple floor timing from recognition | 放弃 | 本方 `MirrorPlanRuntime.floor_times` + 卡包页计时已解耦 |
| `10fced18` | 10-03 | fix(mirror): 楼层识别失败后本层内不再重试，进入新层再识别 | 已存在 | `_should_detect_floor()` / `_record_floor_detection()`（`mirror.py:240-253`） |
| `5d38271a` | 10-05 | fix(mirror): 完善楼层独立计时与卡包页重试处理 | 放弃 | 同 `68d1697a` |
| `9c592a9a` | 10-05 | chore(tests): 移除楼层识别与计时测试文件 | 已存在 | `tests/unit/tasks/test_mirror_floor_detection.py`、`test_mirror_plan_runtime.py` |
| `55847b6a` | 10-07 | fix(mirror): 修复镜牢寻路的三处崩溃 | 已存在 | 本方 numpy NMS（`argmax`）不触发 `cv2.minMaxLoc` 一维问题；空节点/阈值守卫已有 |
| `9746bdf9` | 09-14 | feat(settings): make default task stall timeout configurable | 已存在 | `task_stall_timeout`（`config_typing.py:388`，`retry.py`/`back_init_menu.py` 在用） |
| `0a79fa25` | 10-08 | fix(config): keep stall timeout default in example config | 已存在 | `assets/config/config.example.yaml` 已含 |
| `2b88666f` | 10-08 | fix(recovery): honor configured return-home timeout | 已存在 | `back_init_menu.py` 用配置值 |
| `39e75516` | 10-09 | ✨ feat(收尾动作): 优化排队任务的收尾动作抑制逻辑 | 放弃 | 收尾由侧车执行（`module/after_completion.py`），"队列待启动"状态在 `nsh-app` Rust 侧，需改契约；按独立需求处理 |
| `29c30716` | 10-09 | ♻️ refactor(completion): 简化收尾动作抑制状态管理 | 放弃 | 同 `39e75516` |
| `c8b96556` | 10-15 | 🌐 i18n(settings): 简化任务卡死超时说明文案 | 放弃 | i18n |
| `23e41f08` | 10-15 | ♻️ refactor(mirror): 优化镜牢观测饰品识别逻辑 | 已存在 | 本方 `module/observe_ego_gift.py` + `tests/unit/module/test_observe_ego_gift.py` |
| `c9117238` | 10-16 | ♻️ refactor(mirror): 重写镜牢观测饰品体系选择逻辑 | 放弃 | 同上，且上游版本含 `print` 调试输出 |
| `2abbe5cc` | 10-18 | 🐛 fix(mirror): 修复镜牢进入与星光界面逻辑 | 部分合并 | 批次 A3（星光两张图）+ D2（`"of"` 关键词、`coins` 用 `model="normal"`） |
| `218c6f75` | 10-18 | ⚙️ chore(config): 统一配置模块导入路径 | 已存在 | `get_prize.py` 已 `from module.config import cfg` |
| `55ae1715` | 10-18 | ⚙️ chore(mirror): 优化镜牢自动流程的交互稳定性 | 已合并 | 批次 D2，无限镜牢进入 `take_screenshot=True`、拖拽 `sleep(1.5)` |
| `a276e2f0` | 10-19 | 修改aalc图标 | 放弃 | 品牌资源自有 |
| `1e0e7420` | 11-01 | ✨ feat: 更新Dark主题识别图片 | 已合并 | 批次 A2，`acquire_ego_gift_box_assets.png` 更新 + dark `acquire_ego_gift_card.png` 新增 |
| `f4c744d7` | 11-02 | ⚙️ chore(theme-pack): 补充新主题包的映射配置 | 已合并 | 批次 B9，`theme_pack_list.example.yaml` + `theme_pack_catalog.py`（封面 PNG 属 Qt，不取） |
| `3980b585` | 11-04 | 🐛 fix(event): 根据issue 994 修复判定等级识别误判问题 | 已合并 | 批次 C1 + A4（`event/all_sinner_bbox.png`） |
| `8196e72f` | 11-04 | ♻️ refactor(mirror): 重构商店物品列表的加载逻辑，修复商店部分问题 | 已合并 | 批次 B1，修复 `must_purchase`/`must_be_abandoned`/`fusion_result` 全空的真 bug |
| `732002f2` | 11-05 | ♻️ refactor(mirror): 合并开局饰品点击的重复分支为统一循环 | 已合并 | 批次 D1 |
| `0f456e36` | 11-05 | 🐛 fix(mirror): 修复普牢无法选择开局饰品问题 #fix 987 | 已合并 | 批次 D1，`loop_count` 上移 + 阈值/锚点三级降级 |
| `18deb77c` | 11-06 | ✨ feat(mirror): 单通模式规避事件诅咒饰品选项 | 已合并 | 批次 C2，`click_option_avoiding_unknown()` |
| `fb2ef56d` | 11-07 | 🐛 fix(game): 修复僵尸进程导致游戏无法重启 | 已合并 | 批次 D4，`check_game_alive()` 加窗口校验与僵尸清理 |
| `a3b6766b` | 11-08 | 🐛 fix(mirror): 修正镜牢奖励选择数量判断逻辑 fix #917 | 已合并 | 批次 B6，奖励选择分支顺序 |
| `3fa52b5f` | 11-08 | ✨ feat(setting): 添加更新预览版切换时的自动检查 | 放弃 | Qt 界面 |
| `d4c90041` | 11-08 | ⚡️ perf(mirror): 缓存 onnx 推理会话避免重复加载模型 | 已存在 | `search_road.py` `_node_detector_session` + DirectML/CPU 回退 |

## 落地明细

### 2026-11（批次 A–D）

| 批次 | 上游 | 本分支改动 | 落地提交 |
| --- | --- | --- | --- |
| A1 | `2ab8592e` | `assets/images/default/share/home/season_assets.png` | `8da5f15f` |
| A2 | `1e0e7420` | `assets/images/default/share/mirror/road_in_mir/acquire_ego_gift_box_assets.png`、`assets/images/dark/share/mirror/road_in_mir/acquire_ego_gift_card.png`（新增） | `8da5f15f` |
| A3 | `2abbe5cc` | `assets/images/default/share/mirror/road_to_mir/dreaming_star/{convert,no_convert}_star_to_cost_assets.png` | `8da5f15f` |
| A4 | `3980b585` | `assets/images/default/share/event/all_sinner_bbox.png`（新增） | `8da5f15f` |
| B1 | `8196e72f` | `tasks/mirror/__init__.py`（`_collect()`，枚举 `assets/images/default/share`） | `a6a8c7f0` |
| B2 | `ece221b6` | `module/config/config.py`（`migrate_legacy_team_setting_data`） | `a6a8c7f0` |
| B3 | `4669454f` | `tasks/base/make_enkephalin_module.py` | `a6a8c7f0` |
| B4 `B5` | `f804f807` `1033d5e6` | `tasks/battle/battle.py` | `a6a8c7f0` |
| B9 | `f4c744d7` | `assets/config/theme_pack_list.example.yaml`、`module/config/theme_pack_catalog.py` | `a6a8c7f0` |
| C1 | `3980b585` | `tasks/event/event_handling.py`（`WARN_LEVEL_ASSETS`、`_click_first_hit`、`black_out_red`） | `737cac51` |
| B6 `B7` `B8` | `a3b6766b` `a77f9b8e` `1d7516c6` | `tasks/mirror/mirror.py` | `f713b63e` |
| C2 | `18deb77c` | `tasks/event/event_handling.py`、`tasks/mirror/mirror.py` | `737cac51` / `f713b63e` |
| D1 `D2` | `0f456e36` `732002f2` `55ae1715` `2abbe5cc` | `tasks/mirror/mirror.py` | `f713b63e` |
| D3 | `998c751e` | `utils/image_utils.py`（`match_color_regions`）、`tasks/mirror/mirror.py`（`_count_clear_badges` 仲裁） | `f713b63e` |
| D4 | `fb2ef56d` | `module/game_and_screen/game.py`（`_has_game_window`） | `f713b63e` |
