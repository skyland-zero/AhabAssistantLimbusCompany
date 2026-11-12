# Native Shell 界面开发说明

当前桌面界面由 Native Shell 实现（Rust 状态 + TSX 页面），业务能力通过
Python sidecar 的 JSON-RPC 接口提供。新增页面或组件时，保持界面和业务边界清晰。

## 代码分层

- `nsh-app/App.tsx`：应用入口、窗口壳和路由编排。
- `nsh-app/shell/`：标题栏、托盘、全局热键和窗口级反馈。
- `nsh-app/pages/`：页面布局与交互。
- `nsh-app/components/`：可复用的界面组件。
- `nsh-app/native/src/ipc/`：sidecar 启动、WebSocket、请求和事件分发。
- `nsh-app/native/src/state/`：队伍、主控台、设置、资源等状态与持久化。
- `nsh-app/native/src/i18n/`：界面翻译目录。
- `module/backend_application.py`：sidecar 服务上下文和业务 RPC。
- `core/`、`tasks/`、`utils/`：与界面无关的业务能力。

## 页面与后端交互

页面初始化和写操作都通过 Rust 状态层的 `RpcGateway` 发送 RPC；TSX 只持有
镜像信号并调用状态方法，不能直接读写 Python 配置文件。

长操作应立即返回 `accepted` 和 `runId`，进度及最终结果通过事件通知；错误要
保留稳定错误码、可重试标志和用户可见消息。

## 开发检查

```powershell
cargo +1.99.1 fmt --manifest-path nsh-app/native/Cargo.toml -- --check
cargo +1.99.1 test --manifest-path nsh-app/native/Cargo.toml
nsh check nsh-app
$env:AHAB_BACKEND='mock'; nsh test nsh-app
uv run pytest -q
```
