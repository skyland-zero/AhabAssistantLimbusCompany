# 构建指南

## 环境

- Windows x64
- Python 3.12+
- Rust 1.99.1
- Native Shell 0.5.0（`nsh` 命令在 PATH 中）
- `uv`（推荐）和 7-Zip

安装依赖：

```powershell
uv sync --frozen
rustup toolchain install 1.99.1
irm https://nsh.dev/install.ps1 | iex
```

## 本地运行

```powershell
.\run-nsh.bat
```

程序会启动 Native Shell 窗口，并由 Rust 前端启动同一目录下的 `main_backend.py`。
只做页面视觉调试或没有可用设备时，可以运行 Mock 启动脚本：

```powershell
.\run-nsh-mock.bat
```

该脚本不会启动 Python 后端；生产运行不允许使用 Mock。

## 构建发布包

```powershell
uv run python scripts/build.py --version 1.0.0
```

构建流程会依次编译：

1. `nsh package nsh-app --target windows` 生成 Native Shell 前端；
2. `main_backend.spec` 生成无界面的 `AALC Backend.exe`；
3. `updater.spec` 生成 `AALC Updater.exe`；
4. 组装 `dist/AALC.nsh/` 并复制 `nsh/`、`assets/`、README 和 LICENSE；
5. 使用 7-Zip 生成 `dist/AALC.nsh_<version>.7z`。

没有安装 7-Zip 时会生成等价的 `.zip` 归档。

正式包不需要 Python 或 Qt。首次启动前请确认 `AALC.nsh.exe`、`nsh/`、
`backend/AALC Backend.exe` 和 `AALC Updater.exe` 位于 `dist/AALC.nsh/`。
