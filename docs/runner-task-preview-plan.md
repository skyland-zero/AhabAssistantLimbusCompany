# 任务运行期间实时画面（Runner 预览）修复方案

## 1. 问题

`execution.start` 之后 GPUI 右栏「实时画面」一直停在「正在获取画面 / 等待最新画面」，运行期间没有任何新帧。

链路现状：

1. `BackendApplication._start_runner_execution` 先 `PreviewCapture.stop_and_wait(deadline)`。
2. 再 `DeviceManager.suspend_for_execution(run_id)`：租约切到 `runner`，sidecar 自己的 controller 被
   `_cleanup_session` 关闭，`_active` 只保留一个无 controller 的逻辑目标。
3. 设备控制权交给 Runner，Runner 用 `create_runner_runtime` 建立私有 session。
4. **Runner 侧完全没有预览生产者**：`RunnerTaskHost` 只有 command reader / heartbeat / task worker
   三个线程，不产生 `preview.frame`。
5. sidecar 侧在租约活跃时禁止任何设备写入（`_assert_sidecar_device_available` 直接抛
   `DEVICE_LEASE_ACTIVE`），`preview_set_enabled` 也因 `_device_lease_active()` 不会 `start`。

结论：租约期间没有任何一方截图，所以 GPUI 收不到 `screenshot.frame`。这不是渲染或解码问题。

协议本身早已预留了这条通路（`module/execution/ipc_protocol.py` 的 `preview.frame` /
`preview.status`，4 MB 二进制上限；`docs/gpui-python-runner-plan.md` 也写明「Runner：任务期间
设备控制、任务预览」），只是没有实现。

## 2. 修复

由 Runner 在任务期间采集并发布帧，sidecar 只做协议适配转发。

```text
Runner task worker ──auto.take_monitor_screenshot(gray=False)──▶ JPEG
        │
        └─ emit("preview.frame", payload=jpeg, deviceId, runId, generation, width, height)
                │  (Runner IPC pipe, 二进制负载)
                ▼
        RunnerEventAdapter: preview.frame → screenshot.frame {jpeg: bytes, ...identity}
                ▼
        BackendApplication.emit → WebSocketServer._broadcast_event（二进制帧）
                ▼
        GPUI: preview_accepts_frames() → PreviewView 解码显示
```

要点：

- 复用既有 `encode_screenshot_frame`（540px / JPEG q72），不新增编码路径。
- 采集走 `auto.take_monitor_screenshot(gray=False, max_age=0.5, ensure_window_visible=False)`，
  与 sidecar 预览一致：不移动 PC 窗口、优先复用业务线程刚拿到的帧、不写磁盘。
- 帧必须带 `deviceId` / `runId` / `generation` 身份，GPUI 才会在 `execution.status=非 idle`
  期间接受（`accepts_preview_identity`）。
- 预览是遥测：采集失败只发一次 `preview.status=error`，不影响任务；停止必须发生在
  `_close_device_runtime` 之前，避免对着已清理的 controller 采集。
- 只在 Runner 真正安装了私有设备 session（`get_runner_session()` 非空）时启动，测试用的
  假 runtime 不会触发采集。
- 通过 `start.spec.previewEnabled` 与 `preview` 命令传递「用户是否希望看画面」，与 GPUI 的
  按需收帧（页面/右栏/最小化）保持一致，窗口最小化时不做无谓截图。

## 3. 涉及文件

| 文件 | 改动 |
| --- | --- |
| `module/execution/task_preview.py` | 新增：Runner 侧预览线程 `TaskPreview` |
| `module/execution/runner_host.py` | `start()` 后启动预览、`preview` 命令、`_task_preview_factory` 注入口 |
| `module/execution/ipc_protocol.py` | `preview` 命令类型；`preview.status` 事件校验 |
| `module/execution/command_queue.py` | `preview` 与 `setPaused` 一样只保留最新一条 |
| `module/execution/supervisor.py` | `ExecutionSpec.preview_enabled`、`request_preview_enabled()` |
| `module/execution/event_adapter.py` | `preview.frame` → `screenshot.frame`（JPEG 走二进制） |
| `module/backend_application.py` | 启动 spec 带 `previewEnabled`；`preview.setEnabled` 在租约期间转发给 Runner |

## 4. 生命周期

```text
start()
  initialize()          私有 device runtime 安装
  ready / status=running
  resource.created
  _start_task_preview()  ← 只有 get_runner_session() 非空才启动

task 运行中
  每 2s: capture → 540px JPEG → preview.frame(deviceId, runId, generation)

finally
  _stop_task_preview()   ← 必须在 _close_device_runtime() 之前，且不发 stopped
  _close_device_runtime()
  finished
```

停止预览时**不发** `preview.status=stopped`：`execution.status=idle` 才是清除画面的权威信号，
否则最后一帧会在收尾阶段闪一下。

## 5. 验证

- `tests/unit/module/execution/test_task_preview.py`：帧身份、`previewEnabled` 门控、采集失败
  只报一次错误、JPEG 二进制适配。
- `tests/unit/module/execution/test_runner_host.py`：无私有 session 不启动；有 session 时
  `enabled=False → start → stop` 的顺序；spec 的 `previewEnabled` 往返。
- `tests/unit/module/test_backend_application.py`：启动 spec 带 `previewEnabled`；租约期间
  `preview.setEnabled` 转发给 Runner 且不会让 sidecar worker 抢回设备。

`tests` 全量：533 passed, 1 skipped；`scripts/validate_contract.py` 通过。
