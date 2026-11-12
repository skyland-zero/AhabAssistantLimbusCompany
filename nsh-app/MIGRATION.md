# 迁移台账：GPUI → Native Shell

`nsh-app/` 用 Native Shell 重写原 GPUI 客户端，**Rust 做后端、TSX 做前端**。原 `gpui-app/` 在 P5 删除，因此这里记录的每一个来源文件都要么被搬入 `nsh-app/native/`，要么被 TSX 重写，**不允许运行时引用**。

状态标记：`[ ]` 未开始 · `[~]` 进行中 · `[x]` 完成

## 0. 阶段总览

| 阶段 | 内容 | 状态 | 完成判据 |
| --- | --- | --- | --- |
| P0 | 骨架：`nsh.json`、窗口壳、主题 token、`assets/`、Rust 导出通道 | [x] | `nsh run nsh-app --target gpui` 开窗，侧栏与主题正常 |
| P1 | Rust 后端平移：`model` → `ipc` → `state` → `i18n` → `app` | [ ] | `cargo test` 通过；Mock 与真实 sidecar 双通 |
| P2 | TSX 设计系统：`components/` + `shell/` | [~] | 组件对照页与 `gpui-app` 截图逐个一致 |
| P3 | 页面重写：home → teams → settings → theme_packs → help | [ ] | 每页截图 1:1 对照 + `nsh test` 用例 |
| P4 | 打包切换：`nsh package`，替换发布流程里的 GPUI 可执行文件 | [ ] | 安装包可用；DPI/单实例/托盘/全局热键实机验证 |
| P4.5 | 仓库级引用切换：9 处脚本/CI/文档改指向 `nsh-app/` | [ ] | `python3 scripts/check_no_gpui_app_refs.py` 通过 |
| P5 | `git rm -r gpui-app`，更新根 `AGENTS.md` | [ ] | 完整构建 + 冒烟通过，检查脚本无白名单 |

## 1. P0 已落地

- `nsh.json`：`rust: "native"`、隐藏标题栏（`window.titleBar.style = "hidden"`，820×680 / 最小 800×560，与 `main.rs` 的 `Bounds::centered` 一致）、`singleInstance`、`capabilities.network.hosts = 127.0.0.1 + localhost`（sidecar WebSocket）。
- `theme.json` + `theme/{glass,archive,mist,limbus}.json`：由 `scripts/export_nsh_theme.py` 从原 `palette/skins.rs`、`tokens.rs` 机械导出，**5 皮肤 × 2 方案 × 34 个颜色 token + 4 档圆角**，值逐字节一致。
- `native/`：`aalc-native` crate（空 `[workspace]`，独立构建），`#[native_module("aalc/native")]` 注册 `Runtime` 模块（`appVersion`、`platformOs`）。
- `App.tsx`：窗口壳 + 占位页面宿主 + Rust 版本回显；`shell/TitleBar.tsx` 自绘标题栏（`windowDrag`）。P0 当时把导航做成了左侧栏，**与 GPUI 客户端的真实结构不符**（真实结构是标题栏内的 6 个导航项 + 右侧设置/外观/窗口按钮，没有侧栏），已在 P2 第一轮纠正。
- `App.test.tsx`：`nsh test` 冒烟用例（骨架渲染、侧栏切页、Rust 模块可调用）。
- `assets/`：原 `gpui-app/resources/assets/` 全量复制（logo、横幅、12 立绘、11 状态图、limbus 主题、2 篇帮助文档）+ `app-icon.png`。
- **截图链路已打通**：`scripts/capture_nsh.py` 在 Xvfb + Mesa lavapipe（软件 Vulkan）里跑 `nsh run --target gpui`，抓 X11 根窗口，产出 `nsh-app/docs/shots/p0-desktop.png`（1100×720）。无 GPU 机器也能出真实渲染图。

### P0 截图发现的问题（转 P2）

- `shell/TitleBar.tsx` 的配色是硬编码的 limbus 色值，浅色主题下与页面背景冲突 → 改主题 token。**【P2 已修】**
- `font-semibold` 在 Linux 上回退（宿主日志 `.SystemUIFont` weight 600 缺失），标题没有真正加粗 → P2 定字体族。**【待办】**
- 侧栏无选中态、内容区留白过多 → P2/P3 落实。**【已重构：没有侧栏，导航在标题栏内，选中态用 `brand` 底面】**

## 1.1 P2 第一轮已落地（设计系统 + 窗口壳）

设计基准见 `DESIGN.md`（token、尺度、组件契约、动效清单），这一轮落地：

- **结构纠正**：`App.tsx` 只剩「标题栏 + 页面宿主」；`store.ts` 拥有 `page`（路由）与
  `appearance`（浅色/深色/跟随系统）；导航改为标题栏内的 6 项（主控台、队伍管理、镜牢主题包、
  工具箱、资源中心、帮助），设置是标题栏右侧的图标按钮，与 `shell/title_bar/mod.rs` 一致。
- **`shell/TitleBar.tsx`**：品牌（横幅 38×20 + `AALC`）、6 个导航项、控制台状态点、外观切换、
  设置按钮、（非 macOS）自绘窗口按钮。拖拽区域只有品牌块与中间弹性空白，点导航是点击不是拖窗。
  全部配色改为 token（原来硬编码的 limbus 色值已删）。
- **`shell/StatusDot.tsx`**：空闲不画；运行 `success`、暂停 `warning`。GPUI 的呼吸动画本来就是
  关闭的（`ENABLE_STATUS_DOT_BREATHING = false`），这里保持一致。
- **`components/`（应用自己的 ui 层）**：`Button`/`IconButton`（6 变体 × 4 尺寸 ×
  disabled/selected/焦点环）、`Card`/`CardHeader`/`Rule`、`Badge`（6 语义色）、`Icon`
  （Lucide SVG + `tintColor`）、`PageRoot`/`PageToolbar`、`EmptyState`。
- **`assets/icons/`**：从 Lucide 官方 SVG 取 17 个图标（`lucide-static`，ISC），名字按用途映射
  （`home`→`house`、`help`→`circle-question-mark`、`restore`→`square-stack`）。
- **皮肤与强调色（第二轮）**：`themes/skins.generated.ts` 由 `scripts/export_nsh_theme.py` 生成
  （5 皮肤 × 竖排 token + 6 强调色的差异表），`themes/index.ts` 在运行时合成 `themeFor(skin, accent)`
  并 `setTheme()`；`store.ts` 拥有 `skin`/`accent`，切皮肤时若用户没选过强调色则跟随该皮肤的默认配色。
- **5 套装饰语言**：卡片（边狱四角角框、玻璃顶部高光、血雾/边狱阴影档位）、标题带（边狱渐变带 +
  竖条 + 污渍、档案向右发丝线、血雾左侧竖条、玻璃顶部高光）、空状态标记（封蜡/横杠/芯片）、
  分隔符（档案与边狱用金色花纹图）、标题栏分隔线、`shell/SkinBackdrop.tsx` 的皮肤背景层
  （边狱星云、血雾渐晕+颗粒、玻璃顶部渐晕）。
- **`pages/Gallery.tsx`**：组件对照页（变体/尺寸/状态/图标/token 色板/圆角），不进导航，开发时把
  `store.ts` 的初始 page 改成 `"gallery"` 打开。
- **测试**：`App.test.tsx` 5 条（导航齐全、切页、设置入口、外观循环、共享窗口壳），
  `nsh test` 全绿（含 2 条 Rust 单测）。

这一轮**没有**做的（仍在 P2/P3 范围）：

- 语言切换/`i18n`：标题栏文案暂时写死中文，等 P1 的 `i18n` 模块。
- 标题栏右侧窗口按钮、托盘、全局热键的 Windows 实机验证。
- 字体族（Linux 上 `font-semibold` 回退）仍待定。

## 2. 来源映射

### 2.1 搬进 Rust（`nsh-app/native/src/`）

| 原位置（`gpui-app/src/`） | 行数 | 目标 | 备注 |
| --- | --- | --- | --- |
| `model/` | 1678 | `native/src/model/` | 纯数据类型，几乎零改动 |
| `ipc/contract.rs` | — | `native/src/ipc/contract.rs` | 与 `contract/contract.json` 由校验脚本对齐 |
| `ipc/gateway.rs` | — | `native/src/ipc/gateway.rs` | 类型化 RPC 边界 |
| `ipc/websocket/` | — | `native/src/ipc/websocket/` | worker/`Entity` 通知改 `Callback` |
| `ipc/mock/` | 647 | `native/src/ipc/mock/` | 供 `nsh test` 与无后端开发 |
| `state/teams/` | 1967 | `native/src/state/teams/` | `Entity<State>` → `#[class]` + 信号回调 |
| `state/home/` | 926 | `native/src/state/home/` | 执行状态机、预览、统计 |
| `state/other/` | 924 | `native/src/state/other/` | 设置、工具箱、资源、主题包 |
| `state/app_state.rs` | 88 | `native/src/state/app_state.rs` | 顶层状态聚合 |
| `i18n/` | 400 | `native/src/i18n/` | 额外导出 `I18n.t()` module 给 TSX |
| `app/lifecycle/` | 897 | `native/src/app/lifecycle/` | 启动/恢复/关闭，去 `cx` 依赖 |
| `app/{scheduling,device,preview,stats}.rs` | — | `native/src/app/` | 调度、设备、预览帧 |
| `assets.rs` | — | `native/src/assets.rs` | 路径改为 `assets!()` 内嵌 |
| `components/style/palette/*` | 875 | **不搬** | 已导出为 `theme.json`，代码丢弃 |

### 2.2 用 TSX 重写

| 原位置 | 行数 | 目标 | 说明 |
| --- | --- | --- | --- |
| `components/base.rs`、`chrome.rs`、`layout.rs` | — | `components/` | 改用 `nsh/base` 的 Button/Dialog 等 + 主题 token |
| `components/controls/` | 401 | `components/controls/` | Switch、Select、Slider、Tabs |
| `components/overlays.rs` | — | `components/overlays/` | Dialog、弹层、空状态、加载态 |
| `components/text_input/` | 88 | `components/text_input/` | 改用原生 `Input` |
| `components/icon.rs`、`action_button.rs`、`keyboard.rs` | — | `components/` | 图标表、动作按钮、快捷键提示 |
| `shell/title_bar/` | 625 | `shell/TitleBar.tsx` | P2 已重写：品牌、6 导航、状态点、外观/设置、窗口按钮 |
| `shell/toast.rs`、`shell/windows.rs`、`shell/mod.rs` | 459 | `shell/` | Toast 用 `nsh/base`；托盘/单实例走 `nsh/desktop` |
| `pages/home/**` | 2855 | `pages/home/**` | 日志面板（`LogView`）与预览画面保留为 Rust 组件 |
| `pages/teams/**` | 784 | `pages/teams/**` | 列表 + 6 个编辑器 + 弹层 |
| `pages/settings/**` | 360 | `pages/settings/**` | 6 张设置卡 |
| `pages/theme_packs/**` | 423 | `pages/theme_packs/**` | 主题包行与预览 |
| `pages/help/**` | 549 | `pages/help/**` | Markdown 用 `TextView` 渲染，解析逻辑可去 Rust |
| `app/render.rs`、`app/interaction.rs` 等 | 2959 | 拆到 `pages/`/`shell/` | 编排逻辑跟随页面走 |
| `resources/windows/{app.rc,app.manifest}` | — | **丢弃** | 由 `nsh package --target windows` 处理，P4 验证 DPI/单实例 |

### 2.3 仓库级引用（P4.5 必须全部改完）

| 文件 | 改为 |
| --- | --- |
| `scripts/build.py`（`gpui_binary`、`copy_tree(resources)`、`cargo build --manifest-path`） | `nsh package nsh-app` 产物 + `nsh-app/assets` |
| `scripts/validate_contract.py`（`contract.rs`、mock 目录） | `nsh-app/native/src/ipc/...` |
| `scripts/generate_limbus_theme.py`、`scripts/render_limbus_tabs.py` | 输出到 `nsh-app/assets/themes/limbus` |
| `.github/workflows/reusable-build.yml` | `nsh-app/native` + `nsh check` / `nsh test` |
| `run-gpui.bat`、`run-gpui-mock.bat` | `run-nsh.bat`、`run-nsh-mock.bat` |
| `README.md`、`assets/doc/{zh,en}/**`、`docs/**`、根 `AGENTS.md` | 另见下节（文档量最大） |

## 3. 待验证风险

- [x] `nsh` CLI 已在开发机安装（0.5.0，`~/.nsh` + `~/.local/bin/nsh`）；本机访问 GitHub 需走代理（`http://127.0.0.1:7890`），CI 需要新步骤。
- [x] `nsh check nsh-app` 通过（TypeScript 对着 `.nsh/types/aalc/native.d.ts` 校验）。
- [x] `nsh test nsh-app` 通过：5 个用例（2 个 Rust 单测 + 3 个 TSX UI 用例）。
- [x] `nsh run nsh-app --target gpui` 已在无 GPU 机器上跑通：Xvfb + `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`（lavapipe）。截图见 `nsh-app/docs/shots/`。
- [x] **mixed 模式会按应用构建宿主**：首次 `nsh test --target gpui` 花了 **11 分钟**编 `nsh-gpui-host`（NSH 引擎 + GPUI Fast + QuickJS + 我们的 crate），之后增量几分之一分钟。这意味着**任何要出包的机器（含 CI）都要有 Rust + 平台系统库**：Linux 需 GTK3 / WebKitGTK 4.1 / GStreamer / clang 的开发包。
- [ ] 截图链路目前只在 Linux（Xvfb + lavapipe）验证；Windows/macOS 要走 `nsh test --target gpui` + `app.screenshot()`，且测试宿主窗口固定 390×800，拿不到桌面尺寸对照图，P3 前需要定方案。
- [x] `cargo check` / `cargo fmt --check` / `cargo test --lib` 在 `nsh-app/native` 通过（nsh-core 0.5.0，Rust 1.99.1）。
- [~] `nsh-testing` 的 `bundle` feature 在本机依赖链编译失败（`rolldown 1.2.13` 与 `rolldown_std_utils` 不兼容），因此原生侧暂不加 `nsh-testing`/`nsh-engine` 开发依赖；不阻塞 `nsh test`（它自己构建 headless runner），P1 需要 crate 级 UI 测试时再解决。
- [ ] mixed 模式引入 QuickJS 后的发布包体积与冷启动时间（原 `gpui-app` 为纯 Rust）。
- [ ] Win32 后端打包能否覆盖原 `app.manifest` 的 DPI 感知与单实例语义；不行则退回 `--target gpui` 自建宿主（需重新评估）。
- [ ] 托盘（`Tray`）、全局热键（`GlobalShortcut`）、多窗口（`Window.open`）、自定义标题栏在 Windows 实机逐项对齐。
- [ ] 应用图标需 1024² 方形 PNG；当前 `assets/app-icon.png` 为 512²，P4 前补 1024 版本。
- [x] 6 个强调色 × 5 皮肤的运行时切换：`scripts/export_nsh_theme.py` 只把「换强调色后变了的 token」写成差异表，`themes/index.ts` 在运行时合成（`nsh test` 有合成规则的单测）。
- [ ] 图标着色是 `Image` 的 `tintColor`（值，不是 CSS 的 `currentColor`），所以图标不跟随 `hover:`/`text-*` 变色；当前用「变体换 token」+「hover 只换底面」实现（见 `DESIGN.md` 第 4 节）。需要 hover 换图标色时用双图标叠加或 signal 驱动的 token。
- [x] 皮肤装饰语言（`Decor::*`）、`ShadowLevel` 阴影档位、皮肤底图/暗角层已迁入（`components/` 按 `currentDecor()` 分支，`shell/SkinBackdrop.tsx` 画氛围层）。

## 4. 每页迁移记录

迁移每一页时补一行：来源文件 → 目标文件 → 截图对照结果 → 测试用例。

| 页面 | 来源行数 | 目标 | 状态 | 截图对照 | 测试 |
| --- | --- | --- | --- | --- | --- |
| 窗口壳 / 标题栏 | 1084 + 459 | `shell/` | [x] | 待有显示机器截图 | `App.test.tsx` 5 条 |
| 组件对照页 | — | `pages/Gallery.tsx` | [x] | 无头跑通全部区块 | 开发期手动打开 |
| 主控台 home | 2855 | `pages/home/` | [ ] | — | — |
| 队伍 teams | 784 | `pages/teams/` | [ ] | — | — |
| 设置 settings | 360 | `pages/settings/` | [ ] | — | — |
| 主题包 theme_packs | 423 | `pages/theme_packs/` | [ ] | — | — |
| 帮助 help | 549 | `pages/help/` | [ ] | — | — |
