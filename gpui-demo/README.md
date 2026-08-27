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

## 当前可操作范围

- 点击左侧导航切换 7 个原生页面：主控台、队伍管理、主题包、工具箱、资源中心、帮助、设置。
- 主控台支持任务开关、展开配置、执行状态、暂停/继续、设备 Mock 连接、日志上限和结束后动作。
- 队伍管理支持 Mock 队伍列表、用途筛选、新建/编辑/删除、5 个编辑 Tab、人格顺序、互斥策略、星光计算和 JSON 剪贴板。
- 主题包、工具箱、资源中心、设置和帮助页均使用同一套 Mock IPC；帮助目录使用 GPUI `ScrollHandle` 跳转，不嵌入 WebView。
- 队伍名称、编队码、观察饰品、Mirror 酱 CDK 使用 `EntityInputHandler` 文本输入，支持 UTF-16 选择、中文 IME 基础路径和剪贴板。
- 使用 `Ctrl-Q` 退出。

当前仍未接入 Python sidecar、真实 JPEG 解码、托盘、全局热键注册和 Windows 电源 API；这些按 `GPUI_MIGRATION_PLAN.md` 的 M2/M5/M6 顺序继续。

## 测量建议

与 Tauri 版本使用相同窗口尺寸和相同测试步骤，记录：

- 工作集
- Private Bytes
- GPU dedicated/shared memory
- CPU 占用
- 连续切页和点击任务按钮后的内存变化
