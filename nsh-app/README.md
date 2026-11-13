# nsh-app

AALC 的新前端：**Rust 后端（`native/`）+ TSX 前端**，基于 [Native Shell](https://nsh.dev)。
界面按迁移前的客户端 1:1 重写，参考截图归档在 `docs/reference/gpui/`。

开发约定见 [AGENTS.md](./AGENTS.md)，迁移进度与来源映射见 [MIGRATION.md](./MIGRATION.md)，
设计标准（token、尺度、组件契约、动效清单）见 [DESIGN.md](./DESIGN.md)。

## 环境

```bash
# macOS / Linux
curl -fsSL https://nsh.dev/install.sh | sh
# Windows (PowerShell)
irm https://nsh.dev/install.ps1 | iex
```

安装到 `~/.nsh`，并在 `~/.local/bin/nsh` 建一个链接（确保 `~/.local/bin` 在 `PATH` 里）。
本机访问 GitHub 受限时需要代理：

```bash
export http_proxy=http://127.0.0.1:7890 https_proxy=http://127.0.0.1:7890
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

## 后端模式

Rust 侧 `ipc/` 已迁入 schema 3 契约、确定性 Mock 与真实 sidecar：

- 默认连接 Python sidecar；也可以用 `AHAB_BACKEND_URL` + `AHAB_BACKEND_TOKEN` 接管外部实例。
- `AHAB_BACKEND=mock` 使用 Rust 内的确定性 Mock，不启动 Python（无后端开发、`nsh test`）。
- TSX 可 import `Backend`（`schemaVersion()` / `mode()` / `ping()`）检查当前传输；
  P1 的 state 层会复用同一个 `BackendClient`。
- `cargo test` 含真实 sidecar 集成用例；设 `AHAB_RUN_SIDECAR_INTEGRATION=1` 才会启动 Python。

## 结构

```text
nsh.json         应用清单（窗口、图标、主题、capabilities、Rust crate）
App.tsx          入口：mount(App)，只做窗口壳与页面宿主
store.ts         屏幕级共享状态（当前页面、外观），信号私有、只暴露函数
theme.json       默认主题（启动用；其余皮肤在 themes/）
native/          Rust 后端 crate —— RPC、状态、领域模型、i18n
shell/           标题栏、状态点、Toast
components/      应用自己的 ui 层（Button/Card/Badge/Icon/Page/EmptyState）
pages/           页面（含开发期的组件对照页 Gallery.tsx）
assets/          随包资源（icons/ 为 Lucide SVG）
```

## 主题

`theme.json`（启动时的默认皮肤）与 `themes/skins.ts` 是 shadcn `cssVars` 形态的皮肤表
（7 皮肤 × 浅/深 × 34 个颜色 token + 4 个装饰 token，另加 7 个强调色的差异表）。
`themes/index.ts` 在运行时合成「皮肤 × 强调色 × 颜色主题」并调 `setTheme()`；颜色主题的
种子（`themeSeed`）由 `themes/seed.ts` 换算成品牌色相关的七个 token。

这张表早期由一次性的 `scripts/export_nsh_theme.py` 从 GPUI 调色板导出，脚本已随旧客户端删除，
之后**直接在 `themes/skins.ts` 手写维护**：`nsh` 皮肤就是按设计指南新增的，不再有可重跑的生成器。
