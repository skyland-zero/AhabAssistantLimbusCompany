# 开发指南

## 环境配置

- Windows x64
- Python 3.12 或更高版本
- Rust nightly（GPUI 当前依赖 nightly 工具链）
- [uv](https://docs.astral.sh/uv/)

```powershell
uv sync
rustup toolchain install nightly
```

## 启动开发版本

推荐使用仓库根目录的启动脚本，它会启动 GPUI 前端；前端需要时会自动拉起
同目录的 Python sidecar。

```powershell
.\\run-gpui.bat
```

也可以直接启动 GPUI：

```powershell
cargo +nightly run --manifest-path gpui-app/Cargo.toml
```

仅调试后端协议时，可以单独启动 sidecar：

```powershell
uv run python main_backend.py --token dev-token
```

视觉开发或没有可用设备时，可显式使用 Mock 后端；生产模式不会静默回退：

```powershell
$env:AHAB_BACKEND = "mock"
cargo +nightly run --manifest-path gpui-app/Cargo.toml
```

## 测试和格式化

```powershell
uv run pytest -q
cargo +nightly fmt --manifest-path gpui-app/Cargo.toml -- --check
cargo +nightly test --manifest-path gpui-app/Cargo.toml
```

## 翻译

GPUI 翻译表位于 `gpui-app/src/i18n/`。新增或修改语言时，同时更新语言枚举、
翻译键和页面调用处；不需要 Qt Linguist、`.ts` 或 `.qm` 编译链。
