# Scrcpy 链路性能优化方案

> 文档状态：已确认实施方案
> 分析基线：当前 `HEAD 62dc1ce8`；Scrcpy Server 4.1；`scrcpy_control.py` 已包含 on-demand decode 和 latest-frame zero-copy 改动。
> 适用范围：ADB / Scrcpy 设备，不包含 MuMu NemuIpc 和 Windows 窗口截图链路。

## 1. 结论先行

**最建议采用“消费者感知的自适应解码 + 最新帧邮箱 + 帧序号同步 + 按需图像格式转换”的方案。**

需要先明确：持续解码本身不是降低 CPU 的手段。正确的做法是：任务或预览活跃时持续解码以保持 H.264 状态；没有消费者时进入低 CPU 的休眠/排空模式，并在恢复时显式重建解码状态、等待关键帧，绝不能直接返回旧帧。

推荐方案的核心原则：

1. **活跃模式持续读取并解码，短暂空闲维持 decoder，长时间空闲只排空编码包**；恢复时重置 decoder 并等待 config + 关键帧；
2. **只保留最新解码帧**，不建立无界队列，避免延迟不断累积；
3. 通过 `frame_seq` 和 `decoded_at` 判断帧的新旧，不再使用双层采样 CRC；
4. 业务识别优先使用灰度 / Y 平面，只有预览或手动彩色截图时才转换 RGB；
5. 输入后可以等待一帧真正的新画面，避免 OCR 读到点击前的旧画面；
6. 保持 `max_size=0` 不变，仅视实测结果评估帧率和码率；
7. 重写拖拽事件的节奏，删除不必要的固定等待。

## 1.1 预期收益

以下是基于当前代码、1080p 内存估算和本地合成 H.264 微基准的预期范围；真实收益必须用目标设备和 OCR 任务复测。

| 优化项 | 预期收益 | 备注 |
|---|---:|---|
| 灰度直接读取 Y 平面 | 转换子项 CPU 降低约 70%～90% | 消除 RGB→灰度的全帧读写 |
| 最新帧内存 | 约降低 50% | RGB 约 6.2 MB/frame，YUV420 约 3.1 MB/frame |
| 灰度临时数据 | 降至最多约 2.1 MB | 仅在下游要求连续内存时复制 Y 平面 |
| RGB 转换频率 | 15 次/s 降至按需 | 默认预览 2 秒一帧，约 0.5 次/s |
| 空闲解码 | 解码子项 CPU 接近 0 | SUSPENDED 只排空 socket |
| 直接 packet decode | 解码阶段预计改善 3%～10% | 主要降低 parser 缓冲和延迟 |
| 最小 FFmpeg | 正式包移除约 62.6 MB PyAV 原生库基线 | OpenCV/ONNX Runtime 体积仍保留 |

本地测试中，1920×1080、15 帧的 RGB 转换约 50.7 ms，直接读取 Y 平面约 7.05 ms；直接 `av.Packet` decode 相较 `parse + decode` 约从 13.11 ms 降至 12.54 ms。以上是子阶段微基准，不代表整体进程 CPU。

## 2. 当前链路

```text
Android 屏幕
  → Scrcpy Server 硬件编码 H.264
  → ADB forward
  → Python TCP socket
  → PyAV H.264 解码
  → YUV / VideoFrame 最新帧邮箱
  → 按需生成灰度或 RGB
  → PIL Image
  → OpenCV 模板匹配 / RapidOCR

预览支路：
  RGB Image → 缩放到 540px → JPEG → WebSocket 二进制事件
  → Rust → GPUI JPEG 解码 → RGBA 通道修正 → 渲染
```

相关代码：

- 视频启动参数：`module/automation/input_handlers/simulator/scrcpy_control.py:328-335`
- Socket 接收和解码：`module/automation/input_handlers/simulator/scrcpy_control.py:356-413`
- 截图读取：`module/automation/input_handlers/simulator/scrcpy_control.py:415-439`
- PIL / 灰度转换：`module/automation/screenshot.py:372-406`
- 业务截图节流：`module/automation/automation.py:562-590`
- 预览编码：`module/preview_capture.py:27-59`
- WebSocket 二进制广播：`module/websocket_server.py:271-317`

## 3. 性能判断

### 3.1 当前主要不是网络带宽瓶颈

1080p 每帧 RGB 数据约为：

```text
1920 × 1080 × 3 ≈ 6.2 MB
```

15 FPS 解码后仅 RGB 输出就约产生：

```text
6.2 MB × 15 ≈ 93 MB/s
```

而 8 Mbps H.264 输入只有约 1 MB/s。因此在 USB ADB 和本机模拟器场景下，优先级通常是：

```text
像素格式转换 / 内存分配 > 解码 CPU > OCR / 模板匹配 > ADB 传输
```

如果是 Wi-Fi ADB，码率才可能成为明显瓶颈。

### 3.2 针对 CPU 和内存目标的可行性判断

这套优化是可行的，但不能把“持续解码”作为全时策略：

- **任务运行期间**，当前 `screenshot_interval=0.2` 意味着业务大约每 200ms 请求一次截图，现有代码实际上已经会持续解码。此时最大的收益来自“不再每个解码帧都转 RGB”，而不是继续调整丢包逻辑；
- **没有任务、预览关闭时**，持续解码反而会增加 CPU 占用。推荐进入空闲模式：继续读取并丢弃编码包，但暂停 `codec.decode()`；恢复时重建 decoder，等新关键帧后再发布；
- **内存方面**，当前 `_latest_frame` 是有界的，通常不会造成持续内存泄漏。主要优化目标是减少 6MB 级 RGB 临时数组、PIL/灰度临时对象和多余缓存引用；
- 固定的 `sleep()` 主要增加任务总时长，不会显著增加进程 CPU 占用，但会延长解码器处于活跃状态的时间。

因此，建议目标应改成：

```text
活跃：持续解码 + 最新帧邮箱 + 按需灰度/RGB
空闲：排空 packet、不解码 + 恢复时关键帧重同步
```

### 3.3 当前已做对的地方

- 采用 Scrcpy 视频流，而不是每次执行 `adb screencap`；
- `audio=false`，没有额外音频传输；
- `_latest_frame` 只保留最新帧，避免 Python 层无界积压；
- 预览使用二进制 JPEG，而不是 Base64 JSON；
- 预览有 2 秒间隔、540px 最大宽度和队列合并；
- GPUI 在页面隐藏、右栏收起或窗口最小化时会停用预览；
- 当前工作区已经移除 `ScrcpyControl.screenshot()` 中的整帧 `frame.copy()`，这项优化应保留。

## 4. 当前问题和优先级

| 优先级 | 位置 | 问题 | 影响 |
|---|---|---|---|
| P0 | `scrcpy_control.py:387-389` | 无请求时直接丢 H.264 packet | 解码参考帧断裂、恢复等待关键帧、返回旧画面 |
| P1 | `scrcpy_control.py:400` | 每个解码帧都转换为 RGB | 1080p 下产生大量 CPU 和内存写入 |
| P1 | `screenshot.py:402-405` | RGB ndarray → PIL → 灰度 | 业务识别路径存在完整帧转换 |
| P1 | `scrcpy_control.py:415-439`、`automation.py:258-288` | 两层采样 CRC 去重 | 重复开销、局部变化可能被漏判、缓存正确性不可靠 |
| P1 | `scrcpy_control.py:499-582` | 拖拽和滚动使用固定 20ms / 500ms 等待 | 手势耗时偏长，滚动的 `duration` 参数没有真正生效 |
| P1 | `scrcpy_control.py:117-127` | `_recv_exact()` 不保留半包状态 | timeout 后可能丢弃已读取字节并造成协议错位 |
| 保持现状 | `scrcpy_control.py:333` | `max_size=0` 按现有设备策略保留 | 本方案不修改分辨率上限，优先优化解码、格式转换和输入时序 |
| P2 | GPUI 预览链路 | JPEG、Vec、RGBA 多次复制 | 当前 2 秒一帧影响较小，提高预览帧率后才明显 |
| P2 | `scrcpy_control.py:361` | 未配置硬件解码 | 低配机器上可能与 OCR 争用 CPU，但硬件解码仍有 GPU→CPU 拷贝成本 |
| P3 | `scrcpy_control.py:203-207` | 每次重连都推送 JAR | 只影响连接建立和重连时间，不影响稳定运行时吞吐 |

## 5. 最推荐的目标架构

### 5.1 消费者感知的自适应解码和最新帧邮箱

新增一个内部帧对象：

```text
DecodedFrame
  - seq: 单调递增帧序号
  - pts: Scrcpy / H.264 PTS
  - decoded_at: 本机 monotonic 时间
  - video_frame: PyAV VideoFrame，或稳定的图像缓冲
  - gray_cache: 可选，按 seq 缓存
  - rgb_cache: 可选，按 seq 缓存
```

邮箱只保存一个最新对象：

```text
解码线程：publish(new_frame)
业务线程：snapshot() / wait_next_frame(after_seq)
预览线程：snapshot(gray=False)
```

根据消费者状态分成三种模式：

```text
ACTIVE：recv → direct packet decode → publish(latest_frame)
QUIET： recv → decode → discard decoded frame，不做颜色转换
SUSPENDED：recv → discard encoded packet，不调用 codec.decode()
```

ACTIVE/QUIET 模式下不能随意丢 packet，否则会破坏 H.264 参考帧。SUSPENDED 模式虽然排空了编码包，但必须设置 `decoder_needs_keyframe=true`；恢复时重建或 `flush_buffers()`，等待 config + 关键帧后才能发布新帧。

如果在限定时间内没有恢复出关键帧，应主动重启视频会话，而不是把 SUSPENDED 期间保存的旧帧当作新截图返回。Scrcpy 默认关键帧间隔约 10 秒，因此启用 SUSPENDED 时增加 `video_codec_options=i-frame-interval:int=1`，并保留超时重启兜底。

### 5.2 按需做图像格式转换，而不是按需解码

建议把“节省 CPU”的位置从活跃模式中的无条件 RGB 转换移到按需像素转换，同时在长时间空闲时暂停 decoder：

- ACTIVE 模式持续 H.264 解码，但不在每个解码帧上立即生成 RGB；
- QUIET 模式继续维持 H.264 状态，但丢弃不需要发布的解码帧；
- SUSPENDED 模式只排空编码包，不调用 `codec.decode()`；
- 解码结果先保留 PyAV `VideoFrame` 或 YUV 数据；
- 业务识别请求 `gray=True` 时，优先提取 Y/luma 平面；
- 预览或手动截图请求 `gray=False` 时，才调用 `to_ndarray(format="rgb24")`；
- 同一个 `seq + mode` 的转换结果只生成一次。

模板资源本身在 `utils/image_utils.py` 中已经统一为灰度，因此优先 Y 平面不会改变主要识别接口。需要通过识别回归测试确认 Y 平面和现有 PIL 灰度转换的阈值差异。

建议保留一个兼容开关：

```text
scrcpy_use_luma_gray: true
```

如果某些 OCR 场景识别率下降，可以暂时退回 `format="gray"` 或 PIL 灰度转换。

### 5.3 使用帧序号解决新旧帧问题

当前 `screenshot()` 只等待首帧，不保证截图发生在最近一次输入之后。建议增加两种语义：

```python
snapshot(gray=True)                 # 返回当前最新帧，可复用
wait_next_frame(after_seq, timeout) # 等待 seq > after_seq
```

建议将接口具体化为：

```python
snapshot(
    *,
    mode="luma" | "rgb",
    after_seq: int | None = None,
    timeout: float = 3.0,
) -> FrameSnapshot
```

`FrameSnapshot` 至少包含 `seq`、`pts`、`decoded_at` 和图像对象。现有 `screenshot() -> np.ndarray` 保留为 RGB 兼容包装，不改变 MuMu/PC 调用方。

输入流程建议：

```text
记录输入前 frame_seq
  → 发送 Scrcpy control message
  → 等待新帧，超时则使用最新可用帧
  → OCR / 模板匹配
```

不需要每次普通轮询都等待新帧，只有点击、拖拽、页面切换等会改变画面的操作之后才等待。这样可以同时降低误读旧画面的概率和无效重试次数。

### 5.4 用帧序号替代双层 CRC

推荐删除以下逻辑：

- `ScrcpyControl.screenshot()` 的采样 CRC；
- `Automation._set_business_screenshot()` 的再次采样 CRC。

解码线程每产生一个新的 `VideoFrame` 就递增 `seq`。同一 `seq` 即同一物理帧，不需要扫描图像内容。

收益：

- 少一次控制器层采样和一次 Automation 层采样；
- 不会漏掉只发生在采样间隔之外的局部变化；
- 缓存键从 `(frame_id, id(image))` 简化为 `seq`；
- 可以直接测量输入后的帧延迟。

### 5.5 保留最新帧，但不要建立普通 packet 队列

H.264 packet 在 ACTIVE/QUIET 解码期间不能简单地随意丢弃，否则会破坏参考帧。CPU/内存优化需要把丢弃动作限制在明确的 SUSPENDED 模式：只排空 Socket，不进入 decoder；恢复时重置 decoder，并等待 config + 关键帧。

推荐第一版仍不做“读线程 + 任意 packet 队列”，而是使用单线程接收循环加最新帧邮箱：

```text
ACTIVE：recv + direct decode + publish latest
QUIET： recv + decode + discard decoded frame
SUSPENDED：recv + discard encoded packet
恢复： recreate/flush decoder + wait config/keyframe + publish
```

如果未来确实需要解码线程和接收线程分离，队列必须具备以下规则：

- 以完整 access unit 为粒度；
- 队列超限时只跳到下一个关键帧；
- 重置 decoder 后等待关键帧再发布；
- 记录丢帧和重同步次数。

## 6. 推荐的视频参数

### 6.1 第一阶段推荐基线

按照当前设备分辨率策略，`max_size` 不做修改，保留现有参数：

```text
max_size=0
max_fps=15
video_bit_rate=8000000
video_codec=h264
```

本方案不通过 Scrcpy 的 `max_size` 限制分辨率，避免引入坐标映射、模板缩放和识别精度变化。高分辨率带来的成本优先通过减少 RGB/灰度转换、缓存和无效识别来缓解。

### 6.2 低配或 Wi-Fi ADB 预设

作为可选预设，不建议一开始默认启用；`max_size` 仍保持现状：

```text
max_size=0
max_fps=10
video_bit_rate=4000000~6000000
```

使用该预设时必须验证：

- 模板匹配准确率；
- OCR 准确率；
- 点击坐标是否需要缩放；
- 竖屏设备旋转后的宽高是否正确。

### 6.3 暂不推荐的方向

- 不建议第一阶段切换 H.265 / AV1：设备编码器、PyAV 解码器和软硬件兼容性更复杂；
- 不建议直接把 FPS 降到 5：虽然接近当前 `screenshot_interval=0.2` 的 5 次/秒，但会增加输入后等待时间和动画漏采样概率；
- 不建议用 `adb screencap` 替代视频流：每次截图的进程和 PNG 开销通常更大。

### 6.4 Decoder 参数

PyAV 阶段使用完整媒体 packet 直接解码，不再调用 `codec.parse()`：

```python
packet = av.Packet(access_unit)
packet.pts = pts
packet.dts = pts
frames = codec.decode(packet)
```

decoder 固定配置：

- `low_delay`；
- `thread_type=SLICE`；
- `thread_count=max(1, min(4, os.cpu_count() or 4))`；
- 分辨率变化时重建 decoder；
- SUSPENDED 恢复时 `flush_buffers()` 或重建 decoder。

不使用 `skip_frame` 牺牲识别质量，也不使用 FRAME threading 增加解码延迟。PyAV 的 codec API 支持显式线程类型、线程数和 decoder buffer reset。[PyAV CodecContext 文档](https://pyav.basswood-io.com/docs/15.0/api/codec.html)

## 7. 控制输入优化

### 7.1 当前问题

`module/automation/input_handlers/simulator/scrcpy_control.py:499-582` 中：

- `mouse_drag()` 的 `drag_time` 没有用于 MOVE 事件的真实时间规划；
- 普通拖拽固定等待约 500ms；
- `mouse_swipe_for_scroll()` 接收 `duration`，但实际没有用它控制总时长；
- `speed=8, min_distance=1` 可能生成大量 MOVE 事件；
- 每个 MOVE 都单独调用一次 `sendall()`。

这部分会直接增加镜牢任务总耗时。

### 7.2 推荐实现

采用“时间驱动、点数上限”的调度：

```text
目标时长：drag_time
目标频率：30~60 Hz
最大事件数：例如 32 或 48
每个事件时间点：按目标结束时间计算
```

建议：

- 普通拖拽按 `drag_time` 生成 30~60 Hz 的 MOVE；
- 滚动按传入 `duration` 计算 MOVE 间隔；
- 通过最大点数限制超长路径；
- 必要时每 2~4 个事件批量发送，但不能把需要明显时间间隔的整个手势一次性发送；
- 将 settle 时间改为配置项，默认不要固定 500ms；
- 可对 control socket 设置 `TCP_NODELAY`，但需要实机验证是否有效。

触控事件的顺序和 DOWN/UP 必须保持不变。优化目标是减少无效等待，不是把所有事件无间隔发送。

## 8. Socket 接收优化和稳定性

### 8.1 持久化接收缓冲

建议将 `_recv_exact()` 改为对象级接收缓冲：

```text
_recv_buffer = bytearray()
ensure(n) 只负责补齐数据
read(n) 从持久化 buffer 消费数据
```

这样可以：

- 正确处理 header 或 payload 被拆成多个 TCP read 的情况；
- timeout 后不丢弃已经接收的半包；
- 减少每个 packet 的 bytearray / bytes 临时对象；
- 方便统计接收缓冲和 packet 延迟。

同时建议：

- 对 packet size 设置上限，例如 16 MB；
- `recv()` 返回空字节时直接结束解码线程；
- 将协议错位和非法 size 记录为连接错误，而不是继续空转。

## 9. 预览和 GPUI 支路

当前预览默认 2 秒一帧、最长边 540px，已经是低成本设计，不应优先提高帧率。

后续可选优化：

1. `PreviewCapture` 使用控制器最新 RGB 帧，避免因业务缓存是灰度图而重新请求一次截图；
2. 预览缩放使用 `BILINEAR`，如果视觉质量可接受，替代较重的 `LANCZOS`；
3. Rust `ScreenshotFrame` 的 JPEG 使用 `Arc<[u8]>`，避免 `latest_screenshot.clone()` 和 `frame.jpeg.clone()`；
4. 事件中直接使用后端序号，避免 `ScreenshotFrame` 全量比较 JPEG；
5. 修复 GPUI 图像格式后，移除 `adapt_preview_render_image()` 中的 RGBA 全量复制和红蓝通道交换。

这些优化只有在预览提高到 5~10 FPS 时才值得优先处理。

手动截图工具 `tool_screenshot()` 目前会先以质量 92 写磁盘，再以质量 80 编码一次事件。可以在后续将文件输出和事件编码合并，但它不是持续运行链路的热点。

## 10. 分阶段实施计划

### 阶段 0：增加指标，不改变行为

在 `ScrcpyControl` 中增加周期性统计：

- 收到 packet 数和字节数；
- 解码帧数；
- 当前发布帧序号；
- `codec.decode` 耗时；
- RGB / gray 转换耗时；
- 截图请求次数；
- 截图返回时的 frame age；
- 输入发送到下一帧的耗时；
- packet timeout、decoder error、重同步次数。

建议使用 `time.perf_counter()` / `perf_counter_ns()`，每 5~10 秒输出一次汇总，避免每帧日志。

### 阶段 1：先修复正确性并建立自适应资源模式

1. 用显式 ACTIVE/QUIET/SUSPENDED 状态替换当前基于 `_last_request_mono` 的隐式丢包；
2. ACTIVE 模式持续解码，QUIET 模式解码但丢弃结果，SUSPENDED 模式只排空 packet；
3. 恢复时重建或 flush decoder，等待 config + 关键帧后才发布新帧；
4. 增加最新帧邮箱、`frame_seq` 和 `wait_next_frame()`；
5. 使用持久化 Socket 接收缓冲；
6. 增加 H.264 packet size 校验和 EOF 处理。

### 阶段 2：优化像素转换

1. 解码结果保留为 PyAV VideoFrame / YUV；
2. 业务识别按需生成灰度图；
3. 预览按需生成 RGB 图；
4. 按 `seq + image_mode` 缓存转换结果；
5. 删除两层 CRC 去重。

### 阶段 3：优化输入耗时（主要改善任务总时长）

> 固定 `sleep()` 本身不会显著增加 CPU 占用，本阶段主要减少任务总时长，从而间接减少解码器处于 ACTIVE 模式的累计时间。

1. 重写普通拖拽和滚动的时间调度；
2. 限制 MOVE 事件数量；
3. 删除固定 500ms 等待或改成可配置 settle 时间；
4. 实测 `TCP_NODELAY` 和少量批量发送。

### 阶段 4：参数化和设备适配

1. 仅增加 `scrcpy_max_fps`、`scrcpy_video_bit_rate` 配置；
2. 保持 `max_size=0`，默认使用 `max_fps=15, bitrate=8M`；
3. 增加低配 / Wi-Fi 预设；
4. 保持现有流分辨率、识别坐标和控制坐标约定不变。

### 阶段 5：可选高级优化

只有指标显示软件解码是主要瓶颈时，再评估：

- 最小 FFmpeg + C ABI native decoder；
- FFmpeg D3D11VA / DXVA2 硬件解码；
- H.265；
- 更复杂的关键帧感知丢帧队列。

硬件解码不应作为第一步，因为识别最终仍需要 CPU 可访问的图像，GPU 解码后的回读也有成本。

### 10.6 原生 H.264 后端实施细节

PyAV 和 native 后端共用以下抽象：

```python
class H264Decoder(Protocol):
    def submit(self, access_unit, *, pts, keyframe, config): ...
    def receive(self): ...
    def reset(self): ...
    def close(self): ...
```

native 后端采用 Windows x64 最小 FFmpeg + C ABI DLL：

- 只启用 H.264 decoder、必要 parser、`libavcodec` 和 `libavutil`；
- RGB 路径需要时再加入 `libswscale`；
- 输出优先使用 YUV420P/NV12；
- Python 通过 `ctypes` 调用；
- 使用调用方提供的 buffer 复制 luma/RGB，避免借用指针生命周期问题；
- 正式 native profile 移除 PyAV 运行时依赖，开发/基准 profile 保留 PyAV 对照；
- D3D11VA 仅在软件 native 版本完成后评估 GPU→CPU 回读后的端到端收益。

当前本地 `av.libs` 约 62.6 MB，应将“移除完整 PyAV 原生库”作为 native 包体积目标；最终包仍会包含 OpenCV、ONNX Runtime 等依赖。FFmpeg 支持按需只启用指定 decoder。[FFmpeg codec 文档](https://www.ffmpeg.org/ffmpeg-codecs.html)

## 11. 验收指标

### 11.0 收益门槛

- 灰度路径不得调用 `to_ndarray("rgb24")`；
- RGB 转换子项 CPU 目标降低 70%～90%；
- 活跃截图 p95 延迟不得劣化超过 10%；
- SUSPENDED 状态下 H.264 decoder CPU 应接近 0；
- RSS 连续运行 10 分钟不持续增长；
- native 版本至少移除完整 PyAV 原生库，且端到端性能不劣于优化后的 PyAV 版本。

### 11.1 正确性

- 连续运行 10 分钟，不出现持续的 `reference picture missing` 或 decoder 错误；
- 点击、拖拽后 OCR 不读取输入前画面；
- Wi-Fi ADB 下 TCP 半包不会造成协议永久错位；
- 断开设备后解码线程能够退出，不空转。

### 11.2 延迟

建议记录以下指标的 p50 / p95：

```text
输入发送 → 下一张新解码帧
screenshot() 调用 → 返回
新帧解码 → OCR 开始
OCR / 模板匹配耗时
```

15 FPS 下，输入到新帧的理论视频采样等待约为 0~67ms，实际还要加上设备渲染、编码、传输和解码时间。

### 11.3 资源

分别比较优化前后：

- Python 进程 CPU 占用；
- Python 进程内存和每秒分配量；
- ADB 视频吞吐；
- 解码帧率和业务截图帧率；
- OCR / 模板匹配平均耗时。

### 11.4 识别质量

至少测试：

1. 静态菜单；
2. 快速页面切换；
3. 战斗动画中截图；
4. 点击后立即识别；
5. 拖拽地图后识别；
6. 1920x1080 和 1920x864；
7. 15 FPS 与 10 FPS 低帧率预设。

## 12. 建议增加的测试

现有 `tests/unit/module/test_scrcpy_control.py` 主要覆盖颜色格式、启动参数和 session metadata，建议增加：

- 帧序号递增和最新帧覆盖；
- `wait_next_frame()` 超时与成功路径；
- Socket 半包、timeout、EOF；
- 非法 packet size；
- H.264 暂停恢复后的关键帧重同步（可用构造的短测试流）；
- 灰度 / RGB 转换按 seq 缓存；
- 拖拽 MOVE 数量和目标总时长；
- 输入后不会复用输入前 frame_seq。

## 13. 最终建议

第一轮只做以下四项，不要同时引入 H.265 或硬件解码：

```text
1. 用 ACTIVE/QUIET/SUSPENDED 自适应模式替换当前 lazy packet drop；
2. 加 frame_seq + latest-frame mailbox + wait_next_frame；
3. 将 RGB 转换改为按需，业务优先使用灰度/Y 平面；
4. 记录 CPU、内存、帧年龄和解码耗时，再决定是否降低 FPS。
```

视频参数先固定为（`max_size` 按现有策略不改）：

```text
max_size=0
max_fps=15
video_bit_rate=8000000
```

这套方案在 CPU/内存目标下更可行：ACTIVE/QUIET 模式保持 H.264 状态，SUSPENDED 模式释放解码 CPU；按需图像转换减少 6 MB 级 RGB 临时对象；帧序号避免重复 CRC 和旧帧误判。完成指标采集后，再决定是否将 FPS 从 15 降到 10、降低码率或接入硬件解码。

## 14. 参考资料

- [scrcpy 官方开发文档](https://github.com/Genymobile/scrcpy/blob/master/doc/develop.md?plain=1)
- [scrcpy 官方 demuxer](https://raw.githubusercontent.com/Genymobile/scrcpy/master/app/src/demuxer.c)
- [scrcpy 官方 decoder](https://raw.githubusercontent.com/Genymobile/scrcpy/master/app/src/decoder.c)
- [scrcpy 官方视频参数文档](https://github.com/Genymobile/scrcpy/blob/master/doc/video.md)
- [PyAV CodecContext 文档](https://pyav.basswood-io.com/docs/15.0/api/codec.html)
- [FFmpeg codec 文档](https://www.ffmpeg.org/ffmpeg-codecs.html)
