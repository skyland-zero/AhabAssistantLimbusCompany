# 上游同步实施方案（2026-11）

- 上游范围：`d7a6b376`（merge-base，2026-08-28）→ `origin/main`（2026-11-08，`2fcb1b46`）
- 提交规模：102 个提交（68 个非 merge，4 个纯 `app/`+`i18n/`）
- 逐提交结论见 [`upstream-merge-ledger.md`](./upstream-merge-ledger.md)
- 本文只列**要动手的部分**（P0 + P1），以及明确不做的部分和验证方式
- 状态：**已完成**。落地提交 `8da5f15f`（资源）、`a6a8c7f0`（配置/自动化小修）、`737cac51`（事件判定）、`f713b63e`（镜牢流程 / 进程存活 / 颜色计数）

## 0. 总原则

1. **不做整体 merge**。本分支的 `tasks/mirror`、`module/automation/input_handlers`、`module/device_manager.py` 已被 Runner/sidecar 架构重写，`origin/main` 的 patch 与本分支 patch-id 全不匹配（`git cherry` 全 `+`），直接 merge 会用上游版本覆盖我们自己的路线目录、私有 session、原生 scrcpy 解码与观测饰品模块。
2. **能整份拿的只有资源**（PNG / yaml），代码一律手工移植。
3. 落在 `protected_prefixes` 下的改动必须进 `upstream-boundary.toml` 的 `allowed_protected_files`，并在注释里写明理由。
4. 每个批次单独提交，出问题可单批回滚。

## 1. 分批与文件清单

### 批次 A：纯资源（零代码风险）

| 项 | 上游 | 动作 |
| --- | --- | --- |
| A1 | `2ab8592e` | 覆盖 `assets/images/default/share/home/season_assets.png`（赛季入口改版，旧图导致日常/周常收不到） |
| A2 | `1e0e7420` | 覆盖 `mirror/road_in_mir/acquire_ego_gift_box_assets.png`；新增 dark 主题 `assets/images/dark/share/mirror/road_in_mir/acquire_ego_gift_card.png` |
| A3 | `2abbe5cc` | 覆盖 `mirror/road_to_mir/dreaming_star/convert_star_to_cost_assets.png`、`no_convert_star_to_cost_assets.png` |
| A4 | `3980b585` | 新增 `assets/images/default/share/event/all_sinner_bbox.png`（批次 C 依赖） |

验证：`git status` 只应有二进制/资源变更；`python -c "from utils.image_utils import ImageUtils"` 能加载。

> `50d23593`（rapidocr `font_path`）经核对**不需要**：本项目锁定的 rapidocr 3.9.2 中
> `font_path` 只流入 `VisRes` 绘制路径，且 `get_font_path()` 是 lazy + `lru_cache` 属性，
> 只有真的绘制结果时才打开字体；我们从不绘制，`return_word_box: false` 也不会触发
> 字符字典下载。硬编码一个 Windows 字体路径反而增加脆性，故放弃。

### 批次 B：配置与自动化小修

| 项 | 上游 | 文件 | 内容 |
| --- | --- | --- | --- |
| B1 | `8196e72f` | `tasks/mirror/__init__.py` | **真 bug**：现路径 `./assets/images/share/mirror/shop/...` 不存在，`must_purchase` / `must_be_abandoned` / `fusion_result` 全为空 → 商店"必买/必弃/合成产物"空转。改用 `_collect()` 指向 `assets/images/default/share`（应得 5 / 19 / 31 项） |
| B2 | `ece221b6` | `module/config/config.py` | `migrate_legacy_team_setting_data` 去掉 `np.array`（缺 `opening_bonus_level` 时会产生 object 数组直接抛异常），改为 list + 缺省补 0 |
| B3 | `4669454f` | `tasks/base/make_enkephalin_module.py` | `int(ocr_result[0])` → 先过滤非数字字符 |
| B4 | `f804f807` | `tasks/battle/battle.py` | 镜牢阵亡判定补 `and not infinite_battle` |
| B5 | `1033d5e6` | `tasks/battle/battle.py` | `key_press("enter")` 前补 `sleep(0.5)` |
| B6 | `a3b6766b` | `tasks/mirror/mirror.py` | 奖励选择：`0/2`、`2/2`、`1/2` 分支放到 `0/1`… 之前（`1/2` 表示"2 选已选 1"，旧顺序会只拿 1 个） |
| B7 | `a77f9b8e` | `tasks/mirror/mirror.py` | `to_window_assets` 三处 `threshold=0.75` |
| B8 | `1d7516c6` | `tasks/mirror/mirror.py` | `teams/12_sinner_live_assets.png` 加 `threshold=0.75` |
| B9 | `f4c744d7` | `assets/config/theme_pack_list.example.yaml`、`module/config/theme_pack_catalog.py` | 补 5 个新主题包：`campout/开店`、`anarchy/纷繁`、`creed/保存的`、`readvent/暴食`（中英权重必须一致，由 `tests/test_theme_pack_catalog.py` 校验）。封面 `assets/app/theme_packs/*.png` 属 Qt 界面，不取 |

### 批次 C：事件判定（依赖 A4）

| 项 | 上游 | 文件 | 内容 |
| --- | --- | --- | --- |
| C1 | `3980b585` | `tasks/event/event_handling.py` | 彩色截图 + 罪人区域红色涂黑 → 灰度找 极高/高/一般；失败再换原流程找 低/极低。修复 very_low 被灰度匹配误判成 very_high（#994） |
| C2 | `18deb77c` | `tasks/event/event_handling.py`、`tasks/mirror/mirror.py` | 新增 `click_option_avoiding_unknown()`；单通（`defense_for_solo`）时遇到"？？？"选项改选邻近其它选项，规避诅咒饰品 |

### 批次 D：镜牢流程与进程存活

| 项 | 上游 | 文件 | 内容 |
| --- | --- | --- | --- |
| D1 | `0f456e36` + `732002f2` | `tasks/mirror/mirror.py` | `select_init_ego_gift()`：`loop_count` 递减提到循环顶部（现在放在底部，`continue` 能绕过 → 有死循环风险）；饰品点击改为 阈值 0.8 → 0.7 → 锚点坐标 三级降级，并加 `_click_init_gift_row()` |
| D2 | `55ae1715` + `2abbe5cc` | `tasks/mirror/mirror.py` | 无限镜牢进入判定加 `take_screenshot=True`、关键词补 `"of"`；星光 `coins_assets` 改 `model="normal"`；观测饰品拖拽 `sleep(1)` → `1.5` |
| D3 | `998c751e`（部分） | `utils/image_utils.py`、`tasks/mirror/mirror.py` | 移植 `ImageUtils.match_color_regions()`（1440 基准 ROI 的颜色连通域计数）；在 `get_which_floor()` 里用 CLEAR 红字计数作为 `not_passed_floor` 裁剪区/全屏计数**不一致时的仲裁***。不替换现有识别主路径（见 §3） |
| D4 | `fb2ef56d` | `module/game_and_screen/game.py` | `check_game_alive()` 加 `win32gui.FindWindow("UnityWndClass", ...)`：进程在但 180s 无窗口 = 僵尸进程，清理后允许真正重启 |

## 2. 需要登记 `upstream-boundary.toml`

`tasks/mirror/__init__.py`、`tasks/event/event_handling.py` 目前不在允许清单里（其它文件已在）。两条都要写清"为什么由本分支持有"。

## 3. 明确不合并（附理由，详见台账）

- **BlueStacks / 远程 ADB 整套**（`b7bd4bb9` `8a0e08f0` `0fdcba12` `4a827f51` `1da957dd` `4f586968`）：给"仅 MuMu"的旧架构加第二套 input handler；本分支 `device_manager` 已用 ADB/scrcpy 统一接管并识别蓝叠（`device_manager.py:1810`），再叠一套会和 `runner_policy` 冲突。`4f586968` 里 `screenshot.py` / `kill_game()` 的空连接守卫本方已用 Runner 私有 session + 显式 `ConnectionError` 实现。
- **楼层识别主路径迁移**（`ddc8022d` `f9d4d38e` `ac36d70c` `68d1697a` `10fced18` `5d38271a`）：本分支是"静默 OCR 读层数字（多帧融合投票）→ 设置弹窗标记 → `MirrorPlanRuntime.detect_floor()`"，失败重试预算（`MIRROR_FLOOR_DETECT_MAX_ATTEMPTS`）已等价于上游"本层失败不再重试"。仅移植其通用颜色工具函数（D3）。
- **§3 补充：`50d23593` rapidocr 字体**：当前 rapidocr 版本用不到该配置项，理由见批次 A 的说明。
- **观测饰品重写**（`c9117238` `23e41f08`）：我们有 `module/observe_ego_gift.py`，上游版本还带 `print` 调试输出。
- **MuMu 连接/编队拖拽**（`44717602` `cae50bfb`）：本方已有 `ca72c0b6` 编队安全点击与自有连接重建。
- **收尾动作抑制**（`39e75516` `29c30716`）：本分支收尾动作由侧车执行（`module/after_completion.py`），"队列还有待启动任务"的状态在 `nsh-app` Rust 侧，要表达必须动 `contract/contract.json`。作为独立需求处理，不搬 Qt 实现。
- **全部 `app/`、`i18n/`、`assets/logo`、`tasks/tools/*`（Qt 小工具）**：产品已删除 Qt 层（`expected_deleted_prefixes = ["app/"]`）。

## 4. 验证

```powershell
uv run python scripts/check_upstream_boundary.py
uv run python scripts/check_no_legacy_client_refs.py
uv run ruff check .
uv run pytest tests/unit tests/test_theme_pack_catalog.py -q
```

实机回归（有条件时）：
1. 镜牢普牢一局：开局饰品（D1）、商店必买（B1）、战斗后奖励选择（B6）。
2. 困牢单通：事件判定等级（C1）、"？？？"规避（C2）。
3. 日常/周常：赛季入口（A1）。
4. 体力换饼：OCR 解析（B3）。
5. 深色主题：饰品获取弹窗识别（A2）。

## 5. 风险与回滚

| 风险 | 处置 |
| --- | --- |
| D1 改动镜牢开局饰品循环，若坐标锚点偏移会导致点错行 | 锚点只在两次模板匹配都失败后使用，且保留原有 `retry()`/`loop_count` 退出条件；出问题单批 revert |
| D3 仲裁逻辑误判 CLEAR 计数 | 只在"裁剪区与全屏标记数不一致"这条既有告警分支里介入，其余路径不变 |
| C1 涂黑红色可能连带涂掉金色徽章边缘 | 涂黑区域限 `all_sinner_bbox.png` 内，且失败后仍回退原 OCR 流程 |
| B9 新增主题包关键词过短会遮蔽其它包 | `tests/test_theme_pack_catalog.py` 的"英文关键词互不遮蔽"用例会拦住 |

每批次一个提交，提交信息带上游短哈希，便于 `git revert` 与台账对照。
