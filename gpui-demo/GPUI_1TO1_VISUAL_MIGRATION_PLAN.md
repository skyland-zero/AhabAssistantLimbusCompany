# Ahab Assistant：`ui` → GPUI 1:1 视觉迁移实施计划

> 状态：二次审查通过（修订版）
>
> 目标：以 `D:\Github\AhabAssistantLimbusCompany\ui` 的实际运行结果为唯一视觉真源，将当前 Tauri/React 界面迁移为不依赖 WebView2 的 GPUI Windows 原生客户端。
>
> 本文只规划 GPUI 迁移，不修改现有 `ui`。

## 当前实现进度

本轮已在 `gpui-demo` 落地一批阻塞项，但尚未达到完整 1:1 验收：

- ✅ Shell：顶部 TabBar、无 Sidebar 根布局、无边框自绘标题栏、窗口尺寸约束。
- ✅ 运行时主题基础：页面旧 token 通过 render snapshot 跟随 light/dark/accent，输入控件同步 Palette；页面遗留色值已集中映射，Teams 图标改为 currentColor；全局 Toast 骨架已接入。
- ✅ Mock 边界：同一窗口内各页面共享一个 Mock backend，避免 Home/Teams/Settings 状态互相脱节。
- ✅ Home 第一批：General/Advanced 选项 Tab、日常队伍名称绑定、数量加减、Lucide SVG 任务/执行图标、日志 payload/时间/级别/复制/自动滚动。
- ✅ 其他页面修正：Resources 同步终态保留可见窗口，Teams 在 900px 下保持单列并补齐列表/编辑器主要英文标签。
- ⏳ P0/P8：参考截图、尺寸表、像素差异和内存回归尚未建立。
- ⏳ P2/P3/P4/P5/P6：下拉 Select/对话框语义、全量 i18n、Home 全部 Select/描述、Teams 全量主题/英文仍需继续；离散设置/队伍选择已支持方向键，主题包权重 Slider 已支持鼠标拖拽。
- ⏳ P7：真实 WebSocket、JPEG 帧、托盘、全局热键注册和 Windows 系统 API 尚未接入。

因此以下通过条件仍然是后续门禁，不把当前 Mock/骨架状态误报为完成。

## 0. 审查结论和原则

原有迁移方向可行，但必须先修正以下问题：

1. **视觉真源必须是 `ui/src` 的实际布局，而不是旧计划中的描述。** 当前 `ui` 使用顶部 `TabBar`，不使用左侧 Sidebar；GPUI 必须移除现有 Sidebar。
2. **当前 GPUI 页面属于功能原型，不能继续在原布局上打补丁。** 保留模型、IPC、状态和测试，重写 Shell 与页面 Render 层。
3. **“1:1”定义为固定窗口尺寸下的结构、尺寸、间距、颜色、字体规格和交互状态一致。** 浏览器和原生渲染的字体抗锯齿差异不作为失败条件。
4. **视觉迁移先于真实 WebSocket、托盘和系统 API。** 先使用可重复 Mock 完成视觉和行为回归，避免平台能力阻塞页面对齐。
5. **所有主题、i18n、资源路径必须在页面迁移前建立统一抽象。** 页面不得继续散落硬编码颜色、路径和用户可见文本。
6. **每个页面在迁移时同步接入 Mock 行为。** P7 只负责跨页面行为、真实 IPC 和平台能力，不能把交互实现推迟到所有页面完成之后。
7. **先建立 GPUI 对 CSS 能力的差异清单。** 对 `backdrop-blur`、阴影、过渡、动画、Portal、字体 fallback 等能力逐项验证；不支持时必须明确替代方案并记录为视觉验收例外，不能悄悄改变样式。

视觉冲突的优先级：

```text
ui 实际 DOM/布局
> ui/src/index.css 和 ui/src/themes
> ui 组件的计算样式
> 本文的迁移约束
> 旧 GPUI 实现和旧计划中的冲突描述
```

## 1. 固定视觉基准

### 1.1 窗口

沿用 `ui/src-tauri/tauri.conf.json`：

- 初始尺寸：`900 × 680`
- 最小尺寸：`800 × 560`
- 居中启动
- 无系统装饰边框
- GPUI 使用自绘标题栏，不能同时显示系统标题栏和自绘标题栏

### 1.2 必须采集的参考状态

在修改 GPUI 页面前，从 `ui` 生成参考截图和尺寸记录：

- 中文 + 浅色
- 中文 + 深色
- English + 浅色
- English + 深色
- 窗口 `900 × 680`
- 窗口 `800 × 560`
- Home 默认、任务展开、运行中、暂停中
- Teams 列表、空分类、编辑弹窗、删除确认
- Theme Packs 停用主题包和困难镜牢提示
- Toolbox 工具运行中
- Resources 同步中
- Settings 全部设置卡片
- Help 目录和长文档滚动

参考截图必须来自与 GPUI 相同的生产窗口条件。浏览器开发模式中 `isTauri()` 为 false，标题栏按钮缺失，不能直接作为最终窗口基准。

### 1.3 视觉验收容差

- 区块边界、标题栏和控件高度：目标误差不超过 `1~2px`
- 颜色：使用同一组设计 token，不允许页面自行近似
- 字号、字重、圆角、边框和阴影：逐项对齐
- 文本抗锯齿和平台字体 fallback 的微小差异允许存在
- 不允许出现额外 Sidebar、额外滚动层、重复标题栏或不同的默认主题

## 2. 当前代码处理策略

### 2.1 保留

- `gpui-demo/src/model/`
- `gpui-demo/src/ipc/contract.rs`
- `gpui-demo/src/ipc/mock.rs`
- `gpui-demo/src/state/`
- 配置版本迁移和原子写入
- 已有 Mock IPC 和状态单元测试
- `TextInput` 的 `EntityInputHandler`、UTF-16、IME 和剪贴板基础

### 2.2 重构或重写

- `src/app.rs`
- `src/shell/title_bar.rs`
- `src/shell/sidebar.rs`：停止使用，不能作为视觉 Shell
- 新增 `src/shell/tab_bar.rs`
- `src/components/style.rs`
- `src/pages/*.rs`
- 队伍编辑器 Overlay
- 主题和 i18n 模块

### 2.3 不修改

- `ui/` 的源代码、资源和未提交修改
- Python 自动化后端业务逻辑
- 原有 IPC 业务语义

## 3. 实施阶段

### P0：基准、测量和结构冻结

### 目标

在动 GPUI 页面前，锁定实际 `ui` 的结构和计算样式。

### 工作项

- 记录 `App.tsx` 的根布局：

  ```text
  TitleBar 40px
  TabBar 36px
  main flex-1
  ```

- 记录所有页面的根节点、滚动边界和响应式断点。
- 记录主要区块的 bounding box、padding、gap、border、radius、字体和**浏览器解析后的 sRGB 颜色**；不能只复制未解析的 `oklch` 或 Tailwind class。
- 区分窗口外尺寸和内容 client viewport，确保 Tauri 的 `900×680` 与 GPUI 的可渲染区域一致。
- 建立 React 组件到 GPUI 组件的映射表。
- 明确 `sidebarCollapsed` 是当前 store 中的兼容字段，不代表当前 UI 存在 Sidebar。
- 建立截图命名规则和对比目录。
- 建立可重复的截图流程：React/Tauri 与 GPUI 使用相同 client viewport、相同 DPI 和相同系统主题；禁止只依赖手工目测截图。
- 固定 Tailwind 默认断点：`sm=640`、`md=768`、`lg=1024`、`xl=1280`、`2xl=1536`；900px 窗口下 `md` 生效而 `lg/xl` 不生效，Teams/Resources/Toolbox 的列数必须按此验证。

### 通过条件

- `ui` 的 7 个页面都有固定尺寸参考图，且截图流程可以重复执行。
- Shell、Home、Teams 的主要区块尺寸、滚动边界和 resolved colors 已记录。
- light/dark/system 的 system 模式在截图时被固定为确定的操作系统主题。
- 后续页面不再根据目测随意调整尺寸。

### P1：GPUI Shell 1:1 重做

### 目标

让 GPUI 的根布局首先与 `ui/src/App.tsx` 一致。

### 工作项

1. 移除 `src/app.rs` 中的 Sidebar 和 `main-scroll`。
2. 新增 `src/shell/tab_bar.rs`，对应 `ui/src/components/layout/TabBar.tsx`：
   - 左侧 6 个页面按钮。
   - 右侧主题切换按钮。
   - 右侧 Settings 图标入口。
   - 激活、hover、focus、disabled 状态一致。
3. 重做 `title_bar.rs`，对应 `ui/src/components/layout/TitleBar.tsx`：
   - Logo 图片。
   - `Ahab Assistant` 标题。
   - 拖动区域。
   - 双击最大化。
   - 最小化、最大化/还原、关闭按钮。
4. Windows 下使用 GPUI 的 `WindowOptions { titlebar: None, .. }`（或等效的 `TitlebarOptions { appears_transparent: true, .. }`）隐藏系统标题栏，并验证拖动命中区域；不能直接照搬 Tauri 的 `decorations:false` 字段。
5. 根布局固定为 `TitleBar → TabBar → main`，页面自身负责滚动。
6. 设置主窗口和最小窗口尺寸。

### 通过条件

- GPUI 不再出现左侧 190px Sidebar。
- GPUI 与 React 的标题栏、TabBar 高度和页面起始位置一致。
- 不存在重复系统标题栏，且比较的是与 Tauri 相同的内容 client viewport。
- 900×680 和 800×560 下无额外横向滚动。

### P2：设计 Token 和通用控件对齐

### 目标

把 `ui/src/index.css`、Tailwind 和 shadcn 组件的计算样式转换为 GPUI 的统一样式层。

### 工作项

新增或重构：

```text
src/theme.rs
src/shell/toast.rs
src/components/style.rs
src/components/icon.rs
src/components/button.rs（或现有 components/mod.rs）
src/components/card.rs
src/components/badge.rs
src/components/switch.rs
src/components/select.rs
src/components/slider.rs
src/components/tabs.rs
src/components/dialog.rs
src/components/scroll_area.rs
src/components/text_input.rs
```

对齐以下 token：

- background、foreground
- card、popover
- primary、secondary、muted
- muted-foreground
- border、input、ring
- success、warning、danger
- brand、brand-hover、brand-light
- radius、spacing、字体大小

必须支持：

- 默认浅色主题
- 深色主题
- 跟随系统
- crimson、blue、amber、emerald、violet 五种强调色
- Button 的 default、outline、secondary、ghost、destructive、icon（必要时补齐 `link`）
- Switch、Select、Slider 的键盘和焦点状态
- Dialog 的遮罩、Esc、焦点和确认/取消
- 全局 Toast 层，位置和 `Toaster offset=80`、成功/信息/警告/错误/加载状态与 React 一致
- 6px 低存在感滚动条
- 默认禁止界面文本选择，日志等 `.selectable` 区域允许选择和复制
- disabled、loading、hover、active、focus-visible 状态
- `backdrop-blur`、阴影、过渡和扫光动画的能力验证及替代方案
- Lucide 图标不能用 Unicode/Emoji 替代；统一迁移 SVG path、viewBox、stroke width 和尺寸
- Palette 变化后根 Entity 立即重绘，system 模式监听系统主题变化
- `AppState.settings.themeMode/accentId/language` 继续作为持久化源；`Palette` 和当前语言只能是由它们派生的单一运行时视图，不能再维护第二套可写状态

颜色必须使用浏览器在目标主题下解析出的 sRGB 值转换为 GPUI token；不能用目测近似 `oklch`。

GPUI 不应继续使用当前固定的深色 `0x0f141c` 色板作为默认主题。

### P3：资源、字体和 i18n

### 目标

让页面实现可以直接复用 `ui` 的图片、文案和主题行为。

### 工作项

新增：

```text
src/assets.rs
src/i18n/mod.rs
src/i18n/zh_cn.rs
src/i18n/en_us.rs
```

统一管理：

- `ui/src/assets/limbus_title_banner.png`
- `ui/src/assets/logo.png`
- `ui/public/sinners/*.png`
- `ui/public/status_effects/*.png`
- `ui/src/content/help-zh.md`
- `ui/src/content/help-en.md`

资源解析器必须同时支持开发运行和 release 打包，页面不能直接拼接绝对路径。固定图片和 Markdown 优先在构建时嵌入；若 GPUI 图片 API 必须接收路径，则由构建步骤复制到明确的应用资源目录，并通过统一解析器访问。

迁移：

- `ui/src/i18n/locales/zh-CN.ts`
- `ui/src/i18n/locales/en-US.ts`
- 页面标题、按钮、提示、状态、错误信息和 Badge
- 参数化文本，例如 `{count}`、`{version}`、`{task}`

字体优先级对齐：

```text
Segoe UI
Roboto
Microsoft YaHei
sans-serif
```

禁止新增用户可见的固定中文文案；调试日志除外。

### P4：Home 视觉和行为迁移

对应：

```text
ui/src/pages/HomePage.tsx
ui/src/components/tasks/FixedTaskCard.tsx
ui/src/components/tasks/ExecutionToolbar.tsx
ui/src/components/connection/ConnectionPanel.tsx
ui/src/components/tasks/*.tsx
```

### 根布局

```text
Home
├─ 左侧任务区 flex: 1
│  ├─ 顶部任务标题和执行状态
│  ├─ 任务卡片 ScrollArea
│  └─ 底部 ExecutionToolbar
├─ 1px/4px 可拖动分隔条
└─ 右侧面板
   ├─ ConnectionPanel
   ├─ 实时画面卡片
   └─ 日志卡片
```

### 必须对齐

- 任务卡片的头部高度、Switch、图标和箭头。
- 折叠时的 Preview Badge。
- 展开时的背景和内容间距。
- 运行中左侧品牌色扫光条。
- 左侧任务区域独立滚动。
- 工具栏吸底，不被外层页面滚动带走。
- 右侧面板宽度默认 `280px`。
- 分隔条严格对应 React 的 `w-1`（4px），折叠状态为 `w-4`（16px），内部指示线为 2px。
- 拖拽、折叠、恢复宽度。
- 截图卡片的 16:9 占位区域。
- 日志卡片独立滚动、自动到底、300 条上限。
- Link Start、Stop、Pause、Resume 的图标、颜色和禁用条件。
- 结束后操作弹窗。

现有 `HomeState` 的配置、执行和日志逻辑可以保留，Render 层按 React 结构重写。

### P5：Teams 和完整编辑器

对应：

```text
ui/src/pages/TeamsPage.tsx
ui/src/components/teams/TeamEditModal.tsx
```

### 列表

- 顶部分类 Tabs 与数量 Badge。
- 右侧新建按钮。
- 大屏 `xl:grid-cols-2`，小屏单列。
- Card、Badge、头像、状态和按钮尺寸一致。
- 空列表和空分类状态一致。

### 编辑弹窗

对齐：

- `max-w-3xl` 在 Tailwind 默认值下对应最大宽度 `768px`
- 高度 `620px`
- 最大高度 `88vh`
- 小窗口下遵守 Dialog 默认的 `calc(100% - 2rem)` 最大宽度
- Header、复制 JSON、五个 Tabs、ScrollArea、Footer
- Basic、Shop、Combat、Starlight、Advanced 五个 Tab
- 12 名人格头像及顺序标记
- 10 种体系图标
- 队伍码、固定用途、星光、观察饰品、JSON 导入/导出
- Select、Switch、Input、Badge、错误提示和保存/取消行为

GPUI Overlay 必须提供与 Radix Dialog 等价的遮罩、Esc、焦点和确认行为。

### P6：其余页面逐页迁移

### Theme Packs

对应 `ui/src/pages/ThemePacksPage.tsx`：

- 顶部操作栏。
- 困难镜牢提示条。
- 主题包卡片列表。
- 开关、Tier、权重 Slider、排序和批量操作。
- 停用状态和同步反馈。

### Toolbox

对应 `ui/src/pages/ToolboxPage.tsx`：

- 1/2/3 列响应式卡片。
- 图标、状态 Badge、描述和操作按钮。
- 启动、停止、截图通知。
- Mock 后端提示。

### Resources

对应 `ui/src/pages/ResourcesPage.tsx`：

- 检查更新和立即同步操作栏。
- `max-w-3xl`、双列资源卡片。
- 本地版本、远端版本、同步时间。
- 同步 Badge、进度条和重复操作保护。

### Settings

对应 `ui/src/pages/SettingsPage.tsx`：

- `max-w-2xl` 居中卡片堆叠。
- 外观、热键、模拟器、系统行为、版本信息卡片。
- 五种强调色圆点。
- 主题、语言、热键捕获、输入框和 Select 状态。

### Help

对应 `ui/src/pages/HelpPage.tsx`：

- 左侧目录宽度 `208px`。
- 右侧正文 `max-w-2xl`。
- Markdown 标题、段落、列表、代码、链接、粗体。
- 目录跳转、语言切换和独立滚动。
- 不使用 WebView。

### P7：跨页面行为、IPC 和平台能力对齐

### 行为和 Mock

Home、Teams 和其余页面在各自迁移阶段就必须接入 Mock；本阶段负责统一复核所有跨页面行为和 IPC 边界：

- 页面进入和重复切换。
- 任务开关、展开和配置保存。
- 执行状态：idle → running → paused → running → idle。
- 设备扫描、连接、断开。
- 队伍增删改、筛选、JSON 导入导出。
- 主题包、工具箱、资源同步。
- Toast、错误和 loading 状态。
- 中文输入法、退格、选择、复制、粘贴。
- Select、Slider、Tabs、Dialog 的键盘行为。

Mock、WebSocket client 和页面必须共用同一份 canonical contract，不能为视觉迁移重新定义业务字段。

### 真实能力

在视觉回归通过后接入：

- WebSocket JSON-RPC。
- `screenshot.frame` JPEG 解码和纹理复用。
- 托盘及托盘菜单。
- 全局热键。
- Windows 窗口控制。
- Windows 电源操作。
- 最小化到托盘、开机自启、系统主题监听。
- 打开 GitHub/Mirror-Chyan 链接。

危险系统操作必须二次确认，平台线程事件必须安全投递到 GPUI 主线程。

### P8：视觉回归和性能验收

### 视觉回归

每个里程碑都在相同环境生成，且不写入 `ui/` 源码目录：

```text
gpui-demo/artifacts/visual/reference-ui/
gpui-demo/artifacts/visual/gpui/
```

至少对比：

- 4 种主题/语言组合。
- 900×680 和 800×560。
- Home 默认、展开、运行、暂停。
- Teams 列表、编辑、删除确认。
- 其他页面的 loading、empty、warning、success 状态。

使用截图叠加或像素差异检查；发现差异时先修正布局和 token，再修动画。
- 截图工具、DPI、client viewport 和主题模式必须记录在每份对比产物旁，保证差异可复现。
- 发布版确认所有页面由 GPUI 绘制，依赖和运行进程中不存在 WebView2/浏览器 UI 层。

### 功能和构建

以下命令从仓库根目录 `D:\Github\AhabAssistantLimbusCompany` 执行。每个里程碑至少执行（若参考 UI 有变更，先重新构建参考版本）：

```powershell
pnpm --dir ui build
cargo +nightly fmt --manifest-path gpui-demo/Cargo.toml -- --check
cargo +nightly test --manifest-path gpui-demo/Cargo.toml
cargo +nightly build --release --manifest-path gpui-demo/Cargo.toml
```

### 内存回归

发布版执行：

1. 冷启动等待 30 秒。
2. 访问全部 7 个页面。
3. 打开队伍编辑器全部 5 个 Tab。
4. 加载全部头像和状态图。
5. 执行、暂停、恢复、停止一次任务。
6. 产生 300 条日志并滚动帮助页。
7. 隐藏/恢复托盘。
8. 持续运行 30 分钟。

记录 Working Set、Private Bytes、Commit、GPU 内存和页面切换耗时。重点是 30 分钟内不能无界增长。

## 4. 里程碑和门禁

| 里程碑 | 交付 | 门禁 |
|---|---|---|
| V0 | 基准截图、尺寸表、资源和字体方案 | 参考状态完整，冲突已解决 |
| V1 | TitleBar、TabBar、根布局、主题 Palette | 无 Sidebar、无重复标题栏，Shell 对齐 |
| V2 | 通用控件和 i18n | 控件状态、浅色/深色/英文通过 |
| V3 | Home | 三栏结构、滚动、工具栏和任务卡片对齐 |
| V4 | Teams | 列表和编辑弹窗对齐，输入与 JSON 行为通过 |
| V5 | Theme Packs、Toolbox、Resources、Settings、Help | 7 个页面完成截图回归 |
| V6 | WebSocket、托盘、热键、Windows API | 平台能力不破坏视觉和主线程安全 |
| V7 | 发布验证 | 功能、视觉、内存、release 构建全部通过 |

## 5. 第一批实际任务

1. 将本文作为视觉迁移计划，并在旧计划中明确标记与当前 `ui` 冲突的 Sidebar 描述为过时。
2. 采集 `ui` 在 900×680 和 800×560 下的截图。
3. 重写 `src/app.rs` 根布局。
4. 新增 `src/shell/tab_bar.rs`，删除 Sidebar 的使用。
5. 重做 GPUI 无边框窗口和 TitleBar。
6. 将当前深色固定色板替换为与 `index.css` 对应的 Palette。
7. 建立统一图标、资源、字体和 i18n 层。
8. 按 React JSX 结构重写 Home。
9. 完成 Home 截图回归后，再迁移 Teams。
10. 逐页迁移其余页面，最后接入真实平台能力。
