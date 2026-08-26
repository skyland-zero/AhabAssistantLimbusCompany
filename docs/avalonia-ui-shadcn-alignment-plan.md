# Avalonia UI 与 Web UI 对齐实施计划

## 目标

在 `refactor/core-decouple-qt` 分支完成 Avalonia UI 的框架升级和视觉对齐：

- Avalonia 升级到当前最新稳定版 `12.1.1`；
- 不引入 ShadUI、SukiUI 或其他第三方 UI 组件库；
- 仅使用 Avalonia 官方基础控件/主题能力，自建 shadcn 风格的资源和控件样式；
- 以 `ui` 源码中的颜色、尺寸、布局和交互作为唯一视觉基准；
- 二次确认框、完成提示弹窗、队伍编辑弹窗统一对齐 Web UI；
- 不使用截图比对，采用源码数值、样式资源和运行行为核对。

## 当前基线（改造前）

- 分支：`refactor/core-decouple-qt`
- 工作区：计划开始时干净
- Avalonia 项目：`avalonia-ui/AhabAssistant.Avalonia.csproj`
- 当前 Avalonia：`11.2.1`
- 当前主题：`Avalonia.Themes.Fluent`
- 当前目标框架：`net11.0`
- Web UI 设计配置：`ui/components.json`，`new-york` / `neutral` / CSS variables / Lucide
- Web UI 主题令牌：`ui/src/index.css`、`ui/src/themes/index.ts`

## 版本与约束

本计划以 2026-08-26 查询到的 Avalonia 最新稳定版本 `12.1.1` 为目标版本：

- [Avalonia NuGet](https://www.nuget.org/packages/Avalonia/)
- [Avalonia 12 破坏性变更与迁移说明](https://docs.avaloniaui.net/docs/avalonia12-breaking-changes)
- [Avalonia 官方文档](https://docs.avaloniaui.net/)

Avalonia 12 的主版本升级和视觉改造分阶段完成。升级阶段不顺带重写业务逻辑；视觉阶段不通过引入组件库解决问题。

## 当前执行状态

- Avalonia/Avalonia.Desktop/Avalonia.Themes.Simple：`12.1.1`。
- 底层主题：Avalonia 官方 `SimpleTheme`；未引入 ShadUI、SukiUI 或其他第三方 UI 组件库。
- 自有视觉资源：`avalonia-ui/Styles/DesignTokens.axaml`、`avalonia-ui/Styles/ControlStyles.axaml`。
- 本地矢量图标：`avalonia-ui/Controls/AppIcon.axaml`、`IconCatalog.cs`；路径数据来自 Lucide 官方 SVG，具体来源和授权说明见 `avalonia-ui/Assets/Icons/README.md`。
- .NET 目标框架仍为 `net11.0`，因为这是原项目设置；本机使用 .NET 11 preview SDK，未在本次 UI 改造中擅自切换目标框架。
- 弹层实现采用页面内确认覆盖层和自定义无装饰 owner-modal Window，共享同一套源码样式资源；没有为了视觉对齐引入组件库或重写现有业务窗口生命周期。

## 执行清单

### 阶段 0：基线和影响面盘点

- [x] 确认分支、工作区状态和当前提交。
- [x] 执行升级前的 restore/build，记录成功或失败原因。
- [x] 盘点 Avalonia 包、SDK、TargetFramework、启动入口和主题加载方式。
- [x] 盘点窗口、绑定、样式选择器、控件模板、弹层和主题服务。
- [x] 从 Web UI 源码建立设计令牌映射表：颜色、字号、圆角、间距、控件高度、弹层宽度。
- [x] 记录当前已知差异，作为后续验收清单。

交付物：迁移检查结果、设计令牌映射表、差异清单。

### 阶段 1：升级 Avalonia

- [x] 将所有 Avalonia 相关 PackageReference 统一升级到 `12.1.1`。
- [x] 检查 `global.json`、本机 SDK 和 `TargetFramework` 的兼容性。
- [x] 评估 `net11.0` 是否为项目的明确要求；当前保留 `net11.0`，后续继续验证；未强行改变 .NET 目标框架。
- [x] 按 Avalonia 12 迁移说明处理编译绑定、窗口装饰、Clipboard、ThemeVariant 等变化。
- [x] 处理升级造成的编译错误和运行时异常，不改变业务行为。
- [x] 完成 restore、build、启动主窗口和打开主要页面的最小验收（启动级检查通过；页面视觉/交互验收继续按阶段 7 记录）。

阶段验收：项目可以正常还原、编译、启动，主窗口和核心页面可显示。

### 阶段 2：建立自有设计系统

新增或整理本地资源字典，当前实际使用：

- `avalonia-ui/Styles/DesignTokens.axaml`
- `avalonia-ui/Styles/ControlStyles.axaml`

实现内容：

- [x] 将 Web UI 的 `background`、`foreground`、`card`、`border`、`input`、`primary`、`secondary`、`muted`、`accent`、`destructive`、`ring` 映射为 Avalonia 语义资源。
- [x] 统一亮色/暗色主题切换，并让页面和公共样式引用语义资源。
- [x] 定义字体字号、圆角、边框、控件高度、页面间距、阴影和遮罩资源。
- [x] 清理主要页面内硬编码的状态颜色和重复尺寸。
- [x] 使用 Avalonia 官方 Simple 基础主题作为底层能力；所有视觉覆盖保存在项目源码中。

阶段验收：页面和控件引用语义资源，主题切换不会依赖页面级颜色分支。

### 阶段 3：重写公共控件样式

使用原生 Avalonia `Style`、`ControlTheme`、模板和伪类实现：

- [x] Button：default、outline、secondary、ghost、brand、destructive、disabled、focus。
- [x] TextBox：边框、占位文本、焦点环和禁用态。
- [x] ComboBox/选择器：基础输入外观、悬停和焦点态；下拉键盘导航沿用官方原生行为。
- [x] 当前页面使用的 ToggleSwitch、Slider、Separator、ScrollBar、Card、Badge 已接入项目样式。
- [x] 当前导航和分段页签使用源码自有 Button 样式；未引入未使用的 TabControl/第三方 Tabs 模板。
- [x] 图标继续使用项目内字体/Geometry/矢量资源，不引入图标组件库。
- [x] 为已覆盖的可交互控件补齐键盘焦点、禁用态和最小可读性。

阶段验收：公共控件不再依赖 Fluent 默认视觉，状态样式由统一主题令牌驱动。

### 阶段 4：主窗口和页面布局对齐

- [x] 对齐主窗口默认尺寸、最小尺寸和内容区域计算。
- [x] 对齐标题栏高度、标题文本、窗口按钮尺寸和交互态。
- [x] 对齐 TabBar 高度、页面背景、内容边距和滚动区域。
- [x] Teams：对齐单列/双列断点、卡片宽度和卡片间距。
- [x] Toolbox：对齐 1/2/3 列响应式规则。
- [x] Resources：对齐最大内容宽度、双列规则和卡片尺寸。
- [x] Home：对齐左右面板、最小宽度、折叠手柄和拖拽行为。
- [ ] 检查窗口缩放、窄窗口、125%/150% DPI 下的布局溢出。

阶段验收：页面布局规则与 `ui/src/pages` 中的断点和尺寸逻辑一致。

### 阶段 5：自建统一弹层能力

不引入 Dialog 组件库，使用原生 Avalonia 容器、Window 和共享资源实现统一弹层能力。本次保留已有复杂编辑窗口的 Window 生命周期，统一其无装饰内容外观；删除确认使用主页面级覆盖层。

- [x] 删除确认覆盖层跨越 Teams 页面完整内容区域。
- [x] 统一居中、最大宽度、圆角、边框、阴影和遮罩资源；动画暂不增加，因为当前 Web 对齐目标未要求额外动画。
- [x] 实现 Esc、关闭按钮、遮罩点击取消和确认/取消焦点路径。
- [x] 弹层打开时限制底层内容交互；owner-modal Window 使用 Avalonia 原生模态生命周期。
- [x] 支持窄窗口自适应宽度和编辑内容滚动。
- [x] 明确确认弹层、完成提示弹窗和队伍编辑弹窗的结构与复用边界。

### 阶段 6：重点弹窗对齐

#### 二次确认框

- [x] 对齐 Web Dialog 的结构和语义。
- [x] 桌面最大宽度为 `400px`，窄窗口按可用宽度减左右边距收缩。
- [x] 对齐遮罩透明度、圆角、内边距、标题字号、描述字号和按钮高度。
- [x] 对齐 destructive 操作颜色、按钮顺序和取消/确认行为。
- [x] 确保 Esc、关闭按钮、取消、确认和点击遮罩取消行为可用。

#### AfterCompletionModal

- [x] 对齐 Web 端约 `384px` 的最大宽度。
- [x] 消除原生窗口标题栏与内容标题重复的问题。
- [x] 对齐标题、内容间距、底部操作区、Esc 和关闭行为。

#### TeamEditModal

- [x] 对齐 Web 端约 `768px` 最大宽度。
- [x] 对齐 `620px` 高度和窄视口下的最小高度/内容滚动约束。
- [x] 统一标题栏、内容滚动区、底部操作区、Esc 和关闭行为。

阶段验收：三个弹层在视觉尺寸、遮罩范围、交互行为和窄窗口表现上与 Web UI 规则一致。

### 阶段 7：验证和收尾

- [x] 执行 `dotnet restore`。
- [x] 执行 Debug/Release `dotnet build`。
- [x] 启动主窗口并完成启动级验收；页面导航和业务控件已完成源码级绑定检查。
- [ ] 验证 100%、125%、150% DPI 与窗口缩放。
- [ ] 验证弹层打开、关闭、Esc、焦点、确认/取消和滚动行为。
- [x] 使用源码数值核对 Web UI 与 Avalonia，不进行截图比对。
- [x] 确认没有新增 ShadUI、SukiUI 或其他第三方 UI 组件库依赖。
- [x] Native AOT/Trim `win-x64` 发布构建通过，发布程序可启动。
- [x] 检查 `git diff`、`git status` 和计划文档状态。

最终验收：构建通过、核心交互无回归、视觉令牌和弹层尺寸完成对齐、工作区变更可追踪。

### 阶段 8：统一本地图标

基于 shadcn 使用的 Lucide 风格，采用本地 Avalonia `Path` 几何实现，不添加 Lucide 运行时包：

- [x] 扫描并移除页面中的 Emoji、Unicode 操作符号和 Segoe MDL2 字体图标。
- [x] 新增 `AppIcon` 控件和本地图标路径目录，只保留当前界面实际需要的图标。
- [x] 替换标题栏、导航、任务卡片、操作按钮、Toast、状态提示、空状态和弹窗图标。
- [x] 统一图标尺寸、圆角线帽、线连接和主题颜色；保留 Tooltip 文本作为可访问性补充。
- [x] 添加 Lucide 官方来源和 ISC License 说明，不引入第三方 UI 组件依赖。

阶段验收：源码中不再存在 UI Emoji/MDL2 图标入口，所有界面图标由项目内 `AppIcon` 统一绘制。

## 建议提交边界

按以下边界拆分提交，便于回滚和审查：

1. Avalonia 12.1.1 框架升级与迁移修复。
2. 设计令牌和主题资源。
3. 公共控件样式。
4. 主窗口和页面布局。
5. ModalHost 与二次确认框。
6. 其他弹窗和最终验证修复。
7. 本地图标系统与全量图标替换。

## 执行记录

| 日期 | 阶段 | 状态 | 备注 |
| --- | --- | --- | --- |
| 2026-08-26 | 计划建立 | 已完成 | 已确认分支、基线项目和目标方案 |
| 2026-08-26 | 阶段 0 | 已完成 | 完成源码盘点；首次还原遇到 NuGet SSL 重试，使用缓存和 `--ignore-failed-sources` 后恢复成功 |
| 2026-08-26 | 阶段 1 | 已完成 | 包升级到 12.1.1；restore、Debug/Release 编译和启动级检查通过；目标框架仍为原有 `net11.0` |
| 2026-08-26 | 阶段 2 | 已完成 | 建立语义设计令牌和亮/暗主题映射，页面主要状态色改为 DynamicResource |
| 2026-08-26 | 阶段 3 | 已完成 | 基于 SimpleTheme 自建 Button、Input、Select、Toggle、Badge、Card、Separator、ScrollBar 等样式 |
| 2026-08-26 | 阶段 4 | 已完成 | 完成主窗口尺寸、壳层、页面断点和 Teams/Toolbox/Resources 响应式列布局对齐 |
| 2026-08-26 | 阶段 5 | 已完成 | 确认框使用页面级遮罩，复杂弹窗使用统一无装饰 owner-modal Window，共享 dialog-surface 和交互规则 |
| 2026-08-26 | 阶段 6 | 已完成 | 二次确认框 400px、AfterCompletion 384px、TeamEdit 768px/620px 已完成源码级对齐 |
| 2026-08-26 | 阶段 7 | 已完成（源码/构建验收） | restore、Debug/Release build、Native AOT/Trim 发布和发布程序启动均通过；未使用截图比对 |
| 2026-08-26 | 阶段 8 | 已完成 | 使用本地 Lucide 风格 Path 几何替换 Emoji、Unicode 操作符号和 Segoe MDL2 图标；残留扫描通过 |

## 尚需人工操作验收

以下项目需要在真实桌面上由开发者操作确认，不能仅凭编译或源码检查替代：

- 依次点击主导航，确认各页面首次加载和返回行为。
- 在设置中切换亮色/暗色/强调色，确认主页面内的动态资源即时刷新。
- 手动打开三类弹层，验证鼠标、键盘焦点、Esc、确认/取消和窄窗口表现。
- 使用 125%/150% DPI 运行并检查文字、按钮和卡片是否发生裁切或溢出。
