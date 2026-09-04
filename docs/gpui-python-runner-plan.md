# GPUI–Python 任务执行隔离与快速停止实施规格

> 状态：第一阶段已实现，真机发布验收待完成
> 主发布平台：Windows x64
> 外部 WebSocket schema：3
> Runner IPC protocol：1

## 实施状态（阶段性）

第一阶段的实现、合同和自动化验证已落地，但这不等于已经满足生产发布门。当前仍需完成真实设备、平台边界和性能验收；规范目标与后续发布门保持不变。

已落地的主要模块包括：

- Python 配置路径与 `ConfigRepository`（快照、revision/hash、白名单 delta/CAS、Runner 临时配置）。
- `DeviceManager` 设备目标快照、execution lease/generation、Runner 私有 runtime，以及 `PreviewCapture` 的 run/generation 事件和 `stop_and_wait`。
- 输入暂停/停止与可唤醒等待、MuMu/ADB/Scrcpy Runner 策略、run-scoped scrcpy 标识和 `ResourceCleanupExecutor` 补偿接口。
- Runner bootstrap、execution supervisor/lease/IPC/cleanup ledger、任务取消检查点、完成后动作拆分和有界优先级 command queue。
- GPUI schema 3 状态/事件处理、旧 run/generation 过滤，以及 sidecar/Runner onedir 构建和 release manifest 相关实现。

自动化验证方面，当前已确认 Python 完整测试 378 passed；Rust 153 passed，并通过 fmt/check/clippy；execution/build 定向测试 54 passed；最新 PyInstaller onedir frozen complete/hang-stop、真实 Windows Job、无 console/PPID 验证均通过。`main_backend.spec` onefile frozen smoke 也已通过 GUI 无 console、schema 3 WebSocket hello、ping、完整 idle `getState` 和受控 shutdown；Runner onedir smoke 保持通过。这些结果只代表自动化门，不替代下列真机与发布验收。

尚未通过或仍需明确记录的发布门：

- Windows 真实 MuMu、ADB/真机和 HWND 的停止、恢复、离线、forward/socket 清理矩阵。
- 另一 Windows 环境，以及外部嵌套 Job Object 下的 suspended-create → assign → resume 失败关闭路径。
- macOS adapter 的 best-effort 行为与崩溃/不可中断 native 调用边界；macOS 仍不是第一阶段生产承诺。
- 停止确认、协作取消、冷/热启动和恢复的 SLA p95 实测；不能仅以单测推断性能达标。
- 完整 GPUI 真机链（状态、停止/恢复、schema 3 事件和错误展示）的端到端验收。

## 1. 目标、边界与最终结论

本方案把一次任务执行隔离到一个一次性的 Runner 进程中。GPUI 和 Python sidecar 始终存活；任务正常结束、异常、卡死或被用户停止时，只回收本次 Runner 及其任务专属资源。

最终进程结构：

    AALC.exe（GPUI）
    └─ AALC Backend.exe（Python sidecar）
       └─ AALCRunner.exe（每次执行新建，用完即退）
          └─ 本次任务的本地辅助进程，例如 adb shell

MuMu 管理器、MuMu 虚拟机、PC 游戏和全局 adb server 都是外部共享进程，不得放入 Runner 的强制终止范围。设备端 scrcpy server 不是 Windows 子进程，必须额外清理。

职责划分：

| 组件 | 唯一职责 |
| --- | --- |
| GPUI | 界面、用户操作、外部 WebSocket 客户端、停止状态展示和连接恢复 |
| sidecar | 权威执行状态、设备租约、Runner 监管、主配置写入、统计、通知、持久化日志和结束后动作 |
| Runner | 单次任务、任务期间设备控制、任务预览、可取消等待和本地辅助进程 |

核心约束：

- 一个 Runner 只运行一个 runId，结束后绝不复用。
- sidecar 和 Runner 绝不能同时持有同一设备控制器。
- 强制终止后不依赖 Runner 的 finally；sidecar 必须执行补偿清理。
- execution.stop 的接收和状态切换不能排在普通 mutation、截图或设备连接之后。
- sidecar 是用户主配置、统计文件和通知的唯一写入者。
- 不再使用 TerminateThread，也不把进程层级当作隐藏或反检测手段。

## 2. 当前问题与必须修正的行为

当前实现有以下直接风险：

- execution.stop 只设置协作取消事件，之后对任务线程执行无界 join，任务卡在 native 调用时会拖死请求线程。[backend_application.py](../module/backend_application.py#L608)
- mutation RPC 共用单线程 executor，设备连接、截图等慢操作可能排在停止请求前面。[websocket_server.py](../module/websocket_server.py)
- 暂停等待使用原始 time.sleep，取消无法唤醒暂停中的任务。[input_handlers](../module/automation/input_handlers/__init__.py)
- PreviewCapture.stop 会先清空引用，再对线程做有界 join；返回时旧线程仍可能存活。[preview_capture.py](../module/preview_capture.py)
- MuMu 截图线程存在超时后遗留的已知路径。[mumu_control.py](../module/automation/input_handlers/simulator/mumu_control.py)
- module 包导入阶段会创建配置单例，而且默认固定读取 ./config.yaml。单纯复制临时文件不会让 Runner 使用它。[module/__init__.py](../module/__init__.py) [config/__init__.py](../module/config/__init__.py)
- 任务会合法修改 last_auto_change、hard_mirror、hard_mirror_chance、队伍轮换字段和 set_win_size；全部丢弃会改变现有业务语义。[script_task_scheme.py](../tasks/base/script_task_scheme.py) [mumu_control.py](../module/automation/input_handlers/simulator/mumu_control.py)
- 当前 scrcpy 路径的本地子进程主要是 adb shell；设备端 server 使用 cleanup=false，单靠 Job Object 不能清掉远端 server 或 adb forward。
- 当前 GPUI 的停止超时是 15 秒且只记录日志；本文实施时要改为 5 秒状态对账，而不是把它描述成已有行为。[scheduling.rs](../gpui-app/src/app/scheduling.rs)

因此，进程隔离、事务式设备租约、明确 IPC、配置增量合并和补偿清理必须作为同一个改造交付，不能只增加一个 subprocess。

## 3. 权威执行模型

### 3.1 ExecutionSupervisor

sidecar 新增单一的 ExecutionSupervisor actor，独占以下状态和行为：

    reserve_start(spec)
    request_pause(paused)
    request_stop(requested_by)
    on_runner_event(frame)
    on_runner_exit(exit_code)
    acquire_device_lease()
    restore_device()
    finalize_once()

它使用专用控制 mailbox 和监督线程，不占用普通 mutation executor。WebSocket handler 只做校验、原子状态更新或投递命令，然后立即返回。

规则：

- execution.start 在一把状态锁内检查 idle、预留 runId、递增 stateRevision 并切到 starting，然后立即返回。设备停机和 Runner 冷启动在 supervisor 中异步完成。
- execution.stop 在一把状态锁内记录停止意图并切到 stopping，然后唤醒 supervisor；它不得同步写可能阻塞的 pipe，也不得等待任务线程。
- Runner command writer 使用独立线程和有界优先队列。stop、finishAck 的优先级高于 setPaused；即使命令 pipe 堵塞，3 秒监管计时也继续运行。
- 重复 stop 幂等；重复 pause/resume 使用明确目标值；非 idle 的重复 start 返回 EXECUTION_BUSY。
- 共享状态只由 supervisor 或同一状态锁修改。普通 mutation executor 不得直接改执行状态。
- 一次 runId 只允许 finalize_once 一次；任务统计、最终状态、通知和结束后动作都经过同一完成门。

### 3.2 状态机

外部状态：

    idle
      └─ start → starting
                    ├─ runner ready → running ⇄ paused
                    ├─ stop / startup failure → stopping
                    └─ lease or launch failure → restoring

    running / paused
      ├─ stop → stopping
      ├─ normal completion → restoring
      └─ crash → restoring

    stopping
      ├─ cooperative exit → restoring
      └─ grace timeout → terminate tree → restoring

    restoring
      └─ restored / disconnected / restore failed → idle

设备租约是正交状态：

    none → acquiring → runner → restoring → none

不变量：

- state != idle 或 deviceLease != none 时，HomeState::is_busy() 必须为 true。
- state 为 running 或 paused 时，deviceLease 必须为 runner。
- 进入 idle 前，Runner 进程已退出、事件 pipe 已读到 EOF、补偿清理已完成或明确失败。
- restoring 期间仍禁止新任务和全部设备写操作。
- stop 可以发生在 acquiring、Runner 握手、设备初始化、running 或 paused 的任意阶段。
- stop 发生在 Runner 创建前时，取消租约获取并恢复 sidecar；创建后则进入同一协作停止与强杀流程。

权威状态载荷：

    {
      "schemaVersion": 3,
      "state": "idle|starting|running|paused|stopping|restoring",
      "stateRevision": 27,
      "currentTaskId": "mirror",
      "runId": "uuid",
      "runnerPid": 1234,
      "deviceLease": "none|acquiring|runner|restoring",
      "outcome": null|"completed"|"stopped"|"failed"|"crashed",
      "forced": false,
      "requestedBy": null|"user"|"shutdown"|"watchdog",
      "error": null|{"code":"...","message":"...","phase":"...","recovery":"retry|reconnect_device|restart_backend|report_bug"},
      "deviceRestore": "not_needed|pending|restored|disconnected|failed"
    }

outcome 描述业务结果；requestedBy 描述谁请求停止；forced 描述是否用过进程级终止。三者不能再混入一个含义不清的 stopReason。

最终 idle 事件保留刚结束的 runId、outcome、forced、error 和 deviceRestore，直到下一次 start 被接受；下一次 start 才清空旧结果。进入 idle 后 runnerPid 和 currentTaskId 为 null，runId 仍表示最近一次运行。GPUI 以 stateRevision 判断新旧状态，以 runId 和事件 seq 丢弃迟到数据。

### 3.3 统计结算

- start 被原子接受时创建统计 run，并记录 runner_ready_at 以区分冷启动耗时。
- task.completed 事件带单调 seq，只能结算一次。
- 完成、用户停止、任务异常、Runner 崩溃和强杀都走 finalize_once。
- sidecar 持久化统计和配置后，才能发送完成通知或执行关机等结束后动作。
- Runner 不直接写统计文件。

## 4. 事务式设备租约

### 4.1 DeviceLeaseManager

新增 DeviceLeaseManager，并让所有 sidecar 设备入口经过同一租约守卫。不能只在 GPUI 禁用按钮；服务端必须拒绝越权调用。

租约对象至少包含：

    runId
    generation
    DeviceTarget snapshot
    previewWasEnabled
    originalWindowState
    reservedScrcpyScid
    reservedSocketName
    reservedAdbForwardPort
    acquiredAt

generation 是 sidecar 内单调递增的能力代号。旧 PreviewCapture 回调、旧连接回调和旧 controller 必须在发布画面或修改状态前校验 generation，防止“已经 stop 但迟到回调又复活”。

### 4.2 获取租约

获取过程是一个事务：

1. start 在 supervisor 中预留 runId，deviceLease 变为 acquiring。
2. 若设备正在连接或断开，等待其在有界时间内完成；无法安静停下则失败关闭，不能继续创建 Runner。
3. 若有活跃设备工具、工具预览或一次性截图操作，拒绝 start 并返回 DEVICE_TOOL_ACTIVE。第一版不自动停止用户工具。
4. 记录选中 DeviceTarget、预览意图、PC 窗口矩形/样式以及清理所需的保守快照。
5. 调用新的 PreviewCapture.stop_and_wait(deadline)，只有线程确实退出才算成功；不能在 join 前清空唯一引用。
6. 调用 DeviceSession.suspend_for_execution(deadline)，禁止新调用，等待所有 in-flight 操作清零，关闭 controller，并保留逻辑选中目标。
7. 校验 sidecar 中没有该 generation 的 controller、预览线程、截图 future 或工具句柄。
8. 为本次运行预留唯一 scrcpy scid、socket 名和 adb forward port，提前写入补偿清理账本。
9. deviceLease 切为 runner 后才允许 Runner 初始化设备。

任一步失败：

- 不启动 Runner，或立即终止尚未初始化业务模块的 Runner。
- 进入 restoring，按快照恢复 sidecar 设备会话。
- 发布明确错误；不能在“可能还有旧预览线程”的状态下继续。
- 若旧 PreviewCapture/native worker 无法证明退出，标记 recovery=restart_backend；这是允许走受控 sidecar 恢复的安全故障，不得伪装成已恢复或继续启动任务。

需要改造 PreviewCapture.stop 为可证明停机的 stop_and_wait：

- 先请求停止；
- 保留线程和 controller 引用；
- 等待线程退出；
- 成功后再清引用；
- 超时返回结构化失败，并让租约获取失败关闭。

MuMu 已知可能泄漏的截图 worker 同样必须计入 quiescence 检查。不能仅依赖 Python 对象引用已清空。

### 4.3 租约期间的 sidecar 行为

以下操作必须经过租约守卫：

- device.connect、device.disconnect、设备切换；
- preview 启停和抓帧；
- tool.start、tool.stop、工具截图、分辨率和窗口写操作；
- 任何直接访问 active DeviceSession 或 controller 的后端方法；
- 可能触发 MuMu/ADB 重连、重启或启动游戏的恢复路径。

行为约定：

- 设备只读列表和 execution.getState 可以调用，但要显示 leased 状态。
- preview.setEnabled 可只更新“期望值”，不得在租约期间创建 controller；恢复后再应用。
- screenshot、resolution 等即时设备调用返回 DEVICE_LEASED。
- 活跃工具导致 start 被拒绝；租约开始后不会再出现 sidecar 工具 controller。
- 所有异步回调同时校验 runId/generation。

### 4.4 恢复租约

Runner 进程树完全退出且 event pipe 排空后：

1. 执行幂等补偿清理。
2. 根据 deviceDisposition 决定是否重连。
3. 若 disposition=restore，重新构造全新的 sidecar controller。
4. controller 健康检查通过后恢复预览。若租约期间 preview.setEnabled 改过期望值，使用最新 desiredPreviewEnabled；否则使用 previewWasEnabled 快照。
5. 将 deviceRestore 置为 restored；失败则置为 failed 或 disconnected。
6. 清租约，进入 idle。

绝不能复用 Runner 中的对象，也不能假定强杀执行了析构函数。恢复失败不重启一个仍响应的 sidecar；应保持 sidecar 可用、显示错误，并允许用户手动重连。

## 5. Runner 启动与进程监管

### 5.1 一次性启动器

开发入口不能再使用 python -m module.execution_runner，因为 Python 会先导入 module/__init__.py，导致配置单例在 AALC_CONFIG_PATH 设置前创建。

本方案确定使用仓库根目录、且不依赖 module 包的最小入口：

    python runner_bootstrap.py

打包的 execution_runner.spec 同样以 runner_bootstrap.py 为入口。

bootstrap 只能导入标准库和 IPC/平台监管最小代码。流程：

1. 从不可变启动参数读取 runId、protocol、expectedParentPid；pipe handle/fd 只通过显式继承获得。配置内容和秘密不放进命令行。
2. 在 Linux 立即安装 PDEATHSIG，并校验 expectedParentPid；Windows 此时已经处于 Job 内。
3. 初始化原始二进制 command/event pipe；stdout 专用于协议。
4. 安装 stderr 捕获和最小崩溃处理。
5. 发送 hello。
6. 等待 attached。
7. 等待 start，并校验 runId、协议和绝对路径。
8. 设置 AALC_CONFIG_PATH、AALC_RUN_ID、资源根目录和 Runner 策略。
9. 确认环境变量已生效后，才 import module 和任务代码。
10. 初始化 controller，发送 ready，再发送 running。

任何业务模块、控制器或辅助进程都不得在 attached/start 前创建。

Runner 是一次性的，以下状态不会跨任务污染：

- mediator 的全局 signal handler；
- pause/cancel Event；
- controller 类变量和截图缓存；
- 任务插件模块缓存；
- 临时配置单例；
- logging handler 和后台线程。

### 5.2 Windows

Windows 是首个正式发布平台，必须使用 Job Object：

- 建立 Job 并设置 JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE。
- Job handle 不可继承，也不能被 Runner 或孙进程复制持有。
- 使用 CreateProcessW 的 CREATE_SUSPENDED 和 CREATE_NO_WINDOW 创建 Runner。
- 只把 command/event/stderr 所需 pipe handle 放入显式继承列表。
- 在恢复主线程前调用 AssignProcessToJobObject。
- AssignProcessToJobObject 失败时关闭本次启动并报错，不能降级成不受监管的 Runner。
- 不允许 CREATE_BREAKAWAY_FROM_JOB；Runner 创建的本地辅助进程默认继承 Job。
- sidecar 正常停止使用 TerminateJobObject 兜底；sidecar 崩溃时由 KILL_ON_JOB_CLOSE 清理整棵本地进程树。

如果 sidecar 自己位于不允许嵌套分配的外部 Job 中，启动必须失败并给出诊断。测试要覆盖这一情形。不能以“握手足够快”替代 suspended-create → assign → resume 的无竞态顺序。

父子关系验收同时比较 ParentProcessId、进程创建时间和本次 runId，避免 PID 重用误判。[Win32_Process](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-process)

相关资料：

- [Creating Processes](https://learn.microsoft.com/en-us/windows/win32/procthread/creating-processes)
- [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
- [AssignProcessToJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject)
- [TerminateJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-terminatejobobject)

### 5.3 Linux

- 使用 start_new_session=True 创建独立 session/process group。
- Runner bootstrap 在任何业务 import 前设置 PR_SET_PDEATHSIG(SIGKILL)，随后再次校验 getppid() 等于 spec 中的 sidecarPid；若父进程已变化则立刻退出。
- command pipe EOF 也是父进程死亡信号。
- 协作超时后先 killpg(SIGTERM)，短暂等待，再 killpg(SIGKILL)。
- Runner 的本地子进程不得自行 setsid 或逃离进程组。

仅有 start_new_session 不能保证 sidecar 崩溃后自动回收，PDEATHSIG 和父 PID 二次校验不可省略。[PR_SET_PDEATHSIG](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html)

### 5.4 macOS

macOS 没有 PDEATHSIG。可使用 command pipe EOF 加 kqueue 父进程监视，并由 sidecar 正常路径 killpg；但若 Runner 同时卡在不可中断 native 调用且 sidecar 崩溃，无法提供与 Windows 相同的硬保证。

因此：

- macOS adapter 可实现和测试；
- macOS 不作为第一阶段发布门；
- 若未来要求崩溃后硬保证，需要增加极小的原生 watchdog，再提升支持等级。

### 5.5 Runner 定位

sidecar 按以下顺序定位 Runner：

1. AHAB_RUNNER_EXE 指定的绝对路径，仅供开发和测试；
2. frozen 环境下应用根目录 runner/AALCRunner.exe；
3. 开发环境下仓库根目录 runner_bootstrap.py，并由 sys.executable 启动。

working directory 固定为应用根目录；配置和资源路径一律传绝对路径，不依赖当前目录。

## 6. Runner IPC 协议

### 6.1 传输

Runner 是 sidecar 的直接子进程，使用两个继承的匿名 pipe，不开放 127.0.0.1 端口：

- command pipe：sidecar → Runner；
- event pipe：Runner → sidecar；
- stderr pipe：Runner → sidecar，仅用于启动失败和崩溃诊断。

匿名 pipe 本身限定为父子持有，不再需要 token。创建 Runner 时仅继承必要 handle/fd；Runner 再启动辅助进程时必须关闭或禁止继承 command/event pipe。

stdout 完全保留给 event 协议。业务 print、Python logging 和第三方库输出必须重定向到 logging bridge 或 stderr，任何裸文本写入 stdout 都视为协议损坏。

stderr 由 sidecar 独立持续排空到有界环形缓冲区，例如最后 256 KiB，并在崩溃错误中附带尾部；stderr 满不能反向卡住 Runner。

### 6.2 帧格式

每一帧使用网络字节序：

    [u32 totalLength]
    [u32 headerLength]
    [headerLength bytes UTF-8 JSON]
    [optional binary payload]

定义：

- totalLength 是第一个 u32 之后的字节数，即 4 + headerLength + binaryLength。
- totalLength 上限 8 MiB。
- headerLength 上限 64 KiB。
- preview JPEG binaryLength 上限 4 MiB。
- header 中声明 binaryLength；它必须与帧剩余长度完全一致。
- JSON 必须是对象；缺少必填字段、未知消息 type、非法 UTF-8、负值、溢出、截断或超限都立即终止本次 Runner。同一 protocol 内可忽略明确声明为可选的未知字段。
- 两端都必须用 read_exact/write_all 循环处理短读短写；一条 pipe 只有一个 writer，禁止多线程交错写帧。帧中途 EOF 视为 Runner crash 或协议错误。

除 hello 外，Runner 事件头必含：

    {
      "type": "event.name",
      "protocol": 1,
      "runId": "uuid",
      "seq": 18,
      "binaryLength": 0
    }

seq 在每个 runId 内从 1 严格单调递增，在 event writer 真正序列化帧时才分配，因此被合并或丢弃的低优先级事件不消耗 seq。sidecar 丢弃 seq 小于等于已处理值的重复帧；发现非法范围、溢出或 runId 不匹配时将协议标记为失败。命令使用独立 commandSeq。

### 6.3 握手

固定顺序：

1. sidecar 已预留 runId 和设备租约，创建受监管但尚未导入业务代码的 Runner。
2. Runner 发送 hello：

       {"type":"hello","protocol":1,"runId":"...","pid":1234}

3. sidecar 校验协议、runId、PID 和进程创建信息，发送 attached：

       {"type":"attached","protocol":1,"runId":"...","commandSeq":1}

4. sidecar 发送 start，包含 ExecutionSpec：

       {"type":"start","protocol":1,"runId":"...","commandSeq":2,"spec":{...}}

5. Runner 设置配置环境、导入业务模块、建立设备控制器，成功后发送 ready。
6. Runner 发送 status=running；此时 sidecar 才对外发布 running。

hello 超时、协议不匹配、PID 不匹配、配置初始化失败或 ready 超时都进入启动失败 → 终止 Runner → restoring。

### 6.4 命令

| type | 关键字段 | 语义 |
| --- | --- | --- |
| attached | runId | 确认进程已纳入监管，可继续启动 |
| start | runId, spec | 一次且仅一次 |
| setPaused | runId, paused | 显式目标状态，不使用 toggle |
| stop | runId, requestedBy | 设置取消并解除暂停；不等待完成 |
| finishAck | runId, finalSeq | sidecar 已持久接收 finished 及之前帧 |
| shutdown | runId | 仅用于 start 前撤销或测试 |

写 command pipe 的线程与 WebSocket handler 分离。命令重复时按 commandSeq 去重；stop 始终幂等。

### 6.5 事件

| type | 载荷 | 丢弃策略 |
| --- | --- | --- |
| ready | device metadata | 不可丢 |
| status | running/paused/stopping | 不可丢 |
| task.started | taskId, timestamp | 不可丢 |
| task.completed | taskId, result | 不可丢、按 seq 去重 |
| mirror.progress | completed, total | 可合并为最新值 |
| mirror.floor | floor | 可合并为最新值 |
| preview.frame | width, height, format + JPEG | 只保留最新帧 |
| preview.status | enabled, error | 不可丢 |
| log.entry | timestamp, level, logger, message | debug/info 可限流；warning/error 不可丢 |
| warning | code, message | 不可丢 |
| hdr.warning | message | 不可丢，不传 threading.Event |
| config.delta | deltaId, baseRevision, changes/operations | 不可丢 |
| resource.created | resourceType, resourceId, metadata | 不可丢 |
| resource.released | resourceType, resourceId | 不可丢 |
| afterCompletion.requested | typed actions, disposition | 不可丢 |
| app.focusRequested | reason | 不可丢 |
| heartbeat | monotonic timestamp | 可覆盖 |
| finished | outcome, error, forced=false, deviceDisposition | 不可丢 |

现有 mediator 中的 task_started、task_completed、mirror_signal、mirror_floor_signal、warning 和 hdr_warning 改成以上类型事件。script_finished 由 finished 取代；kill_signal 改成 app.exitRequested/afterCompletion.requested。update_progress、download_complete 若只属于 sidecar 更新流程则留在 sidecar，不为“对齐名称”盲目跨进程桥接。

### 6.6 背压、心跳与终止顺序

Runner 内至少有三个执行域：

- command reader：只处理控制命令；
- task worker：运行任务；
- event writer：按优先级写事件。

预览和普通日志使用有界队列；预览永远只保留最新一帧。关键事件进入独立的保留容量。若关键事件在规定时间内仍不能入队或写出，Runner 将该运行标记为 IPC_FAILURE 并退出；不能静默丢 task.completed、config.delta、resource 账本或 finished。

Runner 每 1 秒发送 heartbeat。sidecar 以进程 handle 为权威：

- 进程已退出：立即排空 pipe 并结算。
- 进程仍在但 5 秒无 heartbeat 且 command 无响应：标记 unhealthy 并强制终止。
- 仅预览停更不能判定 Runner 死亡。

正常结束顺序：

1. task worker 停止并完成 Runner 内可执行的清理。
2. 发送最后一个 config.delta、resource.released 和 afterCompletion.requested。
3. 发送 finished，并停止产生新事件。
4. sidecar 按 pipe FIFO 接收，将 finalSeq、待合并 delta 和完成意图写入可恢复 journal 后，发送 finishAck。
5. Runner 最多等待 1 秒收到 ack，然后退出。
6. sidecar 等待进程退出并读到 event EOF，再做补偿清理和设备恢复。

finished 是报告，不是“进程已死”的证明。Runner 收到 ack 后仍不退出时，supervisor 需在短超时后强制回收。若没有 finished，sidecar 根据停止意图、退出码、心跳和 stderr 将 outcome 推断为 stopped、failed 或 crashed。

## 7. ExecutionSpec 与配置一致性

### 7.1 ExecutionSpec

只传可序列化快照：

    runId
    taskId
    task configuration snapshot
    runtime configuration snapshot
    configPath（Runner 临时文件绝对路径）
    configRevision
    resourceRoot（绝对路径）
    platform policy
    allowEmulatorLaunch=false
    DeviceTarget:
      id
      kind
      hwnd
      endpoint
      instanceNumber
    CleanupReservation:
      scid
      socketName
      adbForwardPort
      originalWindowState

不传 DeviceSession、controller、线程锁、Event、Python callback 或全局对象。

runId、protocol 和 expectedParentPid 属于 bootstrap 启动参数，在创建进程时就提供，不等待 ExecutionSpec；Runner 必须校验启动参数与 start 中的 runId 一致。

### 7.2 临时配置

sidecar 在获取租约时：

- 从 ConfigRepository 中的权威配置生成规范化快照，并记录 configRevision、规范化内容的 baseConfigHash 与每个允许回写字段的基线值。
- 在本次 run 专属临时目录写 config.yaml。
- POSIX 目录权限 0700、文件 0600；Windows ACL 仅授予当前用户。
- 不向 Runner 提供通知 token、更新凭据等任务不需要的秘密；若旧 Config 必须包含这些字段，先实现可裁剪的 RunnerConfig。
- 进程退出且 delta 已处理后删除临时目录。

module/__init__.py 必须改为优先读取 AALC_CONFIG_PATH，并将默认 ./config.yaml 解析为应用根目录绝对路径。Runner bootstrap 在任何 module import 前设置该变量，测试必须证明配置单例使用的是临时路径。

sidecar 内现有所有 cfg.save/cfg.set_value 调用要逐步收口到 ConfigRepository。它负责原子保存、递增 configRevision 和重新计算 canonical SHA-256。应用外手工编辑 config.yaml 时，Repository 在合并 delta 前重新读取磁盘哈希；哈希变化即按并发修改处理，不能因为内存 revision 未变化而覆盖外部编辑。

任务资产、主题、路线和权重文件按只读资源传递。Runner 禁止写共享日志、统计、主题包或主配置。

### 7.3 增量回写

不能选择“全部丢弃”或“把临时 config.yaml 覆盖回来”。sidecar 是唯一主配置 writer，Runner 只发送白名单 delta。

第一版白名单：

- last_auto_change
- hard_mirror
- hard_mirror_chance
- set_win_size
- 队伍轮换语义操作 rotateTeamQueue；sidecar 根据当前 schema 重建 teams_active_queue、teams_be_select、teams_order、teams_be_select_num，Runner 不直接覆盖四个旧字段

Runner 在每个已完成任务检查点和最终退出前计算 delta：

    {
      "type": "config.delta",
      "runId": "...",
      "seq": 31,
      "deltaId": "uuid",
      "baseRevision": 42,
      "baseConfigHash": "sha256:...",
      "changes": {
        "hard_mirror_chance": 2,
        "last_auto_change": "..."
      },
      "operations": [
        {"op":"rotateTeamQueue","completedTeamId":"team-1"}
      ]
    }

sidecar 合并规则：

1. 先按 runId、seq、deltaId 去重。
2. 非白名单字段拒绝并记安全日志。
3. 当前 configRevision 与 baseRevision、当前 canonical hash 与 baseConfigHash 都相同，直接应用合法 delta。
4. revision 已变化时逐字段比较：当前值仍等于该字段基线才应用；用户已修改则保留用户值并记录 CONFIG_CONFLICT。
5. rotateTeamQueue 使用稳定 team id 做语义合并；目标队伍不存在或队列被用户重构时跳过并提示，不能覆盖整队列。
6. 校验类型、范围和业务约束后，以 sidecar 现有原子保存路径一次提交并递增 revision。

已收到并确认的检查点 delta 即使之后强杀也可保留，因为它代表已完成的业务步骤。只存在 Runner 临时文件、尚未发送的修改在强杀后丢弃，不能猜测回写。

## 8. 日志与 mediator 适配

- Runner 安装 IPC logging.Handler，把结构化 log.entry 发给 sidecar。
- sidecar 做敏感字段清理后，接入现有 rotating file logger 并转发 GPUI。
- Runner 不打开主日志文件，避免多进程轮转冲突。
- debug/info 日志可限流和合并；warning/error 不可丢。
- traceback 作为结构化 error 和 stderr 尾部传递，单条消息仍受帧上限约束。

hdr_warning 当前携带 threading.Event 并等待 acknowledged.wait。跨进程后不得序列化 Event。bridge 只发送消息，并在成功放入关键事件队列后立刻 set 本地 acknowledged；这与 sidecar 当前自动确认行为一致，且不会让任务因 UI 未响应而死锁。若未来需要用户确认，应另设计带超时的 request/response 协议。

所有 completion toast、外部通知和持久化日志只由 sidecar 发出。任务完成音效也转成 typed request 由 sidecar 执行，避免音频辅助进程在 Runner 退出时被 Job 杀掉。

## 9. 取消、暂停与协作退出

Runner 不得实例化 BackendApplication，也不得启动 WebSocket、更新器或 sidecar 的设备管理器。应从现有 my_script_task 调度中抽出 RunnerTaskHost/TaskRuntime，通过显式依赖注入获得：

    config
    execution_control
    mediator_bridge
    runner_owned_controller
    resource_ledger_client
    task_spec

每个 run 创建新的 mediator 和 handler，退出时全部注销。任务模块若反向 import backend_application、全局 DeviceSession 或主配置 writer，视为迁移阻断项，必须先拆除。

Runner 内新增单一 ExecutionControl：

    cancel_event
    paused
    condition
    set_paused(bool)
    request_stop()
    wait_if_paused()
    interruptible_sleep(seconds)
    checkpoint()

规则：

- request_stop 先设置 cancel_event，再把 paused=false，并 notify_all。
- wait_if_paused 使用 Condition 的短超时循环，每次检查 cancel_event。
- 新建或替换 controller 时立即绑定同一个 ExecutionControl，不允许复制一次性布尔值。
- 任务关键路径中的 time.sleep 改为 interruptible_sleep。
- 长循环、楼层切换、识图重试、网络重试和输入序列都有 checkpoint。
- 截图锁、输入锁采用短超时获取并检查取消，不做无界 Lock.acquire。
- adb 和外部命令必须有超时，子进程注册到本次资源账本。
- 鼠标拖动、按键、触摸序列用 try/finally 对称释放。

允许在经过审查的底层传输代码中保留短 sleep，但必须有明确上限且外层可被 3 秒强杀兜底。新增静态检查，阻止 tasks 目录继续引入未经标注的直接 time.sleep。

停止时限从 sidecar 收到 execution.stop 并发布 stopping 开始：

- RPC 接收/确认目标：p95 小于 200 ms。
- Runner 协作宽限：3 秒。
- TerminateJobObject 或 SIGKILL 后等待树死亡：最多 1 秒。
- 设备补偿与恢复使用独立时限，默认 20 秒；慢恢复不能被误判为“停止请求没收到”。

## 10. 强杀后的补偿清理

强制终止不会运行 Python finally、atexit 或对象析构。sidecar 在创建 Runner 前建立 CleanupLedger，先放入保守清理信息，再用 resource.created/resource.released 事件细化。

CleanupLedger 同时是可恢复 journal，不只是内存对象：

- 每个 run 使用应用数据目录下的独立 journal，采用临时文件 + 原子替换更新；不记录配置秘密。
- 在 Windows 恢复 Runner 主线程、或其他平台发送 start 前，必须先把 runId、目标、进程创建信息、预留 scid/port、窗口快照和状态 cleanup_pending 持久化。
- 关键 resource 事件、已接收 config.delta、finalSeq 和完成意图写入 journal 后才向 Runner 确认。
- 正常清理全部成功后标记 complete，再删除 journal；删除失败不影响结果，但下次启动可识别 complete 并安全清扫。
- sidecar 启动时先扫描未完成 journal，按 PID + 创建时间验证本地残留，并执行同一幂等补偿；在恢复检查完成前不接受新任务。
- journal 已持久记录合法 finished 时，恢复其原 outcome；没有 finished 的未完成统计 run 才以 crashed 结算一次。已持久接收但尚未合并的合法 config.delta 按原 CAS 规则恢复处理。
- 设备离线导致远端清理暂时不可执行时，保留最小 pending journal，将设备标记 disconnected；下次连接同一目标时先清理旧 scid/socket，再允许新租约。

这保证 sidecar 自身崩溃时，Windows 的 KILL_ON_JOB_CLOSE 负责本地进程，新 sidecar 则能继续清理远端 server、forward、窗口和输入状态。Linux/macOS 也用 journal 辅助识别异常遗留，不能只依赖父进程死亡信号。

账本包含：

- runId、target id、generation；
- Runner 和已知本地 helper PID；
- 预留的 adb forward port、scrcpy scid/socket 和设备序列号；
- PC HWND、原始窗口矩形、样式和置顶状态；
- 本次可能按下的鼠标键、修饰键和普通按键；
- Android 触点/指针 id；
- 设备 disposition。

清理按幂等步骤执行，每一步有独立超时并记录结果：

1. 确认 Runner Job/进程组已无存活进程。
2. 只对账本中的目标窗口发送对应 button-up/key-up；不激活窗口，不向全局无差别释放按键。
3. 能重连控制通道时发送 Android touch cancel/up；不能重连则记录 best-effort 失败。
4. 恢复 PC 窗口原始矩形和样式，不强制抢焦点。
5. 删除本次唯一 adb forward。
6. 按本次 scid/socket 验证并终止残留的设备端 scrcpy server；不能误杀其他会话。
7. 清理临时目录、缓存和 sidecar 中的旧 generation 引用。
8. 创建全新的 sidecar controller，再恢复预览。

keep-awake 等随连接或进程终止通常会由系统释放，但测试必须验证；不能仅写在假设里。

即使 resource.created 在崩溃前没来得及发送，父进程预留的 scid/port、窗口快照和保守输入释放集合仍可执行清理。resource 事件用于缩小范围和提供诊断，不是唯一安全依据。

## 11. MuMu、ADB 与 scrcpy 约束

Runner 的默认策略是只附加到当前已存在的 MuMu 实例：

    allowEmulatorLaunch = false

在该策略下：

- adb_connect、start_game 和自动恢复路径不得调用 MuMu close/start/restart。
- 实例未运行、连接丢失或需要重启时，本次任务失败并返回明确错误。
- 用户明确配置的成功后 exit_emulator 是例外，但只能在任务成功且 Runner 仍持有租约时执行。
- MuMu 管理器和 VM 主进程永远不加入 Runner Job。
- Runner 模式禁止通过 shell、计划任务或 breakaway flag 启动不受监管的本地 helper。

scrcpy 清理不能只找 scrcpy.exe：

- 当前本地进程可能是 adb shell，Job/进程组负责清它。
- adb server 可能是共享常驻进程，不得整体终止。
- 每次运行使用唯一 local forward、scid/socket；sidecar 预先知道这些标识。
- 设备端 server 使用 cleanup=false 时，补偿流程必须显式验证并清理。
- 正常退出也执行同一幂等清理，只是通常所有项已被 resource.released 标记。

## 12. 完成后的动作

把结束后动作按所有权拆分。

Runner 在仍持有设备租约时可执行：

- exit_game
- exit_emulator

Runner 通过 finished.deviceDisposition 报告：

    restore
    game_closed
    emulator_closed

sidecar 行为：

- restore：正常重连 controller 和预览。
- game_closed：不自动重新启动游戏；将设备会话标为 disconnected/needs-user-action。若未来要显示模拟器桌面，另做显式产品选项。
- emulator_closed：不重连，设备状态为 disconnected。

sidecar 在配置、统计、日志和清理落盘后执行：

- completion notification/toast/sound；
- exit_aalc：发送 app.exitRequested 给 GPUI，GPUI 走正常关闭流程并抑制自动恢复 sidecar；
- shutdown/reboot/sleep 等电源动作。

默认只有 outcome=completed 执行结束后动作。用户停止、任务失败、Runner crash 或 forced=true 时不执行，除非未来增加明确且单独的策略开关。

每个 sidecar 动作使用 runId + actionType 作为 actionId，并在 journal 记录 pending/executing/done。通知可按 actionId 幂等补发；exit_aalc 和电源动作若 sidecar 在 executing 状态崩溃，重启后默认标记 unknown/skipped，不自动重放潜在破坏性动作。没有持久 finished 的恢复运行按 crashed 处理，自然不会触发任何完成动作。

## 13. WebSocket 与 GPUI

### 13.1 Python WebSocket

外部 RPC 名称保留：

    execution.start
    execution.pause
    execution.resume
    execution.stop
    execution.getState

schema 3 的控制请求携带关联信息：

    execution.start:
      {"clientRequestId":"uuid","taskId":"...","options":{...}}

    execution.pause/resume/stop:
      {"runId":"当前 UI 所见的 runId"}

start 的 runId 由 sidecar 分配；clientRequestId 在短期缓存中幂等，避免响应丢失后的重试创建第二次运行。pause、resume、stop 必须校验 runId，旧页面或迟到请求返回 STALE_RUN，绝不能作用到后来启动的新任务。

成功响应都带完整或最小权威快照：

    {
      "accepted": true,
      "runId": "uuid",
      "state": "starting|paused|running|stopping|restoring|idle",
      "stateRevision": 27
    }

语义：

- pause 只接受 running；resume 只接受 paused，否则返回 INVALID_EXECUTION_STATE。
- stop 接受 starting/running/paused/stopping；已是 stopping 时幂等返回当前快照。
- stop 在 idle 且 runId 等于最近 run 时幂等返回 idle；runId 不同返回 STALE_RUN。
- getState 不需要 runId，并始终返回完整状态载荷。

统一错误码至少包括：

| code | 含义 |
| --- | --- |
| EXECUTION_BUSY | 已有运行或租约未释放 |
| STALE_RUN | 请求 runId 不是当前/最近目标 |
| INVALID_EXECUTION_STATE | 当前状态不接受该命令 |
| DEVICE_TOOL_ACTIVE | 有工具或设备操作尚未结束 |
| DEVICE_QUIESCE_TIMEOUT | 旧 controller/preview 未证明退出 |
| DEVICE_LEASED | Runner 正在占用设备 |
| RUNNER_NOT_FOUND | 打包文件缺失 |
| RUNNER_SUPERVISION_FAILED | Job/进程组绑定失败 |
| RUNNER_HANDSHAKE_TIMEOUT | hello/ready 超时 |
| RUNNER_PROTOCOL_ERROR | 帧或版本不合法 |
| RUNNER_INIT_FAILED | Runner 设备/任务初始化失败 |
| RUNNER_UNRESPONSIVE | 心跳和命令均无响应 |
| IPC_BACKPRESSURE | 关键事件无法可靠传输 |
| CONFIG_CONFLICT | delta 与用户修改冲突；通常作为 warning |
| DEVICE_RESTORE_FAILED | 清理或设备恢复失败 |

路由：

- execution.start/pause/resume/stop 进入 ExecutionSupervisor 的轻量入口。
- stop 设置原子意图并唤醒专用 writer，不进入普通 mutation executor。
- 其他 mutation 继续使用普通 executor。
- 所有设备 RPC 额外经过 DeviceLeaseManager，不依赖 executor 隔离来保证安全。

### 13.2 GPUI 状态

GPUI 修改：

- ExecutionState 增加 Starting 和 Restoring。
- HomeState::is_busy 覆盖所有非 idle 状态和非 none 租约。
- start 提交成功前可显示本地 submitting；RPC 接受后以服务端 starting 为准。
- stop 点击后乐观显示 Stopping，但随后用 stateRevision 对账。
- 展示 completed/stopped/failed/crashed、是否 forced 和设备恢复失败。
- 恢复期间显示“正在恢复设备”，不能允许再次启动。
- 所有外部事件按 runId、stateRevision；Runner 派生事件还按 seq 去重。
- 旧 runId 的状态、统计、日志和预览不能覆盖新运行。

当前 15 秒 STOP_TIMEOUT 改为 5 秒“状态对账计时器”：

1. 5 秒未见权威状态推进时调用 execution.getState。
2. sidecar 仍响应且状态为 stopping/restoring：继续等待并显示阶段，不重启 backend。
3. sidecar 响应但状态矛盾：按返回快照修正 UI 并记录协议错误。
4. WebSocket 断开或 sidecar ping 失败：才进入现有 backend 恢复路径。

Runner 的 3 秒强杀由 sidecar 负责，GPUI 不直接杀 Runner。

### 13.3 Schema 升级

外部 schema 从 2 升到 3，Python 与 Rust 同步升级：

- 新状态 Starting、Restoring；
- 新增 stateRevision、runId、runnerPid、deviceLease、outcome、forced、requestedBy、error、deviceRestore；
- execution.getState 返回完整快照；
- app.exitRequested 和结构化错误加入事件合同。

当前握手要求 schema 精确相等，所以这不是仅靠 serde default 就能完成的向后兼容改动。发布包必须同时更新 GPUI 和 sidecar；若要支持混合版本，需要单独实现 2↔3 兼容窗口。Runner IPC protocol 版本独立为 1。

## 14. 构建与发布

新增：

    runner_bootstrap.py
    module/execution/supervisor.py
    module/execution/device_lease.py
    module/execution/ipc_protocol.py
    module/execution/cleanup_ledger.py
    execution_runner.spec

实际文件位置可按现有目录习惯调整，但 bootstrap 不能位于会提前导入业务模块的 package 下。

构建要求：

- 抽取 sidecar 与 Runner 共用的 PyInstaller collect 配置，避免 hiddenimports、data files 和二进制依赖漂移。
- Runner 固定使用 PyInstaller onedir/onedir-equivalent 形态，产物位于 runner/AALCRunner.exe 及其私有运行库目录；不使用 one-file。
- 原因是 PyInstaller one-file bootloader 可能再派生实际 Python 子进程，使 hello PID 不再等于 sidecar 直接创建并加入 Job 的 PID，同时增加冷启动解压成本。
- build 脚本同时生成并 staging 整个 runner 目录。
- release.json/更新清单加入 Runner bundle 内每个文件的相对路径、哈希、大小和版本。
- 若现有发布链有签名，Runner 必须进入同一签名流程。
- Windows 使用无控制台入口。
- 打包 smoke test 验证 runner bundle 定位、握手、任务模块导入和缺失/损坏 Runner 的可理解错误。

Runner onedir 可以暂时重复 sidecar 已有依赖，但必须测量体积和冷启动。启动性能门：

- warm ready 小于 3 秒；
- cold ready 小于 10 秒；
- starting 阶段持续发布进度；
- 冷启动耗时不计入停止 3 秒 SLA。

若体积超标，再评估受测试保护的共享 runtime；不能为了省体积或启动时间改回 one-file 或同进程任务线程。若未来确实改用会产生 bootloader 子进程的封装，握手必须同时校验 launchedPid、helloPid、祖先链和 Job membership，不能继续假定 PID 相等。

## 15. 实施顺序与迁移

### M0：合同和盘点

- 冻结外部 schema 3、Runner protocol 1、状态机和错误码。
- 盘点所有 sidecar 设备入口、直接 controller 访问、任务 sleep/锁/subprocess、配置写入和结束后动作。
- 为现有路径补基线测试。

### M1：Supervisor 与假 Runner

- 实现一次性进程、匿名 pipe、帧解析、状态 actor、事件去重和 finalize_once。
- 先用可控假 Runner 测正常、卡死、崩溃、协议损坏和海量预览。
- 实现 Windows Job；Linux adapter 同时保留接口测试。

### M2：配置、日志和事件桥

- 实现 bootstrap 与 AALC_CONFIG_PATH。
- 实现临时配置、白名单 delta、revision/CAS 合并。
- 接入 logging 和 mediator typed events，修正 hdr_warning。

### M3：设备租约和清理账本

- 实现 stop_and_wait、suspend_for_execution、generation guard。
- 把全部设备 RPC 接到租约守卫。
- 实现预留 scid/port、输入/窗口/远端 server 补偿。
- 加入 MuMu 禁止自动重启策略。

### M4：真实任务与取消

- 接入 my_script_task。
- 替换关键 sleep、无界锁和无超时 subprocess。
- 实现任务检查点 config.delta 和完成后动作拆分。

### M5：GPUI

- 升级 schema 和状态枚举。
- 接入 stateRevision/runId、5 秒对账和 restoring UI。
- 更新 mock gateway、快照和 Rust 合同测试。

### M6：打包与 E2E

- 增加 execution_runner.spec、release manifest、签名和 frozen smoke。
- Windows 真机执行完整停止/恢复矩阵。
- 达到发布门后删除旧线程执行和 TerminateThread 路径。

迁移期使用默认关闭的 AALC_EXECUTION_RUNNER 特性开关，只用于开发/灰度。旧路径和新路径不能同时持有设备；每次进程启动只选择一种执行引擎。达到发布门后默认开启，再在一个稳定版本后删除旧路径。回滚只切换下一次任务的引擎，不能在运行中迁移。

## 16. 测试矩阵

### 16.1 Python 合同与状态

- start 原子预留；双击 start 只有一个成功。
- 相同 clientRequestId 重试返回同一 runId；旧 runId 的 stop/pause 不影响新任务。
- acquiring、hello 前、hello 后、设备初始化中、running、paused 各阶段 stop。
- paused 中 stop 会解除等待，下一次运行不继承 pause/cancel。
- 普通 mutation 永久阻塞时，stop 仍在 200 ms 目标内确认。
- command pipe 堵塞时，supervisor 仍按 3 秒强杀。
- normal、failed、cooperative stopped、forced、crashed 都只 finalize 一次。
- 重复 seq、回退 seq、旧 runId 和 finished 后事件被拒绝。
- 预览洪水和日志洪水不阻塞关键事件或 stop。
- malformed/oversized/truncated frame 终止本次 Runner，不影响 sidecar。
- heartbeat 丢失但进程仍活着会触发 watchdog；进程退出不等待 heartbeat。
- hdr.warning 不序列化 Event，也不会无限等待确认。

### 16.2 配置

- Runner 在 import module 前设置 AALC_CONFIG_PATH。
- Runner 的 cfg.set_value 只写临时文件。
- 每个白名单字段正确回写；非白名单拒绝。
- revision 未变化直接合并；用户并发修改时保留用户值并报告冲突。
- config.yaml 被应用外手工修改时，baseConfigHash 能触发冲突合并。
- rotateTeamQueue 使用语义合并，不覆盖用户重排。
- 检查点 delta 在后续强杀时仍只应用一次。
- 未发送 delta 的临时修改不会覆盖主配置。
- 临时配置不含不必要秘密，退出后删除。

### 16.3 设备租约

- 活跃 tool、进行中截图或无法退出的 PreviewCapture 会阻止 Runner 启动。
- stop_and_wait 返回前线程确实死亡。
- stale generation 回调不能发布帧或修改设备状态。
- 租约期间每个设备写 RPC 都返回 DEVICE_LEASED。
- 正常、失败、协作停止、强杀和 Runner 崩溃后都不会出现重复 controller。
- previewWasEnabled=true/false 分别正确恢复。
- 恢复失败进入 idle+deviceRestore=failed，sidecar 仍响应。

### 16.4 强杀与资源清理

- 在 PC 鼠标拖动、修饰键按下、Android 多点触摸期间强杀，输入最终释放。
- 改变 PC 窗口大小/样式期间强杀，原状态恢复且不抢焦点。
- adb shell 被 Job/进程组清理。
- 本次 adb forward 和设备端 scrcpy server 被清理，不影响其他会话。
- 强杀后无 Runner、本地 helper、重复 server 或重复预览线程。
- 强杀后 sidecar ping、配置、日志和下一次任务仍可用。
- sidecar 自身在运行中崩溃后，新 sidecar 能读取 pending journal、结算 crashed 并继续远端清理。

### 16.5 平台

Windows：

- suspended create 后成功 assign 才 resume。
- Job 清理 Runner 和多层孙进程。
- sidecar 崩溃触发 KILL_ON_JOB_CLOSE。
- 外部嵌套 Job 导致 assign 失败时 fail closed。
- 无 console 窗口。
- ParentProcessId、创建时间与 runId 匹配。
- MuMu 管理器、VM 和 PC 游戏不在 Job 中。

Linux：

- process group TERM/KILL 清理多层子进程。
- sidecar 崩溃触发 PDEATHSIG。
- 父进程在 prctl 前死亡时，getppid 二次校验使 Runner 退出。

macOS：

- 正常 sidecar 路径的 killpg 和 pipe EOF 可用。
- 明确记录为 best-effort，不作为首发硬保证。

### 16.6 GPUI/Rust

- idle → starting → running ⇄ paused → stopping/restoring → idle 全转换。
- startup failure、crash、restore failure 和 stop during acquiring。
- stateRevision 较旧的快照不覆盖新状态。
- 旧 runId 的预览、日志和统计事件被丢弃。
- restoring 和非 none lease 均使 is_busy=true。
- 5 秒后 sidecar 响应 restoring 时不重启 backend。
- sidecar 失联时才进入现有恢复路径。
- app.exitRequested 走受控退出且不误触发 backend 自动拉起。
- schema 3 mock、序列化和缺字段错误信息一致。

### 16.7 打包 E2E

- 安装包/便携包中能定位 AALCRunner.exe。
- Runner 缺失、损坏、协议版本错误时给出可操作错误并恢复租约。
- 冷/热启动、正常完成、暂停停止、native 卡死强杀和下一次重跑。
- 发布清单、哈希、签名和更新后文件完整。

## 17. 发布验收

必须同时满足：

- execution.stop 从 WebSocket 收到到 stopping 确认 p95 < 200 ms。
- 已改造的可取消路径协作停止 p95 ≤ 2 秒。
- 不可中断路径在 3 秒宽限后强杀；从 stop 到本地进程树确认死亡 ≤ 4 秒。
- 停止期间 sidecar 的 ping 和 execution.getState 始终可响应。
- 设备补偿/恢复使用独立目标：默认 20 秒内 restored，或明确进入 disconnected/failed；不能无限 restoring。
- ordinary stop 不重启 sidecar；只有 sidecar transport/health 真正失败才走恢复。
- 不遗留 Runner、本地 helper、本次 adb forward、设备端 scrcpy server、按下输入或重复 controller。
- 正常完成、停止、失败、崩溃的统计各只结算一次。
- 已确认的任务配置增量不丢，用户并发修改不被覆盖。
- GPUI 能显示 starting、stopping、restoring、forced 和恢复失败。
- 结束后动作严格遵守 outcome 和 deviceDisposition。
- frozen Windows 构建通过完整 E2E；Linux 进程抽象测试通过。

## 18. 明确的非目标与风险

- 不隐藏 AALCRunner.exe；用户会在任务管理器看到 GPUI → sidecar → Runner。
- 不保证第三方游戏无法识别自动化或独立进程。
- 不允许任务/plugin 使用 breakaway、计划任务、系统服务或其他方式创建不受监管的 helper；若开放任意插件代码，就无法宣称完整进程树清理保证。
- Windows x64 是第一阶段生产承诺；Linux 是进程抽象和未来运行目标；macOS 暂为 best-effort。
- Job/进程组只解决本地进程生命周期，设备端资源仍依赖 CleanupLedger。
- 强杀后的 Android 输入释放和远端 server 清理可能因设备离线而失败；此时必须报告 disconnected/failed，不能伪报 restored。
- 设备恢复超时与停止接收超时是两件事，UI 和监控必须分别度量。

## 19. 决策清单

实施时以下决策不得再留作隐式选择：

- IPC：继承匿名 pipe，不使用 loopback TCP/token。
- Runner：每个 runId 一个新进程，不复用。
- Windows：CREATE_SUSPENDED → Job assign → resume；失败关闭。
- 配置：bootstrap 先设 AALC_CONFIG_PATH；sidecar 单写；白名单 checkpoint delta + revision 合并。
- 设备：事务式 lease；旧预览/controller 未证明退出则不启动。
- 停止：200 ms 接收、3 秒协作、1 秒强杀确认、独立恢复阶段。
- 清理：父进程预建账本，不能依赖 Runner finally。
- 状态：加入 starting 和 restoring；outcome/requestedBy/forced 分离。
- 完成动作：设备动作属于 Runner，通知/应用退出/电源动作属于 sidecar。
- MuMu：默认仅附加，不自动启动或重启。
- 发布：schema 3 与 Runner protocol 1 独立版本化，Windows frozen E2E 是上线门。

该方案只有在设备租约、进程监管、IPC、配置合并、补偿清理和 GPUI 状态机全部落地后，才能宣称“任务可快速停止且 sidecar 无需重启”。只实现 Runner 子进程并不能满足这一目标。
