# Ahab Assistant GPUI App

这是仓库内的独立 GPUI 原生应用，用于提供不依赖 WebView2 的 Windows 桌面界面。

依赖直接指向 Zed 的 GitHub 仓库，但固定在经过验证的 commit，而不是随构建时间漂移的 `main`：

```toml
gpui = { git = "https://github.com/zed-industries/zed", package = "gpui", rev = "f66ed399cdde86092af8af3dc7b418abf45f37f8" }
gpui_platform = { git = "https://github.com/zed-industries/zed", package = "gpui_platform", rev = "f66ed399cdde86092af8af3dc7b418abf45f37f8" }
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

普通运行时 GPUI 会自动启动仓库根目录的 `main_backend.py`（发布包中为 `AALC Backend.exe`），通过 loopback WebSocket 使用 JSON-RPC 连接；视觉回归或显式设置 `AHAB_BACKEND=mock` 时才使用 Mock。生产连接失败会显示后端不可用状态，不会静默伪造业务数据。也可以显式选择真实 sidecar：

```powershell
$env:AHAB_BACKEND = "sidecar"
cargo +nightly run --manifest-path gpui-app/Cargo.toml
```

如果使用已启动的 sidecar：

```powershell
$env:AHAB_BACKEND_URL = "ws://127.0.0.1:9000"
$env:AHAB_BACKEND_TOKEN = "your-token"
```

真实 sidecar 已提供配置、任务执行、设备、队伍、主题包、资源同步、工具、截图、热键、系统设置和更新检查等 RPC；设备标识继续兼容 `pc:limbus`、`mumu:0` 以及 `adb:<serial>`。

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
- 主控台支持任务开关、General/Advanced 配置、日常队伍选择、数量边界、执行状态、暂停/继续、设备选择、结构化日志和结束后动作；设备列表和连接已支持 Python sidecar，连接设备后右侧实时画面会独立持续刷新。
- 队伍管理支持 Mock 队伍列表、用途筛选、新建/编辑/删除、5 个编辑 Tab、人格顺序、互斥策略、星光计算和 JSON 剪贴板。
- 主题包、工具箱、资源中心、设置和帮助页均通过统一 RPC 边界工作；帮助目录使用 GPUI `ScrollHandle` 跳转，不嵌入 WebView。
- 队伍名称、编队码、观察饰品、Mirror 酱 CDK 使用 `EntityInputHandler` 文本输入，支持 UTF-16 选择、中文 IME 基础路径和剪贴板。
- 页面旧 token 会通过 render-time Palette 跟随浅色/深色和强调色；全局 Toast、日志复制和自动滚动已接入。
- 使用 `Ctrl-Q` 退出。

Python sidecar 负责业务服务和全局热键，GPUI 负责窗口与页面；真实 JPEG 事件、连接设备后的持续实时预览、资源同步进度和任务镜牢进度已纳入事件泵。实时预览默认以 5 FPS、最长边 720 像素的 JPEG 推送，并在断开设备后清理画面。

## 测量建议

使用固定窗口尺寸和相同测试步骤，记录：

- 工作集
- Private Bytes
- GPU dedicated/shared memory
- CPU 占用
- 连续切页和点击任务按钮后的内存变化
