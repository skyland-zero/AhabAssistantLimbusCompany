# GPUI + Python sidecar 最终迁移实施计划

> 依据：`项目目录精简分析.md`
>
> 目标平台：Windows x64
>
> 当前基线提交：`0d95053`（已删除 `ui/`，并迁移 GPUI 所需资源）

## 1. 目标和最终边界

最终发布结构：

```text
AALC.exe                 # Rust/GPUI：窗口、页面、托盘、交互
AALC Backend.exe         # Python：配置、设备、任务、工具、同步、更新
AALC Updater.exe         # 独立更新器
assets/                  # 后端和动态资源
gpui-app/resources/      # GPUI 内置资源
```

边界约定：

- Python 只运行无界面的后端服务，不导入 Qt，不创建窗口或 Qt 对象。
- Rust/GPUI 负责页面、窗口、托盘、单实例、窗口激活和用户交互。
- Python 负责配置、设备、任务执行、工具、资源同步、更新和全局热键。
- `config.yaml` 仍由 Python 作为业务配置唯一来源。
- Rust `AppSettings` 只保存语言、主题、侧栏和窗口布局等 UI 偏好。
- 所有业务配置通过 JSON-RPC 访问，GPUI 不直接读写 YAML。
- Mock 只允许测试和显式开发模式使用，生产模式不得静默回退到 Mock。
- 保留旧版配置、队伍、主题包和权重文件的兼容性。

## 2. 执行原则

- 以“协议、后端服务、GPUI 页面、测试”作为一个功能闭环交付。
- 先完成真实功能接入，再删除旧 Qt 代码和依赖。
- 每个里程碑单独提交，旧体系删除必须是独立提交。
- 不移动 `module/`、`tasks/`、`utils/` 和 `assets/images/` 等稳定目录，降低同步上游时的冲突。
- 长耗时操作立即返回 `accepted + runId`，最终状态使用事件推送。
- 所有状态变更通过统一锁或串行执行器处理，避免 RPC 并发覆盖状态。
- 每个里程碑完成后更新本文的进度、验收结果和提交号。

## 3. 里程碑总览

| 编号 | 里程碑 | 当前状态 | 完成门槛 |
| --- | --- | --- | --- |
| M0 | 基线、资源迁移和旧 UI 清理 | 已完成 | `ui/` 不存在，资源校验通过，现有测试通过 |
| M1 | Python 后端边界和 RPC 契约 | 待实施 | sidecar 可独立启动，契约和错误模型稳定 |
| M2 | Qt 业务逻辑抽取为 Python 服务 | 待实施 | 后端服务不依赖 Qt，任务和工具可测试 |
| M3 | 全部真实 RPC 功能闭环 | 待实施 | 所有生产功能拥有真实服务、事件和错误处理 |
| M4 | GPUI 异步客户端和页面脱离 Mock | 待实施 | 生产路径只使用真实 sidecar，UI 不阻塞 |
| M5 | GPUI 系统壳和资源发布结构 | 部分完成 | 托盘、单实例、权限和资源归档可用 |
| M6 | 构建、更新和 CI 切换 | 待实施 | 干净 Windows 机器可安装、启动和更新 |
| M7 | 发布验收和旧体系删除 | 待实施 | 回归通过后删除 Qt/Tauri 遗留并完成收尾 |

## 4. M0：基线、资源迁移和旧 UI 清理（已完成）

- [x] 保存 GPUI 当前状态并建立提交 `0d95053`。
- [x] 将 logo、标题横幅、12 个 sinner 图、11 个状态图和 2 个帮助文档迁移到 `gpui-app/resources/assets/`。
- [x] 修改 `gpui-app/src/assets.rs` 的 `include_bytes!` 和开发/发布资源搜索路径。
- [x] 对 27 个迁移资源执行 Git 对象级字节校验。
- [x] 删除 `ui/`、Tauri 参考脚本、视觉差异工具和无效 Avalonia 构建文件。
- [x] 清理 GPUI 文档和注释中的旧 UI 路径。
- [x] Rust 测试 87/87 通过，Python 测试 23/23 通过，Rust 格式检查通过。

## 5. M1：Python 后端边界和 RPC 契约

### 5.1 后端结构

- [ ] 保持 `main_backend.py` 为纯启动器。
- [ ] 新增统一的 `BackendApplication` 和服务上下文。
- [ ] 将 `rpc_dispatcher.py` 限制为协议校验、方法路由、参数校验和错误转换。
- [ ] 建立统一日志、事件总线、任务运行表和优雅退出流程。
- [ ] 增加父进程退出检测，sidecar 断开时释放设备和任务资源。
- [ ] 增加静态检查，确保 backend 导入链不包含 `PySide6`、`qfluentwidgets` 或 Qt 控件。

### 5.2 协议保持兼容

保留现有方法名和兼容字段：

```text
app.ping/version/checkUpdate/shutdown
tasks.getConfig/setConfig
execution.getState/start/stop/pause/resume
team.list/save/delete
sinner.list
themePack.list/updateAll/resetWeights
resource.status/checkUpdate/sync.start
tool.start/stop/screenshot
hotkey.get/set
systemSettings.get/set
device.list/connect/disconnect
```

- [ ] `TaskConfig`、`TeamConfig`、`SystemSettings` 增加 `schemaVersion`。
- [ ] 长任务响应统一增加 `runId`。
- [ ] 统一错误结构、错误码、可重试标志和用户可见消息。
- [ ] 统一 Token 握手、请求 ID、事件序号和连接关闭语义。
- [ ] 配置和队伍保存采用 patch/merge，不覆盖未知旧字段。
- [ ] 为所有方法补充参数校验和契约测试。

### 5.3 事件契约

至少支持以下事件：

```text
execution.status
execution.mirrorProgress
device.status
tool.status
resource.sync.progress
screenshot.frame
log.entry
app.notice
```

完成门槛：

- sidecar 可以不启动 GPUI 独立运行并响应 `app.ping`、`app.version` 和 `app.shutdown`。
- 并发请求不会造成配置、队伍或运行状态丢失。
- Token 错误、未知方法、参数错误和业务异常都返回稳定错误结构。
- Python 后端测试覆盖协议、事件顺序、优雅关闭和父进程退出。

## 6. M2：抽取 Qt 业务逻辑为 Python 服务

按以下顺序迁移，旧 `app/` 在整个阶段保留作为参考和回滚来源：

1. `farming_interface.py` → `ExecutionService`。
2. `app/tools_windows/` → 无限战斗、截图、理智工具的纯 Python 服务。
3. `team_setting_card.py` → 队伍字段映射和旧格式兼容层。
4. `theme_pack_setting_interface.py` → 主题包服务。
5. `resource_sync_coordinator.py` → RPC 适配层，复用现有资源同步逻辑。
6. `update_ui.py` → 更新状态和事件接口。
7. `hotkey_listener.py` → 后端热键服务。

实施要求：

- [ ] 业务服务不引用窗口、控件、信号、`QTimer` 或 Qt 线程对象。
- [ ] 任务停止优先使用 `threading.Event` 协作式取消。
- [ ] `TerminateThread` 只能保留为异常兜底。
- [ ] 设备、任务、工具、同步和更新服务都有可重复的单元测试。
- [ ] 设备服务保留现有 `pc:limbus`、`mumu:0` 和 `adb:<serial>` 兼容标识。

完成门槛：所有 M2 服务可由 `main_backend.py` 创建，且导入这些服务不会加载 Qt。

## 7. M3：真实 RPC 功能闭环

按依赖关系逐项完成，每项均包含 Python 服务、RPC、事件、GPUI 调用和测试：

| 顺序 | 功能 | 重点方法/事件 | 验收内容 |
| --- | --- | --- | --- |
| 1 | 应用、配置、设备 | `app.*`、`tasks.*`、`device.*` | 配置合并、设备连接、断线和重连 |
| 2 | 任务执行 | `execution.*` | 开始、暂停、恢复、停止、进度和结束摘要 |
| 3 | 队伍和罪人 | `team.*`、`sinner.list` | 旧字段不丢失，新增/编辑/删除可回读 |
| 4 | 主题包 | `themePack.*` | 列表、批量更新、权重和重置 |
| 5 | 资源同步 | `resource.*` | 检查、同步、进度、失败和取消 |
| 6 | 工具和截图 | `tool.*`、`screenshot.frame` | 启停、截图、状态和错误 |
| 7 | 更新和热键 | `app.checkUpdate`、`hotkey.*` | 状态推送、快捷键修改和冲突提示 |
| 8 | 系统设置 | `systemSettings.*` | 读写、兼容默认值和持久化 |

完成门槛：M3 功能在不启动 Qt 前端的情况下，可以通过 sidecar 集成测试完成一次完整调用链。

## 8. M4：GPUI 异步客户端和页面脱离 Mock

### 8.1 客户端基础设施

- [ ] 建立统一 `BackendClientHandle`，隐藏 websocket 连接和请求线程。
- [ ] 所有请求在 GPUI 后台线程执行，禁止页面同步等待网络结果。
- [ ] 建立唯一事件泵，并将事件分发到页面 reducer。
- [ ] 支持请求 ID、超时、取消、重连和连接状态恢复。
- [ ] 生产连接失败显示错误和重试入口。
- [ ] `MockClient` 只由测试和 `AHAB_BACKEND=mock` 使用。

### 8.2 页面接入顺序

- [ ] 主控台：任务配置、设备、执行状态和日志。
- [ ] 队伍管理：列表、编辑、保存、删除和兼容字段。
- [ ] 主题包：列表、更新和权重。
- [ ] 工具箱：工具启动、停止、截图和状态。
- [ ] 资源中心：检查、同步和进度。
- [ ] 设置：热键、系统设置、更新和 UI 偏好。
- [ ] 帮助：内置资源加载和语言切换。

完成门槛：生产模式不存在静默 Mock 回退；页面切换、RPC、断线和重连不阻塞 UI；Rust DTO、事件 reducer 和 Mock 隔离测试通过。

## 9. M5：系统壳和资源发布结构

### 9.1 Rust/Win32 系统能力

- [ ] 托盘图标、菜单和显示/隐藏窗口。
- [ ] 单实例检测和二次启动参数转发。
- [ ] 窗口激活、DPI 和前置显示。
- [ ] 保持旧版启动参数、最小化到托盘和退出行为。
- [ ] 使用 Rust manifest/UAC 替代 `pyuac` 的窗口权限逻辑。
- [ ] Python 继续负责全局热键和业务动作。

### 9.2 资源规则

- [x] GPUI 固定资源统一位于 `gpui-app/resources/assets/`。
- [ ] `assets/` 仅保留后端和动态资源。
- [ ] 发布过程复制 GPUI resources，不能依赖源码 checkout、Node 或 WebView2。
- [ ] 校验字体、主题包、模型和动态图片的实际使用者后再删除旧 Qt 资源。

## 10. M6：构建、更新和 CI

- [ ] 将 `scripts/build.py` 改为 GPUI + Python sidecar 构建流程。
- [ ] 执行 `cargo +nightly build --release`。
- [ ] 使用 `main_backend.spec` 构建 backend，使用 `updater.spec` 构建 updater。
- [ ] 组装 `AALC.exe`、`AALC Backend.exe` 和 `AALC Updater.exe`。
- [ ] 复制 `assets/`、`gpui-app/resources/`、README 和 LICENSE。
- [ ] 生成 `AALC_<version>.7z` 或等价发布包。
- [ ] 更新 `.github/workflows/reusable-build.yml` 和中英文构建文档。
- [ ] CI 固定 Rust nightly，启用 Cargo 缓存和 sidecar 启动测试。
- [ ] 增加干净 Windows 机器测试：无 Python、Qt、Node、WebView2 时启动并完成握手。
- [ ] 验证 updater 的下载、校验、替换、回滚和父进程退出行为。

完成门槛：正式包可以在干净 Windows x64 环境启动，GPUI 能自动启动 backend，更新流程可完成一次成功和一次失败回滚。

## 11. M7：发布验收和旧体系删除

### 11.1 删除前硬性门槛

- [ ] 生产路径所有页面均使用真实 RPC。
- [ ] 任务、设备、队伍、主题包、工具、资源同步、更新、热键和系统设置通过 Windows E2E。
- [ ] 旧版配置、队伍、主题包和权重文件升级后无字段丢失。
- [ ] 正式发布包在干净机器通过启动、退出、重连和更新测试。
- [ ] 构建脚本、CI、README 和文档不再依赖 Qt/Tauri/UI。
- [ ] `rg` 检查确认没有生产路径引用已删除目录。
- [ ] 保留一个可回滚的旧版发布包。

### 11.2 独立删除提交

门槛全部满足后，单独提交删除：

- `app/`、`main.py`、`main_dev.py`、`main.spec`。
- Qt 专用构建和开发代码。
- 根目录 Qt 翻译文件及翻译脚本。
- `pyside6`、`pyside6-fluent-widgets`、`pysidesix-frameless-window`、`pyuac` 等确认无其他用途的依赖。
- 旧 Qt 字体、主题包和 SVG 资源。
- 生产路径不再使用的宽泛 `dead_code`、`unused_imports` 和兼容构造函数。

必须保留：

- `main_backend.py`、`core/`、`module/`、`tasks/`、`utils/`。
- `assets/images/`、`assets/model/`、`assets/minitouch/`、`assets/config/`、`assets/binary/`、`assets/audio/`。
- `gpui-app/` 及测试 Mock。
- `markdown-it`，因为更新检查器仍在使用。

## 12. 测试和验收命令

```powershell
cargo +nightly fmt --manifest-path gpui-app/Cargo.toml -- --check
cargo +nightly test --manifest-path gpui-app/Cargo.toml
uv run --no-sync pytest -q
```

代码和资源清理后执行：

```powershell
rg -n --hidden -g '!target/**' -g '!.git/**' `
  '(ui[\\/](src|public)|src-tauri|capture_tauri|reference-ui|tauri\\.md)' .
```

最终验收还包括：

- RPC 参数、错误码、事件顺序、Token、并发、重连和优雅关闭。
- Rust DTO、异步客户端、事件 reducer 和页面状态恢复。
- Windows 真实游戏窗口、MuMu、ADB、任务控制、托盘、热键、截图、同步和更新。
- 干净机器发布包和旧配置兼容性。

## 13. 提交计划和进度记录

建议提交顺序：

1. `0d95053 refactor: remove legacy ui and relocate gpui assets`（已完成）
2. `docs: add GPUI sidecar implementation plan`
3. `refactor: establish backend application and rpc contract`
4. `feat: extract execution and tool services`
5. `feat: connect all backend rpc services`
6. `feat: connect gpui pages to real backend`
7. `feat: add gpui system shell and release packaging`
8. `ci: switch windows build to gpui sidecar`
9. `chore: remove qt legacy runtime`
10. `refactor: remove migration compatibility layer`

进度更新：

- 2026-08-28：M0 完成；资源迁移、`ui/` 删除和测试基线已提交。
- 2026-08-28：M1 计划文档建立，下一步从 Python backend 契约和服务边界开始。
