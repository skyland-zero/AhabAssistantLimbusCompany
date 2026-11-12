# 开发指南

## 环境配置

- Windows x64
- Python 3.12 或更高版本
- Rust 1.99.1
- Native Shell 0.5.0
- [uv](https://docs.astral.sh/uv/)

```powershell
uv sync
rustup toolchain install 1.99.1
irm https://nsh.dev/install.ps1 | iex
```

## 启动开发版本

推荐使用仓库根目录的启动脚本，它会启动 Native Shell 前端；前端需要时会自动拉起
同目录的 Python sidecar。

```powershell
.\\run-nsh.bat
```

也可以直接启动 Native Shell：

```powershell
nsh run nsh-app
```

仅调试后端协议时，可以单独启动 sidecar：

```powershell
uv run python main_backend.py --token dev-token
```

视觉开发或没有可用设备时，可使用仓库根目录的 Mock 启动脚本；生产模式不会静默回退：

```powershell
.\run-nsh-mock.bat
```

## 测试和格式化

```powershell
uv run pytest -q
cargo +1.99.1 fmt --manifest-path nsh-app/native/Cargo.toml -- --check
cargo +1.99.1 test --manifest-path nsh-app/native/Cargo.toml
$env:AHAB_BACKEND='mock'; nsh test nsh-app
```

## 翻译

Native Shell 的目录位于 `nsh-app/native/src/i18n/`，TSX 通过 `I18n.text()` 读取；
页面内成对文案放在对应页面里。新增或修改语言时，同时更新语言枚举、翻译键和页面
调用处；不需要 Qt Linguist、`.ts` 或 `.qm` 编译链。
