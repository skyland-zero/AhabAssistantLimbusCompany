# GPUI / Python 后端审查修复计划（2026-09）

来源：2026-09-11 的 UI + Python 后端审查（CI 门禁、跨语言契约、并发、测试隔离、AGENTS.md 合规）。
本文档是修复的执行清单；每一项都包含问题、修复方案、涉及文件和验收方式。

约定：

- **P1** = 阻塞/安全/数据丢失，必须修；
- **P2** = 正确性/健壮性/合规，排期修；
- 每阶段完成后必须通过该阶段的验证命令，全部完成后跑 `AGENTS.md` 第 9 节的完整门禁。

## 阶段 0：文档与基线

- 本文件。
- 基线（修复前实测）：
  - `cargo +nightly test --all-targets`：161 passed。
  - `cargo +nightly fmt --check`：失败（11 处）。
  - `cargo +nightly clippy --all-targets -- -D warnings`：失败（`enum_variant_names`）。
  - `pytest`：407 passed / 2 failed（scrcpy 配置泄漏、supervisor 0.2s 时序），集成测试 3s 超时抖动，且会重写开发者 `config.yaml`。

## 阶段 1：CI 门禁（P1-1）

| ID | 问题 | 修复 | 文件 |
| --- | --- | --- | --- |
| 1.1 | `cargo fmt --check` 失败 | `cargo +nightly fmt --all` 后复核 | `src/app/render.rs`、`src/assets.rs`、`src/components/base.rs`、`src/components/style/mod.rs`、`src/pages/home/cards.rs`、`src/pages/home/stats.rs`、`src/pages/settings/cards/appearance.rs`、`src/pages/settings/mod.rs` |
| 1.2 | `clippy::enum_variant_names`（`ThemeAsset` 全部 `Limbus*`） | 增加 `#[allow(clippy::enum_variant_names)]` 并在枚举文档注明原因 | `src/assets.rs` |

验收：`cargo +nightly fmt --all -- --check`、`cargo +nightly clippy --all-targets -- -D warnings` 全绿。

## 阶段 2：Runner 日志契约（P1-2）

| ID | 问题 | 修复 | 文件 |
| --- | --- | --- | --- |
| 2.1 | Runner `log.entry` 发 `timestamp`（float 秒）/`levelname.lower()`（可能 `warning`/`critical`），adapter 原样透传，wire 别名变成 `log.entry`；Rust `LogEntryPayload` 需要 `ts: i64` + `LogLevel(debug/info/warn/error)`，解析失败被 `if let Ok` 吞掉 → Runner 日志全部丢失 | 在 adapter `_payload` 的 `log.entry` 分支统一规范化：`timestamp`→`ts`（毫秒整数），`warning`→`warn`、`critical`→`error`，`message` 保持；Rust `LogLevel` 增加 `warning`/`critical` 别名，`ts` 增加兼容默认，避免未来生产者再犯 | `module/execution/event_adapter.py`、`gpui-app/src/model/other.rs` |
| 2.2 | 现有测试用伪造的 `ts/level` 形状，测不到真实生产者 | 新增 adapter 级测试（真实 Runner 形状），并让 websocket 测试使用真实形状 | `tests/unit/module/execution/test_event_adapter.py`（新增/补充）、`tests/unit/module/test_websocket_server.py` |

验收：adapter 输出可被 Rust 契约夹具解码；新增测试通过。

## 阶段 3：RunnerSupervisor 锁序（P1-3）

| ID | 问题 | 修复 | 文件 |
| --- | --- | --- | --- |
| 3.1 | `_handle_event` 持 `session.lock` 再取 `self._condition`；`request_stop` 持 `_condition` 再取 `session.lock`，ABBA 可永久死锁 | 事件处理在 `session.lock` 内只更新会话字段/拷贝快照入参，出锁后再修改 supervisor 快照；`request_stop` 先在 `_condition` 内取 `session` 引用与决策，出 `_condition`（或至少在 `session.lock` 前）再写 `session.stop_requested` | `module/execution/supervisor.py` |
| 3.2 | 无并发回归测试 | 增加“事件 reader 与 request_stop 并发”的有界时间测试（注入慢回调/事件洪泛） | `tests/unit/module/execution/test_supervisor.py` |

验收：新增并发测试通过；现有 supervisor 测试全绿。

## 阶段 4：更新签名（P1-4）

| ID | 问题 | 修复 | 文件 |
| --- | --- | --- | --- |
| 4.1 | `AALC_UPDATE_REQUIRE_SIGNATURE` 未设置时缺签名文件直接跳过校验 | 默认强制：仅显式 `0/false/no` 才允许兼容模式；同时在 `checker.start_update` 启动 updater 时注入 `AALC_UPDATE_REQUIRE_SIGNATURE=1`（无签名发布流程可通过显式环境变量退出） | `updater.py`、`module/update/checker.py`、`docs/update-signing.md`（文档同步） |

验收：无签名包默认拒绝；显式关闭时才允许；相关单测更新。

## 阶段 5：测试隔离与时序（P1-5）

| ID | 问题 | 修复 | 文件 |
| --- | --- | --- | --- |
| 5.1 | 集成测试继承真实 `config.yaml`、`runtime_stats.json`、`.aalc-runs`，会改写开发者状态 | `AALC_CONFIG_PATH` 指向 `tmp_path`，并新增 `AALC_THEME_PACK_LIST_PATH`/`AALC_THEME_PACK_WEIGHT_PATH` 覆盖；子进程环境显式快照 | `tests/conftest.py`（新增）、`tests/integration/test_backend_sidecar.py`、`module/__init__.py` |
| 5.2 | 单元测试构造 `BackendApplication(stats_path=None)` 会写真实 `runtime_stats.json` | conftest 在导入期设置隔离的 `AALC_CONFIG_PATH`；`BackendApplication` 仍显式接受 `stats_path` | `tests/conftest.py`、`module/backend_application.py`（保持行为） |
| 5.3 | `test_scrcpy_control` 依赖本机 config（15fps vs 期望 30） | 由 5.1 的隔离配置从 example 取默认，测试不再依赖开发者配置；测试显式断言默认值 | `tests/unit/module/test_scrcpy_control.py` |
| 5.4 | 20–30ms / 0.2s / 1.0s / 3s 阈值在负载下抖动 | 保留“不等待阻塞管道”的语义，把上界放宽到远小于阻塞时长的值（如 5s 阻塞 → 断言 <1s）；RPC 超时按操作分级（device.list 放宽） | `tests/unit/module/execution/test_supervisor.py`、`tests/integration/test_backend_sidecar.py` 等 |

验收：连续运行 3 次 `pytest` 全绿，且 `git status` 不再出现 `config.yaml`/`config_backup` 变更。

## 阶段 6：Python 后端 P2

| ID | 问题 | 修复 | 文件 |
| --- | --- | --- | --- |
| 6.1 | `_cleanup_ledgers` 等 per-run 容器永不清理，`delete()` 后仍可复活 PENDING journal | 在 run finalize/删除时 `pop` per-run 状态；`_cleanup_ledger_for_run` 对已删除的 run 返回 `None` | `module/backend_application.py` |
| 6.2 | Runner `heartbeat` 1Hz 直通 wire，不在 coalesce/drop 集合 | adapter 过滤 `heartbeat`（内部事件不入 wire）；并把 `heartbeat` 加入 `_DROPPABLE_EVENTS` 兜底 | `module/execution/event_adapter.py`、`module/websocket_server.py` |
| 6.3 | `resource_sync` 的 `entry.path` 可穿越 `assets_dir` | manifest 校验拒绝绝对路径/`..`，复制前 `resolve()` 包含性校验 | `module/resource_sync/manifest.py`、`module/resource_sync/service.py` |
| 6.4 | `tasks.setConfig` 数值无范围、浮点无 `isfinite` | 为已知数值字段补充 min/max 与 `math.isfinite` | `module/backend_application.py` |
| 6.5 | `notification.test` 在唯一 mutation lane 上可阻塞 ~30s | 降低单次重试预算（测试通知只发一次、短超时），或把它移到 read lane | `module/backend_application.py`、`module/notification/wxpusher.py` |
| 6.6 | `_emit_status` 持 `_status_lock` 调监听器（含 preview join） | 快照监听器列表，出锁后回调 | `module/device_manager.py` |
| 6.7 | `device_manager.close()` lease 非 none 抛异常；`WebSocketServer.stop()` executor 无超时 | `close()` 捕获并记录；`stop()` 的 executor 关闭加超时/`wait=False` | `module/device_manager.py`、`module/websocket_server.py`、`module/backend_application.py` |
| 6.8 | parent watchdog 只查 PID 存在 | 记录父进程 `create_time`，PID+身份双校验 | `main_backend.py` |
| 6.9 | `_on_task_completed` 无锁读执行状态 | 在 `_lock` 内读取快照 | `module/backend_application.py` |
| 6.10 | `preview_set_enabled`/`_on_device_event` 持 `_lock` 做 `stop_and_wait` | 锁内只改状态，锁外执行停止/启动 | `module/backend_application.py` |
| 6.11 | `device.connect` 接受任意 endpoint | 限制为最近一次发现的目标/合法 endpoint 模式 | `module/device_manager.py` |

验收：`pytest tests/unit/module -q` 全绿；新增 6.1/6.3/6.4/6.8 的单元测试。

## 阶段 7：Rust 状态 / IPC P2

| ID | 问题 | 修复 | 文件 |
| --- | --- | --- | --- |
| 7.1 | Mock 先 stats 后 status，导致第二轮 run 统计残留；真实后端靠发射顺序 | Mock 改为 status 先、stats 后；`accepts_stats_event` 增加“同 run 且 execution 已接受该 run”的准入 | `src/ipc/mock/handlers.rs`、`src/state/home/execution.rs` |
| 7.2 | idle 后带 runId 的 log/notice 被丢 | `LOG_ENTRY`/`APP_NOTICE` 对“当前 run 已完成但事件属于该 run”放宽（保留最近完成 run） | `src/state/home/execution.rs` |
| 7.3 | `restart_sidecar` 无状态复位 | `RpcGateway::restart_sidecar` 返回重启结果并由调用方复位，或删除公开 API；保留路径必须复位 `last_event_sequence`/`execution_event_sequences` | `src/ipc/gateway.rs`、`src/ipc/backend.rs`、`src/app/**` |
| 7.4 | `stopping_since` 只写不读；停止对账一次性 | 删除死字段或用它做二次对账；把停止对账改为有界重试（最多 N 次） | `src/state/home/execution.rs`、`src/app/scheduling.rs` |
| 7.5 | hydration 用阻塞 `recv()` 占用 background executor | `request_async` 改用 `async_channel`（已有依赖），异步等待不再阻塞线程 | `src/ipc/websocket/mod.rs`、`src/ipc/websocket/worker.rs`、`src/ipc/backend.rs`、调用点 |
| 7.6 | `preview_generation_floor`、`execution_event_sequences` 无上限 | 保留当前/最近 N 个 run，其余裁剪 | `src/state/home/execution.rs` |

验收：`cargo test` 全绿，新增 7.1/7.2/7.3 的单测。

## 阶段 8：Rust UI P2 + 合规

| ID | 问题 | 修复 | 文件 |
| --- | --- | --- | --- |
| 8.1 | i18n 泄漏：state 中文字符串 + 页面不完整映射 + stats 硬编码 | 在 `i18n` 增加集中式 `feedback(text, language)` 映射表，页面统一调用；stats 的 `format!` 走 `text()/paired` | `src/i18n/**`、`src/pages/**` |
| 8.2 | 执行工具栏无键盘路径 | 为 `select-all/clear-all/after-completion/pause-resume/start-stop` 增加 `on_key_down` + `is_activation_key` | `src/pages/home/execution_toolbar.rs` |
| 8.3 | 硬编码颜色（device_panel、stats、keycap） | 改为 `palette_rgb(palette.danger/warning/card_foreground)` 等语义 token | `src/pages/home/device_panel.rs`、`src/pages/home/stats.rs`、`src/pages/home/execution_toolbar.rs` |
| 8.4 | 开始时间硬编码 UTC+8 | 由 Python 返回本地时间或 Rust 侧用系统本地偏移换算 | `src/pages/home/stats.rs`（或 `model/stats.rs`） |
| 8.5 | inert 控件仍 `tab_index(0)` | `button/select/switch` 在 inert 时不设置 tab_index/focus_visible | `src/components/base.rs`、`src/components/controls/**` |
| 8.6 | 渲染期改状态 | `resources.rs` 的完成调度、`updates.rs` 的 feedback 改为 state 方法 | `src/pages/resources.rs`、`src/pages/settings/cards/updates.rs`、对应 state |
| 8.7 | 非交互卡片 hover；toolbox 死分支 | 去掉无点击卡片的 hover；删除恒真分支 | `src/pages/teams/list.rs`、`src/pages/toolbox.rs` |
| 8.8 | 两处重复 select 实现 | 抽到 `components/controls/select.rs` 的通用 helper | `src/pages/home/controls.rs`、`src/pages/teams/mod.rs` |
| 8.9 | `CARGO_MANIFEST_DIR` 开发路径 | 保留 debug-only 分支，文档注明；sidecar 查找顺序已有 release 优先项 | `src/assets.rs`、`src/ipc/websocket/sidecar.rs` |

验收：`cargo fmt/check/test/clippy` 全绿；英文界面抽查无中文；键盘可完成 Run/Stop。

## 阶段 9：文件拆分（AGENTS.md §3.2）

| ID | 文件 | 拆分方案 |
| --- | --- | --- |
| 9.1 | `src/pages/home/stats.rs` 1877 行 | `stats/mod.rs`（入口+快照）、`stats/overview.rs`（当前运行/周期概览）、`stats/history.rs`（镜牢历史行）、`stats/details.rs`（统计弹层）、`stats/format.rs`（时间/时长纯函数+测试） |
| 9.2 | `src/pages/teams/overlay.rs` 711 行 | `teams/overlay/{mod,delete,preset,save}.rs` |
| 9.3 | `src/pages/home/tasks.rs` 532 行 | `tasks/{mod,task_card,after_completion}.rs` |
| 9.4 | `src/pages/teams/list.rs` 527 行 | `teams/list/{mod,team_card,empty_slot}.rs`（或合并进 `teams/list.rs` + `teams/list_cards.rs`） |
| 9.5 | 复核所有 >500 行文件 | 剩余 >500 行必须有顶部例外注释（生成代码/静态表） |

验收：`wc -l` 全部 ≤500（例外除外并注明）；`cargo test` 数量不减少。

## 阶段 10：Mock 与契约一致性 P2

| ID | 问题 | 修复 | 文件 |
| --- | --- | --- | --- |
| 10.1 | Mock 对 4 个方法返回 `true`，真实返回对象 | Mock 返回与 Python 同形状的对象 | `src/ipc/mock/handlers.rs` |
| 10.2 | Mock 无 `seq`、无 `screenshot.frame`/`preview.status`/`app.exitRequested`/`log.entry` | Mock 事件带自增 `seq`；补预览/日志/退出事件的最小确定性序列 | `src/ipc/mock/**` |
| 10.3 | Rust `schema_version()` 默认 1 vs 线上 3 | 常量对齐为 3（stats 保持 1） | `src/model/tasks.rs`、`src/model/other.rs`、`src/model/teams.rs` |
| 10.4 | `-32011/-32012` 同名 | 拆分为 `INVALID_EXECUTION_STATE` 与 `INVALID_PAUSE_STATE`/`INVALID_RESUME_STATE`，两端一致 | `module/rpc_dispatcher.py`、`src/ipc/mock/handlers.rs` |

验收：契约测试更新并通过。

## 完整验收（交付前）

```powershell
# gpui-app
cargo +nightly fmt --all -- --check
cargo +nightly check --all-targets
cargo +nightly test --all-targets
cargo +nightly clippy --all-targets -- -D warnings
# 仓库根
uv run pytest -q
uv run python scripts/check_upstream_boundary.py
git status --short   # 不应出现 config.yaml / config_backup / runtime_stats.json 变更
```

## 风险与回滚

- 阶段 3、7.5 涉及并发/传输改动，优先级最高，单独提交并先跑并发/契约测试；
- 阶段 9 是纯移动 + 重导出，保持 `pub use`/`pub(super)` 稳定，行为不变；
- 阶段 4 改变默认安全策略，需要在 `docs/update-signing.md` 同步说明，发布流程如有未签名渠道需显式设置兼容开关。

---

# 执行结果（2026-09-11 完成）

所有阶段均已实施并通过完整门禁。

## 验收命令与结果

| 命令 | 结果 |
| --- | --- |
| `cargo +nightly fmt --all -- --check` | ✅ |
| `cargo +nightly check --all-targets` | ✅ |
| `cargo +nightly test --all-targets` | ✅ 168 passed（修复前 161） |
| `cargo +nightly clippy --all-targets -- -D warnings` | ✅ |
| `pytest tests` | ✅ 437 passed（修复前 407/2 failed/1 flaky） |
| `ruff check .` | ✅（顺带清掉两个 limbus 脚本的 T201） |
| `check_upstream_boundary.py` | ✅（顺带把既有的 `tasks/mirror/reward_card.py`、`vision_regions.py` 记录进 allowlist） |
| `git status`（配置污染） | ✅ 运行测试不再改写 `config.yaml` / `runtime_stats.json` / `.aalc-runs` |

## 与计划的差异与说明

- **10.2 部分实现**：Mock 事件已带自增 `seq`，并在执行开始时发出 `log.entry`；确定性 JPEG 预览帧仍由真实 sidecar 覆盖（Mock 没有可编码的截图内容），已在 Mock 契约测试中保留为显式边界。
- **AGENTS.md §3.2 例外**：仅 `src/assets.rs`（静态资源身份表）与 `src/model/tasks.rs`（单一 serde 契约）保留 >500 行，均已在文件顶部写明例外原因；其余原 >500 行文件全部拆分：
  - `pages/home/stats.rs` → `pages/home/stats/{mod,overview,backend,run,history,details,mirror,format}.rs`；
  - `state/home/execution.rs` → `state/home/execution/{mod,commands,preview,events,tests/*}.rs`；
  - `app/lifecycle.rs` → `app/lifecycle/{mod,bootstrap,construct,recovery}.rs`；
  - `pages/teams/overlay.rs` → `pages/teams/overlay/{mod,render,preset}.rs`；
  - `pages/home/tasks.rs` → `pages/home/tasks/{mod,mirror}.rs`；
  - `pages/teams/list.rs` → `pages/teams/list/{mod,cards}.rs`；
  - `ipc/mock/handlers.rs` → `ipc/mock/handlers/{mod,execution,teams,settings,device}.rs`。
- **额外修复（不在原审查报告内但阻断 CI）**：`upstream-boundary.toml` 缺失的两个 fork 文件白名单、两个程序化主题脚本的 `T201`。

## 遗留与后续

- 真实设备的视觉回归（`scripts/capture_visual.ps1`）未在本机执行，需在有 MuMu/游戏窗口的机器上跑一次 `-Skins limbus` 与动态状态覆盖。
- 本次修复涉及的 `config.yaml` 在修复前已被旧集成测试重写过一次，备份位于 `config_backup/config_20260911_221705.yaml`，如需可自行恢复。
