# Ahab Assistant：React/Tauri → GPUI 功能迁移计划

> 目标：将 `D:\Github\AhabAssistantLimbusCompany\ui` 当前界面及其交互功能，完整迁移到同级的 `gpui-demo`，最终形成不依赖 WebView2 的 Windows 原生 GPUI 客户端。
>
> 本文只描述迁移计划，不改变现有 `ui`。迁移期间保留 Tauri 版本作为功能和视觉基准。
>
> **视觉 1:1 实施计划**：见 [`GPUI_1TO1_VISUAL_MIGRATION_PLAN.md`](./GPUI_1TO1_VISUAL_MIGRATION_PLAN.md)。该文档已审查并覆盖本文中与当前 `ui` 实际布局冲突的旧 Sidebar 描述；当前 `ui` 的顶部 TabBar 是唯一视觉真源。

## 实施进度（截至当前工作区）

- **已完成：GPUI 工程基线**：nightly toolchain、GitHub `main` commit `f66ed399`、锁定 `Cargo.lock`、Windows 原生窗口和发布配置。
- **已完成：M0 的模型/Mock/配置基础**：JSON-RPC 请求、响应、事件常量；任务、队伍、主题包、资源、设备、设置模型；版本化 `%APPDATA%` 配置的原子写入；可重复 Mock。
- **已完成：M1 的可复用控件骨架**：palette、Button、Badge、Card、Switch、Select/Slider 视觉控件、Tabs、Dialog、ScrollArea、EntityInputHandler 文本输入（含 UTF-16/IME/剪贴板基础）。
- **已完成：M3 队伍页面第一版**：列表筛选、数量、Mock 队伍/人格、创建/编辑/删除确认、5 个编辑 Tab、12 人格顺序、互斥选项、星光计算、JSON 导入/复制。
- **已完成：M4/M5 的页面骨架**：主题包、工具箱、资源中心、设置、帮助已接入导航；主题包批量/权重操作、工具状态、资源检查/同步、设置保存、热键捕获、原生 Markdown 子集和目录跳转均已有 Mock 行为。
- **进行中：M2/M5/M6**：真实 WebSocket 尚为 transport seam；i18n 目前保留模型和中文界面，英文界面及 palette 动态换肤待接入；Home 的完整任务表单、结束后动作、截图帧、右侧拖拽、托盘/窗口控制/Windows 电源与热键后端仍待完成。
- **验证结果**：`cargo +nightly test --manifest-path gpui-demo/Cargo.toml` 当前通过；尚未完成真实后端、托盘和 30 分钟内存回归。

## 1. 当前状态与边界

### 1.1 当前 GPUI Demo

`gpui-demo` 目前是一个独立的可行性 Demo，已经验证：

- GPUI GitHub `main` 依赖可以在 Windows 编译运行。
- 单窗口原生布局、导航、状态更新、日志、图片加载可行。
- 当前锁定的 Git commit：`f66ed399`，使用 nightly Rust。
- Demo 没有 WebView2。
- 已观察到发布版约 `53.7 MB Working Set / 106.3 MB Private Bytes`；该数值不是完整功能版本的承诺，只是原型基线。

### 1.2 迁移范围

必须迁移：

- 所有 7 个页面：主控台、队伍管理、主题包、工具箱、资源中心、帮助、设置。
- 标题栏、导航、主题、语言、持久化布局状态。
- 所有任务配置控件、队伍编辑器、弹窗、选择器、开关、滑块、滚动区域和通知。
- 现有 IPC 请求和事件语义。
- 托盘、窗口控制、全局热键、剪贴板、打开仓库链接等平台能力。
- 中英文帮助文档及目录跳转。
- 现有图片、状态效果图、人格头像和相关资源加载。

不在第一阶段改变：

- Python 自动化后端的业务逻辑。
- 后端协议的业务含义。
- 现有 `ui` 目录及其未提交改动。
- 任务自动化算法本身。

## 2. 迁移前必须先完成的工作

### M0：冻结数据和 IPC 契约

先把 `ui/src/services/ipc/types.ts` 作为协议基准，抽出与前端无关的文档或 Rust 类型定义，明确：

- JSON-RPC 请求/响应结构。
- 所有方法名、参数、返回值。
- 所有事件名和 payload。
- 错误码、错误提示和取消语义。
- 配置默认值及旧配置兼容策略。

当前 UI 与 Mock 中存在需要先统一的名称差异：

| 功能 | 当前页面调用 | 当前 Mock 中的名称 |
|---|---|---|
| 主题包列表 | `themePack.list` | 一致 |
| 主题包保存 | `themePack.updateAll` | 当前主要实现为 `themePack.save` |
| 主题包恢复 | `themePack.resetWeights` | 当前 Mock 未完整实现 |
| 资源列表 | `resource.status` | 当前 Mock 主要实现为 `resource.list` |
| 资源检查更新 | `resource.checkUpdate` | 当前 Mock 未完整实现 |
| 资源同步 | `resource.sync.start` | 当前 Mock 主要实现为 `resource.sync` |

迁移前建立一份 canonical contract，Mock 和未来 WebSocket client 都实现同一份契约，避免 GPUI 复制现有不一致行为。

### M0：建立 GPUI 版本和构建策略

- 继续使用 Zed GitHub `main`，而不是 crates.io 版本。
- `Cargo.lock` 必须提交，记录实际 commit。
- 保留 `rust-toolchain.toml`，因为当前 main 使用了 stable 尚未稳定的 API。
- 为 GPUI 单独设置 release 配置，不影响仓库其他 Rust 项目。
- 每次更新 GPUI main 都执行完整编译、视觉回归和内存回归。

## 3. 目标架构

建议将当前单文件 Demo 拆成以下结构：

```text
gpui-demo/
├─ Cargo.toml
├─ Cargo.lock
├─ rust-toolchain.toml
├─ README.md
├─ GPUI_MIGRATION_PLAN.md
└─ src/
   ├─ main.rs
   ├─ app.rs                    # Application、根 Entity、页面路由
   ├─ model/
   │  ├─ mod.rs
   │  ├─ tasks.rs               # 任务配置和默认值
   │  ├─ teams.rs               # 队伍配置和默认值
   │  ├─ resources.rs
   │  ├─ themes.rs
   │  └─ settings.rs
   ├─ ipc/
   │  ├─ mod.rs                 # RpcClient trait
   │  ├─ contract.rs            # 方法、事件、错误类型
   │  ├─ mock.rs                # 开发和视觉测试用 Mock
   │  └─ websocket.rs           # 真实 Python sidecar client
   ├─ state/
   │  ├─ app_state.rs           # 页面、主题、语言、布局
   │  ├─ task_state.rs
   │  ├─ team_state.rs
   │  └─ event_bus.rs
   ├─ shell/
   │  ├─ title_bar.rs
   │  ├─ tab_bar.rs
   │  ├─ toast.rs
   │  └─ layout.rs
   ├─ components/
   │  ├─ button.rs
   │  ├─ badge.rs
   │  ├─ card.rs
   │  ├─ switch.rs
   │  ├─ select.rs
   │  ├─ slider.rs
   │  ├─ tabs.rs
   │  ├─ dialog.rs
   │  ├─ scroll_area.rs
   │  ├─ text_input.rs
   │  ├─ number_stepper.rs
   │  └─ empty_state.rs
   ├─ pages/
   │  ├─ home.rs
   │  ├─ teams.rs
   │  ├─ team_editor.rs
   │  ├─ theme_packs.rs
   │  ├─ toolbox.rs
   │  ├─ resources.rs
   │  ├─ help.rs
   │  └─ settings.rs
   ├─ i18n/
   │  ├─ mod.rs
   │  ├─ zh_cn.rs
   │  └─ en_us.rs
   ├─ assets.rs
   └─ platform/
      ├─ mod.rs
      └─ windows.rs              # 托盘、热键、窗口和系统 API
```

### 3.1 状态管理

React hooks 和 Zustand 替换为 GPUI Entity：

- 根 Entity 持有当前页面、主题、语言和布局设置。
- 页面独立持有页面状态，避免所有页面状态常驻根对象。
- IPC 事件进入统一 event bus，再更新相关 Entity。
- 异步请求通过 GPUI executor 执行，不能阻塞 UI 线程。
- 页面关闭或组件销毁时取消事件订阅和未完成任务。
- 日志使用有界 `VecDeque`，最多保留 300 条。
- 截图只保留最新帧，不累积 JPEG buffer 或纹理。

### 3.2 持久化

替代浏览器 `localStorage`：

- 使用 `%APPDATA%` 下的应用配置文件。
- 持久化当前 Zustand `partialize` 中已有的字段：
  - `sidebarCollapsed`
  - `rightPanelWidth`
  - `rightPanelCollapsed`
  - `themeMode`
  - `accentId`
  - `language`
- 页面 `currentPage` 当前并未持久化，迁移时保持这一行为。
- 配置写入采用临时文件 + 原子替换。
- 增加版本号和迁移函数，避免结构变化导致配置丢失。

## 4. 页面与功能迁移顺序

### M1：Shell 和基础控件

先完成所有页面共同依赖的控件，再迁移具体页面。

#### Shell

- 单原生窗口，初始尺寸 `900×680`，最小尺寸 `800×560`。
- 标题栏：Logo、标题、拖动区域、双击最大化。
- 最小化、最大化/还原、关闭按钮。
- ~~左侧页面导航~~（**过时**；当前 `ui` 实际使用顶部 TabBar，详见 [`GPUI_1TO1_VISUAL_MIGRATION_PLAN.md`](./GPUI_1TO1_VISUAL_MIGRATION_PLAN.md)）。
- 设置入口位于顶部 TabBar 右侧。
- 主题循环：浅色 → 深色 → 跟随系统。
- Toast：成功、信息、警告、错误、加载状态。

> 旧建议（先使用原生标题栏）仅适用于功能 Spike；为满足当前 1:1 视觉目标，必须按 [`GPUI_1TO1_VISUAL_MIGRATION_PLAN.md`](./GPUI_1TO1_VISUAL_MIGRATION_PLAN.md) 直接实现无边框自绘标题栏。

#### 基础控件

实现并统一视觉行为：

- Button：default、outline、secondary、ghost、destructive、icon。
- Badge：default、secondary、outline、状态色。
- Card、CardHeader、CardContent、CardFooter。
- Switch：点击、键盘、禁用、焦点状态。
- Select：弹出列表、键盘上下选择、Esc 关闭、滚动。
- Slider：鼠标拖动、键盘调整、范围和步长。
- Tabs：页面 Tab 和编辑器 Tab。
- Dialog：遮罩、焦点、Esc、确认/取消、嵌套 JSON 导入面板。
- ScrollArea：垂直滚动、日志自动滚动、长表单滚动。
- Input/TextArea：中文输入法、剪贴板、选择、退格、光标和焦点。
- NumberStepper：最小值、最大值、加减按钮。
- EmptyState、Skeleton、Loading。

重点：GPUI 的文本输入需要使用 `EntityInputHandler`，不能只用展示文本模拟输入；必须实际测试中文 IME 和剪贴板。

### M2：IPC、i18n、主题和资源

#### IPC

实现：

- `RpcClient` trait。
- Mock client，保证没有 Python 后端时页面仍可操作。
- WebSocket JSON-RPC client，连接 Python sidecar。
- 请求序列号、超时、断线、重连、取消和错误提示。
- 事件订阅的生命周期管理。

保留现有方法和事件语义：

```text
请求：
app.ping
app.version
app.checkUpdate
tasks.getConfig / tasks.setConfig
execution.getState / start / stop / pause / resume
team.list / team.save / team.delete
sinner.list
themePack.list / updateAll / resetWeights
resource.status / checkUpdate / sync.start
tool.start / tool.stop / tool.screenshot
hotkey.get / hotkey.set
systemSettings.get / systemSettings.set
device.list / device.connect / device.disconnect

事件：
execution.status
execution.mirrorProgress
tool.status
device.status
log.entry
resource.sync.progress
screenshot.frame
app.notice
```

#### 国际化

完整迁移 `zh-CN.ts` 和 `en-US.ts` 的键结构：

- 页面和导航标题。
- 任务、队伍、主题包、资源、工具箱、设置、帮助文本。
- 带参数文本，例如 `{count}`、`{version}`、`{task}`。
- 语言切换后立即刷新当前窗口。
- 所有页面禁止直接写用户可见的固定文案，调试文案除外。

#### 主题

将 CSS 变量主题改为 GPUI palette：

- background、foreground、card、popover、muted、border、input、ring。
- success、warning、danger。
- crimson、blue、amber、emerald、violet 五种强调色。
- 浅色、深色、系统模式。
- Windows 系统主题变化监听。
- 所有页面和弹窗使用同一套 palette，不在组件内复制颜色。

#### 资源

- 使用 GPUI 图片元素读取 `ui/public` 中的图片。
- 打包时将图片、帮助 Markdown 和必要字体复制/嵌入到 GPUI 资源目录。
- 人格头像按需加载，并限制纹理缓存。
- 状态效果图统一建立 ID → 图片的映射。
- 版本号替代 Vite 的 `__APP_VERSION__`，从 Cargo/package metadata 注入。

### M3：主控台 HomePage

按“先读状态，再编辑，再执行”的顺序实现。

#### 页面布局

- 左侧：任务卡片滚动流 + 底部执行工具栏。
- 中间：可拖动分隔条。
- 右侧：设备连接、实时画面、运行日志。
- 右侧面板宽度默认 `280px`，支持拖动调整。
- 向右拖到底折叠右侧面板，恢复后保留宽度。

#### 任务卡片

每张卡片支持：

- 启用/停用开关。
- 展开/折叠。
- 折叠时显示配置预览 Badge。
- 执行中的品牌色边线和状态提示。
- 任务执行时禁用不允许修改的控件。

六类任务必须完整迁移：

1. **窗口设置**
   - 分辨率：720、1080、1440。
   - 窗口位置：居中、靠左、靠右、保持原位。
   - 结束后恢复窗口。
   - 截图间隔：0.2、0.5、1.0 秒。
   - 鼠标操作间隔：0.1、0.3、0.5 秒。
   - 异步 PostMessage 输入。

2. **日常任务**
   - 经验本次数 0~99。
   - 纽本次数 0~99。
   - 默认日常队伍。
   - 连续作战和连续次数 1~10。
   - 经验本按周属性选择 4 组队伍。
   - 纽本按星期属性选择 7 组队伍。

3. **领取奖励**
   - 全部。
   - 狂气与通行证。
   - 仅邮件。

4. **狂气换体**
   - 换体次数 0~10。
   - 葛朗台模式。
   - 跳过模块合成。

5. **镜牢**
   - 次数 1~99。
   - 无限模式。
   - 困难镜牢。
   - 运行中显示当前进度。
   - 高级设置中的 10 个开关。

6. **亚哈共鸣**
   - 启用/停用。
   - 配置预览状态。

#### 执行工具栏

- 全选和清空，执行中禁用。
- 结束后操作配置入口。
- Link Start!
- Stop!
- Pause / Resume。
- F10 快捷键提示。
- 空任务启动时显示警告。

#### 结束后操作弹窗

- 退出游戏。
- 退出模拟器。
- 退出 AALC。
- 睡眠、休眠、锁屏、关机或无动作。
- 仅本次生效。
- 保存为默认配置。
- 保留现有 `keepAfterCompletion` 语义。

#### 右侧面板

**设备连接：**

- 设备列表。
- 重新扫描。
- 连接、断开。
- disconnected / connecting / connected 三种状态。

**实时画面：**

- 目前 React 页面只是等待画面占位符。
- GPUI 迁移时实现 `screenshot.frame` 后，保留最新 JPEG 帧。
- 将 JPEG 解码为可复用纹理，禁止每帧创建不可释放的纹理。
- 截图帧尺寸和刷新频率受配置限制。

**运行日志：**

- 订阅 `log.entry`。
- 最多 300 条。
- 按 debug/info/warn/error 显示颜色。
- 自动滚动到底部。
- 支持文本选择和复制。
- 清空日志。

### M4：队伍管理与完整编辑器

#### 队伍列表

- 按全部、镜牢、经验本、通用筛选。
- 显示分类数量。
- 新建队伍。
- 编辑队伍。
- 删除确认弹窗。
- 空列表和空分类提示。
- 显示队伍名称、用途、主体系、人数、人格顺序。
- 显示星光已配、第二体系、良秀单通、编队码、舍弃体系等 Badge。
- 显示启用/停用状态。

#### 编辑器基础编成 Tab

- 队伍名称。
- 队伍用途。
- 10 种主体系。
- 12 名人格选择上限。
- 按点击顺序分配 #1~#12。
- 清空人格。
- 编队码启用、输入和粘贴。
- 固定队伍用途及难度范围。
- 队伍启用开关。

#### 商店与合成 Tab

- 商店策略：默认、保守、激进。
- 10 种舍弃饰品体系。
- 不治疗、不购买、不合成、不出售、不升级。
- 激进合成、公式合成、保留体系饰品。
- 四级饰品后的行为。
- 定向刷新和普通刷新上限。
- 忽略第 1~5 层商店。

#### 二体系与战斗 Tab

- 第二体系启用。
- 第二体系选择和起始楼层。
- 第二体系购买、合成、选奖励、升级。
- 避免三技能 / 优先三技能互斥。
- 每楼层重新编队。
- 首回合防御 / 良秀单通互斥。
- 防御回合数 1~5。
- 技能替换及替换模式。

#### 开局星光 Tab

- 星光换钱开关。
- 10 个星光项目。
- 每项 0、1、2+、3++ 四级。
- 一键全部设置。
- 实时计算总消耗。
- 中英文名称和说明。

#### 观测与高级 Tab

- 观测 E.G.O 饰品开关。
- 输入、回车添加、删除已选饰品。
- 队伍专属主题包权重。
- 复制 JSON 配置到剪贴板。
- 粘贴 JSON 并校验、合并默认字段、覆盖当前配置。
- 无效 JSON 错误提示。
- 保存、取消和名称校验。

### M5：主题包、工具箱、资源中心

#### 主题包

- 困难镜牢周期提示条。
- 已启用主题包总权重。
- 按权重排序。
- 全部启用、全部停用。
- 恢复默认权重。
- 每个主题包的启用开关、名称、Tier、0~10 权重滑块。
- 停用时的视觉状态。

#### 工具箱

- 自动战斗：启动、停止、状态事件。
- 体力换饼：启动、停止、状态事件。
- 辅助截图：执行后通知成功。
- 工具状态 Badge。
- 后端未接入时显示 Mock 提示。

#### 资源中心

- 模板资源和 ONNX 模型列表。
- 本地版本、远端版本、上次同步时间。
- 检查更新。
- 立即同步。
- 0~100% 进度。
- 同步完成后刷新列表并显示通知。
- 同步中禁用重复启动。
- 从未同步和已是最新状态。

### M6：设置页面和 Windows 平台能力

#### 外观

- 浅色、深色、跟随系统。
- 五种强调色。
- 简体中文 / English。
- 修改后立即刷新整个界面。
- 持久化到应用配置文件。

#### 全局热键

- 启用全局热键。
- 设置启动/停止热键。
- 设置暂停/继续热键。
- 捕获组合键并过滤单独修饰键。
- 清除热键。
- 后端保存后立即更新本地状态。

Windows 实现优先使用 `RegisterHotKey` 或经过验证的 `global-hotkey` crate，并将消息转发到 GPUI 主线程。

#### 模拟器与系统设置

- 模拟器模式。
- MuMu / 其他模拟器。
- ADB 端口。
- 启动超时。
- 内存保护。
- 最小化到托盘。
- 开机自动启动。
- 运行时阻止休眠。
- HDR 检测提示。

平台替代实现：

- 阻止休眠：Windows `SetThreadExecutionState`。
- 电源动作：Windows shutdown / lock / sleep API，执行前必须确认危险操作。
- 开机自启：Startup 文件夹或注册表，并提供卸载清理。
- HDR 检测：DXGI/显示器 API；失败时只显示未知，不阻断任务。

#### 更新与关于

- 预览版渠道开关。
- GitHub / Mirror-Chyan 更新源。
- Mirror-Chyan CDK。
- 检查更新及 Toast。
- 版本号。
- 打开 GitHub 仓库链接。

#### 托盘和窗口

将当前 Tauri 实现替换为原生平台模块：

- 托盘图标和菜单。
- 打开主窗口。
- 托盘开始任务 / 停止任务。
- 退出程序。
- `minimize_to_tray` 开关。
- 关闭窗口时隐藏到托盘或真正退出。
- GPUI 窗口最小化、最大化、还原、关闭。

托盘事件必须在 GPUI 主线程安全地投递给根 Entity，不能直接从托盘线程修改 UI 状态。

### M7：帮助页面

- 读取 `help-zh.md` 和 `help-en.md`。
- 解析一级标题、二级标题和三级标题。
- 根据二级标题生成目录。
- 点击目录滚动到对应标题。
- 支持段落、列表、有序列表、代码、粗体、链接。
- 支持长文档滚动。
- 切换语言后重新解析目录和正文。
- 不引入 WebView；使用 Markdown parser + GPUI 原生元素渲染。
- 对未知 Markdown 语法采用可读的纯文本降级。

## 5. 视觉迁移策略

### 5.1 先保持结构，再补细节

迁移顺序：

1. 页面尺寸和 Flex 布局。
2. 颜色、边框、圆角、间距、字号。
3. 交互状态：hover、active、focus、disabled、loading。
4. 弹窗和滚动行为。
5. 动画和过渡。
6. 无障碍语义和键盘操作。

GPUI 不支持直接使用 Tailwind class，因此将 CSS 视觉规范整理成 Rust style helper，避免每个页面手写一套颜色和间距。

### 5.2 性能原则

- 页面切换时只保留必要的页面 Entity。
- 大型队伍编辑器按 Tab 创建或释放详细内容。
- 日志使用有界列表和可见区域渲染。
- 人格头像使用缩略图，并按需加载。
- 截图纹理复用，限制刷新频率。
- 避免长期存在的透明叠加、模糊和大范围阴影。
- 不在渲染函数中执行文件、网络或耗时计算。
- 统一处理 GPU 纹理和字体图集缓存。

## 6. 测试计划

### 6.1 功能测试

- 每个页面可进入、返回和重复切换。
- 每个开关、选择器、滑块、输入框都能修改并显示新值。
- 任务执行状态可正确经历 idle → running → paused → running → idle。
- 镜牢进度可更新并在任务结束后清理。
- 日志 300 条上限和清空功能正确。
- 队伍新增、编辑、删除、筛选和 JSON 导入导出正确。
- 所有队伍编辑器互斥选项行为正确。
- 主题包权重计算、排序和批量操作正确。
- 资源同步进度、完成刷新和重复点击保护正确。
- 工具启动/停止/截图通知正确。
- 设置修改后重启仍然保留。
- 中英文切换不出现缺失键或布局溢出。
- 托盘开始/停止/打开/退出正确。
- 全局热键在窗口失焦时仍然生效。

### 6.2 输入和窗口测试

- 中文输入法、英文输入法、退格、选中、复制、粘贴。
- Tab、Shift-Tab、Enter、Esc、方向键。
- 窗口最小化、最大化、还原、拖动、关闭。
- 关闭到托盘后恢复窗口。
- 低 DPI、高 DPI、窗口缩放和最小尺寸。
- 低端或无独显环境下启动和渲染。

### 6.3 内存和性能测试

使用发布版，和当前 Tauri 版本执行相同步骤：

1. 冷启动后等待 30 秒。
2. 空闲 5 分钟。
3. 访问全部 7 个页面。
4. 打开队伍编辑器的全部 5 个 Tab。
5. 加载全部头像和状态图。
6. 启动、暂停、恢复、停止一次任务。
7. 产生至少 300 条日志。
8. 打开帮助长文档并滚动到底部。
9. 隐藏到托盘，再恢复窗口。
10. 持续运行 30 分钟。

记录：

- Working Set。
- Private Bytes。
- Commit Size。
- GPU Dedicated Memory。
- GPU Shared Memory。
- CPU 占用和帧率。
- 页面切换耗时。

验收重点不是单个瞬时数字，而是 30 分钟内 Private Bytes 和 GPU 内存不能无界增长。建议把当前 GPUI Demo 作为基础线，再为完整版本设定“增量上限”，而不是直接假设固定内存值。

## 7. 风险与应对

| 风险 | 影响 | 应对 |
|---|---|---|
| GPUI GitHub main 为 pre-1.0 | API 可能变化 | 锁定 Cargo commit，集中封装 GPUI API |
| GPUI Windows DirectX 字体图集内存增长 | 内存可能异常升高 | 监测字体上传次数，必要时提交/应用批量上传修复 |
| GPUI 原生输入控件不如浏览器成熟 | 中文输入和焦点问题 | 先独立完成 TextInput 原型，再迁移大型表单 |
| 托盘与 GPUI 事件循环集成 | 可能卡死或线程不安全 | 先做独立托盘 spike，统一通过主线程消息投递 |
| Markdown 原生渲染工作量较大 | 帮助页延期 | 先支持当前文档使用到的 Markdown 子集 |
| Windows 电源操作具有破坏性 | 用户数据/系统风险 | 二次确认、明确文案、默认无动作 |
| WebSocket 后端尚未完整存在 | 无法验证真实任务 | Mock 与真实 client 使用同一 contract |
| 图片和显存缓存滞留 | GPU 内存增长 | 统一 ImageCache、限制纹理数量和截图帧生命周期 |

## 8. 推荐里程碑和验收门槛

### M0：协议和基础设施

完成 contract、配置持久化、i18n、主题、基础控件骨架和 Mock。

### M1：Shell + Home 基础版

完成标题栏、导航、主控台任务卡片、日志和设备连接。

### M2：Home 完整版

完成所有任务配置、执行工具栏、进度、截图帧、结束后动作和右侧面板调整。

### M3：Teams 完整版

完成列表、筛选、删除确认和 5-Tab 队伍编辑器。

### M4：其他页面

完成主题包、工具箱、资源中心和帮助页。

### M5：Settings + Windows 平台

完成设置、热键、托盘、系统设置、更新和窗口行为。

### M6：双版本回归和替换评估

只有满足以下条件，才考虑用 GPUI 版本替换 Tauri 版本：

- 所有当前页面和交互通过功能清单。
- Mock 和真实 IPC 均可运行。
- 中文输入、剪贴板和托盘稳定。
- 30 分钟内没有持续内存增长。
- GPUI 的 Working Set、Private Bytes 和 GPU 内存达到预设目标。
- 现有 `ui` 仍可作为回退版本保留至少一个发布周期。

## 9. 第一批实际开发任务

后续开始实施时，建议按以下顺序落地：

1. 把 `gpui-demo/src/main.rs` 拆成 `app`、`shell`、`components`、`pages` 四层。
2. 将 `types.ts` 的模型转换为 Rust model，并补默认值/序列化测试。
3. 实现 `RpcClient` trait、canonical contract 和可重复的 Mock。
4. 先实现 GPUI `TextInput`、`Select`、`Switch`、`Dialog`、`ScrollArea`。
5. 将当前 Demo 改成真实的 Home 页面骨架。
6. 迁移 Home 的任务配置和执行状态。
7. 再迁移最大、最复杂的 TeamEditModal。
8. 迁移其他页面和 Settings。
9. 最后接入托盘、全局热键、真实 WebSocket 和打包。
10. 每个里程碑都执行功能、视觉、内存三类回归测试。
