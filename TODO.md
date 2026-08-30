# 待办事项

- [x] 将 GPUI 启动窗口默认宽度从 860 px 调整为 820 px（默认高度保持 680 px；最小尺寸保持 800×560 px，最小宽度仍为 800 px）。
- [x] 将纽本次数默认逻辑恢复为旧 Qt 语义：新配置或缺少字段时默认 3；已有配置中的显式值（包括 0）保持不变；同步 GPUI 默认值与 Python 后端缺省值，不自动迁移当前本地配置。
  - 实施目标：新配置或缺少 `set_thread_count` 字段时显示并使用 `3`；已有配置中的显式值原样保留，`0` 继续表示不执行纽本。
  - GPUI：修改 `gpui-app/src/model/tasks.rs` 中 `DailyTaskConfig::default()` 的 `set_thread_count` 为 `3`，并同步更新默认值测试和 Mock RPC 测试。
  - Python 后端：修改 `module/backend_application.py` 中 `tasks_get_config()` 对缺失字段的缺省值为 `3`；不得将已有配置中的 `0` 转换为 `3`。
  - 配置来源：保持 `assets/config/config.example.yaml` 的 `set_thread_count: 3`；保持 `module/config/config.py` 的加载优先级，即 `config.yaml` 显式值覆盖示例默认值。
  - 兼容范围：不自动修改当前被 Git 忽略的 `config.yaml`，不新增配置迁移，不改变 RPC 字段或配置文件格式；执行目标计算中的异常安全兜底 `0` 保持不变。
  - 测试场景：验证 GPUI 默认值和 Mock 返回值为 `3`；验证后端对显式 `0` 返回 `0`；验证字段缺失时返回 `3`；验证其他已保存数值不被覆盖。
  - 完成验证：执行 `cargo +nightly fmt --all -- --check`、`cargo +nightly check --all-targets`、`cargo +nightly test --all-targets`、Python 单元测试和 `git diff --check`。

- [x] 集成 WxPusher 任务通知（个人 SPT）。
  - 使用 WxPusher App/桌面端接收通知，SPT 在设置页配置；v1 不实现 Telegram、通用 Webhook 或多用户 UID。
  - 不发送原始日志、debug 日志和逐条 warning，只发送任务完成/失败摘要。
  - 每次成功完成事件发送一条进度通知：镜牢等单次事件逐次发送；EXP/线程等批量事件将 `N` 次合并为一条“完成 N 次”通知；无限模式仍按完成事件发送。
  - 整个任务正常结束后发送最终汇总；未捕获异常发送失败摘要；手动停止不发送远程通知。
  - 新增 WxPusher 通知服务和后台 FIFO 发送队列，发送频率限制在约 2 QPS；通知失败只记录本地日志，不影响任务执行。程序强制关闭时，尚未发送的通知允许丢失。
  - 配置新增 `wxpusher_spt`，同步更新 Python 配置模型、默认配置、Rust 设置模型和 Mock；默认值为空，SPT 不写入代码仓库。
  - 设置页新增 SPT 遮蔽输入、保存按钮和“发送测试通知”按钮；新增通知测试 RPC，支持使用未保存的当前输入测试，错误信息不得回显 SPT。
  - 对配置快照、配置变更日志和异常日志统一脱敏，避免 SPT 写入 `debugLog.log` 或发送到前端。
  - 保留通知服务抽象，后续可在同一接口下增加 Telegram、Webhook 等渠道。
  - 测试：验证 WxPusher 成功/API 错误/HTTP 错误/超时/重试、请求体和 SPT 脱敏；验证单次完成、批量 `N`、最终汇总、异常失败、手动停止、未配置 SPT 和通知失败不影响任务；执行 Python 测试、Rust 测试及打包 smoke test。
  - 外部限制：SPT 单次最多发送给 10 个 SPT；WxPusher 默认接口约 2 QPS，个人 App/桌面端仍受服务方每日通知限制。

- [x] 修复主控台任务切换时左侧流光和外边框不跟随当前任务的问题。
  - 在核心任务循环进入每个顶层任务前发布 `task_started` 事件，按 `daily_task → get_reward → buy_enkephalin → mirror` 顺序同步。
  - Python sidecar 在当前运行 ID 内更新 `execution.status.currentTaskId` 和 `execution.stats.currentRun.currentTaskId`；停止或全部完成后清除当前任务。
  - Mock 启动状态返回第一个可执行任务，GPUI Home 状态测试覆盖连续任务切换和结束清理。
  - 每个任务卡使用独立的流光动画 ID，避免切换任务时复用动画元素状态。
  - 不新增 RPC，继续复用现有 `execution.status` / `execution.stats` 契约。
  - 已完成 Python 单元测试、Rust 测试、格式检查和 `git diff --check` 验证。
