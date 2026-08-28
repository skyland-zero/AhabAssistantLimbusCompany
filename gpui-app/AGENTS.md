# GPUI App 开发约束

本文档适用于 `gpui-app/` 及其所有子目录。以后新增、修改和重构 GPUI 代码时，必须遵守本文档；如果代码与本文档冲突，应先调整代码结构或更新本文档，不得默默恢复旧的大文件和职责混合模式。

## 1. 项目目标

`gpui-app` 是与 `ui/` 同级的 GPUI 原生桌面应用。它应保持：

- 原生 GPUI 渲染，不通过 WebView 实现页面。
- 页面、状态、领域模型、IPC 和 Shell 分层。
- Mock IPC 与 Python sidecar 使用同一套 JSON-RPC 契约。
- 浅色/深色主题、强调色和中英文由统一的模型与 Palette 驱动。
- 在不改变现有用户行为和 RPC 契约的前提下持续拆分复杂模块。

## 2. 目录职责

新增代码必须放到职责对应的目录，不得为了方便全部堆进 `mod.rs` 或页面入口文件。

```text
src/
├── app/                 应用编排、生命周期、页面切换、全局交互
├── components/          可复用 GPUI 组件和设计系统
│   ├── base.rs          基础按钮、卡片、Badge 等
│   ├── controls/        switch、select、slider、tabs、输入等控件
│   ├── overlays.rs      Dialog、弹层、滚动区、空状态、加载状态
│   ├── style/           ColorToken、Palette、主题运行时兼容层
│   └── text_input/      Entity-backed 文本输入、编辑、IME、剪贴板
├── ipc/                 RPC 契约、网关和传输适配器
│   ├── contract.rs      method、event、请求/响应和错误契约
│   ├── gateway.rs       类型化 RPC 边界
│   ├── mock/             确定性 Mock 状态、处理器、客户端和测试
│   └── websocket/        WebSocket 客户端、worker、sidecar 生命周期
├── model/               可序列化领域模型、配置和纯数据类型
├── pages/               页面渲染和页面专属 UI 辅助逻辑
│   ├── home/             主控台及其任务、设备、日志、执行面板
│   ├── settings/         设置页及独立设置卡片
│   ├── teams/            队伍页、编辑器、列表和弹层
│   ├── help/             帮助页渲染和 Markdown 解析
│   └── theme_packs/      主题包页、图标和主题包行
├── shell/                标题栏、侧栏、Toast 等窗口壳层
├── state/                状态容器、状态变更和持久化
│   ├── home/             主控台状态和执行状态
│   ├── other/            设置、工具箱、资源、主题包状态
│   └── teams/            队伍筛选、编辑、镜牢配置和持久化
├── assets.rs             资源身份、嵌入资源和发布版解析
├── i18n/                 双语文案目录
└── theme.rs              从应用设置派生运行时主题快照
```

目录职责边界：

- `model/` 不依赖 GPUI，不执行 RPC，不渲染 UI。
- `state/` 负责状态变更、RPC 调用和持久化，不构建 GPUI 元素。
- `pages/` 负责把状态渲染成 UI；事件闭包必须调用 state/app 的方法，不得在页面内复制一套业务状态。
- `components/` 不知道具体页面业务，不直接访问 `AhabApp`、页面状态或 Mock 状态。
- `ipc/` 不依赖页面和 GPUI 组件；传输层只能实现或调用统一契约。
- `shell/` 只处理窗口级 UI 和全局反馈，不承载具体页面业务。
- `assets.rs` 是资源入口；页面不得自行拼接仓库绝对路径。

## 3. 模块和文件规模

### 3.1 单一职责

一个文件应围绕一个清晰职责组织。以下内容必须拆到不同文件：

- UI 渲染与状态变更。
- 状态模型与状态操作。
- RPC 契约与传输实现。
- 主题 token、Palette 计算与运行时兼容逻辑。
- Markdown/JSON/数值等纯解析或转换逻辑。
- 静态 SVG、资源映射与页面布局。

### 3.2 行数阈值

- 新建 Rust 文件目标控制在 **300 行以内**。
- 超过 **350 行**必须检查是否可以按职责拆分，并在代码审查说明原因。
- 超过 **500 行**原则上禁止合入；必须拆成子模块。只有纯静态资源表、由工具生成的代码或无法合理拆分的兼容映射可以例外，并要在文件顶部注释原因。
- `mod.rs`/`lib.rs`/入口文件只负责模块声明、少量编排和稳定的 `pub use` 导出；不要把完整页面或大型实现继续放回入口。
- 拆分后通过 facade/re-export 保持调用方稳定，例如继续支持 `components::button`、`state::TeamsState` 等既有路径，避免无必要地扩散改动。

### 3.3 拆分方式

推荐结构：

```text
feature/
├── mod.rs          模块说明、子模块声明、聚合导出、少量入口编排
├── types.rs        数据类型和常量
├── state.rs        状态变更
├── persistence.rs  持久化和 RPC 适配
└── view.rs         UI 渲染
```

页面或组件子模块应按领域命名，例如 `daily.rs`、`windows.rs`、`parser.rs`、`rows.rs`，不要使用含义不明的 `misc.rs`、`utils.rs` 承载越来越多的代码。

## 4. GPUI 和组件规范

- 优先复用 `components` 中已有的 `button`、`card`、`badge`、`switch`、`select`、`slider`、`tabs`、`dialog`、`scroll_area` 等组件，不在页面重复实现同一套视觉样式。
- 新增通用组件时，放入正确的 `components` 子模块，并在 facade 中按需导出；页面专属控件留在对应页面目录。
- 页面状态由 `AhabApp`、页面 state 或 entity 持有；渲染函数只读取快照并绑定事件。
- 交互控件必须考虑键盘操作：可操作元素应提供稳定 `id`、`tab_index`、`aria_label`、`focus_visible` 样式，并处理 Enter/Space 或方向键等对应操作。
- 激活键统一使用现有 `is_activation_key` 逻辑，不要在各页面复制不一致的判断。
- 选择框、弹窗和滚动区的打开状态应由所属页面 state 管理，避免组件内部再维护一份选中值或打开状态。
- 需要 IME、选区、剪贴板或中文输入的字段必须使用 `components::text_input::TextInput` 和 `EntityInputHandler`；简单展示文本才使用轻量的视觉输入组件。
- 自定义 GPUI `Element`、绘制 seam 或事件处理时，必须保留 UTF-8/UTF-16 边界处理和选区语义，不得按字节索引用户输入字符串。

## 5. 主题、颜色和样式

- 所有语义颜色必须来自当前 `Palette`，例如 `background`、`card`、`foreground`、`brand`、`danger`、`warning`、`success`。
- 新代码优先使用 `palette_rgb(token)`；只有仍在迁移的旧页面兼容代码才使用 `render_rgb`/`render_rgba`。
- 不得在页面中新增散落的硬编码颜色、旧版暗色常量或独立主题状态。
- 需要新增颜色时，先在 `components/style/tokens.rs` 和 `palette.rs` 定义语义 token，再修改使用方。
- 主题模式和强调色必须从应用设置派生；不要在页面或组件内部缓存一份长期有效的主题副本。
- 间距、字号、圆角和颜色优先使用已有设计 token 或组件约定，避免同一语义出现多组近似常量。

## 6. 文案、资源和外部进程

- 所有用户可见文案必须进入 `i18n` 双语目录，并通过 `i18n::text`、`i18n::paired` 等现有入口读取。
- 不得在事件提示、空状态、错误信息或按钮中新增只支持中文或只支持英文的裸字符串。
- 所有固定图片、帮助文档和状态资源通过 `assets::Asset`、`assets::image_source`、`assets::help` 或 `AssetResolver` 获取。
- 不得把 `D:\...`、仓库绝对路径或用户机器路径写入源码；资源解析只能使用已知相对路径、环境变量或可执行文件附近的资源目录。
- 打开外部链接前必须限制为 `http://` 或 `https://`，不能把任意用户输入直接传给系统 shell。

## 7. 状态、模型和 IPC

- 页面事件只调用 `app`/`state` 提供的领域方法；不要从页面直接改动深层配置字段并绕过边界校验、持久化和 RPC。
- 所有后端访问必须经过 `RpcClient`、`RpcGateway` 或对应的 state 适配层；页面不得直接依赖 `MockState`、WebSocket worker 或 sidecar 进程。
- 新增 RPC 方法时，先更新 `ipc/contract.rs`，再同步 gateway、Mock 和真实传输实现，并补充契约测试。
- RPC 错误使用结构化错误信息；不要用空字符串、未说明的 `None` 或 panic 代替后端错误。
- Mock 必须是确定性的，默认值和事件顺序要可测试；不要让单元测试依赖真实设备、网络或用户配置。
- 配置变更必须经过统一持久化入口；涉及版本变化时要增加迁移逻辑和 round-trip 测试。
- 外部输入（RPC、JSON、环境变量、用户文本）必须显式校验边界；运行时路径不得用 `unwrap()` 掩盖错误。

## 8. 测试要求

修改行为或公共边界时，必须同时更新测试。优先在最靠近逻辑的位置测试：

- `model/`：默认值、序列化、版本迁移、边界值。
- `state/`：状态转换、互斥选项、RPC 调用、持久化和事件处理。
- `ipc/`：契约 JSON、结构化错误、Mock 与 WebSocket 行为。
- `components/style/`：主题 token、浅色/深色和强调色映射。
- `components/text_input/`：键盘动作、UTF-16 选区、IME 和剪贴板边界。
- `pages/`：纯解析器、标签转换、边界计算和稳定 UI 文案映射。

不要为了让测试通过而放宽生产代码校验；应修复契约或补齐正确的测试输入。

## 9. 完成前验证

在提交或交付前，从 `gpui-app/` 目录执行：

```powershell
cargo +nightly fmt --all -- --check
cargo +nightly check --all-targets
cargo +nightly test --all-targets
cargo +nightly clippy --all-targets -- -D warnings
git diff --check
```

涉及页面布局、主题、窗口壳层或交互时，还应按需要运行 `scripts/` 下的视觉回归脚本，并检查 `artifacts/visual/` 的差异报告。

交付前还要确认：

- `rg` 搜索不到已经移除的旧模块路径或重复实现。
- 新文件没有超过行数阈值，必要时已说明例外。
- 没有新增编译警告、尾随空白、绝对路径或未本地化文案。
- 现有测试数量和行为没有无理由减少。
- `git status` 中的修改都属于当前任务，未覆盖或重置用户已有改动。

## 10. 推荐工作流

1. 先读取相关 `mod.rs`、调用方和现有测试，确认模块边界与公共 API。
2. 先设计目录和职责，再移动实现；不要边写边把新逻辑继续塞入旧大文件。
3. 用 facade/re-export 保持外部调用稳定，优先减少行为无关的调用方改动。
4. 每完成一个逻辑组就运行 `cargo fmt` 和 `cargo check --all-targets`。
5. 全部完成后运行第 9 节的完整验证，并复核行数、目录和差异。
6. 汇报时说明改动范围、是否有行为变化、验证命令和任何明确的例外。

## 11. 禁止事项

- 不得恢复单个数百至数千行的页面、状态或组件总文件。
- 不得把业务状态、RPC 处理和 GPUI 渲染写在同一个新模块中。
- 不得在页面绕过 state/gateway 直接操作 Mock 或 WebSocket。
- 不得复制按钮、选择框、主题颜色、键盘激活和本地化逻辑形成平行实现。
- 不得为了快速通过编译而删除测试、降低校验、吞掉错误或增加无依据的 `allow`。
- 不得使用 destructive Git 操作覆盖用户未提交的工作。
- 不得在没有行为测试或视觉验证依据的情况下进行大范围样式重写。

如果确实无法遵守某条规则，必须在代码注释、提交说明或交付报告中写明原因、影响范围和后续处理计划。
