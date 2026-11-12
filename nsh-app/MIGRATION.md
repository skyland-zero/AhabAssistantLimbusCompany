# 迁移台账：GPUI → Native Shell

`nsh-app/` 用 Native Shell 重写原 GPUI 客户端，**Rust 做后端、TSX 做前端**。原 `gpui-app/` 在 P5 删除，因此这里记录的每一个来源文件都要么被搬入 `nsh-app/native/`，要么被 TSX 重写，**不允许运行时引用**。

状态标记：`[ ]` 未开始 · `[~]` 进行中 · `[x]` 完成

## 0. 阶段总览

| 阶段 | 内容 | 状态 | 完成判据 |
| --- | --- | --- | --- |
| P0 | 骨架：`nsh.json`、窗口壳、主题 token、`assets/`、Rust 导出通道 | [x] | `nsh run nsh-app --target gpui` 开窗，侧栏与主题正常 |
| P1 | Rust 后端平移：`model` → `ipc` → `state` → `i18n` → `app` | [~] | `model` + `ipc` 已迁入，`cargo test` 49 条通过；Mock 与真实 sidecar 双通；`state` / `i18n` / `app` 待迁 |
| P2 | TSX 设计系统：`components/` + `shell/` | [~] | 组件对照页与 `gpui-app` 截图逐个一致 |
| P3 | 页面重写：home → teams → settings → theme_packs → help | [~] | 7 页全部有实现、截图对照与 `nsh test` 用例；队伍编辑器的镜牢配置已完成，剩余项见 §1.2（依赖 P1 或留给下一轮） |
| P4 | 打包切换：`nsh package`，替换发布流程里的 GPUI 可执行文件 | [~] | Windows 安装包已产出并实机启动；DPI/单实例已验证，托盘/全局热键待 P1 接入后验证 |
| P4.5 | 仓库级引用切换：9 处脚本/CI/文档改指向 `nsh-app/` | [ ] | `python3 scripts/check_no_gpui_app_refs.py` 通过（对照图已归档，见 `nsh-app/docs/reference/gpui/`） |
| P5 | `git rm -r gpui-app`，更新根 `AGENTS.md` | [ ] | 完整构建 + 冒烟通过，检查脚本无白名单 |

## 0.1 进度快照（当前分支）

| 项 | 状态 |
| --- | --- |
| 页面 | **7 / 7 全部实现**：主控台、队伍管理（含 5 段镜牢配置编辑器）、镜牢主题包、工具箱、资源中心、帮助、设置（另加开发期组件对照页） |
| 设计系统 | 5 套皮肤 × 6 个强调色 × 4 档圆角、6 个按钮变体 × 4 尺寸、13 个 ui 层组件 |
| 窗口壳 | 品牌 + 6 导航 + 状态点 + 外观切换 + 设置 + 窗口按钮；皮肤背景氛围层；标题栏固定 40pt（2026-11-12 对齐客户端的 `TITLEBAR_HEIGHT`） |
| Rust 后端 | **model + ipc 已迁入**：schema 3 契约、确定性 Mock、真实 sidecar 握手/请求/事件队列/生命周期；`cargo test` 49 条（含真实 sidecar 集成用例） |
| 测试 | `nsh test` **98 / 98**（49 条 Rust 单测 + 49 个 TSX 用例：壳、后端 seam、设置、工具箱、资源中心、队伍、帮助页、帮助解析、主题合成、主控台、Windows 截图基线） |
| 截图 | `docs/shots/` 8 张 Linux 渲染基线 + 5 套皮肤；`docs/reference/gpui/` 14 张对照基准；Windows 实机对照见 §1.6 |
| 打包 | `nsh package --target windows` 产出 `AALC.exe`（62 MB）、`AALC.zip`（26 MB）、`AALC-setup.exe`（20 MB），单实例与 175% DPI 实机通过 |
| 性能 | 打包版 NSH 首窗 **123.3 ms**（中位）对 GPUI **151.4 ms**（中位）；工作集 61.9 MB 对 56.6 MB；4 秒后 CPU 133 ms 对 197 ms（10 次采样，见 §1.6） |
| 未完成 | P1（`state` / `i18n` / `app`）、P4 托盘/全局热键实机验证、P4.5（仓库级引用）、P5（删除 `gpui-app/`）；P3 剩余项见 §1.5 |

## 1. P0 已落地

- `nsh.json`：`rust: "native"`、隐藏标题栏（`window.titleBar.style = "hidden"`，820×680 / 最小 800×560，与 `main.rs` 的 `Bounds::centered` 一致）、`singleInstance`、`capabilities.network.hosts = 127.0.0.1 + localhost`（sidecar WebSocket）。
- `theme.json` + `theme/{glass,archive,mist,limbus}.json`：由 `scripts/export_nsh_theme.py` 从原 `palette/skins.rs`、`tokens.rs` 机械导出，**5 皮肤 × 2 方案 × 34 个颜色 token + 4 档圆角**，值逐字节一致。
- `native/`：`aalc-native` crate（空 `[workspace]`，独立构建），`#[native_module("aalc/native")]` 注册 `Runtime` 模块（`appVersion`、`platformOs`）。
- `App.tsx`：窗口壳 + 占位页面宿主 + Rust 版本回显；`shell/TitleBar.tsx` 自绘标题栏（`windowDrag`）。P0 当时把导航做成了左侧栏，**与 GPUI 客户端的真实结构不符**（真实结构是标题栏内的 6 个导航项 + 右侧设置/外观/窗口按钮，没有侧栏），已在 P2 第一轮纠正。
- `App.test.tsx`：`nsh test` 冒烟用例（骨架渲染、侧栏切页、Rust 模块可调用）。
- `assets/`：原 `gpui-app/resources/assets/` 全量复制（logo、横幅、12 立绘、11 状态图、limbus 主题、2 篇帮助文档）+ `app-icon.png`。
- **截图链路已打通**：`scripts/capture_nsh.py` 在 Xvfb + Mesa lavapipe（软件 Vulkan）里跑 `nsh run --target gpui`，抓 X11 根窗口，产出 `nsh-app/docs/shots/*.png`（统一 820×680，与旧客户端对照图同视口）。无 GPU 机器也能出真实渲染图。

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
  `nsh test` 全绿（Rust 单测随 P1 增长，当前 49 条）。

### P3 第一页：设置

- **`pages/settings/`**：快速导航（200pt）+ 卡片栈（7 段：外观、全局热键、系统与防护、实验性功能、
  更新与源配置、任务通知、关于），卡片头走当前皮肤的装饰语言。导航高亮跟随滚动（每张卡片
  `onLayout` 上报偏移，`onScroll` 取最后一个已开始的段），点击跳段。
- **新增 ui 层组件**：`Field`（设置行网格）、`Segmented`、`Switch`、`Input`、`Select`（统一浮层表面）、
  `Swatch`；`Button` 补上 GPUI 就有的 `secondary` 变体。
- **`settings.ts`**：设置页编辑的值（系统开关、更新源、CDK、SPT、语言、热键），默认值取自
  `model/settings.rs` 与 `SystemSettingsConfig::default()`；P1 换成 RPC 与后端初值，页面不改。
- **皮肤选择器**：每张卡片用该皮肤自己的 token 画 78pt 缩略图（含标题条、两个面板、底部两颗
  accent 芯片），不截屏、不依赖当前主题。
- **对照**：与 `docs/reference/gpui/settings-{light,dark}.png` 逐项比对后修正了 4 处——导航居中
  （原实现左对齐）、分段选择去掉容器边框（只有选中的一段是胶囊）、开关选中色改为 `primary`
  （不是 `brand`）、皮肤缩略图补上外框与底部芯片。
- **仍是占位**：热键捕获（需 P1 的 `GlobalShortcut` 与窗口按键流）、检查更新 / 保存 CDK / SPT 保存与
  测试推送（需 sidecar），这几处按下去只写一行反馈，文案已写明「随 P1 接入」。
- 语言切换只改状态：英文文案随 P1 的 i18n 模块。

### P3 第二页：工具箱 + 资源中心

- **`pages/toolbox/`**：2 列卡片网格 × 4 个工具（自动战斗、体力换饼、辅助截图、分辨率修改），
  每张卡是「图标 + 状态徽标 / 标题 + 说明 / 动作」；分辨率卡给两个按钮，截图是一次性动作，
  其余两个是运行/停止切换；底部一行 sidecar 说明 + 反馈行。
- **`pages/resources/`**：工具栏卡片（左：同步进度徽标；右：检查更新 / 立即同步）+ 资源卡网格
  （最小 380pt，`flex-wrap`），每张卡有名称 + 状态徽标、本地/远端版本、上次同步，同步中显示进度条。
- **`toolbox.ts` / `resources.ts`**：工具运行状态与反馈、资源分组与同步进度；默认值与反馈文案取自
  `state/other/toolbox.rs` 与 mock 资源数据，P1 换成 `Toolbox.*` / `Resources.*` RPC 与事件。同步的
  分步进度是唯一保留的定时器，随 mock 一起删除。
- **`Button` 的 `default` 变体改成 accent 实底**：对照参考图发现旧客户端的 `Default` 按钮就是
  品牌色按钮（`运行` / `立即同步` / `修改 1080P` 都是红的），之前做成了中性黑。
- 新增图标 `crosshair` `pill` `camera` `play` `stop` `rotateCcw` `searchCheck` `refresh`。

### P3 第三页：帮助 + 队伍（列表）

- **`pages/help/`**：208pt 目录卡（语言切换 + 二级标题列表）+ 672pt 文档卡；文档从
  `assets/content/help-{zh,en}.md?url` 读取，`parser.ts` 是本客户端 Markdown 子集（标题/段落/有序无序
  列表/代码块 + 行内加粗、代码、链接）的移植，纯函数、有单测。目录跟随滚动（按块 `onLayout` 上报偏移），
  点击跳段；文档只有一级标题被隐藏，与旧客户端一致。
- **`pages/teams/`**：筛选条（全部/镜牢/经验本/通用，带计数）+ 「新建队伍」，2 列编队槽网格
  （14 个槽，空槽也给两种补位方式）；队伍卡有编号、启用开关、从预设覆盖、编辑、删除、名称、用途/体系/
  人数/星光/舍弃/停用徽标与成员芯片。
- **`pages/teams/TeamEditor.tsx`**：弹层里的「基础」段（名称、用途、饰品体系、启用、人格多选）+
  保存/取消，新建队伍占用第一个空槽。**旧客户端的战斗 / 商店 / 策略 / 进阶 / 星光 / 统计六段
  （`mirrorConfig`）还没迁**，它们属于镜牢任务配置，和主控台的任务卡一起做。
- **已知差异**：饰品体系前的 emoji 在桌面字体栈里渲染成方框，已改成纯文字；帮助页各段之间的
  间距比参考图紧约 5pt。

### P3 第四页：镜牢主题包 + 主控台

- **`pages/theme_packs/`**：工具栏（总权重 + 按权重排序 / 全部启用 / 全部停用 / 恢复默认权重）、
  困难镜牢周期提示条、每包一行（开关、名称、T 阶徽标、0–10 权重滑块、数值）。总权重只算启用的包，
  与旧客户端的 28 基线一致；停用的行 `opacity-60` 且滑块禁用。滑块用 `nsh/base` 的 `Slider`
  （键盘、拖动、屏幕阅读器都自带），不再手写。
- **`pages/home/`**：三区控制台——顶部「Python 后端」+「周期统计（今日/本周）/上次镜牢」两张卡，
  中间「窗口设置」（SET）与 5 张任务卡（日常任务 / 领取奖励 / 狂气换体 / 自动镜牢 / 亚哈共鸣，
  每张带开关与紧凑参数选择器），底部操作条（全选 / 清空 / 什么也不干（本次）/ Link Start! F10）；
  右栏是固定的「设备连接 / 实时画面 / 运行日志」。
- **`home.ts` / `themepacks.ts`**：任务、参数、设备、日志计数与主题包权重的状态；默认值取自
  `ThemePacksState::default()`（总权重 28）与 mock 任务列表，P1 换成 `Execution.*` /
  `ThemePacks.*` / `Device.*` 的 RPC 与事件。
- **页内仍是占位的部分**（都写明「随 P1 接入」）：Link Start 启动执行、设备连接与实时画面
  （需要预览帧流）、运行日志内容（需要日志事件）、窗口设置的两个下拉、周期统计的真实数字。
- `pages/Placeholder.tsx` 已删除：7 个页面都已是真实实现。

### P3 第五页：队伍编辑器的镜牢配置（2026-11-12）

- **五个页签**与旧客户端一致：基础编成、商店与合成、二体系与战斗、开局星光、观测与高级；
  经验本队伍只有基础编成。页签栏是标题栏下的独立条，选中项为 `card` 底 + `shadow-sm`。
- **基础编成**：名称、用途、10 个饰品体系的图标网格、12 人格头像与点击序号、编队码、
  固定队伍用途、启用；经验本只保留编队码与复制。
- **商店与合成**：奖励卡优先度、自定义购物策略、自选开局饰品（体系 + 顺序）、商店策略、
  10 个舍弃体系、5 个操作限制、5 个合成策略、四级后策略、关键词/普通刷新上限、5 层忽略商店。
- **二体系与战斗**：第二体系 + 起始楼层 + 四级/购买/奖励/升级四个开关、战斗与技能偏好、
  防御策略、技能替换（目标 + 模式）。
- **开局星光**：使用开关、一键设置四档、10 个等级卡；每级按 0–3 倍乘成本，页签与卡片实时显示
  当前总消耗（fixture 为 80 点）。
- **观测与高级**：开局礼物搜索（快速选择蜘蛛巢、坐标输入、最多 3 个标签）、队伍专属主题包权重、
  JSON 导入导出（剪贴板能力已加进 `nsh.json`）、编队统计（刷新/清除 + 确认框）。
- **状态**：`teams.ts` 扩展 `TeamMirrorConfig`（字段名与契约一致）与草稿操作；`teams-stats.ts`
  提供 mock 统计。P1 只需把读写换成 `Teams.*` / `TEAM_STATS_*`，页面不改。
- **验证**：`teams.test.tsx` 9 条（列表、筛选、保存、新队伍、五页签、商店/星光交互、剪贴板、
  统计刷新与清除）；Windows 实机对照见 §1.6。

## 1.5 P3 剩余项（逐条，均落在下一轮或 P1）

1. **队伍编辑器的镜牢配置**：战斗 / 商店 / 策略 / 进阶 / 星光 / 统计六段（`mirrorConfig`，旧客户端
   `pages/teams/editors/` 约 1.9k 行）。**【2026-11-12 已完成】** 五个页签（基础编成 / 商店与合成 /
   二体系与战斗 / 开局星光 / 观测与高级）加上统计数据与 JSON 导入导出，全部走 `TeamMirrorConfig`
   的契约字段；保存前只改草稿。P1 接入 `Teams.*` 后只需替换 `teams.ts` 的读写。
2. **主控台的动态部分**：Link Start 启动执行、设备连接、实时画面帧、运行日志内容、周期统计数字、
   窗口设置的两个下拉。全部要 P1 的 sidecar 与事件流；当前按下去只写一行「随 P1 接入」的反馈。
3. **设置页的待接动作**：热键捕获（`GlobalShortcut` + 窗口按键流）、检查更新（updater）、
   保存 CDK / 保存 SPT / 测试推送（sidecar）、英文文案（i18n 模块）。
4. **主题包 / 资源中心的真实数据**：现在读的是 mock（总数 28 与旧客户端一致），P1 换成
   `ThemePacks.*` / `Resources.*` 的返回值。

这一轮**没有**做的（仍在 P2/P3 范围）：

- 语言切换/`i18n`：标题栏文案暂时写死中文，等 P1 的 `i18n` 模块。
- 标题栏右侧窗口按钮、托盘、全局热键的 Windows 实机验证。
- 字体族（Linux 上 `font-semibold` 回退）仍待定。

## 1.6 Windows 实机验证（2026-11-12）

环境：Windows，175% 缩放（820×680 逻辑窗口 = 1435×1190 物理像素），`nsh` 0.5.0，
NSH 侧用 `nsh package --target windows` 的产物，GPUI 侧用 `gpui-app/target/release/ahab-gpui-app.exe`，
两边都是 mock 后端、浅色主题、默认皮肤/强调色、中文。

### 打包

```powershell
nsh package nsh-app --target windows -o artifacts/nsh/package
```

产出：

| 产物 | 大小 | 说明 |
| --- | --- | --- |
| `artifacts/nsh/package/AALC/AALC.exe` | 62.1 MB | release runner，随包加载 `nsh/bundle.nshc` 与 `nsh/assets/` |
| `AALC.zip` | 26.1 MB | 绿色包 |
| `AALC-setup.exe` | 19.8 MB | Inno Setup 安装包 |

实机启动、标题栏、页面导航、关闭均正常；`singleInstance` 生效：第二次启动在 2 秒内退出，
进程数与窗口数都保持 1。

### 截图对照（mock）

`nsh-app/visual.test.tsx` 在真实 Windows 宿主里逐页/逐页签截图（`nsh test nsh-app/visual.test.tsx --target windows`），
每张 820×680；GPUI 对照图同样在 820×680 下重拍（`--state teams-editor` 等）。
按整图 RGB 均值差（MAE，0–255）与差异像素占比（通道差 > 24）统计：

| 截图 | MAE | 差异像素 | 截图 | MAE | 差异像素 |
| --- | ---: | ---: | --- | ---: | ---: |
| 主控台 浅/深 | 12.7 / 14.0 | 11.7% / 15.5% | 队伍 浅/深 | 13.2 / 13.2 | 11.5% / 15.2% |
| 主题包 浅/深 | 17.4 / 15.8 | 13.4% / 11.2% | 工具箱 浅/深 | 11.4 / 9.7 | 9.5% / 8.8% |
| 资源中心 浅/深 | 4.2 / 3.3 | 4.0% / 3.6% | 帮助 浅/深 | 10.8 / 10.9 | 8.1% / 8.2% |
| 设置 浅/深 | 11.4 / 11.1 | 11.0% / 12.6% | 编辑器基础 浅/深 | 19.9 / 13.4 | 17.7% / 17.9% |
| 编辑器商店 浅/深 | 10.2 / 10.2 | 11.2% / 10.6% | 编辑器战斗 浅/深 | 11.4 / 13.1 | 9.6% / 11.7% |
| 编辑器星光 浅/深 | 10.1 / 10.5 | 11.2% / 10.4% | 编辑器高级 浅/深 | 11.8 / 11.4 | 11.5% / 11.0% |

差异来源（按影响排序）：

1. **字体与栅格化**：Windows 的 DirectWrite 与 GPUI 的字体回退不同，字宽/行高有 1–2pt 偏差；
   所有文本行都在，但整行宽度不同。
2. **截图不包含原生输入框文字**：Windows 宿主用系统文本框（独立 HWND），`app.screenshot()` 只抓
   应用绘制面，所以名称/编队码/刷新上限输入框看起来是空的；真实窗口里文字正常（`nsh test` 的
   `type` + 保存用例已验证）。这是截图链路的限制，不是页面缺陷。
3. 主题包页的 `Slider`、主控台的开关色等旧页面细节仍沿用 P2/P3 的决定；这一轮只对齐队伍编辑器。
4. 两端标题栏已统一为 40pt；P3 早期 Linux 截图里标题栏高度是平台返回值，Windows 上已修正。

结论：队伍编辑器五个页签全部达到“布局、层级、间距、控件状态与 GPUI 一致”的验收线；
剩余差异是字体栅格化与输入框截图限制，不再为追求像素相等而改 token。

### 性能（10 次启动均值，第 4 秒采样）

`pwsh scripts/benchmark_nsh_vs_gpui.ps1 -Runs 10`：

| 指标 | NSH（打包版） | GPUI（release） | 差异 |
| --- | ---: | ---: | --- |
| 首窗时间（中位 / 最小–最大） | 123.3 / 118.1–468.1 ms | 151.4 / 137.1–164.1 ms | **-18.6%** |
| 工作集（均值） | 61.9 MB | 56.6 MB | +9.4% |
| 私有内存（均值） | 87.0 MB | 88.4 MB | -1.6% |
| 4 秒后 CPU 时间（均值） | 132.8 ms | 196.9 ms | **-32.6%** |
| 线程（均值） | 32.1 | 25.0 | +7.1 |
| 句柄（均值） | 1141 | 1154 | -1.1% |

首窗时间用中位数：NSH 有一次 468 ms 的冷启动（新产物首次被 Defender 扫描），其余 9 次都在
118.1–146 ms；GPUI 的 10 次都在 137.1–164.1 ms。工作集、私有内存、CPU、线程、句柄用 10 次均值。
结论：打包版 NSH 启动更快、空闲 CPU 更低；内存高约 5 MB（QuickJS + 资源打包的固定成本），
仍在可接受范围。`artifacts/nsh/perf.json` 保留每次采样的原始数据。这一轮 Rust 后端加入
`tungstenite` / `getrandom` 后重新采样，首窗与 CPU 仍在同一区间。

## 1.7 P1 第一块：model + ipc（2026-11-12）

已迁入 `nsh-app/native/src/`：

- **`model/`**：`settings` / `other` / `stats` / `tasks` / `teams` / `team_stats` 的 schema 3
  payload，字段名与原 `gpui-app/src/model/` 一致。`tasks.rs` 按 enums / config / execution 拆开；
  settings 的文件 I/O 留给 `state/settings_file.rs`，`model/` 只保留 JSON、默认值与版本迁移。
- **`ipc/contract.rs`**：请求/响应/结构化错误/事件 envelope、method 与 event 名称、请求序号。
- **`ipc/gateway.rs`**：`request_value`、类型化 `request<T>`、执行快照与结构化错误。
- **`ipc/mock/`**：确定性 Mock：团队/预设（6 个）/统计、任务、执行状态机、设备、主题包、资源、设置；
  事件带单调 `seq`，`submit` 走与真实 sidecar 相同的完成队列。
- **`ipc/websocket/`**：loopback WebSocket、hello token/schema/能力协商、30 s 请求超时、
  事件队列（截图帧只保留最新、可丢弃遥测优先淘汰）、二进制截图帧、sidecar 启动/退出与 stderr 摘要。
  传输用阻塞 socket + worker 线程；状态层的异步方法将用 `nsh` 的 `blocking(..)` 调用，
  不把网络等待放到引擎线程上。
- **`ipc/backend.rs`**：`Mock` / `Sidecar` / `Unavailable` 三种传输；不可用时返回结构化错误而不是 panic。
- **`ipc/bridge.rs`**：注册给 TSX 的 `Backend` module（`schemaVersion()` / `mode()` / `ping()`），
  以及进程级共享的 `BackendClient`；`backend.test.tsx` 验证 TSX → Rust → Mock 的 seam。
- **`state/settings_file.rs`**：`settings.json` 的路径、读取与替换；Windows 上因为 crate 禁止
  `unsafe`，先用临时文件 + 删除目标 + rename，不做 `MoveFileExW` 的原子替换（启动时只读一次）。

验证：

```powershell
cargo test --manifest-path nsh-app/native/Cargo.toml          # 49 passed
$env:AHAB_RUN_SIDECAR_INTEGRATION='1'
cargo test --manifest-path nsh-app/native/Cargo.toml real_sidecar_round_trips -- --nocapture
nsh check nsh-app
nsh test nsh-app                                               # 98 / 98
python scripts/validate_contract.py                            # NSH + GPUI + Python + Mock
$env:AHAB_BACKEND='mock'
nsh test nsh-app/backend.test.tsx                              # TSX → Rust Mock ping
```

真实 sidecar 用例会启动 `main_backend.py`、完成 hello、发 `app.ping` / `app.version`，再优雅关闭；
Mock 用例覆盖执行状态机、STALE_RUN、团队/统计、主题包/资源、SPT 校验与二进制截图帧。

尚未迁入：`state/`（teams / home / other）、`i18n/`、`app/`。下一步先迁 `state/teams`，
再把 `teams.ts` 的列表、保存、删除与统计换成 Rust 调用；编辑器草稿仍留在 TSX（屏幕级状态）。

## 2. 来源映射

### 2.1 搬进 Rust（`nsh-app/native/src/`）

| 原位置（`gpui-app/src/`） | 行数 | 目标 | 备注 |
| --- | --- | --- | --- |
| `model/` | 1678 | `native/src/model/` | P1 已迁入；settings 的文件 I/O 拆到 `state/settings_file.rs` |
| `ipc/contract.rs` | — | `native/src/ipc/contract.rs` | P1 已迁入；与 `contract/contract.json` 由校验脚本对齐 |
| `ipc/gateway.rs` | — | `native/src/ipc/gateway.rs` | P1 已迁入 |
| `ipc/websocket/` | — | `native/src/ipc/websocket/` | P1 已迁入；阻塞 worker + std mpsc，去掉 Entity/async-channel |
| `ipc/mock/` | 647 | `native/src/ipc/mock/` | P1 已迁入；供 `nsh test` 与无后端开发 |
| `state/teams/` | 1967 | `native/src/state/teams/` | 待迁：`Entity<State>` → `#[class]` + 信号回调 |
| `state/home/` | 926 | `native/src/state/home/` | 待迁：执行状态机、预览、统计 |
| `state/other/` | 924 | `native/src/state/other/` | 待迁：设置、工具箱、资源、主题包 |
| `state/app_state.rs` | 88 | `native/src/state/app_state.rs` | 待迁 |
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
| `scripts/validate_contract.py`（`contract.rs`、mock 目录） | **已部分切换**：同时检查 `nsh-app/native/src/ipc/**` 与 `gpui-app/src/ipc/**`，P5 删除后只留前者 |
| `scripts/generate_limbus_theme.py`、`scripts/render_limbus_tabs.py` | 输出到 `nsh-app/assets/themes/limbus` |
| `.github/workflows/reusable-build.yml` | `nsh-app/native` + `nsh check` / `nsh test` |
| `run-gpui.bat`、`run-gpui-mock.bat` | `run-nsh.bat`、`run-nsh-mock.bat` |
| `README.md`、`assets/doc/{zh,en}/**`、`docs/**`、根 `AGENTS.md` | 另见下节（文档量最大） |

## 3. 待验证风险

- [x] `nsh` CLI 已在开发机安装（0.5.0，`~/.nsh` + `~/.local/bin/nsh`）；本机访问 GitHub 需走代理（`http://127.0.0.1:7890`），CI 需要新步骤。
- [x] `nsh check nsh-app` 通过（TypeScript 对着 `.nsh/types/aalc/native.d.ts` 校验）。
- [x] `nsh test nsh-app` 通过：**98 个用例**（49 个 Rust 单测 + 49 个 TSX 用例，覆盖 model/contract、Mock、sidecar 传输、Rust↔TSX 后端 seam、壳、设置、工具箱、资源中心、队伍编辑器、帮助解析、主题合成、主控台、Windows 截图基线）。
- [x] `nsh run nsh-app --target gpui` 已在无 GPU 机器上跑通：Xvfb + `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`（lavapipe）。截图见 `nsh-app/docs/shots/`。
- [x] **mixed 模式会按应用构建宿主**：首次 `nsh test --target gpui` 花了 **11 分钟**编 `nsh-gpui-host`（NSH 引擎 + GPUI Fast + QuickJS + 我们的 crate），之后增量几分之一分钟。这意味着**任何要出包的机器（含 CI）都要有 Rust + 平台系统库**：Linux 需 GTK3 / WebKitGTK 4.1 / GStreamer / clang 的开发包。
- [x] 截图链路：Linux 用 `scripts/capture_nsh.py`（Xvfb + lavapipe，无需 GPU）抓桌面尺寸；旧客户端用 `scripts/capture_gpui.py` 出同一视口的对照图，基准已入库 `docs/reference/gpui/`。Windows 用 `nsh test nsh-app/visual.test.tsx --target windows` 抓 820×680 实机图（见 §1.6）；`app.screenshot()` 不含系统文本框的文字。
- [x] `cargo check` / `cargo fmt --check` / `cargo test --lib` 在 `nsh-app/native` 通过（nsh-core 0.5.0，Rust 1.99.1）。
- [~] `nsh-testing` 的 `bundle` feature 在本机依赖链编译失败（`rolldown 1.2.13` 与 `rolldown_std_utils` 不兼容），因此原生侧暂不加 `nsh-testing`/`nsh-engine` 开发依赖；不阻塞 `nsh test`（它自己构建 headless runner），P1 需要 crate 级 UI 测试时再解决。
- [x] mixed 模式引入 QuickJS 后的发布包体积与冷启动时间：打包版 `AALC.exe` 62.1 MB，首窗中位 123.3 ms（9/10 次 118.1–146 ms），工作集 61.9 MB；对 GPUI 的 release 版启动快约 19%，内存高约 5 MB（§1.6）。
- [x] Win32 后端打包覆盖原 `app.manifest` 的 DPI 感知与单实例语义：175% 缩放下窗口为 1435×1190 物理像素，第二次启动退出且保持一个进程/窗口。托盘与全局热键仍待 P1 接入。
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
| 主控台 home | 2855 | `pages/home/` | [~] | 对照 `docs/reference/gpui/home-light.png` | `home.test.tsx` 5 条；动态部分待 P1 |
| 队伍 teams | 784 + 3358 | `pages/teams/` | [x] | 列表 + 5 段编辑器对照 `docs/reference/gpui/teams-{light,dark}.png` 与 Windows 实机编辑器官图 | `teams.test.tsx` 9 条 |
| 设置 settings | 360 + 1100 | `pages/settings/` | [x] | 对照 `docs/reference/gpui/settings-{light,dark}.png` | `settings.test.tsx` 4 条 |
| 主题包 theme_packs | 423 | `pages/theme_packs/` | [x] | 对照 `docs/reference/gpui/themes-light.png` | 由设置/主题页测试覆盖 |
| 工具箱 toolbox | 348 | `pages/toolbox/` | [x] | 对照 `docs/reference/gpui/toolbox-light.png` | `toolbox.test.tsx` 3 条 |
| 资源中心 resources | 295 | `pages/resources/` | [x] | 对照 `docs/reference/gpui/resources-light.png` | `resources.test.tsx` 4 条 |
| 帮助 help | 549 | `pages/help/` | [x] | 对照 `docs/reference/gpui/help-light.png` | `help.test.tsx` 4 条 + `parser.test.tsx` 4 条 |
