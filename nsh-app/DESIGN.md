# 设计标准（NSH 客户端）

本文档是 `nsh-app/` 的设计基准：视觉语言、token、尺度、组件契约、动效清单。
页面和组件的改动必须先满足这里，再谈实现；与代码冲突时，改代码或先改本文档。

迁移目标仍是 **1:1 照搬 GPUI 客户端**（见 `MIGRATION.md`），因此这里的数值来自原客户端的
调色板、形状表与各页面实现（来源映射见 `MIGRATION.md` 第 2 节），而不是套用一套新风格。

## 1. 视觉语言

- **层级靠结构，不靠颜色**：背景分层 + 1pt 描边 + 间距。卡片不再套卡片。
- **主题色只从 token 取**：JSX 里不出现 `#hex` / `rgb()` / `hsl()` / 调色板类；缺角色先在
  `theme.json` 加，再在组件里用。
- **皮肤（skin）改变的是表面、圆角、装饰与阴影，不改变布局**：切皮肤不能让内容位移。
- **紧凑但可读**：桌面密度，控件 20–32pt，正文 12pt，次要信息用 `muted-foreground`。
- **状态必须可见**：hover 是淡洗（`accent`/`secondary`），pressed 略收缩并加深，
  focus-visible 是控件外侧的 ring，selected 是持久的底色 + 前景色。
- **动效只用来说明变化**：短（≤200ms 的桌面反馈）、可被打断、`motion-reduce` 下全部瞬时。

## 2. Token

`theme.json`（Bettbox，`nsh.json` 的 `"theme"`，即首启与热重载前的启动主题）与 `themes/skins.ts`
一起构成皮肤表。
这张表原本由 `scripts/export_nsh_theme.py` 从 GPUI 调色板机械导出，脚本随旧树删除后改为手写：

- 7 皮肤 × 浅/深 × 34 个颜色 token + 4 个装饰 token（`band-top` `band-bottom` `wash` `grain`）
  + 4 档圆角（`radius-sm` `radius` `radius-lg` `radius-xl`）。
- 7 个强调色不各存一份全表：`ACCENT_OVERRIDES` 只记「换强调色后变了的 token」
  （`brand*` `decor-line` `ring` `selection`，以及 limbus 的 `accent-foreground`、glass 的 `wash`），
  运行时由 `themes/index.ts` 的 `themeFor(skin, accent)` 合成，`applyTheme()` 调 `setTheme()`。
- token 名就是 shadcn 的名字，TSX 直接用类：`bg-card` `text-muted-foreground` `border-border`
  `rounded-(--radius)`；需要值（图标着色、`style`、渐变）时用 `token("brand")`。

按角色的用法：

| 角色 | token | 用在哪 |
| --- | --- | --- |
| 窗口与页面底 | `background` / `foreground` | 根视图、正文 |
| 面板 | `card` / `card-foreground` | 标题栏、卡片、工具栏 |
| 浮层 | `popover` / `popover-foreground` | 菜单、弹层、对话框 |
| 次要信息 | `muted` / `muted-foreground` | 说明文字、未选中导航 |
| 普通动作 | `primary` / `primary-foreground` | 默认按钮 |
| 强调（品牌/强调色） | `brand` `brand-hover` `brand-light` `brand-foreground` | 选中的导航、主按钮、链接 |
| 悬停/选中底 | `accent` / `accent-foreground`，`secondary` | hover 淡洗、行/幽灵按钮 |
| 结构 | `border` `input` `ring` | 描边、输入框边、焦点环 |
| 状态 | `success` `warning` `danger` `*-light` `danger-foreground` | 徽标、状态点、破坏性动作 |
| 皮肤装饰 | `decor-line` `hilite` `glow` | 角框/分隔线、玻璃高光、投影光晕 |
| 皮肤专属 | `band-top` `band-bottom` `wash` `grain` | 边狱卡片头渐变、雾/玻璃顶部氛围、雾的颗粒 |

### 皮肤装饰语言

7 个皮肤共用同一套组件结构（同样的内边距、同样的 44pt 标题带），只换装饰语言；组件用
`currentDecor()` 分支，**不按皮肤 id 分支**：

| 皮肤 | decor | 卡片 | 标题带 | 空状态标记 | 分隔符 |
| --- | --- | --- | --- | --- | --- |
| 现代 | `plain` | `shadow-sm`，无边框（`border` token 透明） | 底部 `input` 发丝线 | 无 | 发丝线 |
| 玻璃 | `glass` | `shadow-lg` + 顶部 1px `hilite` | 顶部 `hilite` 边 | 抬起的方形芯片 | 发丝线 |
| 档案 | `archive` | 无阴影，靠留白 | 标题 + 向右延伸的 `decor-line` 发丝线 | 2px 横杠 | 金色花纹图 |
| 边狱 | `limbus` | 无阴影 + 四角 L 形角框 | 渐变带（`band-top`→`band-bottom`）+ 左侧 3px 竖条 + 污渍 | 封蜡图 | 金色花纹图 |
| 血雾 | `mist` | `shadow-md` + 顶部红色氛围 | 左侧 3px 竖条 + 底部 `border` | 细横杠 | 发丝线 |
| NSH | `hairline` | 无阴影 + 1px `border` 描边 | 底部 `border` 发丝线 | 无 | 发丝线 |
| Bettbox | `gradient` | `shadow-sm`，无边框 | 无分隔线 | 无 | 发丝线 |

`nsh` 是唯一不来自 GPUI 的皮肤：它按设计指南新增，用结构而非装饰分层。浅色下也有可见描边
（其余平皮肤浅色 `border` 是透明的）、卡片不投影、六个装饰 token（含 `hilite` `glow`）全透明；
状态色直接沿用 `default`，强调色默认赤红，所以皮肤之间切换只换表面。圆角 4/6/10/14：
控件 6 是指南起点，容器再柔一档。

`bettbox` 是第二套不来自 GPUI 的皮肤：页面底色不画在卡片上，而由 `SkinBackdrop` 用
`wash` 画一条横向渐变（0% 到 85% 宽度），标题栏透明让它穿过；卡片无边框、只有一层软阴影。
`wash` 因此是这皮肤的结构 token；换强调色/颜色主题时渐变跟着变色。

### 颜色主题

设置页「颜色主题」用一份 11 色的预设（Bettbox 的色板）加自定义 `#RRGGBB`，写入
`settings.json` 的 `themeSeed`。`themes/seed.ts` 把种子转成 OKLCH，按「浅色 L∈[0.45,0.57]、
深色 L∈[0.74,0.86]，并且品牌色上的文字至少 4.5:1」推出 `brand` 系列、`primary`、
`ring`、`selection`、`wash` 七个 token，再由 `themeFor(skin, accent, seed)` 覆盖在皮肤表之上。
颜色主题只改色相，不改表面层级：`background`、`card`、`muted` 仍来自皮肤。

皮肤背景层由 `shell/SkinBackdrop.tsx` 统一画（页面不参与）：边狱深色铺星云原图（浅色不铺，否则
羊皮纸会被压成灰褐）、血雾红色顶部渐晕 + 1px 斜纹颗粒、玻璃顶部 accent 渐晕。

切皮肤时强调色会跟着走，**仅当用户没自己选过强调色**（当前强调色 == 旧皮肤的默认强调色）：
每套皮肤自带配色（边狱 = 黄铜），把赤红留在边狱上会把角框和标题带画成错的红。

## 3. 尺度

字号（来自 `runtime.rs` 的 `FONT_*` 常量 → Tailwind 档位）：

| 用途 | 值 | 类 |
| --- | --- | --- |
| 极小标签 | 9.5–10pt | `text-[10px]`（仅限确有需要的密集数据，需说明） |
| 控件/次要文字 | 11–12pt | `text-xs` |
| 主数字 | 21pt | `text-xl font-bold tabular-nums`，只给 `Figure`，单位/分母用 `text-xs` |
| 卡片标题 | 14pt | `text-sm font-semibold` |
| 区块标题 | 16pt | `text-base font-semibold` |

间距：一律用 Tailwind 档位（`p-1` 4pt、`p-2` 8pt、`p-3` 12pt、`p-4` 16pt、
`gap-2`/`gap-3`/`gap-6`）。页面外框固定 `p-2.5`（10pt，对齐 GPUI 的 `page_root`）。

圆角：只用 `rounded-(--radius-sm)`（徽标、小部件）、`rounded-(--radius)`（控件、内嵌块）、
`rounded-(--radius-xl)`（卡片）、`rounded-(--radius-lg)`（弹层、对话框），以及 `rounded-full`（仅圆形：
状态点、开关滑块、头像、药丸徽标）。

控件尺寸（`components/Button.tsx` 的 `SIZE`，一并决定高度、内边距、图标）：

| size | 高度 | 内边距 | 图标 | 用在 |
| --- | --- | --- | --- | --- |
| `xs` | `min-h-5` 20pt | `px-2` | 12 | 表格行内、密集工具栏 |
| `sm`（默认） | `min-h-5` 20pt | `px-3` | 13 | 普通按钮，等于 GPUI 的 21pt 控件 |
| `md` | `min-h-6` 24pt | `px-3` | 14 | 标题栏导航、图标按钮 |
| `lg` | `min-h-8` 32pt | `px-4` | 16 | 页面主操作、操作条（GPUI 的主控台启动栏） |

所有尺寸的文字都是 `text-xs`（12pt）：旧客户端按钮没有更大的字。字高与控件高度都用 `min-h-*` 而不是固定 `h-*`：文字随系统字号放大时控件跟着长。

数据系列用 `chart-1` … `chart-5`（设计指南的图表角色）：它们不是皮肤 token，由 `themes/index.ts` 统一注入，
只分明暗两套，所以同一组数据在换皮肤、换强调色、换颜色主题时仍是同一个颜色，图例可以跨皮肤阅读。

## 4. 组件契约

`components/` 是应用自己的 ui 层，`nsh/base` 只提供行为（按下、键盘、焦点、无障碍）。

| 文件 | 组件 | 说明 |
| --- | --- | --- |
| `Button.tsx` | `Button` / `IconButton` / `Pressable` | 变体 `default` `primary` `secondary` `outline` `ghost` `danger` `link` `nav`，尺寸 `xs`–`lg`；`Pressable` 是不带外观的整面按压容器（页面专属选项卡用）；`nav` 只给标题栏的选中项（胶囊由标题栏的滑块画） |
| `Field.tsx` | `SettingList` / `SettingLine` / `SettingRow` | 标签/控件网格（行距 12pt，控件固定尾列）：设置页与主控台的窗口设置都用它 |
| `Tabs.tsx` | `Tabs` | 分段选择的本体：一条选中胶囊在选项下移动（宽度一起伸缩），选项自己是透明按钮；统计范围/难度、队伍筛选、设置的主题与语言都走它 |
| `Segmented.tsx` | `Segmented` / `SwatchRow` | `Tabs` 的简短形式（`secondary` 胶囊）；`SwatchRow` 是色块行 |
| `Switch.tsx` | `Switch` | 开关（轨道 `input`→`primary`，滑块 16pt，`transition-transform`） |
| `Input.tsx` | `Input` | 单行输入（与开关同高的描边框） |
| `Select.tsx` | `Select` | 下拉选择：触发器与输入框同高，弹出层用统一浮层表面（`popover` + `input` 描边 + 阴影，因浅色下 `popover` 与卡片同为白）；列表行 24pt 高、2pt 间距、`focus:` 淡洗、选中行 `brand/10` + 行首对勾；`value=null` + `placeholder` 显示未选提示，`bare` 去掉自带边框/底色，供字段组使用 |
| `Swatch.tsx` | `Swatch` | 强调色色块（28pt，选中环在色块外侧） |
| `Card.tsx` | `Card` / `CardHeader` / `CardBody` / `Rule` | 卡片表面、44pt 标题带（图标 + 标题 + 注脚 + 尾部控件）、正文内缩；`Card` 的 `arrive` 让卡片在页面进入时随波纹到位（动效绑在卡片自己的 View，网格的 `items-stretch` 不变） |
| `Figure.tsx` | `Figure` | 数据卡里的一项读数：标签与注脚同一行，主值 `text-xl` + 单位/分母 |
| `Notice.tsx` | `Notice` | 页面级一行提示（警告/成功/信息），不是卡片：贴在页面上，不另开一个盒 |
| `Reveal.tsx` | `Reveal` | 非卡片行的进场包裹层（帮助目录这类按钮列），参数同波纹 |
| `Badge.tsx` | `Badge` | 六种语义色，默认 `neutral`；药丸形（`rounded-full`），`dot` 是状态自己的标记 |
| `Icon.tsx` | `Icon` | Lucide SVG 资产 + `tintColor` |
| `Page.tsx` | `PageRoot` / `PageToolbar` | 页面外框与工具栏卡片 |
| `EmptyState.tsx` | `EmptyState` | 空/未就绪状态：说明 + 下一步 |

`motion.ts` 不是组件，是原生动效的 token 出处（毫秒、曲线、`RISE`、波纹节奏），页面与组件只用它，不写裸数值（见第 5 节）。

规则：

- 页面**只**组合 `components/` 与布局原语（`View` `Text` `Scroll` `List`），不直接引
  `nsh/base` 的控件，不写颜色、圆角、字号。
- **`default` 与 `primary` 画的是同一种东西**（accent 实底）：旧客户端的 `Default` 按钮就是
  accent 按钮（GPUI Kit 的 `Primary` 映射到 `palette.brand`）。调用方按语义挑名字：`primary`
  表示「决策区唯一的默认提交」，`default` 表示普通填充动作。
- 变体按**含义**选：`primary` 只给决策区里唯一的默认提交；工具栏里的普通动作是 `default`；
  `danger` 只给不可逆的破坏性提交；`link` 只给外部资源；`nav` 只给标题栏当前页（胶囊由滑块画）。
- 图标按钮必须有 `accessibilityLabel`，且是正方（`aspect-square`）。
- 焦点环在控件外侧：`focus-visible:outline-2 outline-offset-2 outline-ring`；祖先 `overflow-hidden`
  会裁掉它，布局要留出位置。

### 卡片的三种形态

三形共用一套表面（`radius-xl` + 当前皮肤的阴影/描边）、`px-4` 左内缩、14pt 半粗标题、药丸徽标：

| 形态 | 结构 | 用在哪 |
| --- | --- | --- |
| 带标题的卡片 | `Card title` + `CardHeader`（44pt：图标 + 标题 + 注脚 + 尾部控件）+ `CardBody`（`px-4 pt-3 pb-4`，行距 12pt） | 设置各段、统计各卡、工具箱每件工具、队伍卡、帮助目录 |
| 单行卡片 | `Card class="p-0"` + 一行 `px-4 py-3`：开关/图标 + 名称 + 摘要 + 箭头 | 主控台队列与窗口设置、主题包行、队伍页的镜牢轮换条 |
| 数据卡片 | `Card class="px-4 py-2.5"`：紧凑标题行（12pt 半粗 + 10pt 注脚 + 尾部幽灵按钮）→ `Figure` 行 → `Rule` → 一行 10pt 注脚 | 主控台顶部卡、统计的总览、主控台右侧栏（设备/画面/日志） |

规则：

- 单行卡片的标题就是行内容，不加标题带；带标题的卡片不加行内标题。
- 页面不自己写卡片内缩：标题卡用 `CardBody`，单行卡用 `px-4 py-3`，数据卡用 `px-4 py-2.5`。
- 卡片里不再套卡片：分组用 `Rule` 和间距；页面的警告/成功提示用 `Notice`，不用带底色的卡片。
- 数据卡的主值只出自 `Figure`，单位与分母在 `unit` 里；`—` 用 `quiet`。

### 图标着色的已知限制

`nsh` 通过 `Image` 画 SVG，`tintColor` 是**值**而不是 CSS 的 `currentColor`，所以图标不能跟着
`text-*`/`hover:` 变色。当前做法：

- 变体自带图标 token（`Button` 的 `VARIANT[x].icon`），调用方可用 `iconColor` 覆盖；
- 图标跟随状态变化的场景（选中/未选中导航）用**变体切换**实现，即换 token；
- 纯 hover 的变色用**底色淡洗**代替（符合设计指南的“hover 是安静的淡洗”）。

若后续需要 hover 换图标颜色，用两个图标叠放 + `opacity` 过渡，或给 `Icon` 传一个 signal 驱动的
token；不要为此改 `nsh`。

### 主控台的结束后操作

运行栏左侧的按钮打开「结束后操作」弹层（`pages/home/HomeCompletion.tsx`）：退出动作可多选（退出游戏 / 退出模拟器 / 退出 AALC）、最终电源动作（无 / 睡眠 / 休眠 / 锁屏 / 关机），底部两个提交——「仅本次生效」与「保存为默认」，分别把 `keepAfterCompletion` 置为 `false` / `true`。弹层在草稿上工作，提交前不改配置；按钮标签是摘要（`退出游戏 · 锁屏（默认）` / `什么也不干（本次）`）。

### 主控台的顶部卡

空闲时是「周期统计（今日/本周）」，运行中换成本轮进度；两者共用一套骨架，切任务时卡片不变高：

- 标题行：14pt 字形 + 12pt 半粗标题 + 尾部「明细」；
- 数字行：三列，标签与注脚同一行（`经验本 本周 11`、`镜牢次数 今日 2 · 本周 7`），下面是主值（`text-xl`、700、`tabular-nums`），目标写成分母（`/ 2`、`/ 5 层`）；
- 状态行：药丸（运行中 / 已暂停 / 后端状态）+ 设备名 + `上次镜牢 6:12 · 5 层`。

分状态的内容：空闲 = 今日；`daily_task` = 经验本/纽本/镜牢的 `已完成 / 目标`；`mirror` = 镜牢次数、当前层、本轮用时；`get_reward` / `buy_enkephalin` 没有分步计数，退回今日/本周并在标题后标「今日累计」。

数据来自 `execution.status.currentTaskId`、`execution.stats.currentRun`（targets / completed / startedAt / isMirrorInfinite）与 `execution.mirrorFloor`。`execution.mirrorProgress.total` 是「剩余」不是总数（`script_task_scheme.py` 用递减后的 `mir_times` 发送），分母只取 `currentRun.targets`。镜牢层数只在 `currentTaskId === "mirror"` 时显示，运行结束（状态回到 idle）时清空，不会挂在下一次任务上。

### 主控台的设置面板

卡片设置由 `home.ts` 的 `TaskParam` 描述：`kind`（`select` 下拉 / `switch` 开关）、`group`（有分组时按「常规 / 高级」分节）、`summary`（决定折叠行显示哪几条）、`options`（数组或函数——日常的编队下拉读队伍 store）。页面只渲染，字段落到 `TasksConfig` 的映射在 Rust `state/home/mod.rs` 的 `configure` / `selection`。

数量对齐迁移前的客户端任务设置面板：

| 卡片 | 设置 |
| --- | --- |
| 窗口设置 | 6：分辨率、窗口位置、减少误触、截图间隔、鼠标操作间隔、异步 PostMessage |
| 日常任务 | 18：经验本、纽本、默认日常编队、连战（开关 + 次数）、按属性/按星期配队开关 + 11 个按天编队 |
| 领取奖励 | 1：领取模式（全部 / 狂气与通行证奖励 / 仅领取邮件奖励） |
| 狂气换体 | 3：换体次数、葛朗台模式、跳过模块合成 |
| 自动镜牢 | 17：轮次、难度、困牢目标、无限模式 + 13 个高级开关 |
| 亚哈共鸣 | 0：卡片开关本身即设置，不展开 |

设置行都走 `Field.tsx` 的 `SettingList` / `SettingLine`，控件固定尾列（`w-36` 的下拉、开关）。

### 队伍页

队伍页的主轴是**游戏内 1–40 个编队位**，不是用途分类，也不是卡片数量：

- 工具栏：`全部 / 镜牢 / 资源本 / 通用` 筛选、`从预设新建`、`新建队伍`。
- 镜牢轮换：单行卡片，按 `teams_active_queue` 显示 `#1 → #2 → #3` 与难度限定；点击队伍打开编辑器。它是状态条，不占一个完整区块。空状态直接说明「镜牢任务不会启动」。
- 编队位：8×5 的 40 个编号格；已用格显示 `#NN`、备注名和用途色点，空位显示「AALC 未配置」。点击空位在该编号新建，点击已用格打开该队伍。标题旁的「按队名 / 按序号」是全局选队方式，不是筛选。
- 已配置队伍：只渲染实际存在的队伍，2 列卡片。标题是 `#NN 备注名`，注脚是 `轮换 #n` / `日常` / `手动`；正文是用途、体系、路线、编队来源、前 7 / 后 5 的人格顺序和状态徽标。卡片只在镜牢队伍上出现「加入镜牢轮换」开关。
- 预设选择器是一个独立弹层：2 列模板卡（名称、用途、层数提示、路线、人数、编队码），底部选目标空位；`使用并编辑` 只把模板写入草稿，保存前不落盘。
- 删除是 `AlertDialog`，文案说明统计、主题包权重会一起删除，日常任务/轮换里的编号不会自动替换。
- 编辑器顶部是 `编辑编队 #NN` / `新建编队 #NN`；基础编成的「编队来源」用 `Segmented` 在 `编队码 / 手动顺序` 之间切换，并保留另一边的数据。资源本队伍优先使用编队码，留空才按人格顺序；通用队伍只手动使用，不参与轮换。镜牢路线放在「商店与合成」的第一张卡。

### 主控台的设备连接

右栏第一张卡：标题行是字形 + 「设备连接」+ 状态徽标（带状态点，连接后不会给卡片加品牌色底）。下方是**一整块** `bg-muted` 字段组，内部由无框 `Select`（`bare`，未选时显示灰色占位「选择窗口 / 模拟器」）、`decor-line` 竖分隔线和纯图标刷新按钮组成；选中的设备 id 由 Rust `HomeStore.selected_device_id()` 持有，TSX 只镜像，连接成功后触发器的文字直接变成设备名。

运行日志卡：头部两个纯图标按钮（`fileText` 明细、`eraser` 清空），行内不再显示日志级别，只保留正文；`List` 的 `initialScrollAtEnd` + `maintainVisibleContentPosition` 让日志在读者位于底部时跟随新行，上滚阅读时不强制回底。

## 5. 动效

### Token（沿用 Tailwind 档位，不另建常量）

| Token | 值 | 用于 |
| --- | --- | --- |
| `instant` | `duration-100` | 淡洗、描边、颜色 |
| `fast` | `duration-150` | 按下、控件状态（桌面反馈） |
| `base` | `duration-200` | 选择移动、图标替换 |
| `slow` | `duration-300` | 弹层、对话框进出 |
| 缓动 | `ease-out` 进入、`ease-in-out` 位移、`ease-in` 退出 | 位移类 |
| 收缩 | `scale-[0.97]`（图标按钮 0.95） | 按下 |
| 位移 | `4` / `8` / `16`pt | 标签浮现 / 弹层滑动 / 列表进入 |

`motion-reduce:` 与 `reduceMotion()` 必须写到每一处动效上；减少动效时状态仍要看得懂。

原生动效（`createMotion` / `Animated` / `Plot` 的 `motion`）不读 CSS，毫秒与曲线从 `motion.ts` 取同一套：
`MS`（`instant` 100 / `fast` 150 / `base` 200 / `slow` 300 / `slide` 240）、`ENTER_EASING`
（`cubic-bezier(0.32, 0.72, 0, 1)`，和标题栏胶囊同一条）、`MOVE_EASING`、`LEAVE_EASING`、`RISE` 6pt、
`WAVE`（20ms 一档、前 5 项）。页面和组件里不写毫秒数。

### 动效清单（镜牢主题包 / 主控台，本轮）

| 变化 | 动效 |
| --- | --- |
| 主题包开关 | 轨道 `transition-colors`、滑块 `transition-transform`（同 `Switch`） |
| 权重滑块 | 拖动/按键即时跟手（滑块自己处理，不加缓动）；范围与滑块位置由 `Slider` 原生放置 |
| 停用的主题包 | 整行 `opacity-60`，滑块 `disabled`；颜色变化即时 |
| 任务卡开关与参数 | 同 `Switch` / `Select`：淡洗 + 按下收缩 + 颜色 150ms |
| 顶部卡切换（任务开始 / 结束 / 暂停） | 标题与数字整块替换，无过渡；数字 5Hz 直接换，不做滚动或插值 |
| 顶部卡的本轮用时 | 每秒一跳，同样不做动画 |
| 任务参数选择器 | 弹层由 `Select` 提供（NSH 默认进出），不额外加动画 |
| 窗口设置/任务卡展开收起 | 点卡片行即切换（没有单独的展开按钮）；高度是 `Collapsible reveal` 的原生弹簧，下面的卡片跟着走；卡片在可视区外时 `reveal({ animated })` 把它滚进来；减少动效时两处都瞬时 |
| 卡片行 hover / 按下 | hover 淡洗 `accent`、按下 `secondary`，`duration-150`；行尾箭头随状态转 180°（瞬时，不做旋转动画） |
| 展开区里的选择与开关 | 同 `Select` / `Switch`：弹层由 NSH 提供，开关淡洗 + 颜色 150ms |
| 队列滚动 | 用户拖动是原生的 1:1 + 惯性（`decelerationRate` 原生默认），不加缓动；只有程序滚动（展开后 `reveal`）带动画 |
| 下拉列表行 | 指针/键盘高亮 `focus:bg-accent`（瞬时，`Select` 自带），触发器 hover 淡洗 `duration-150` |
| 结束后操作弹层 | `Dialog` 自带的进出动画（NSH 默认），不额外加；草稿摘要行随开关即时替换 |
| 队列全选/清空 | 开关状态瞬时切换；反馈一行出现在队列下方，无入场动画 |
| 设备刷新 | 图标按钮同 Button：hover 淡洗、按下收缩；列表替换为瞬时；连接状态徽标换色 150ms |
| 设备选择 | 弹层由 `Select` 提供；连接成功后触发器文字与徽标一起更新，无额外过渡 |
| 运行日志跟随 | 停在底部时新行由 `List` 原生跟随；上滚后保持阅读位置，不强制回底；无额外动画 |
| 日志明细 / 清空 | 纯图标按钮同 Button：hover 淡洗、按下收缩；计数与空状态瞬时 |
| 减少动效 | 以上全部瞬时；颜色变化保留 |

### 动效清单（帮助 / 队伍）

| 变化 | 动效 |
| --- | --- |
| 目录项 / 筛选标签 | 目录项按顺序进场（`Reveal`，见「页面内容」）；筛选标签的选中底是一条滑动胶囊（`Tabs`），文字 `duration-150`；hover 淡洗、按下 `scale-[0.97]` |
| 目录跳转 | `scrollTo({ animated: true })`；滚动改变当前项时高亮瞬时切换 |
| 语言切换（帮助） | 文档整块替换（`resource` 重新加载），无过渡：内容整体换掉时淡入只会读到半截 |
| 队伍卡片开关 | 轨道 `transition-colors`、滑块 `transition-transform`，同 `Switch` |
| 队伍筛选 / 编队位格 | 筛选选中底走 `Tabs` 滑动胶囊；编队位格的 hover 淡洗、按下收缩，选中格 `border-brand + bg-brand-light` 立即出现；40 个格子不逐个 `arrive` |
| 镜牢轮换条 | 单行卡片 `arrive=0`；队伍 chip hover 淡洗、按下收缩；顺序变化只换位置，不做逐项动画 |
| 队伍卡片进场 | 已配置队伍按卡片顺序 `arrive=n`；筛选切换不重播 |
| 预设选择器 | `Dialog` 自带进出；模板卡选中换边框/底色 `duration-150`；目标空位下拉由 `Select` 自带 |
| 删除确认 | `AlertDialog` 自带进出；删除后槽位与卡片即时替换，反馈条无入场动画 |
| 编辑弹层 | `Dialog` 自带的进出动画（NSH 默认），不额外加 |
| 队伍保存/删除 | 卡片增删即时生效；反馈条出现在工具栏下方，无入场动画 |
| 减少动效 | 以上全部瞬时；颜色变化保留 |

### 动效清单（工具箱 / 设置里的资源同步）

| 变化 | 动效 |
| --- | --- |
| 工具卡上的按钮 | 同 Button：hover 变深、`active:scale-[0.97]`，`duration-150` |
| 运行 → 停止 | 图标与文字整块换（`play` → `stop`，颜色转 `danger`）；不做图标交叉淡入（按钮本身已经换了底色，再叠动画只会晃） |
| 工具体反馈行 | 出现/替换，无入场动画（一行文字） |
| 资源同步进度 | 徽标文字与进度条宽度随进度更新；宽度走样式而非动画（进度每 150ms 一跳，插值无意义） |
| 同步结束 | 徽标消失、进度条移除、反馈行出现，瞬时 |
| 减少动效 | 以上全部瞬时；颜色变化保留 |

### 动效清单（设置页）

| 变化 | 动效 |
| --- | --- |
| 导航项 hover / 按下 | 淡洗 + `active:scale-[0.97]`，`duration-150` |
| 导航项选中 | 换变体（`ghost`→`secondary`），颜色 `duration-150` 过渡 |
| 导航项点击跳段 | `scrollTo({ animated: true })`，原生滚动，非信号驱动 |
| 滚动改变当前段 | 高亮瞬时切换（不做淡入：滚动手势中再叠动画会晃） |
| 开关翻转 | 轨道 `transition-colors duration-150`，滑块 `transition-transform duration-150`，位移 16pt |
| 分段选择 | 选中底是一条滑动胶囊（`Tabs`）：左边缘与宽度一起滑到目标，`240ms`，`ENTER_EASING`；文字颜色 `duration-150`；不位移、不缩放容器 |
| 强调色色块 | hover 透明度、按下 `scale-[0.92]`，选中环由 `outline` 立即出现（无动画：环是状态而非过程） |
| 皮肤卡片 | hover 边框变亮，按下 `scale-[0.99]`，选中 `border-2` + `ring` 色 |
| 反馈行 | 出现在卡片栈下方，颜色由 token 决定，无入场动画（一行文字，动效无信息量） |
| 减少动效 | 以上全部瞬时；颜色变化保留 |

### 动效清单（窗口壳）

| 变化 | 动效 |
| --- | --- |
| 导航项 hover | 底色淡洗 `hover:bg-accent`，`duration-150`，不改变尺寸 |
| 导航项按下 | `active:bg-secondary` + `active:scale-[0.97]`，`duration-150` |
| 导航项选中 | 标题栏只在选中项下面画**一条**滑动胶囊（`nav` 变体自己透明）：左边缘与宽度以 `scaleX` 一起滑到目标，`240ms`，`cubic-bezier(0.32, 0.72, 0, 1)`；原生值（位移 / 缩放）先落，再改胶囊宽度，否则会漏出一帧“新宽度 + 旧缩放”的胖胶囊 |
| 离开导航页（统计 / 设置 / 帮助） | 胶囊 `160ms` 淡出；回到任一导航页时从当时的胶囊位置滑回 |
| 图标按钮 hover/按下 | 同导航项（`ghost` 的淡洗 + 收缩） |
| 窗口按钮 hover | 淡洗；关闭按钮 `hover:bg-danger` |
| 执行状态点 | 空闲不显示；运行 `success`、暂停 `warning`。GPUI 的呼吸动画默认关闭（`ENABLE_STATUS_DOT_BREATHING = false`），这里同样不做动画 |
| 外观切换 | 图标交换（sun/moon/monitor）即时替换 + 颜色过渡；不做交叉淡入（6px 图标，动效无信息量） |
| 页面切换 | **交叉溶解**（`PageHost`）：新页面在创建时就以 `createMotion({ enter })` 起跑（第一帧即起点，无静止帧），`200ms` 淡入并朝目标在导航里的方向偏移 `6pt`；离开的页面以**同一时长、同一曲线淡出**。两层互补，任一帧叠起来都是一整页——旧页面文字不会透过新页面的空白区看到，也不会露出窗口底色。淡变结束后（`200+60ms`）移除旧层 |
| 快速连点导航 | 最多两层：被压下去的旧层从当时的不透明度继续淡出，新层照常入场 |
| 减少动效 | 以上全部瞬时；底色变化保留（颜色不是动画）；`createMotion` 自动落到终点不移动 |

### 动效清单（页面内容：进场波纹 / 分段选择）

窗口壳负责“换页”，页面内容负责“到位”：新页面在交叉溶解里浮上来，页内的卡片和行再以一条短波纹跟进。两者加起来不超过一次切换的时间，不拖到下一次操作。

| 变化 | 动效 |
| --- | --- |
| 页面进入，列表 / 卡片 | 波纹：`opacity 0→1` + 上浮 `RISE` 6pt，`fast` 150ms，`ENTER_EASING`；第 n 项延迟 `min(n, 5) × 20ms`，第 6 项起一起进场。整条波纹（最后一档延迟 + 一次时长）是 250ms，落在页层保留的 260ms 里 |
| 网格里的卡片 | `Card` 的 `arrive`：动效绑在卡片自己的 `View` 上，网格的 `items-stretch` 不变——加包裹层会让卡片失去等高 |
| 非卡片的行（帮助目录） | `Reveal` 包裹层，参数同波纹 |
| 主控台 | 窗口设置卡 `arrive=0`，任务卡按队列序号 `arrive=n+1`；卡片自身的展开仍走原生高度弹簧（见主控台清单） |
| 队伍 / 工具箱 / 主题包 | 卡片按格子 / 行顺序 `arrive`；筛选、增删不重播（只在挂载时跑一次） |
| 统计 | KPI、图表、环节、队伍、最近记录、每日次数 `arrive=0…5`；筛选与轮询不重播 |
| 统计图表 | `Plot` 的 `motion.appear`：柱子从基线长出（`slow` 300ms，`stagger` 0.35 逐列），`update` 用 `base` 跟形状；`appearGeneration` 只随筛选变化重播，数据轮询不重播 |
| 分段选择（统计范围与难度、队伍筛选、设置的主题模式与语言） | 选中底是**一条**滑动胶囊（`Tabs`）：左边缘与宽度一起滑到目标，`slide` 240ms + `ENTER_EASING`；原生值先落再改宽度（同标题栏胶囊）；胶囊高度取实测项高，轨道的内边距不吃进胶囊；选项自己透明，文字 `foreground` / `muted-foreground` `duration-150` |
| 减少动效 | 波纹由 `createMotion` 自动落位，胶囊走 `reduceMotion()` 分支直接落位，`Plot` 的 `motion` 自动停；颜色变化保留 |

组件对照页（`pages/Gallery.tsx`）每个变体/尺寸/状态都是静态展示，无动效。

### 动效清单（队伍编辑器，本轮）

| 变化 | 动效 |
| --- | --- |
| 编辑器打开/关闭 | `Dialog` 自带进出（NSH 默认）；不叠自定义过渡 |
| 页签切换 | 选中页签换 `card` 底 + `shadow-sm`，颜色 `duration-150`；内容整块替换（无滑动指示条） |
| 编队位 / 来源 / 路线 | 编队位下拉、`编队码 / 手动顺序` 分段、镜牢路线下拉都走现有 `Select` / `Segmented`；切换即时生效，不播额外过渡；切换来源时保留另一边的数据 |
| 系统/人格选择 | 选中换边框与浅色底（`brand` / `danger`），`duration-150`；按下 `active:scale-[0.97]` |
| 开关 / 下拉 | 同 `Switch` / `Select`：轨道与滑块 150ms，弹层由 NSH 默认进出 |
| 星光等级 / 一键设置 | 选中描边与底色即时切换；总消耗数字随状态更新，不做数字滚动 |
| 统计刷新 / 清空 | 数值与确认框瞬时出现；清除后反馈行替换，无入场动画 |
| JSON 面板展开 | 展开/收起瞬时（面板高度由布局决定，不做高度动画） |
| 减少动效 | 以上全部瞬时；颜色与边框变化保留 |

## 6. 校验

```bash
nsh check nsh-app     # Rust 描述 + TSX 类型
nsh build nsh-app     # 打包；未知 class 会在这里 warn
nsh test nsh-app      # 无头跑 App.test.tsx
nsh run nsh-app       # 开窗看
```

- **组件对照页**：`pages/Gallery.tsx`。开发/评审时把 `store.ts` 的初始 page 改成 `"gallery"`
  打开它，覆盖全部变体、尺寸、状态、图标、token 色板与圆角档位；评审完改回 `"home"`。
  它不进导航，不是用户可见功能。
- 无头测试跑在 `820×680`、`platform: "windows"`（`nsh.json` 的窗口，即原客户端的窗口），默认的
  手机视口会把标题栏裁掉。最小窗口 800×560 下标题栏仍然放得下 6 个导航项。
- 真实渲染截图：`python3 scripts/capture_nsh.py`（Xvfb + lavapipe，无需 GPU）；基线在
  `docs/shots/`：`p2-titlebar-light.png`、`p2-gallery-light.png`、`p2-gallery-dark.png`。

## 7. 本轮校验结果

- `nsh check` / `nsh build` 通过（无未知 class 告警）；P2 时 `nsh test` 7 项全过（5 条 TSX + 2 条 Rust；
  当前总数与后端覆盖见 `MIGRATION.md`）。
- 截图（浅色/深色）逐项对照 Finishing checklist：
  - **圆角**：全部来自 `--radius-sm` / `--radius` / `--radius-lg` / `--radius-xl`；状态点、开关滑块用
    `rounded-full`。通过。
  - **字号**：控件与正文 `text-xs`（12pt），卡片标题 `text-sm`（14pt）；`font-semibold` 只用于
    卡片标题与品牌名。字号档位通过（粗体回退见“未验证”）。
  - **间距**：全部为 `1 / 1.5 / 2 / 2.5 / 3 / 4` 档；除控件最小高度 `min-h-5/6/7`（控件尺度本身）
    外没有字面量点值。
  - **对齐**：导航项、工具按钮、窗口按钮共享 40pt 带中心线；卡片标题与内容共享 `px-4` 内缩。
  - **颜色**：JSX 无 `#hex` / `rgb()`，全部走 token；浅色与深色截图里都没有旧调色板残留。
  - **hover / pressed / focus**：导航、按钮、图标按钮都有淡洗 + `scale-[0.97]` + 外侧焦点环。
    截图无法验证交互态，需在有显示机器的环境再确认。
  - **键盘与无障碍**：导航与按钮都建在 `nsh/base` 的 `Button` 上（Tab 可达、Enter/Space 激活）；
    图标按钮都有 `accessibilityLabel`，装饰图标 `accessibilityElementsHidden`；`App.test.tsx`
    用 `role:button[...]` 选择，名字不对就会失败。
- **未通过 / 未验证**：
  - `font-semibold` 在 Linux 宿主回退（`.SystemUIFont` 缺 600 字重），粗体实际没生效 → 待定字体族。
  - 7 个皮肤（`nsh`、`bettbox` 为新增）、7 个强调色、DPI 缩放、Windows 实机（窗口按钮 / 托盘 / 全局热键）均未验证。
  - 键盘 Tab 顺序与屏幕阅读器朗读未在本轮验证。

### 本轮（卡片语言统一到全部页面）

- 每个页面只用三种卡片形态中的一种：设置各段 / 统计各卡 / 工具箱工具 / 队伍卡 / 帮助目录用标题卡；
  主控台队列与主题包行用单行卡；主控台顶部卡、统计总览与右侧栏用数据卡。
- 卡片圆角 `radius-lg` → `radius-xl`（Bettbox 16 → 20），`EmptyState` 同档；标题卡的正文改由
  `CardBody` 统一内缩，页面不再各写 `p-3`/`p-4`。
- 页面级提示不再用带底色的卡片：`Notice` 一行贴在页面上（主题包的困难镜牢提示、各页的成功反馈）。
- 统计的 KPI 行换成 `Figure` 四列（与主控台顶部卡同一套数字排版）；“最近记录”去掉每行的描边框。
- 队伍卡把名字提到标题带，`#编号` 作注脚；空槽位改为居中的占位块（`bg-muted/25`）。P6 起改为「编队位图 + 已配置队伍卡」：标题是 `#NN 备注名`，注脚是轮换/日常/手动，空槽位是 8×5 图里的编号格（见「队伍页」）。
- 校验：`nsh check` / `nsh build` 无告警；`AHAB_BACKEND=mock nsh test nsh-app` 145 项全过；
  截图由 `nsh test nsh-app/visual.test.tsx --target windows` 写入 `artifacts/nsh/`（浅/深各一页）。
- 未验证 / 未覆盖：只在设置页看过 7 皮肤（现代 / 玻璃 / 档案 / 边狱 / 血雾 / nsh / Bettbox）的标题带；
  带图标或尾部控件的标题带只在 Bettbox 与边狱上看过（工具箱、帮助、统计图例）。

