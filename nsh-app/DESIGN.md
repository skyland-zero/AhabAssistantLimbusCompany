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

`theme.json`（默认皮肤，`nsh.json` 的 `"theme"`）与 `themes/skins.generated.ts` 由
`scripts/export_nsh_theme.py` 从 GPUI 调色板机械导出：

- 5 皮肤 × 浅/深 × 34 个颜色 token + 4 个装饰 token（`band-top` `band-bottom` `wash` `grain`）
  + 4 档圆角（`radius-sm` `radius` `radius-lg` `radius-xl`）。
- 6 个强调色不各存一份全表：`ACCENT_OVERRIDES` 只记「换强调色后变了的 token」
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

5 个皮肤共用同一套组件结构（同样的内边距、同样的 44pt 标题带），只换装饰语言；组件用
`currentDecor()` 分支，**不按皮肤 id 分支**：

| 皮肤 | decor | 卡片 | 标题带 | 空状态标记 | 分隔符 |
| --- | --- | --- | --- | --- | --- |
| 现代 | `plain` | `shadow-sm`，无边框（`border` token 透明） | 底部 `input` 发丝线 | 无 | 发丝线 |
| 玻璃 | `glass` | `shadow-lg` + 顶部 1px `hilite` | 顶部 `hilite` 边 | 抬起的方形芯片 | 发丝线 |
| 档案 | `archive` | 无阴影，靠留白 | 标题 + 向右延伸的 `decor-line` 发丝线 | 2px 横杠 | 金色花纹图 |
| 边狱 | `limbus` | 无阴影 + 四角 L 形角框 | 渐变带（`band-top`→`band-bottom`）+ 左侧 3px 竖条 + 污渍 | 封蜡图 | 金色花纹图 |
| 血雾 | `mist` | `shadow-md` + 顶部红色氛围 | 左侧 3px 竖条 + 底部 `border` | 细横杠 | 发丝线 |

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
| 卡片标题 | 14pt | `text-sm font-semibold` |
| 区块标题 | 16pt | `text-base font-semibold` |

间距：一律用 Tailwind 档位（`p-1` 4pt、`p-2` 8pt、`p-3` 12pt、`p-4` 16pt、
`gap-2`/`gap-3`/`gap-6`）。页面外框固定 `p-2.5`（10pt，对齐 GPUI 的 `page_root`）。

圆角：只用 `rounded-(--radius-sm)`（徽标、小部件）、`rounded-(--radius)`（控件）、
`rounded-(--radius-lg)`（卡片、弹层）、`rounded-(--radius-xl)`，以及 `rounded-full`（仅圆形：
状态点、开关滑块、头像）。

控件尺寸（`components/Button.tsx` 的 `SIZE`，一并决定高度、内边距、图标）：

| size | 高度 | 内边距 | 图标 | 用在 |
| --- | --- | --- | --- | --- |
| `xs` | `min-h-5` 20pt | `px-2` | 12 | 表格行内、密集工具栏 |
| `sm`（默认） | `min-h-5` 20pt | `px-3` | 13 | 普通按钮，等于 GPUI 的 21pt 控件 |
| `md` | `min-h-6` 24pt | `px-3` | 14 | 标题栏导航、图标按钮 |
| `lg` | `min-h-7` 28pt | `px-4` | 16 | 页面主操作 |

字高与控件高度都用 `min-h-*` 而不是固定 `h-*`：文字随系统字号放大时控件跟着长。

## 4. 组件契约

`components/` 是应用自己的 ui 层，`nsh/base` 只提供行为（按下、键盘、焦点、无障碍）。

| 文件 | 组件 | 说明 |
| --- | --- | --- |
| `Button.tsx` | `Button` / `IconButton` | 变体 `default` `primary` `secondary` `outline` `ghost` `danger` `link`，尺寸 `xs`–`lg`，状态 `disabled` `selected` |
| `Field.tsx` | `SettingList` / `SettingLine` / `SettingRow` | 设置页的标签/控件网格（行距 12pt，控件固定尾列） |
| `Segmented.tsx` | `Segmented` / `SwatchRow` | 分段选择（容器透明，选中的一段是 `secondary` 胶囊） |
| `Switch.tsx` | `Switch` | 开关（轨道 `input`→`primary`，滑块 16pt，`transition-transform`） |
| `Input.tsx` | `Input` | 单行输入（与开关同高的描边框） |
| `Select.tsx` | `Select` | 下拉选择，弹出层用统一浮层表面 |
| `Swatch.tsx` | `Swatch` | 强调色色块（28pt，选中环在色块外侧） |
| `Card.tsx` | `Card` / `CardHeader` / `Rule` | 卡片表面、44pt 标题带、发丝线 |
| `Badge.tsx` | `Badge` | 六种语义色，默认 `neutral` |
| `Icon.tsx` | `Icon` | Lucide SVG 资产 + `tintColor` |
| `Page.tsx` | `PageRoot` / `PageToolbar` | 页面外框与工具栏卡片 |
| `EmptyState.tsx` | `EmptyState` | 空/未就绪状态：说明 + 下一步 |

规则：

- 页面**只**组合 `components/` 与布局原语（`View` `Text` `Scroll` `List`），不直接引
  `nsh/base` 的控件，不写颜色、圆角、字号。
- 变体按**含义**选：`primary` 只给决策区里唯一的默认提交；工具栏里的普通动作是 `default`；
  `danger` 只给不可逆的破坏性提交；`link` 只给外部资源。
- 图标按钮必须有 `accessibilityLabel`，且是正方（`aspect-square`）。
- 焦点环在控件外侧：`focus-visible:outline-2 outline-offset-2 outline-ring`；祖先 `overflow-hidden`
  会裁掉它，布局要留出位置。

### 图标着色的已知限制

`nsh` 通过 `Image` 画 SVG，`tintColor` 是**值**而不是 CSS 的 `currentColor`，所以图标不能跟着
`text-*`/`hover:` 变色。当前做法：

- 变体自带图标 token（`Button` 的 `VARIANT[x].icon`），调用方可用 `iconColor` 覆盖；
- 图标跟随状态变化的场景（选中/未选中导航）用**变体切换**实现，即换 token；
- 纯 hover 的变色用**底色淡洗**代替（符合设计指南的“hover 是安静的淡洗”）。

若后续需要 hover 换图标颜色，用两个图标叠放 + `opacity` 过渡，或给 `Icon` 传一个 signal 驱动的
token；不要为此改 `nsh`。

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

### 动效清单（设置页，本轮）

| 变化 | 动效 |
| --- | --- |
| 导航项 hover / 按下 | 淡洗 + `active:scale-[0.97]`，`duration-150` |
| 导航项选中 | 换变体（`ghost`→`secondary`），颜色 `duration-150` 过渡 |
| 导航项点击跳段 | `scrollTo({ animated: true })`，原生滚动，非信号驱动 |
| 滚动改变当前段 | 高亮瞬时切换（不做淡入：滚动手势中再叠动画会晃） |
| 开关翻转 | 轨道 `transition-colors duration-150`，滑块 `transition-transform duration-150`，位移 16pt |
| 分段选择 | 选中段变色 `duration-150`，不位移、不缩放容器 |
| 强调色色块 | hover 透明度、按下 `scale-[0.92]`，选中环由 `outline` 立即出现（无动画：环是状态而非过程） |
| 皮肤卡片 | hover 边框变亮，按下 `scale-[0.99]`，选中 `border-2` + `ring` 色 |
| 反馈行 | 出现在卡片栈下方，颜色由 token 决定，无入场动画（一行文字，动效无信息量） |
| 减少动效 | 以上全部瞬时；颜色变化保留 |

### 动效清单（窗口壳）

| 变化 | 动效 |
| --- | --- |
| 导航项 hover | 底色淡洗 `hover:bg-accent`，`duration-150`，不改变尺寸 |
| 导航项按下 | `active:bg-secondary` + `active:scale-[0.97]`，`duration-150` |
| 导航项选中 | 变体切换（`ghost`→`primary`），颜色 `duration-150` 过渡；不滑动指示条（标题栏本身是固定 6 项） |
| 图标按钮 hover/按下 | 同导航项（`ghost` 的淡洗 + 收缩） |
| 窗口按钮 hover | 淡洗；关闭按钮 `hover:bg-danger` |
| 执行状态点 | 空闲不显示；运行 `success`、暂停 `warning`。GPUI 的呼吸动画默认关闭（`ENABLE_STATUS_DOT_BREATHING = false`），这里同样不做动画 |
| 外观切换 | 图标交换（sun/moon/monitor）即时替换 + 颜色过渡；不做交叉淡入（6px 图标，动效无信息量） |
| 页面切换 | 无过渡：切换后内容整块替换（P3 迁入页面时再评估，需保持 GPUI 观感） |
| 减少动效 | 以上全部瞬时；底色变化保留（颜色不是动画） |

组件对照页（`pages/Gallery.tsx`）每个变体/尺寸/状态都是静态展示，无动效。

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

- `nsh check` / `nsh build` 通过（无未知 class 告警）；`nsh test` 7 项全过（5 条 TSX + 2 条 Rust）。
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
  - 5 个皮肤、6 个强调色、DPI 缩放、Windows 实机（窗口按钮 / 托盘 / 全局热键）均未验证。
  - 键盘 Tab 顺序与屏幕阅读器朗读未在本轮验证。

