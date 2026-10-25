# GPUI + Python sidecar 加固与演进方案

> 状态：方案稿，待评审
>
> 适用范围：`gpui-app/`、`module/`、`main_backend.py`、构建与发布链
>
> 基线：当前工作区（2026-10-25）
>
> 目标平台：Windows x64

## 实施进度

> 2026-10-25：M1–M4 的主体工作已落地。

已完成：

- [x] 连接失败优先重连现有 sidecar，不再直接重启进程（`BackendAttach::Reconnected`）。
- [x] GPUI 退出时发送 `app.shutdown`，并等待 sidecar 有界退出；窗口关闭、Cmd-Q、`app.exitRequested` 均走同一流程。
- [x] sidecar 使用继承的 stdin 管道 EOF 作为父进程死亡信号；保留 `--parent-pid` 作为兜底。
- [x] ready 之后重定向 stdout，stderr 由 GPUI 持续 drain，并在启动失败时附带尾部诊断。
- [x] token 改为 OS 随机值，并通过子进程环境变量传递，不再出现在命令行。
- [x] 事件背压不再关闭控制连接；Python/Rust 都优先丢弃可重建事件，保留 warning/error 与生命周期事件。
- [x] 客户端写入增加 5 秒超时，避免单个卡死连接拖住事件循环。
- [x] sidecar 改为 onedir；`scripts/smoke_frozen_sidecar.py` 验证 ready PID 就是被监管进程 PID，并能优雅退出。
- [x] 共享 runtime：`shared_runtime.spec` 把 sidecar 与 Runner 放进同一个 `backend/` 目录，共用 `_internal`。`build.py --separate-bundles` 保留独立包回退。
- [x] `release.json` 记录 backend/runner 入口与逐文件哈希；updater 升级后删除旧版单文件 `AALC Backend.exe`。
- [x] async-only 收口：Mock 也产生 completion，状态层不再出现业务级 `if is_sidecar()`；`RpcCompletion` 带 `requestId`。
- [x] `contract/contract.json` + `scripts/validate_contract.py`，在 pytest/CI 中校验 Rust 常量、Python 路由/错误码、Mock 覆盖和事件。
- [x] 握手增加 `bootId`、`capabilities`、`pid`；事件信封携带 `bootId`，Rust 丢弃旧 boot 的帧。
- [x] `--health` / `--diagnose` 输出 JSON；ready 行与 GPUI 日志记录启动耗时、bootId。
- [x] 拆出 `ThemePackService`、`NotificationUpdateService`、`TeamService`、`TaskConfigService` 和 `ResourceService`；`BackendApplication` 从约 4700 行降到约 3800 行，仍负责执行/设备/预览的组合根。

尚未完成：

- [ ] 继续拆 `BackendApplication` 的执行、设备、预览服务（已拆低耦合部分，剩余部分与 Runner/设备租约耦合较深）。
- [ ] 真机 E2E 覆盖共享 runtime 下的 Runner 启动（代码路径已有单元测试，CI 暂无真机）。
- [ ] AV 误报与冷/热启动长期数据（当前只有本机一次测量）。

验证结果：

- Python：`471 passed, 1 skipped`（skipped 为需要显式 frozen 包的 smoke；设置 `AALC_FROZEN_BACKEND` 后通过）。
- Rust：`199 passed`；`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings` 通过。
- `ruff check` 通过。
- 真实 sidecar 集成测试（重连 + `app.shutdown` + 进程退出）通过。
- 完整 `scripts/build.py --version 0.0.0-test` 构建成功。
- 发行版关闭测试：`dist/AALC/AALC.exe` 退出码 0，关闭后共享 runtime 内无残留进程。

打包测量（Windows x64，本机）：

| 形态 | 大小 | 备注 |
| --- | ---: | --- |
| 旧 onefile sidecar | 109 MB | 每次启动解压 |
| 独立 onedir sidecar | 234 MB | 无解压，但 Runner 再占一套 |
| 共享 onedir runtime（sidecar + Runner） | 246 MB | 当前默认 |
| 发布归档 `.7z` | 126 MB | 315 MB 解压前内容 |
| frozen sidecar ready | 441–585 ms | 本机多次测量 |

frozen smoke 结果：

```json
{"ready":true,"host":"127.0.0.1","port":13105,"pid":20380,"bootId":"...","schemaVersion":3,"version":"0.0.0-test","startupMs":441,"supervisedPid":20380}
```

## 1. 结论

当前 “GPUI 主进程 + Python sidecar + 一次性 Runner” 的总体架构是正确的，不建议替换为 PyO3 内嵌、Rust 重写后端或 gRPC。

这套架构解决了本项目的三个核心问题：

- Python 自动化、OCR、设备控制崩溃时不能拖死 GPUI。
- Python GIL 和重计算不能阻塞 UI 渲染与输入。
- 单次任务必须能被可靠停止，并在强杀后完成设备与资源清理。

Runner 的设计尤其合理：每次执行新建进程、Windows Job Object 监管、CleanupLedger 补偿清理、设备租约和 `runId`/`generation` 校验，已经是比较完整的执行隔离模型。

真正需要改的不是“要不要 sidecar”，而是当前实现把两件事混在了一起：

> **进程生命周期** 和 **连接生命周期**。

由此产生的主要风险是：

- WebSocket 短暂断开会被当成 sidecar 进程死亡，GPUI 重启 sidecar，顺带杀掉正在执行的 Runner。
- 正常退出没有优雅关闭，`BackendApplication.close()` 不执行，只能依赖下次启动的清理账本。
- sidecar 使用 PyInstaller onefile，bootloader 可能派生实际 Python 子进程，`child.kill()` 的终止语义不确定。
- 事件背压会主动关闭连接，且可能丢弃 warning/error 或生命周期事件。
- Rust 状态层同时存在同步/异步两套路径，Mock 与真实 sidecar 的行为容易漂移。

本方案的目标是：**保留现有架构，按“先稳定性、后一致性、再发布体验”的顺序加固边界，不改变 Runner 模型，不重写业务层。**

## 2. 现状架构

### 2.1 进程结构

```text
AALC.exe（GPUI / Rust）
  └─ backend/AALC Backend.exe（Python sidecar，共享 onedir runtime）
       ├─ ws://127.0.0.1:<随机端口> + token + schemaVersion=3
       │    + bootId + capabilities 握手
       ├─ Config / Device / Preview / Stats / Update / Hotkey
       └─ backend/AALCRunner.exe（每次任务新建，共享同一 _internal，Job Object 监管）
```

构建默认使用 `shared_runtime.spec`。`--separate-bundles` 仍可生成旧的
`backend/` + `runner/` 双目录布局作为回退。

### 2.2 职责边界

| 组件 | 职责 |
| --- | --- |
| GPUI | 窗口、页面、托盘、单实例、用户交互、WebSocket 客户端、状态展示与恢复 |
| sidecar | 权威执行状态、设备租约、Runner 监管、配置写入、统计、通知、日志、热键、更新检查 |
| Runner | 单次任务、任务期间设备控制、任务预览、可取消等待、本地辅助进程 |

### 2.3 现有设计的优点

- 页面、状态、模型、IPC、Shell 分层清楚，页面不直接操作 WebSocket。
- Mock 与真实 sidecar 共用 JSON-RPC 契约，便于确定性测试。
- 执行状态有 `schemaVersion`、`stateRevision`、`runId`、`deviceLease`、`outcome`、`forced` 等权威字段。
- 事件有单调 `seq`，页面按 `runId`/`stateRevision`/`seq` 过滤迟到数据。
- 设备租约、`generation` 守卫、Runner 一次性进程、Job Object、清理账本构成了比较完整的故障隔离。
- 结构化错误码、参数校验、有界队列、停止请求独立执行通道都已落地。

这些设计应保留，不应为了“架构更先进”而推翻。

## 3. 问题清单

### 3.1 优先级总表

| ID | 优先级 | 问题 | 影响 |
| --- | --- | --- | --- |
| P0-1 | 高 | 连接断开被当成进程死亡，重启 sidecar 会杀掉正在执行的 Runner | 任务被误中止、设备恢复成本高 |
| P0-2 | 高 | 正常退出没有调用 `app.shutdown`，`SidecarGuard::drop` 直接 kill | 设备/预览/热键/配置清理被跳过 |
| P0-3 | 高 | sidecar 为 onefile，bootloader 子进程终止语义不确定 | 可能残留 Python 子进程、端口/设备冲突 |
| P0-4 | 高 | 事件背压会关闭连接，且可能丢 warning/error/生命周期事件 | 日志丢失、连接重启、任务误中止 |
| P0-5 | 中高 | 父进程监视依赖 psutil；stdout 不再 drain；GUI 下 stderr 不可靠 | 孤儿进程、broken pipe、缺少启动诊断 |
| P0-6 | 中 | token 可预测且通过命令行传递 | 本机其他进程可读取并连接 sidecar |
| P1-1 | 中高 | 同步/异步双轨 + 状态层大量 `is_sidecar()` 分支 | 行为漂移、重复代码、UI 阻塞隐患 |
| P1-2 | 中 | RPC 方法/事件/错误码手写多份，握手要求 schema 精确相等 | 契约漂移、升级/半更新时直接失败 |
| P1-3 | 中 | `BackendApplication` 已超过 4400 行 | 维护、测试和故障定位成本高 |
| P2-1 | 中低 | sidecar onefile + Runner onedir，重复一套 Python 运行时 | 安装体积大、启动慢、AV 误报风险 |
| P2-2 | 低 | 缺少启动阶段耗时、队列深度、丢弃计数等诊断信息 | 线上问题只能靠猜 |
| P2-3 | 低 | 是否支持“关闭 UI 后任务继续跑”未明确 | 影响是否引入常驻 daemon |

### 3.2 P0-1：断线被当成进程死亡

#### 现象

WebSocket 一旦断开，GPUI 会启动一个新的 sidecar，并替换旧 client。旧 client 被 drop 时，`SidecarGuard` 也随之 drop，执行 `child.kill()`。如果旧 sidecar 正在监管 Runner，Runner 所在的 Job 被关闭，任务被强杀。

#### 代码路径

```text
app/lifecycle/recovery.rs
  maybe_recover_backend()
    -> start_backend_connection(Reconnect)
      -> BackendClient::start_or_connect()
        -> SidecarSupervisor::restart()
          -> WebSocketClient::try_new()       // 启动新 sidecar
            -> 替换旧 client                   // 旧 SidecarGuard Drop -> child.kill()
```

相关位置：

- `gpui-app/src/app/lifecycle/recovery.rs:18-30`
- `gpui-app/src/ipc/backend.rs:66-85`
- `gpui-app/src/ipc/backend.rs:185-196`
- `gpui-app/src/ipc/websocket/sidecar.rs:7-14`

#### 为什么“断线但进程还活着”是真实场景

- sidecar 事件队列满时会主动 `client.close(code=1013)`。
- Rust worker 收到格式异常的消息会标记 fatal，连接断开。
- sidecar 主线程被重活拖住时，WebSocket ping 可能超时。
- 本机网络栈、端口或 worker 线程偶发异常。
- 外部开发模式连接不稳定。

#### 影响

一个可恢复的传输故障被放大为：

1. 正在执行的 Runner 被杀。
2. 设备租约进入恢复流程。
3. 用户看到任务失败/中断。
4. 如果设备端有 scrcpy server 或 adb forward，还需要下一次启动补偿清理。

#### 方案

把 supervisor 拆成三层：

```text
SidecarProcess    // 子进程、端口、token、bootId、退出状态
RpcSession        // 当前 WebSocket client，可替换
SidecarSupervisor // 负责 reconnect / restart / resync
```

行为约定：

- 连接断开时，先用**同一个端口和 token** 重连，不立刻杀进程。
- 重连成功后只替换 `RpcSession`，`SidecarProcess` 和 Runner 保持不变。
- 只有 `child.try_wait()` 确认进程已退出，或重连预算耗尽，才进入 `restart()`。
- 重连成功后执行一次 resync：
  - `execution.getState`
  - `stats.getSummary`
  - `team.list`（按需）
  - 必要时 `tasks.getConfig`
- 用 `stateRevision`、`runId`、`bootId` 对齐，丢弃旧会话事件。

#### 验收

- 只断开 WebSocket、不杀 sidecar 进程，正在执行的 Runner 继续运行。
- 重连后 UI 能恢复到正确的执行状态。
- sidecar 进程真的退出时，才触发进程级重启。
- 日志中能区分 `reconnect` 与 `restart`。

### 3.3 P0-2：正常退出没有优雅关闭

#### 现状

Python 端已经有完整的关闭路径：

- `module/backend_application.py:977` 的 `app_shutdown()`。
- `main_backend.py:86-90` 的 `finally: await server.stop(); application.close()`。
- `module/backend_application.py:3541` 的 `close()` 会停止 Runner、停止预览、关闭设备、停止热键、flush 配置。

但 Rust 端：

- `gpui-app/src/ipc/contract.rs` 没有 `APP_SHUTDOWN` 常量。
- GPUI 退出时不会发送 `app.shutdown`。
- `SidecarGuard::drop` 直接 `child.kill()`。
- Windows 上 `child.kill()` 是 `TerminateProcess`，Python 的 `finally` 不会执行。

#### 影响

- 设备预览、工具、热键、配置 flush 可能跳过。
- 只能依赖下次启动的 CleanupLedger 恢复。
- 用户退出后不再启动时，设备端 scrcpy server / adb forward 可能残留。
- onefile 场景下，bootloader 和实际 Python 子进程的终止语义更复杂。

#### 方案

退出流程改为：

1. GPUI 发送 `app.shutdown` 请求。
2. 等待响应或 sidecar 进程退出，最多 5 秒。
3. 超时再 `kill()`。
4. sidecar 同时处理 SIGTERM，保证 `close()` 一定执行。
5. 如果当前有任务在跑，`close()` 按既有 Runner 宽限流程停止；UI 显示“正在关闭后端”。

建议新增：

- `ipc/contract.rs` 增加 `APP_SHUTDOWN`。
- `BackendClient` 增加 `shutdown(timeout)`。
- `main.rs` 的退出路径改为先 shutdown 再 quit。
- 测试：启动 sidecar → 发 shutdown → 断言退出码为 0、无残留子进程。

#### 验收

- 正常退出后 `logs/debugLog.log` 中能看到 `application.close()` 相关清理日志。
- 退出后没有 `AALC Backend.exe` / `AALCRunner.exe` 残留。
- 退出过程中 UI 不冻结。

### 3.4 P0-3：sidecar onefile 的 bootloader 子进程问题

#### 现状

- `main_backend.spec:66` 使用 onefile（`EXE` 直接包含 binaries/datas）。
- `execution_runner.spec:90` 明确使用 onedir（`COLLECT`），并在注释中说明：
  > one-file executable, so the sidecar can supervise the direct Runner process without a second extraction/bootloader process.
- Runner spec 还明确提到 onefile bootloader 可能派生实际 Python 子进程，使 hello PID 不再等于 sidecar 创建并加入 Job 的 PID。

这个理由对 sidecar 同样成立。

#### 影响

- GPUI 的 `child.kill()` 杀掉的可能只是 onefile bootloader。
- 真正持有 WebSocket、设备和 Runner Job 的 Python 子进程可能短暂存活。
- 父进程监视会在约 2 秒内兜底，但单实例互斥体已经释放，用户立刻重开应用时可能出现两个 sidecar 短暂并存。
- 如果 psutil 未被打包或 import 失败，父进程监视直接失效。

#### 方案

优先级从高到低：

1. **sidecar 改为 onedir**，与 Runner 保持一致，消除 PID 和终止语义的不确定性。
2. 如果短期保留 onefile：
   - 使用 `ready.pid` 做身份校验。
   - 用 Job Object 或进程树终止确保 Python 子进程一起退出。
   - 增加“退出后无残留进程”的 smoke test。

#### 验收

- 启动 sidecar 后，GPUI 拿到的 PID 就是实际持有 WebSocket 的进程 PID。
- 终止后任务管理器中无 `AALC Backend.exe` 残留。
- 退出后立即重开应用，不会出现两个 sidecar 同时占用设备。

### 3.5 P0-4：事件背压会关闭连接，且可能丢关键事件

#### 现状

`module/websocket_server.py:229-237`：

- 事件队列达到 `MAX_OUTBOUND_EVENTS` 时，优先删除一个可丢弃事件。
- 如果找不到可丢弃事件，关闭所有客户端（`code=1013`）。
- `_DROPPABLE_EVENTS` 包含整个 `log.entry`，所以 warning/error 也可能被丢。

Rust 端 `gpui-app/src/ipc/websocket/worker.rs:240-242`：

- 队列满时无条件 `pop_front()`，不区分事件类型。
- `app.exitRequested`、`execution.status` 终态、`task.completed` 等生命周期事件可能被丢。

#### 影响

结合 P0-1：

```text
日志/预览太多
  -> sidecar 关闭连接
    -> GPUI 认为后端断开
      -> 重启 sidecar
        -> 正在执行的 Runner 被杀
```

即使没有触发重启，丢掉 `app.exitRequested` 也会导致“任务完成后不退出”；丢掉终态事件会导致 UI 停留在错误状态。

#### 方案

明确两条逻辑通道：

| 通道 | 内容 | 语义 |
| --- | --- | --- |
| Control | 请求/响应、执行控制、生命周期 | 可靠、有序、不因背压关闭 |
| Telemetry | 预览帧、进度、统计、普通日志 | 可合并、可丢弃、可 resync |

具体规则：

- 控制通道永远不因为事件队列满而关闭连接。
- 只丢弃可重建事件：截图帧、进度、debug/info 日志。
- warning/error、`execution.status` 终态、`task.completed`、`app.exitRequested`、`config.delta` 不丢。
- 丢弃时记录 `droppedEvents` 计数，并发送 `resyncRequired` 或在下一次状态事件中携带。
- 客户端重连后调用 `execution.getState` 等接口做 resync。
- Rust 端也按优先级处理，不要简单丢最旧事件。

#### 验收

- 持续灌入预览/日志，控制请求仍能在目标时限内响应。
- 队列压力下 warning/error 不丢。
- 连接不会因为背压被关闭。
- 人为丢弃事件后，UI 能通过 resync 恢复正确状态。

### 3.6 P0-5：父进程监视与 stdio 处理

#### 现状

- `main_backend.py:93-121` 用 psutil 每 2 秒轮询父进程；`except ImportError: return` 意味着没有 psutil 时监视直接失效。
- `gpui-app/src/ipc/websocket/sidecar.rs:68` 把 stdin 设为 `Stdio::null()`。
- `read_ready_line` 在读到 ready 后退出，之后 stdout 不再 drain。
- `sidecar.rs:70` 使用 `stderr(Stdio::inherit())`。
- `module/logger/my_log.py:163` 在 `headless=True` 时仍然安装 console StreamHandler。

#### 影响

- 父进程监视依赖可选依赖，打包遗漏时 sidecar 可能成为孤儿。
- 2 秒轮询存在窗口期。
- Python 或第三方库 `print()` 到 stdout 时可能 broken pipe。
- GUI 启动、没有控制台时，继承的 stderr 可能无效，日志写入失败或丢失。
- 启动失败时缺少 stderr 诊断。

#### 方案

- 用**继承的 stdin 管道 EOF** 作为父进程死亡信号：
  - GPUI 保持写端。
  - sidecar 在独立线程读 stdin；EOF 即父进程退出。
  - 不依赖 psutil，跨平台，且比 2 秒轮询及时。
- 保留 PID + `create_time` 校验，防止 PID 重用。
- ready 之后把 `sys.stdout` 重定向到 stderr 或 devnull。
- stderr 用管道接住并 drain 到日志文件；启动失败时把尾部内容带回 GPUI。
- headless 模式不要安装 console handler；只保留文件日志和事件日志。

#### 验收

- 杀掉 GPUI 进程，sidecar 在 1 秒内退出。
- sidecar 启动失败时，GPUI 能显示 stderr 尾部。
- Python `print()` 不会导致 broken pipe。
- 没有 psutil 时父进程监视仍然有效。

### 3.7 P0-6：token 与本地安全

#### 现状

`gpui-app/src/ipc/websocket/sidecar.rs:34-41`：

```rust
let token = format!(
    "{:x}-{:x}",
    std::process::id(),
    SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
);
```

并通过 `--token` 传给 sidecar。

#### 影响

- token 可预测。
- 命令行对本机其他进程可见。
- 如果本机存在恶意进程，它可以连接 sidecar 并调用业务 RPC。

如果威胁模型只是“用户自己的可信机器”，这不是最高优先级；但修复成本很低。

#### 方案

- 使用 `secrets.token_urlsafe(32)` 或 OS 随机数。
- 通过继承管道、受限文件或环境变量传递，避免 argv。
- 长期如果改用命名管道，用 ACL 限制到当前用户。
- 保留 loopback 地址校验，不监听外网。

#### 验收

- token 不可从命令行直接读取。
- 错误 token 无法连接。
- 非 loopback 地址仍然被拒绝。

### 3.8 P1-1：同步/异步双轨与 `is_sidecar()` 分支

#### 现状

- `RpcGateway::request` 是同步阻塞的，最多等待 30 秒（`websocket/mod.rs:39, 268-279`）。
- 生产 sidecar 路径大多绕开了它，但 API 仍然把同步调用作为默认入口。
- `state/` 中有 20 多处 `is_sidecar()`。
- Mock 走同步 `request_value`，sidecar 走 `submit`/`request_async`。
- `app/team_editor.rs` 中保存、删除、启用三处各写了一遍几乎相同的异步链。
- 设置页/工具箱用 `submit` 后靠 completion 回填，错误反馈路径和 Mock 不一致。

#### 影响

- 新页面如果直接调用同步 `request`，会阻塞 UI 最多 30 秒。
- Mock 与 sidecar 的行为容易漂移，测试通过不代表真实路径正确。
- 重复的异步链增加维护成本。
- `RpcCompletion` 只有 method/params/response，没有 request id，多个同名请求并发时可能匹配错误。

#### 方案

做一次 async-only 收口：

1. `MockClient` 也通过 `request_async` 立即返回，保持同一套异步接口。
2. 状态方法返回 `Pending`/`Operation` 句柄，由 `AhabApp` 统一处理 completion。
3. 删除生产路径的同步 `request`，只在测试里保留 blocking helper。
4. `RpcCompletion` 增加 `requestId`，按请求精确匹配。
5. `is_sidecar()` 只保留给“是否需要启动 hydration / 事件泵”这类真正与传输相关的判断。
6. 对 mutating 请求增加 `clientRequestId` 幂等键，为重连后的安全重试做准备。

#### 验收

- 状态层不再出现 `if is_sidecar() { async } else { sync }` 的业务分支。
- Mock 与 sidecar 走同一条异步状态路径。
- 任何 UI 操作都不会同步等待网络。
- 同名并发请求的 completion 不会串台。

### 3.9 P1-2：契约单一来源与版本协商

#### 现状

同一套方法和事件至少出现在：

- `gpui-app/src/ipc/contract.rs`
- `module/rpc_dispatcher.py` 的 `route_names` / `is_mutating`
- `gpui-app/src/ipc/mock/`
- 两边的测试

握手要求 `schemaVersion` 精确相等（`RPC_SCHEMA_VERSION = 3`，`SCHEMA_VERSION = 3`）。

#### 影响

- 新增/重命名方法时容易漏改 Python 路由、Mock 或测试。
- 应用与 sidecar 半更新时直接握手失败。
- 事件 payload 缺少统一的 schema 校验。

#### 方案

- 增加 `contract.json` / `contract.yaml` 作为唯一事实来源，至少包含：
  - method 名称、是否 mutating、参数/结果 schema。
  - event 名称、payload schema。
  - 错误码与 retryable 语义。
- CI 校验：
  - 每个 method 在 Python 有路由、在 Mock 有处理。
  - 每个 event 在 Rust 有解码分支。
  - 每个错误码在两边都认识。
  - 参数/结果 fixture 能双向解码。
- 短期只做校验，长期再考虑生成 Rust 常量/Python 路由表/DTO。
- 握手改为：
  - `schemaVersion` 主版本匹配。
  - `capabilities` 列表。
  - `bootId`，事件携带 `bootId`，旧会话事件直接丢弃。

#### 验收

- 新增一个 RPC 方法时，漏改任何一端都会在 CI 失败。
- 旧 sidecar/新 GPUI 或新 sidecar/旧 GPUI 能得到明确的能力协商结果，而不是笼统的“协议不兼容”。

### 3.10 P1-3：`BackendApplication` 服务拆分

#### 现状

`module/backend_application.py` 约 4422 行，配置、队伍、设备、预览、统计、执行、更新、通知、结束动作都在一起。`RpcDispatcher` 很薄，但 `BackendApplication` 变成了 god object。

#### 影响

- 单元测试需要构造大量上下文。
- 故障定位困难。
- 新功能容易继续堆进同一个文件。
- 与 `gpui-app/AGENTS.md` 对模块职责和文件规模的要求不一致。

#### 方案

按服务拆分，`BackendApplication` 只做组合根：

```text
BackendApplication
  ├─ ConfigService
  ├─ DeviceService
  ├─ PreviewService
  ├─ ExecutionService
  ├─ StatsService
  ├─ UpdateService
  └─ NotificationService
```

优先拆低耦合服务（统计、主题包、通知、更新），再拆设备、预览和执行。每拆一个补一组现有测试，避免大爆炸式重构。

#### 验收

- `rpc_dispatcher.py` 只做协议校验、路由和错误转换。
- 新服务可以独立构造和测试。
- `BackendApplication` 行数逐步下降，不继续增长。

### 3.11 P2-1：打包与运行时共享

#### 现状

- `AALC Backend.exe`：onefile，约 104–140 MB。
- `AALCRunner.exe`：onedir，和 sidecar 重复一套 Python 依赖。
- 发布包同时包含两者。

#### 影响

- 安装体积大。
- 冷启动需要解压。
- 两个运行时依赖版本可能漂移。
- AV 对 onefile 自解压更敏感。

#### 方案

评估 **共享 onedir runtime**：

```text
runtime/
  ├─ AALCBackend.exe
  ├─ AALCRunner.exe
  ├─ python3xx.dll
  └─ 共享依赖...
```

两个入口共享一份 binaries/datas，PyInstaller 使用两个 `Analysis` + 一个 `COLLECT`。Runner bootstrap 仍然保持“start 前只导入标准库”的约束，因此共享运行时不会破坏启动顺序。

不盲目切换：

- onefile 对 updater 更简单（单文件替换）。
- onedir 需要 updater 处理目录和 manifest。
- 先测冷启动、热启动、AV 误报，再决定。

#### 验收

- 冷启动/热启动有实测数据。
- 安装包体积有对比。
- updater 能正确处理新目录结构，并有回滚路径。
- Runner 的 PID/Job 语义不变。

### 3.12 P2-2：诊断能力

建议给 sidecar 增加 `--health` / `--diagnose`：

- 版本、schemaVersion、bootId、PID。
- 配置路径、资源路径、日志路径。
- 启动阶段耗时：进程启动 → Python 初始化 → server bind → ready。
- 当前连接数、事件队列深度、丢弃计数、最近错误。
- Runner 状态。

GPUI 端记录：

- sidecar 启动耗时。
- 重连次数、重启次数。
- 最近一次传输错误。
- 事件丢弃计数。

这些信息应进入日志和“关于/诊断”页面，而不是只存在于开发者控制台。

### 3.13 P2-3：是否引入常驻 daemon

如果产品目标是“关闭 UI 后任务继续跑”，当前“sidecar 是 GPUI 子进程”的模型就不合适，需要真正的常驻服务：

- 独立启动/退出、单实例、登录自启。
- UI 只是客户端，可随时关闭重开。
- 更新时不能替换正在运行的后端。
- 权限、通知、托盘、退出语义都要重新设计。

如果不需要后台运行，当前“每个 UI 会话一个 sidecar”更简单、更安全，不建议为了“架构先进”改成 daemon。

## 4. 目标架构

### 4.1 进程与连接分离

```text
AALC.exe（GPUI）
  │
  │  Session: bootId + schemaVersion + capabilities
  │  Control lane: request/response + idempotency key
  │  Telemetry lane: stateRevision + seq，可合并/可丢弃
  ▼
AALC Backend.exe（Python，onedir 或共享 runtime）
  ├─ ConfigRepository（唯一写入者）
  ├─ DeviceManager / PreviewCapture（租约守卫）
  ├─ ExecutionSupervisor
  └─ AALCRunner.exe（一次性，Job Object）
```

### 4.2 状态机

#### 进程状态

```text
Starting -> Ready -> Stopping -> Exited
     └──────────────► Failed
```

#### 连接状态

```text
Connecting -> Connected -> Reconnecting -> Connected
                    │            │
                    │            └─► Restarting -> Connected
                    └─► Disconnected（预算耗尽）
```

关键不变量：

- `Connected` 不代表进程一定健康；以进程句柄和心跳共同判断。
- `Reconnecting` 不杀进程，不重置执行状态。
- `Restarting` 才允许替换 `SidecarProcess`。
- 重连后必须 resync，不能假设事件连续。

### 4.3 事件分层

| 层 | 内容 | 可靠性 | 处理 |
| --- | --- | --- | --- |
| Control | 请求/响应、执行控制、生命周期 | 高 | 不丢、不因背压关闭 |
| State | 执行状态、统计、设备状态 | 高 | 可合并，保留最新 revision |
| Preview | 截图帧 | 低 | 只保留最新帧 |
| Logs | 日志 | 分级 | info/debug 可限流，warn/error 不丢 |

### 4.4 所有权

| 数据/资源 | 唯一所有者 |
| --- | --- |
| 主配置 `config.yaml` | sidecar `ConfigRepository` |
| 设备控制器 | 任一时刻：sidecar 或 Runner，不能同时 |
| 执行状态 | sidecar `ExecutionSupervisor` |
| Runner 进程 | sidecar `RunnerSupervisor` |
| 窗口/托盘/单实例 | GPUI |
| 事件序号 | sidecar `BackendEventBus` |

## 5. 详细设计

### 5.1 Supervisor 分层

建议的 Rust 结构：

```rust
struct SidecarProcess {
    child: Mutex<Child>,
    endpoint: Endpoint,      // url + token
    boot_id: String,
}

struct RpcSession {
    client: WebSocketClient,
    generation: u64,
}

struct SidecarSupervisor {
    process: RwLock<Option<Arc<SidecarProcess>>>,
    session: RwLock<Option<RpcSession>>,
    restarting: AtomicBool,
    ...
}
```

职责：

- `connect()`：确保进程存在，建立 session。
- `reconnect()`：复用 `SidecarProcess`，只替换 session。
- `restart()`：仅当进程不存在/已退出/重连失败时调用。
- `resync()`：重连后拉取权威快照。

### 5.2 优雅退出时序

```text
GPUI 退出
  -> 发送 app.shutdown
  -> sidecar 设置 stop_event
  -> main_backend finally:
       server.stop()
       application.close()
         -> RunnerSupervisor.close(timeout=5)
         -> preview.close()
         -> device_manager.close()
         -> hotkey.stop()
         -> config.flush()
  -> 进程退出
  -> GPUI 收到响应或等待超时
  -> 超时才 kill()
```

注意事项：

- `app.shutdown` 必须在 client 被 drop 之前发送。
- 等待期间 UI 不应冻结；显示“正在关闭后端”。
- 如果任务正在运行，按 Runner 的既有宽限流程停止；不要直接 TerminateProcess。
- 退出流程需要有总超时，避免设备恢复卡死导致应用无法关闭。

### 5.3 stdin EOF 父进程监视

GPUI：

- 创建匿名管道，把读端作为 sidecar stdin，写端保留在父进程。
- 父进程退出时，写端关闭，sidecar 读到 EOF。

sidecar：

- 独立线程阻塞读 stdin。
- EOF 时设置 `stop_event`。
- 仍保留 `--parent-pid` + `create_time` 校验，防止 PID 重用。
- 不再依赖 psutil。

优点：

- 不依赖第三方库。
- 比 2 秒轮询及时。
- Windows/Linux/macOS 行为一致。

### 5.4 重连与 resync

重连触发条件：

- WebSocket read/write 错误。
- 心跳超时。
- worker fatal error。
- 服务端主动关闭（背压、维护）。

重连流程：

```text
1. 标记 session = Reconnecting
2. 用同一 endpoint 建立新 WebSocket
3. 发送 hello(token, bootId)
4. 校验 schema/capabilities
5. 替换 session
6. 调用 execution.getState / stats.getSummary / team.list
7. 应用快照，丢弃旧 session 事件
8. 标记 Connected
```

重连预算：

- 立即重试 1 次。
- 然后 1s / 2s / 4s 退避，最多 3 次。
- 预算耗尽后检查 `child.try_wait()`：
  - 进程已退出：`restart()`。
  - 进程仍存活：标记 `Disconnected`，提示用户手动重试，不杀进程。

### 5.5 背压与丢弃策略

Python 端：

- 控制响应直接发送，不进入事件队列。
- 事件队列按优先级：
  - 关键事件：独立保留容量。
  - 可合并事件：同一 key 只保留最新。
  - 预览帧：只保留最新。
  - 日志：info/debug 可限流，warn/error 保留。
- 队列满时：
  - 先丢预览/旧进度/debug 日志。
  - 不关闭连接。
  - 记录 `droppedEvents`。
  - 发送 `resyncRequired`。

Rust 端：

- 事件队列同样按优先级。
- `app.exitRequested`、`execution.status` 终态、`task.completed` 不丢。
- 截图帧只保留最新。
- 队列满时丢可重建事件，并通知 UI 需要 resync。

### 5.6 Async-only Gateway

目标 API：

```rust
pub struct Pending<T> {
    pub request_id: RequestId,
    pub receiver: async_channel::Receiver<RpcResponse>,
    pub _marker: PhantomData<T>,
}

impl RpcGateway {
    pub fn execute(&self, method: &str, params: Option<Value>) -> Pending<Value>;
}
```

状态层：

```rust
pub struct Operation {
    pub id: RequestId,
    pub kind: OperationKind,
    pub started_at: Instant,
}
```

`AhabApp` 统一处理 completion：

```text
take_completions()
  -> 按 request_id 匹配 Operation
  -> 解码 typed payload
  -> 更新对应 state
  -> notify
```

Mock：

- `request_async` 立即返回 ready receiver。
- 事件可以同步入队，由事件泵统一 drain。
- 不再需要 `is_sidecar()` 分支。

### 5.7 契约 Manifest

建议文件：`contract/contract.json`（或 `contract.yaml`）。

示例结构：

```json
{
  "schemaVersion": 3,
  "methods": {
    "execution.start": {
      "mutating": true,
      "params": "ExecutionStartParams",
      "result": "ExecutionStartResult"
    }
  },
  "events": {
    "execution.status": {
      "payload": "ExecutionStatusPayload"
    }
  },
  "errors": {
    "-32013": "STALE_RUN"
  }
}
```

CI 校验：

- Rust `contract.rs` 的方法/事件常量与 manifest 一致。
- Python `rpc_dispatcher.py` 的 `route_names` 覆盖所有 method。
- Rust Mock 覆盖所有 method。
- 每个 event 在 Rust 有解码分支。
- 每个错误码在两边都有定义。
- fixture 能双向解码。

长期：

- 用 `typify` 生成 Rust DTO。
- 用 `datamodel-code-generator` 生成 Python Pydantic 模型。
- 但不要一次性替换所有手写 DTO，先让 manifest 成为校验源。

### 5.8 服务拆分

建议顺序：

1. `StatsService`：纯数据读写，依赖少。
2. `ThemePackService`：文件读写 + 权重。
3. `NotificationService`：外部 HTTP，边界清楚。
4. `UpdateService`：更新检查与下载。
5. `PreviewService`：截图、JPEG 编码、预览线程。
6. `DeviceService`：设备发现、连接、租约。
7. `ExecutionService`：Runner 监管、状态机。

每个服务暴露窄接口，`BackendApplication` 只做组合和生命周期管理。

### 5.9 打包与诊断

打包：

- 先测量 onefile vs onedir 的冷启动/热启动。
- 评估共享 runtime 的体积收益。
- 保留 updater 的单文件替换路径作为回退。
- 所有发布物进入 `release.json`，包含相对路径、哈希、大小。

诊断：

- `AALC Backend.exe --health` 输出 JSON。
- `AALC Backend.exe --diagnose` 输出更详细的环境信息。
- GPUI 启动时记录 sidecar 启动耗时和 bootId。
- 日志中记录 reconnect/restart/resync 事件。

## 6. 实施阶段

### M0：基线与测试

目标：在不改变行为的前提下建立可验证基线。

- 保存当前 GPUI/Python 测试结果。
- 增加 sidecar 启动/退出/断线的最小集成测试。
- 记录当前冷启动/热启动耗时。
- 记录当前发布包体积。

验收：

- 现有测试全部通过。
- 有可重复的 sidecar 启停测试。

### M1：进程生命周期加固

改动：

- `app.shutdown` 接入 GPUI 退出流程。
- `SidecarGuard` 改为有界优雅退出 + kill 兜底。
- stdin EOF 父进程监视。
- stdout/stderr 处理。
- token 随机化。
- sidecar 评估/切换 onedir。

验收：

- 正常退出无残留进程。
- 杀 GPUI 后 sidecar 1 秒内退出。
- 启动失败能看到 stderr 尾部。
- 退出过程中 UI 不冻结。

风险：

- 退出流程涉及 GPUI 生命周期，需要小心 `cx.quit()` 时序。
- onedir 涉及构建和 updater，可单独作为一个提交。

### M2：连接生命周期与背压

改动：

- supervisor 拆分 process/session。
- reconnect-before-restart。
- resync。
- 控制/遥测分层。
- 事件优先级与丢弃计数。

验收：

- 只断 WS 不杀进程，任务继续。
- 重连后状态正确。
- 背压不关闭控制连接。
- warning/error 不丢。

风险：

- 需要新增 sidecar 端 `resync` 或复用现有查询方法。
- 需要处理旧 session 事件过滤。

### M3：Async-only 与契约

改动：

- Mock 异步化。
- 状态层统一 async 路径。
- `RpcCompletion` 增加 request id。
- contract manifest + CI 校验。
- 握手 capabilities/bootId。

验收：

- 状态层不再有业务级 `is_sidecar()` 分支。
- 同名并发请求不串台。
- 契约漂移能被 CI 捕获。

风险：

- 这是最大的代码重构，建议按页面/领域逐步迁移，不要一次性全改。
- 可以先用 `#[deprecated]` 标记同步 API，再逐步删除。

### M4：服务拆分与打包

改动：

- 按服务拆 `BackendApplication`。
- 评估共享 runtime。
- 增加 `--health` / `--diagnose`。
- 更新发布清单和 updater。

验收：

- 服务可独立测试。
- 安装包体积/启动耗时达到目标。
- 诊断信息可用于支持。

## 7. 测试矩阵

### 7.1 进程生命周期

- 正常启动 → ready → shutdown → 退出码 0。
- 启动失败（缺依赖/端口占用/配置错误）→ 结构化错误 + stderr 尾部。
- 杀 GPUI → sidecar 在 1 秒内退出。
- 杀 sidecar → GPUI 检测到进程死亡并重启。
- onefile/onedir 下均无残留子进程。
- 退出后立即重开，不出现双 sidecar。

### 7.2 连接生命周期

- 只断 WebSocket，不杀进程 → 重连成功，任务继续。
- 重连期间执行状态变化 → resync 后 UI 正确。
- 重连预算耗尽且进程存活 → 显示断开，不杀进程。
- 重连预算耗尽且进程死亡 → 重启。
- 旧 session 事件不覆盖新状态。

### 7.3 背压与事件

- 预览洪水 → 只保留最新帧，控制请求仍响应。
- 日志洪水 → info/debug 可丢，warn/error 不丢。
- 队列满 → 不关闭连接，记录 dropped，触发 resync。
- `app.exitRequested` 不丢。
- 终态 `execution.status` 不丢。

### 7.4 退出与清理

- 任务运行中正常退出 → Runner 协作停止/强杀，设备恢复。
- 任务运行中杀 GPUI → Job 清理 Runner，下次启动恢复设备。
- 退出后无 adb forward / scrcpy server 残留（真机）。
- 配置 flush 成功。

### 7.5 契约

- Rust 方法/事件常量与 manifest 一致。
- Python 路由覆盖所有 method。
- Mock 覆盖所有 method。
- 所有错误码两边认识。
- schema 主版本兼容/不兼容场景。

### 7.6 真机 E2E

- Windows 真实游戏窗口、MuMu、ADB。
- 连接、预览、开始、暂停、恢复、停止、退出。
- 断线重连、sidecar 重启、Runner 强杀。
- 退出后重开，设备状态正确。

## 8. 风险与回滚

| 改动 | 风险 | 回滚 |
| --- | --- | --- |
| 优雅退出 | GPUI 退出时序复杂，可能卡住 | 保留 kill 兜底；超时强制退出 |
| stdin EOF | 管道创建/继承在 Windows 上需验证 | 保留 parent-pid 监视作为 fallback |
| reconnect-before-restart | 重连期间事件丢失/状态不一致 | resync 失败则回退到 restart |
| 背压策略 | 关键事件仍可能丢失 | 保留 resync + 状态快照兜底 |
| async-only | 大范围重构，回归风险高 | 分领域迁移；保留同步 API 过渡 |
| onedir/共享 runtime | updater 和发布链改动 | 保留 onefile 发布作为回退 |
| 服务拆分 | 行为回归 | 按服务逐个迁移，测试先行 |

总体原则：

- M1/M2 不改 RPC 契约，风险可控。
- M3 是重构，分阶段进行。
- M4 涉及发布，独立提交并保留回退包。
- 任何阶段都不要动 Runner 的一次性进程模型和 Job 语义。

## 9. 非目标

本方案明确不做：

- 不把 Python 嵌入 GPUI（PyO3）。
- 不用 Rust 重写 Python 自动化/OCR 后端。
- 不引入 gRPC/protobuf。
- 不为了“性能”替换 WebSocket；当前瓶颈不在 IPC。
- 不改变 Runner 的一次性进程模型。
- 不在没有产品需求的情况下引入常驻 daemon。

## 10. 待确认事项

1. 是否需要“关闭 UI 后任务继续跑”？
   - 需要：考虑常驻 daemon，本方案只做前期加固。
   - 不需要：保持当前每 UI 会话一个 sidecar。
   不需要

2. 是否接受一次较大的 async-only 重构？
   - 接受：按 M3 执行。
   - 暂不接受：先做 M1/M2，保留同步 API 但标记为测试专用。
   接受

3. 发布体积/启动速度是否是当前痛点？
   - 是：优先评估 onedir/共享 runtime。
   - 否：放在 M4。
   现在启动的时候再启动py后端偏慢

4. 是否保留外部 sidecar attach 模式（`AHAB_BACKEND_URL`）？
   - 保留：WebSocket 继续作为开发/调试传输。
   - 不保留：可评估 Runner 风格的管道协议。
   保留

## 附录 A：关键代码位置

> 以下行号基于当前工作区，供检索使用；重构后以符号名为准。

| 主题 | 位置 |
| --- | --- |
| sidecar 启动与 ready 读取 | `gpui-app/src/ipc/websocket/sidecar.rs:33-165` |
| SidecarGuard kill | `gpui-app/src/ipc/websocket/sidecar.rs:7-14` |
| token 生成 | `gpui-app/src/ipc/websocket/sidecar.rs:34-41` |
| supervisor restart | `gpui-app/src/ipc/backend.rs:66-85` |
| start_or_connect | `gpui-app/src/ipc/backend.rs:185-196` |
| 断线恢复入口 | `gpui-app/src/app/lifecycle/recovery.rs:18-30` |
| WebSocket 客户端 | `gpui-app/src/ipc/websocket/mod.rs` |
| 请求超时 | `gpui-app/src/ipc/websocket/mod.rs:39, 268-279` |
| 事件队列丢弃 | `gpui-app/src/ipc/websocket/worker.rs:230-249` |
| RPC 契约 | `gpui-app/src/ipc/contract.rs` |
| 背压关闭连接 | `module/websocket_server.py:229-237` |
| 可丢弃事件 | `module/websocket_server.py:43` |
| 父进程监视 | `main_backend.py:93-121` |
| 关闭路径 | `main_backend.py:81-90` |
| `app_shutdown` | `module/backend_application.py:977-980` |
| `BackendApplication.close` | `module/backend_application.py:3541-3610` |
| RPC 路由表 | `module/rpc_dispatcher.py:148-189` |
| 日志 console handler | `module/logger/my_log.py:161-203` |
| sidecar onedir | `main_backend.spec`（`COLLECT`） |
| 共享 runtime | `shared_runtime.spec`、`scripts/build.py` |
| Runner onedir | `execution_runner.spec:90-99` |
| 契约校验 | `contract/contract.json`、`scripts/validate_contract.py` |
| 诊断入口 | `main_backend.py --health/--diagnose`、`app.diagnose` |
| Runner Job Object | `module/execution/platform.py:484-523` |

## 附录 B：建议新增/调整的模块

```text
gpui-app/src/ipc/
  ├─ supervisor.rs          # process/session 分离、reconnect/restart
  ├─ session.rs             # 当前 RPC session、bootId、resync
  └─ shutdown.rs            # 优雅退出

module/
  ├─ backend_services/      # 按服务拆分 BackendApplication
  │   ├─ config_service.py
  │   ├─ device_service.py
  │   ├─ preview_service.py
  │   ├─ execution_service.py
  │   ├─ stats_service.py
  │   └─ notification_service.py
  └─ diagnostics.py         # --health / --diagnose

contract/
  └─ contract.json          # RPC/事件/错误码单一事实来源
```

以上模块命名可按现有目录习惯调整，但职责边界应保持。
