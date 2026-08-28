# GPUI 界面开发说明

当前桌面界面由 Rust/GPUI 实现，业务能力通过 Python sidecar 的 JSON-RPC
接口提供。新增页面或组件时，保持界面和业务边界清晰。

## 代码分层

- `gpui-app/src/main.rs`：应用入口、窗口生命周期和系统壳初始化。
- `gpui-app/src/shell/`：Windows 单实例、托盘和窗口激活。
- `gpui-app/src/pages/`：页面状态、交互和布局。
- `gpui-app/src/components/`：可复用的 GPUI 组件。
- `gpui-app/src/ipc/`：sidecar 启动、WebSocket、请求和事件分发。
- `gpui-app/src/i18n/`：界面翻译表。
- `module/backend_application.py`：sidecar 服务上下文和业务 RPC。
- `core/`、`tasks/`、`utils/`：与界面无关的业务能力。

## 页面与后端交互

页面初始化和写操作都应通过统一的 `BackendClient` 发送 RPC。网络请求在后台
线程执行，页面只消费状态更新和事件，不能直接读写 Python 配置文件。

长操作应立即返回 `accepted` 和 `runId`，进度及最终结果通过事件通知；错误要
保留稳定错误码、可重试标志和用户可见消息。

## 开发检查

```powershell
cargo +nightly fmt --manifest-path gpui-app/Cargo.toml -- --check
cargo +nightly test --manifest-path gpui-app/Cargo.toml
uv run pytest -q
```
