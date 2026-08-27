# Ahab Assistant GPUI Demo

这是一个与 `ui` 同级的独立 GPUI 原生界面 Demo，用于评估 Windows 下 GPUI 的视觉可行性和内存占用。

依赖直接指向 Zed 的最新 GitHub `main` 分支，而不是 crates.io 版本：

```toml
gpui = { git = "https://github.com/zed-industries/zed", package = "gpui" }
gpui_platform = { git = "https://github.com/zed-industries/zed", package = "gpui_platform" }
```

## 运行

在仓库根目录执行：

```powershell
cargo +nightly run --manifest-path gpui-demo/Cargo.toml
```

发布版内存测试：

```powershell
cargo +nightly run --release --manifest-path gpui-demo/Cargo.toml
```

验证模型、Mock IPC 和页面状态：

```powershell
cargo +nightly test --manifest-path gpui-demo/Cargo.toml
```

`Cargo.lock` 会记录实际使用的 Git commit。当前 GitHub main 使用了尚未在 stable 中稳定的 `std::hint::cold_path`，因此 Demo 固定使用 nightly。Demo 会读取现有 `ui/public/sinners` 图片，不会复制资源。

## 固定尺寸视觉回归

截图产物写入被 Git 忽略的 `artifacts/visual/`，不污染 `ui/`：

```powershell
# GPUI release：每个组合会等待窗口、字体和资源完成加载后再截取 client area
pwsh -NoProfile -ExecutionPolicy Bypass -File gpui-demo/scripts/capture_visual.ps1

# Tauri/WebView2 reference：首次启动可能需要编译，脚本会轮询 CDP 并额外等待 2 秒
pwsh -NoProfile -ExecutionPolicy Bypass -File gpui-demo/scripts/capture_tauri_reference.ps1

# 对齐共同物理像素区域并生成 JSON 差异报告
python gpui-demo/tools/visual_diff.py `
  --reference artifacts/visual/reference-ui `
  --gpui artifacts/visual/gpui `
  --output artifacts/visual/pixel-diff.json

# 动态交互状态：GPUI 通过 AHAB_VISUAL_STATE 覆盖，Tauri 通过真实 DOM 操作
pwsh -NoProfile -ExecutionPolicy Bypass -File gpui-demo/scripts/capture_visual.ps1 `
  -OutputDirectory artifacts/visual/gpui-states `
  -States home-expanded,home-select,home-running,home-paused,home-after-completion,teams-editor,teams-delete,teams-select,settings-hotkey,settings-select,settings-latest,toolbox-running,resources-syncing,help-scrolled
pwsh -NoProfile -ExecutionPolicy Bypass -File gpui-demo/scripts/capture_tauri_states.ps1 `
  -OutputDirectory artifacts/visual/reference-ui-states `
  -States home-expanded,home-select,home-running,home-paused,home-after-completion,teams-editor,teams-delete,teams-select,settings-hotkey,settings-select,settings-latest,toolbox-running,resources-syncing,help-scrolled
python gpui-demo/tools/visual_diff.py `
  --reference artifacts/visual/reference-ui-states `
  --gpui artifacts/visual/gpui-states `
  --output artifacts/visual/pixel-diff-states.json
```

脚本通过 `AHAB_VISUAL_THEME`、`AHAB_VISUAL_LANGUAGE`、`AHAB_VISUAL_PAGE` 和 `AHAB_VISUAL_STATE` 启动 GPUI 的确定性参考状态；不会修改用户配置。窗口使用逻辑 client viewport `900×680` / `800×560`，当前验证机 DPI 为 `168`（175%），因此 GPUI client 截图为 `1575×1190` / `1400×980` 物理像素，WebView2 截图因 CSS 像素取整可能多一列。动态脚本会先轮询 Tauri WebView2 CDP，再等待页面与 Mock IPC 稳定后操作和截图。截图工具依赖 Pillow、pywin32 和 agent-browser。当前 `ui` Mock 的 Resources 请求名仍未与页面契约统一，故该动态参考的 Tauri 资源页为空，详见迁移计划。

## 当前可操作范围

- 点击顶部 TabBar 切换 7 个原生页面：主控台、队伍管理、主题包、工具箱、资源中心、帮助、设置。
- 主控台支持任务开关、General/Advanced 配置、日常队伍选择、数量边界、执行状态、暂停/继续、设备 Mock 连接、结构化日志和结束后动作。
- 队伍管理支持 Mock 队伍列表、用途筛选、新建/编辑/删除、5 个编辑 Tab、人格顺序、互斥策略、星光计算和 JSON 剪贴板。
- 主题包、工具箱、资源中心、设置和帮助页均使用同一套 Mock IPC；帮助目录使用 GPUI `ScrollHandle` 跳转，不嵌入 WebView。
- 队伍名称、编队码、观察饰品、Mirror 酱 CDK 使用 `EntityInputHandler` 文本输入，支持 UTF-16 选择、中文 IME 基础路径和剪贴板。
- 页面旧 token 会通过 render-time Palette 跟随浅色/深色和强调色；全局 Toast、日志复制和自动滚动已接入。
- 使用 `Ctrl-Q` 退出。

当前仍未接入 Python sidecar、真实 JPEG 解码、托盘、全局热键注册和 Windows 电源 API；这些按 `GPUI_MIGRATION_PLAN.md` 的 M2/M5/M6 顺序继续。

## 测量建议

与 Tauri 版本使用相同窗口尺寸和相同测试步骤，记录：

- 工作集
- Private Bytes
- GPU dedicated/shared memory
- CPU 占用
- 连续切页和点击任务按钮后的内存变化
