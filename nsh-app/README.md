# nsh-app

AALC 的新前端：**Rust 后端（`native/`）+ TSX 前端**，基于 [Native Shell](https://nsh.dev)。
界面按原 GPUI 客户端 1:1 重写，迁移完成后 `gpui-app/` 会被删除。

开发约定见 [AGENTS.md](./AGENTS.md)，迁移进度与来源映射见 [MIGRATION.md](./MIGRATION.md)。

## 环境

```bash
# macOS / Linux
curl -fsSL https://nsh.dev/install.sh | sh
# Windows (PowerShell)
irm https://nsh.dev/install.ps1 | iex
```

还需要 Rust（`rustup`，本目录的 `rust-toolchain.toml` 固定 1.99.1）。

## 常用命令

```bash
nsh run nsh-app --target gpui      # 开发：GPUI 窗口 + TSX 热更新
nsh run nsh-app                    # 用当前系统自己的后端（Windows = Win32/Direct2D）
nsh check nsh-app                  # TSX 对着 Rust 生成的 .nsh/types 做类型检查
nsh test nsh-app                   # 无头跑 App.test.tsx
nsh package nsh-app --target windows

cargo test --manifest-path nsh-app/native/Cargo.toml    # 后端单元测试
```

首次 `nsh check` / `nsh test` 会编译 `native/` 与 NSH 核心，之后再跑会复用缓存。
Rust 改动会重启应用，TSX 改动是热更新。

## 结构

```text
nsh.json         应用清单（窗口、图标、主题、capabilities、Rust crate）
App.tsx          入口：mount(App)
theme.json       默认主题（其余皮肤在 theme/）
native/          Rust 后端 crate —— RPC、状态、领域模型、i18n
shell/           标题栏、侧栏、Toast
components/      设计系统
pages/           页面
assets/          随包资源
```

## 主题

`theme.json` 与 `theme/*.json` 是从原 GPUI 调色板机械导出的 shadcn `cssVars`
（5 皮肤 × 浅/深 × 34 个颜色 token）。导出脚本为 `scripts/export_nsh_theme.py`，
随 `gpui-app/` 一起在 P5 删除；之后主题**只在本目录维护**。
