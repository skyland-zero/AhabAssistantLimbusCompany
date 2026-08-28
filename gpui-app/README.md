# Ahab Assistant GPUI App

这是仓库内的独立 GPUI 原生应用，用于提供不依赖 WebView2 的 Windows 桌面界面。

依赖直接指向 Zed 的最新 GitHub `main` 分支，而不是 crates.io 版本：

```toml
gpui = { git = "https://github.com/zed-industries/zed", package = "gpui" }
gpui_platform = { git = "https://github.com/zed-industries/zed", package = "gpui_platform" }
```

## 运行

在仓库根目录执行：

```powershell
cargo +nightly run --manifest-path gpui-app/Cargo.toml
```

发布版内存测试：

```powershell
cargo +nightly run --release --manifest-path gpui-app/Cargo.toml
```

验证模型、Mock IPC 和页面状态：

```powershell
cargo +nightly test --manifest-path gpui-app/Cargo.toml
```

`Cargo.lock` 会记录实际使用的 Git commit。当前 GitHub main 使用了尚未在 stable 中稳定的 `std::hint::cold_path`，因此应用固定使用 nightly。GPUI 固定资源位于 `gpui-app/resources/assets`，并在编译期嵌入程序。

## Python sidecar

普通运行时 GPUI 会自动启动仓库根目录的 `main_backend.py`，通过 loopback WebSocket 使用 JSON-RPC 连接；视觉回归或显式设置 `AHAB_BACKEND=mock` 时继续使用 Mock。也可以显式选择真实 sidecar：

```powershell
$env:AHAB_BACKEND = "sidecar"
cargo +nightly run --manifest-path gpui-app/Cargo.toml
```

如果使用已启动的 sidecar：

```powershell
$env:AHAB_BACKEND_URL = "ws://127.0.0.1:9000"
$env:AHAB_BACKEND_TOKEN = "your-token"
```

当前真实接入范围为设备发现和连接：`pc:limbus`、`mumu:0` 以及 `adb:<serial>`；其他页面 RPC 暂时继续使用共享 Mock。

## 固定尺寸视觉回归

截图产物写入被 Git 忽略的 `artifacts/visual/`：

```powershell
# GPUI release：每个组合会等待窗口、字体和资源完成加载后再截取 client area
pwsh -NoProfile -ExecutionPolicy Bypass -File gpui-app/scripts/capture_visual.ps1

# 动态交互状态：GPUI 通过 AHAB_VISUAL_STATE 覆盖
pwsh -NoProfile -ExecutionPolicy Bypass -File gpui-app/scripts/capture_visual.ps1 `
  -OutputDirectory artifacts/visual/gpui-states `
  -States home-expanded,home-select,home-running,home-paused,home-after-completion,teams-editor,teams-delete,teams-select,settings-hotkey,settings-select,settings-latest,toolbox-running,resources-syncing,help-scrolled
```

脚本通过 `AHAB_VISUAL_THEME`、`AHAB_VISUAL_LANGUAGE`、`AHAB_VISUAL_PAGE` 和 `AHAB_VISUAL_STATE` 启动 GPUI 的确定性状态；不会修改用户配置。窗口使用逻辑 client viewport `900×680` / `800×560`，截图工具依赖 Pillow 和 pywin32。

## 当前可操作范围

- 点击顶部 TabBar 切换 7 个原生页面：主控台、队伍管理、主题包、工具箱、资源中心、帮助、设置。
- 主控台支持任务开关、General/Advanced 配置、日常队伍选择、数量边界、执行状态、暂停/继续、设备选择、结构化日志和结束后动作；设备列表和连接已支持 Python sidecar。
- 队伍管理支持 Mock 队伍列表、用途筛选、新建/编辑/删除、5 个编辑 Tab、人格顺序、互斥策略、星光计算和 JSON 剪贴板。
- 主题包、工具箱、资源中心、设置和帮助页均使用同一套 Mock IPC；帮助目录使用 GPUI `ScrollHandle` 跳转，不嵌入 WebView。
- 队伍名称、编队码、观察饰品、Mirror 酱 CDK 使用 `EntityInputHandler` 文本输入，支持 UTF-16 选择、中文 IME 基础路径和剪贴板。
- 页面旧 token 会通过 render-time Palette 跟随浅色/深色和强调色；全局 Toast、日志复制和自动滚动已接入。
- 使用 `Ctrl-Q` 退出。

当前 Python sidecar 已接入设备发现/连接；任务、队伍、资源和工具等业务 RPC 仍使用 Mock。真实 JPEG 解码、托盘、全局热键注册和 Windows 电源 API 将在后续迭代中继续。

## 测量建议

使用固定窗口尺寸和相同测试步骤，记录：

- 工作集
- Private Bytes
- GPU dedicated/shared memory
- CPU 占用
- 连续切页和点击任务按钮后的内存变化
