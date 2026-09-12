# GPUI 主题皮肤与界面优化实施方案

> 状态：**已实施并截图复核**（缺陷 D1–D15 全部落地，5 套皮肤上线，`cargo test` 184 项全绿，30 张基线截图已归档）
> 范围：`gpui-app` 视觉层（`components/style`、`components`、`shell`、`app/render`、`pages/settings`、`pages/home`）
> 约束：不改动持久化契约字段名、不改动页面布局结构、不改动 IPC 与业务逻辑

## 实施结果速览

| 项 | 结果 |
|---|---|
| 缺陷 | D1–D15 全部修复（D9–D15 为实施期与截图复核阶段新发现） |
| 皮肤 | 2 → **5**（现代 / 玻璃 / 档案 / 边狱 / 血雾），全部支持明暗双方案 |
| 主题组合 | 2×2×6 = 24 → **5×2×6 = 60** |
| 字号 | 17 种字面量 → **6 档**，并由源码扫描测试强制 |
| 圆角 | 93 处散落字面量 → `Shape` + `ShapeExt`，组件层零硬编码（测试强制） |
| 资源 | 5 个位图 → 3 个，2 个会被拉伸变形的位图改为程序化几何 |
| 测试 | 全仓 **159 → 184** 项，无删除；其中主题/视觉测试 9 → 21 项 |

---

## 1. 目标

1. 修掉现有主题系统的 9 项缺陷（含 1 项实施期新发现的对比度缺陷）。
2. 把皮肤从 2 套扩展到 5 套，并让「边狱」皮肤真正支持浅色方案。
3. 把视觉常量（圆角 / 描边 / 阴影 / 字号）从散落的字面量收敛为 token，使新增主题不再需要逐个页面改数值。
4. 用可执行的自动化断言（token 唯一性、映射无歧义、文字对比度 AA）替代人工肉眼验收。

**明确不做**（见 §8）：强调色运行时推导、OS 级 Mica/模糊窗口背景、控件结构重排。

---

## 2. 现状盘点

### 2.1 三轴架构（保留）

| 轴 | 取值 | 落点 |
|---|---|---|
| 方案 | Light / Dark / System | `Palette.scheme`，`theme::resolve_scheme` |
| 强调色 | 6 个硬编码表 | `AccentTokens::for_scheme`（`style/tokens.rs`） |
| 皮肤 | 现代 / 边狱 | `Palette::for_skin`（`style/palette/mod.rs`） |

`Palette` 是 33 个语义 token 的 `Copy` 值对象，页面通过线程局部快照 `current_render_palette()`
读取，`AhabApp::render` 每帧写入。**这个架构是正确的，本方案不推翻它，只做加法。**

### 2.2 页面消费方式

- 新建代码：`palette_rgb(token)` / `*_with_palette(...)`。
- 存量代码（约 150 处 / 25 个文件）：`render_rgb(TEXT)` 形式，靠 `style/runtime.rs` 中
  `token_for_legacy_hex` 的**十六进制字面量表**反查语义 token。

---

## 3. 缺陷清单与修复

### D1 边狱皮肤下「浅色/深色」是空转的（功能性缺陷）

**证据**：`style/palette/mod.rs` 的 `for_skin` 在 `match scheme` 之前 `return`（当时的 `palette.rs`），limbus 分支全部硬编码深色。
选「浅色 + 边狱」只有 6 个强调色圆点会变，界面纹丝不动 —— 控件在撒谎。

**修复**：为 limbus 增加 light 分支（羊皮纸 / 牛血红 / 黄铜），使其成为真正的双方案皮肤；
`Shape`/`Decor` 不随方案变化，保证几何语言稳定。

**验收**：`Palette::for_skin(Light, A, Limbus) != Palette::for_skin(Dark, A, Limbus)`，且
两者 `background` 的感知亮度差 > 0.5。

---

### D2 `tagband.png` 位图被无脑拉伸（观感最大扣分项）

**证据**：`pages/settings/mod.rs` 把 800×72 的血污金属位图 `.w_full().h_full()` 铺满卡头。
血污黑斑被横向拉成椭圆，铆钉间距随窗口宽度漂移，1400px 宽即糊。

**修复**：删除位图卡头，改为**程序化几何装饰**（分辨率无关、可随强调色变色）：

```
limbus_card_header(title, palette)
├─ 背景：linear_gradient(180°, #4A1010 → #2A0C0C)
├─ 4 个固定坐标的暗色"血污"椭圆（低透明度，位置写死不随宽度变化）
├─ 底部 1px decor_line 分隔线
├─ 左侧 35×2px 强调色方块（替代原铆钉视觉锚点）
└─ 标题：decor_line 色 + SEMIBOLD
```

同尺寸槽位（44px）保持不变，**不产生布局位移**。

---

### D3 强调色与边狱金色框架互相打架

**证据**：limbus 分支硬编码 `ring = #D8A800`、`accent_foreground = #E8C43C`，却仍放开 6 种
brand 强调色。选「紫罗兰 + 边狱」= 紫按钮 + 金角框 + 金标题 + 金分隔线，四色混战。

**修复**：`AccentTokens` 新增 `decor` 字段（框架/线条专用色调，逐强调色给定），
limbus / mist / archive 皮肤的 `ring`、`accent_foreground`、角框、卡头线条全部改用
`palette.decor_line`。黄铜语义被保留（`LimbusBrass.decor.dark == 0xD8A800`，与改造前一致），
其余强调色自动获得自洽的框架色。

设置页对非黄铜强调色在边狱皮肤下加「签名配色为黄铜」提示徽标。

---

### D4 几何不一致：93 处 `rounded_*`，仅 17 处感知皮肤

**证据**：`tab_surface_with_palette` 永远 `rounded_md`；`switch` 永远 `rounded_full`；标题栏导航永远
`rounded_md`。边狱皮肤下卡片直角、开关胶囊、标签页 6px 圆角。

**修复**：新增 `Shape { radius_sm, radius_md, radius_lg, radius_xl, pill: bool, border_width: f32, shadow: ShadowLevel }`，
`Palette.shape` 承载。`skin_rounded()` 改为读 `shape`，并新增 `shape_rounded(div, radius)` 供
tabs / switch / select / text_input / number_stepper / 标题栏导航统一调用。

`pill` 语义：仅开关保持胶囊（人体工学与可点击面积），其余全部跟随皮肤圆角。

---

### D5 现代皮肤深色「平」到没有层次

**证据**：dark `border` = `rgba(0)`；非执行态首页任务卡边框也是 `rgba(0)`；全项目 `shadow_*` 仅 4 处。

**修复**：
- dark `border` → `rgba(0xFFFFFF1A)`，卡片与输入框自动获得 1px 可见描边。
- 新增 `ShadowLevel { None, Subtle, Raised, Floating }`，`card_with_state` 按 `shape.shadow` 施加真实投影。
- 新增 `hilite` token（顶部 1px 高光），玻璃皮肤专用。

---

### D6 死资源：`Frame` / `Divider`

**证据**：`ThemeAsset::Frame`、`ThemeAsset::Divider` 在 `gpui-app/src` 内引用数均为 0。

**修复**：
- `Tagband`（位图卡头）随 D2 一起删除。
- `Frame` 删除（铆钉边框无法在任意尺寸下不变形，程序化角框已覆盖该语义）。
- `Divider` **保留并真正使用**：以固定等比尺寸（160×3.47px，等比缩放 1200×26 原始比例）
  用作文档/帮助页的章节分隔与卡头下饰线，**永不拉伸**。
- 同步更新 `scripts/generate_limbus_theme.py`、`resources/assets/themes/limbus/README.md`、
  `ATTRIBUTION.md` 与 `assets.rs` 的枚举/`ALL`/`file_name`/`relative_path`/`embedded`。

---

### D7 命名冲突：顶栏「主题包」vs 外观「风格」

**证据**：`Page::ThemePacks` = 游戏内镜牢主题包（`pages/theme_packs/`），视觉主题却藏在
「设置 → 外观 → 风格」。同名不同页，用户找不到换肤入口。

**修复**：
- 导航文案 `nav_themes`：「主题包」→「镜牢主题包」/ "Theme Packs" → "Mirror Packs"。
- 外观卡片标签 `风格` / `Skin` → `界面皮肤` / `Interface Skin`。
- 外观卡片内新增一行说明，指路「镜牢主题包」页面。

---

### D8 间距 / 字号 token 形同虚设

**证据**：`SPACE_*` 使用 2 次、`FONT_*` 使用 1 次；实际有 17 种字面量字号
（9、9.5、10、10.5、11、12、13、14、15、16、18）。

**修复**：
- 字号收敛为 6 档：`FONT_2XS=9.5, XS=10, SM=11, MD=12, LG=14, XL=16`（新增 `FONT_2XS`）。
- 一次性数值（10.5、13、15、18）归一化到最近档位：
  10.5→10，13→12（正文）或 14（标题），15→14，18→16。
- 新增 `style/size.rs`（或并入 `runtime.rs`）暴露统一常量；`FONT_*` / `SPACE_*` 增加旁注，
  在本次改动触及的文件中优先使用 token 而非字面量。

---

### D9 [新发现] 边狱皮肤下破坏性按钮文字对比度不足

**证据**：`ButtonVariant::Destructive` 前景色取 `palette.brand_foreground`。边狱 + 黄铜时
`brand_foreground = #17120A`（深），`danger = #B92828`，对比度 ≈ **2.4:1**（AA 要求 4.5:1）。

**修复**：新增 `Palette.danger_foreground`（各皮肤显式给定，均为浅色），破坏性按钮改用它。

---

### D10 [加固] 存量十六进制映射表的静默错配风险

**证据**：`token_for_legacy_hex` 有约 60 条字面量映射，皮肤从 2 套扩到 5 套后，
「某个字面量在新皮肤下语义变了」无法被编译器发现。

**修复（本轮做到可断言、不追求一次性迁移完）**：
1. 兼容常量 `TEXT/SURFACE/ACCENT/...` 从「疑似普通颜色」改为**显式 tag 值**
   （`TAG_BASE | 1..9`，落在 `render_rgb` 的 24 位掩码内），`render_rgb` 优先按 tag 精确解析。
   彻底消除「常量靠猜」这一层歧义。
2. 新增不变量测试：
   - 所有 tag 唯一，且落在保留区间内；
   - 每个皮肤 × 方案 × 强调色下 9 个兼容常量都能解析为**非透明**颜色；
   - tag 路径保留调用方传入的 alpha；
   - 未知字面量原样透传。
3. 在 `runtime.rs` 中写明该表为**过渡层**，新代码禁止新增字面量。

---

### D11 [实施期新发现] 强调色前景色对比度不足（同一缺陷类）

**证据**：`AccentTokens` 的 `brand_foreground` 硬编码为 `#FAFAFA`（亮色方案）或
`#17120A`（仅黄铜）。实测对比度：

| 强调色 / 方案 | 白字对比度 | AA(4.5:1) |
|---|---|---|
| amber / light | 3.05:1 | ✗ |
| emerald / light | 3.61:1 | ✗ |
| crimson / dark | 3.42:1 | ✗ |
| blue / dark | 2.24:1 | ✗ |
| amber / dark | 1.60:1 | ✗ |
| emerald / dark | 1.84:1 | ✗ |
| violet / dark | 2.54:1 | ✗ |
| brass / dark | 2.08:1 | ✗ |

**修复**：`brand_foreground` 改为**逐强调色 × 方案挑选**（不是硬编码）。
- 亮色方案：crimson/blue/violet/brass 保留浅色字；amber/emerald 改用深色字。
- 暗色方案：全部改用深色字（与既有 `primary` / `primary_foreground`
  「亮底 + 深字」的处理方式一致）。

⚠️ **这是本轮唯一会改变「现代 + 深色」默认观感的改动**：深色方案下
按钮由「亮色底 + 白字」变为「亮色底 + 深字」。回退方式：只改
`AccentTokens::for_scheme` 一处表格即可。

---

### D12 [实施期新发现] 皮肤背景图层被页面根节点完全遮住

**证据**：`app/render.rs` 在根节点顺序上先画皮肤背景图层，再画页面容器；而
`components/layout.rs::page_root()` 与 `pages/home/mod.rs` 的页面根节点都铺了
**不透明的 `palette.background`**，且尺寸为 `size_full()` / `flex_1 + h_full`。
因此 `limbus` 的 `bg.png` 星云底图**自上线以来一直没有任何可见效果**——
每帧都在画，但永远被盖住。本轮新增的 `mist` 暗角、`glass` 强调色洗色也同理。

**修复**：页面根节点不再自画背景（背景色由根窗口 `Div` 统一提供）；
背景图层以「顶部固定百分比的渐变洗色 + 全局低透明纹理」的形式绘制，
避免整页染色。

**回归防线**：新增源码扫描测试
`page_roots_do_not_repaint_the_window_background`，禁止
`.bg(palette_rgb(current_render_palette().background))` 回到 `src/pages`，
并单独断言首页根节点不再铺底。

---

## 4. 主题清单与设计规范

### 4.1 皮肤矩阵（5 皮肤 × 2 方案 × 6 强调色）

| id | 中文 | English | 语言 | 方案语义 |
|---|---|---|---|---|
| `default` | 现代 | Modern | 扁平平铺、圆角、弱描边 | 保持现有值（仅按 D5 修 border/shadow） |
| `glass` | 玻璃 | Glass | 分层半透、发光边缘、大圆角、真投影 | 深浅双方案 |
| `archive` | 档案 | Archive | 纸墨、细发丝线、零阴影、靠留白 | 深浅双方案 |
| `limbus` | 边狱 | Limbus | 冷黑/羊皮纸 + 暗红 + 黄铜、直角、角框、血污 | **新增浅色**（D1） |
| `mist` | 血雾 | Mist | 近黑 + 红色暗角 + 颗粒、锐角、强调色仅用于激活态 | 深浅双方案 |

`SkinId::parse` 增加别名：`modern`→Default，`aero`/`glass`→Glass，`paper`/`archive`→Archive，
`blood`/`mist`→Mist，`limbus`→Limbus，`_`→Default（保持向前兼容）。

### 4.2 `Shape` 定义

| 皮肤 | radius_sm | radius_md | radius_lg | radius_xl | pill | border_width | shadow |
|---|---|---|---|---|---|---|---|
| Default | 4 | 6 | 8 | 12 | ✓ | 1 | Subtle |
| Glass | 6 | 10 | 14 | 18 | ✓ | 1 | Floating |
| Archive | 2 | 4 | 6 | 8 | ✓ | 1 | None |
| Limbus | 0 | 0 | 0 | 0 | ✓ | 1 | None |
| Mist | 0 | 2 | 4 | 6 | ✓ | 1 | Raised |

### 4.3 `Decor` 定义（渲染语言，取代散落的 `is_limbus()`）

| 值 | 皮肤 | 渲染行为 |
|---|---|---|
| `Plain` | Default | 平铺卡面 |
| `Glass` | Glass | 卡面顶部 1px `hilite` 高光 + 强调色微光投影 |
| `Archive` | Archive | 1px 发丝描边，无阴影，章节细线（用 `Divider` 资产） |
| `LimbusFrame` | Limbus | 四角金色 L 角框 + 程序化血污卡头 + 印章空状态 |
| `Mist` | Mist | 全窗口红色暗角 + 低透明斜纹颗粒层；仅激活/聚焦态用强调色 |

### 4.4 新增 token（`Palette` 扩展）

| token | 用途 |
|---|---|
| `decor_line` | 框架 / 线条专用色（= `AccentTokens.decor`），替代硬编码金色 |
| `danger_foreground` | 破坏性按钮 / 危险徽标前景（修 D9） |
| `scrim` | 模态遮罩底色 |
| `hilite` | 顶部高光（Glass 专用，其余透明） |
| `glow` | 强调色微光投影色（Glass 专用，其余透明） |
| `shape` | `Shape` 结构体 |
| `decor` | `Decor` 枚举 |

### 4.5 皮肤配色（关键值）

**Glass / Dark** — `bg #0A0D14`、`card rgba(FFFFFF14)`、`popover rgba(161B24FA)`、
`fg #EEF2F8`、`muted_fg #9FB0C6`、`border rgba(FFFFFF24)`、`input rgba(FFFFFF26)`、
`hilite rgba(FFFFFF33)`、`glow <decor>@40%`

**Glass / Light** — `bg #E9EEF6`、`card rgba(FFFFFFCC)`、`fg #101828`、`muted_fg #5A6B82`、
`border rgba(10182824)`、`input rgba(1018281F)`

**Archive / Light** — `bg #FBFAF7`、`card #FFFFFF`、`fg #16161A`、`muted #F4F2EC`、
`muted_fg #6B6A63`、`border #DEDACD`、`input #DEDACD`、`danger #A8181F`

**Archive / Dark** — `bg #121215`、`card #1A1A1F`、`fg #E8E6E0`、`muted_fg #9A978E`、
`border #2E2E36`、`input #33333C`

**Limbus / Dark** — 保持现有值不变（`bg #0B0A0E`、`card #1B1114`、`border #462C2C`…），
仅把 `ring`/`accent_foreground` 改为 `decor_line`，并补 `danger_foreground`。

**Limbus / Light（新）** — `bg #ECE3D2`（羊皮纸）、`fg #241211`、`card #F6EFE1`、
`popover #FBF6EA`、`secondary #E2D7C1`、`muted #E6DCC8`、`muted_fg #7A6A52`、
`accent_surface #D9CBB0`、`border #C9B795`、`input #B9A37C`、
`success #14713F`、`warning #8A5A10`、`danger #9B1C1C`、`danger_foreground #FDF7EC`

**Mist / Dark** — `bg #07060A`、`card #120E12`、`fg #EBE3D8`、`muted_fg #A08C8A`、
`border #3A1F22`、`input #4A2620`、`danger #FF5A5F`、`scrim rgba(05040AD9)`

**Mist / Light** — `bg #F3EEEA`、`card #FDF9F6`、`fg #221418`、`muted #ECE4DE`、
`border #DBCBC5`、`danger #B81B22`

### 4.6 `AccentTokens.decor` 取值

| 强调色 | Light decor | Dark decor |
|---|---|---|
| crimson | `#A92B42` | `#E05A72` |
| blue | `#1D4ED8` | `#60A5FA` |
| amber | `#9A4708` | `#FBBF24` |
| emerald | `#047857` | `#34D399` |
| violet | `#6D28D9` | `#A78BFA` |
| limbus-brass | `#8A6413` | `#D8A800` ← 与改造前完全一致 |

amber 的亮色 decor 从品牌色 `#B45309` 再压暗到 `#9A4708`：作为角框/分隔线
需要落在浅色底面（羊皮纸 `#ECE3D2`、雾白 `#F3EEEA`）上达到 4.5:1。

### 4.7 `AccentTokens.brand_foreground` 取值（修 D11）

| 强调色 | Light | Dark |
|---|---|---|
| crimson | `#FAFAFA` | `#2A0C12` |
| blue | `#FAFAFA` | `#0A1A30` |
| amber | `#241705` | `#2A1C04` |
| emerald | `#041A10` | `#04170F` |
| violet | `#FAFAFA` | `#150A2E` |
| limbus-brass | `#FAFAFA` | `#17120A` ← 不变 |

---

### 截图复核发现的额外问题（D13–D15）

以上 D1–D12 完成后逐皮肤截图复核，又发现 3 处只有在真实渲染下才暴露的问题：

#### D13 `bg.png` 把底部警示斜纹与金线烘焙进了背景底板

**证据**：`make_bg()` 在图像底部画了金色横线 + 暗红/黑警示斜纹。但该图以
`size_full()` 拉伸到窗口，于是警示纹浮在**窗口 97% 高度处**（不同窗口尺寸位置还不同），
且被纵向拉伸——在首页截图里直接横穿任务卡区域。

**修复**：从背景底板中移除所有页面装饰元素。装饰属于控件，不属于被拉伸的底板。
（该问题在 D12 修复前不可见，因为底板一直被页面根节点遮住。）

#### D14 边狱浅色被冷黑底板“弄脏”

**证据**：`bg.png` 是深色冷黑星云板。以 0.5 不透明度叠在羊皮纸 `#ECE3D2` 上，
实测被压成灰褐 `#7B766F`（像素采样：0.5×(11,10,14) + 0.5×(236,227,210) = (123,118,111)），
整页发脏。同时卡头色带上的黑色血污在羊皮纸上呈现为**灰白色 “涂黑条”** 而非血污。

**修复**：
- 浅色方案不再绘制 `bg.png`（深色底板无法“反转”成浅色底板，宁可不画）。
- 浅色方案的卡头血污改用半透明牛血红 `rgba(4A1010, 0x29)`，而不是黑色。

#### D15 血雾颗粒纹理过强

**证据**：`pattern_slash` 以 4% 白色、1px/6px 间距铺满全窗口，在近黑底上可读出
明显的斜向织物纹理，而非“胶片颗粒”。

**修复**：降到 2% 白色。

---

## 5. 架构改动

### 5.1 类型（`components/style/tokens.rs`）

```rust
pub enum SkinId { Default, Glass, Archive, Limbus, Mist }
impl SkinId {
    pub const ALL: [Self; 5];
    pub const fn as_str(self) -> &'static str;
    pub const fn name_zh(self) -> &'static str;
    pub const fn name_en(self) -> &'static str;
    pub const fn blurb_zh(self) -> &'static str;   // 设置页预览卡副标题
    pub const fn blurb_en(self) -> &'static str;
    pub fn parse(value: &str) -> Self;
}

pub enum Decor { Plain, Glass, Archive, LimbusFrame, Mist }

pub enum ShadowLevel { None, Subtle, Raised, Floating }

pub struct Shape {
    // u32, not f32: `Palette` derives `Eq`, so every token must be `Eq`.
    pub radius_sm: u32, pub radius_md: u32,
    pub radius_lg: u32, pub radius_xl: u32,
    pub pill: bool, pub border_width: u32, pub shadow: ShadowLevel,
}
impl Shape { pub const fn for_skin(skin: SkinId) -> Self; }

pub struct AccentTokens { brand, brand_hover, brand_light, brand_foreground, decor }
```

### 5.2 `Palette`（`style/palette.rs`）

新增字段：`decor_line, danger_foreground, scrim, hilite, glow, shape, decor`。
`for_skin` 重构为「公共骨架 + 每皮肤一节」的显式 `const fn`，五个皮肤各自给出方案分支。

便捷谓词：`uses_frame_decor()`、`uses_divider()`、`is_glass()`、`is_archive()`、`is_mist()`。

### 5.3 渲染器（`components/base.rs`）

| 函数 | 改动 |
|---|---|
| `button_with_palette` | 圆角读 `shape`；Destructive 前景改 `danger_foreground`；Glass 加微光 hover |
| `badge_with_palette` | 圆角读 `shape.radius_sm` |
| `card_with_state` | 圆角读 `shape`；按 `shape.shadow` 施加投影；`Decor::LimbusFrame` 加角框；`Decor::Glass` 加顶部 `hilite`；`Decor::Archive` 用发丝描边 |
| `decor_corner_brackets` | 由 `limbus_corner_brackets` 更名并泛化（颜色取 `decor_line`） |

### 5.4 存量兼容层（`style/runtime.rs`）

- `skin_rounded(div, large)`：改读 `palette.shape`。
- 新增 `shape_rounded(div, radius: f32) -> Div`、`apply_card_shadow(div, level) -> Div`。
- 兼容常量改为 tag 值；`render_rgb` / `render_rgba` 优先解析 tag 区。
- 映射表顶部标注为过渡层。

### 5.5 全窗口装饰（`app/render.rs`）

`limbus_background()` → `skin_background(palette)`：
- `LimbusFrame`：`bg.png` @ 0.5（行为不变）
- `Mist`：`linear_gradient` 顶部暗红 → 透明的暗角层 + `pattern_slash` 低透明颗粒层
- `Glass`：顶部暗角的极轻微 accent 渐隐
- 其余：空 div

### 5.6 设置页（`pages/settings/`）

- `settings_card` 卡头按 `Decor` 分支：Limbus 用程序化血污条，Archive 用 `Divider` 细线，
  Glass 用顶部高光 + 更重的卡投影，其余保持现状。
- `appearance_card` 的「界面皮肤」改为 **2×3 预览卡网格**：每张卡用该皮肤 `Palette`
  纯代码渲染一个 mini 界面（顶栏条 + 卡头 + 一个按钮 + 一个开关），零新资源，
  实时反映当前强调色；选中态用 `ring` 描边 + 强调色勾选角标。
- 新增说明行，指路「镜牢主题包」页面（D7）。
- 非黄铜强调色 + 边狱皮肤时，强调色行右侧显示「签名配色：黄铜」提示徽标。

### 5.7 资源（`assets.rs` / 生成脚本）

- `ThemeAsset` 由 5 项降为 3 项：`Bg`、`Divider`、`Seal`（删除 `Tagband`、`Frame`）。
- 删除对应 PNG；`generate_limbus_theme.py` 同步停止生成，README/ATTRIBUTION 表格更新。

### 5.8 模块拆分（满足 `AGENTS.md` §3.2 行数阈值）

本次改动把三个文件推过了 500 行上限，按项目约定拆为子模块，并通过 facade / `pub use`
保持调用方路径稳定（所有页面调用点无需改动）：

| 拆分前 | 拆分后 | 行数 |
|---|---|---|
| `components/style/mod.rs` 632 | `style/mod.rs`（纯净入口） | 35 |
| | `style/tests/{mod,contrast,scale,support}.rs` | 222 / 138 / 108 / 134 |
| `components/style/palette.rs` 590 | `style/palette/mod.rs`（结构体 + API） | 144 |
| | `style/palette/skins.rs`（皮肤 token 静态表） | 464 |
| `components/base.rs` 558 | `components/base.rs`（表面原语） | 290 |
| | `components/chrome.rs`（卡头 / 角框 / 分隔线） | 278 |
| `pages/settings/cards/appearance.rs` 504 | `appearance.rs`（选项行） | 205 |
| | `cards/skin_preview.rs`（预览缩略图渲染） | 308 |

`style/palette/skins.rs` 是 5 皮肤 × 2 方案的显式 `Palette` 字面量表（十个 `const` 分支），
属 §3.2 中「纯静态资源表」例外，已在文件头注明。它保持单文件以便逐皮肤横向审阅色面。

---

## 6. 文件级改动清单

**核心（必改）**
- `gpui-app/src/components/style/tokens.rs`
- `gpui-app/src/components/style/palette/mod.rs`
- `gpui-app/src/components/style/palette/skins.rs`
- `gpui-app/src/components/style/runtime.rs`
- `gpui-app/src/components/style/mod.rs`（导出）
- `gpui-app/src/components/style/tests/**`（回归测试）
- `gpui-app/src/components/base.rs`
- `gpui-app/src/components/chrome.rs`
- `gpui-app/src/components/mod.rs`（导出）
- `gpui-app/src/components/controls/{tabs,switch,select,inputs,slider}.rs`
- `gpui-app/src/components/overlays.rs`
- `gpui-app/src/app/render.rs`
- `gpui-app/src/shell/title_bar/mod.rs`
- `gpui-app/src/pages/settings/mod.rs`
- `gpui-app/src/pages/settings/cards/appearance.rs`
- `gpui-app/src/pages/settings/cards/skin_preview.rs`
- `gpui-app/src/pages/home/{cards,panel}.rs`
- `gpui-app/src/components/layout.rs`（页面根节点不再自铺底）
- `gpui-app/src/assets.rs`
- `gpui-app/src/i18n/{zh_cn,en_us}.rs`（导航与皮肤文案）

**删除**
- `gpui-app/src/shell/sidebar.rs`（未在任何 `mod` 声明中注册的死文件，且硬编码旧色值）

**次要（字号归一化，随改动触及）**
- `gpui-app/src/pages/home/**`、`pages/teams/**`、`pages/help/mod.rs`、
  `pages/toolbox.rs`、`pages/resources.rs`、`pages/theme_packs/**`

**资源与文档**
- `gpui-app/resources/assets/themes/limbus/`（删 2 图，更新 README/ATTRIBUTION）
- `scripts/generate_limbus_theme.py`
- `docs/gpui主题皮肤与界面优化实施方案.md`（本文件）

---

## 7. 验收标准

### 7.1 自动化（`cargo test`）——已成组

1. **皮肤稳定性**：`SkinId::ALL` 的 `as_str` / `parse` 往返一致，别名（`modern`/`aero`/
   `paper`/`blood`）可读，未知值回落 `Default`。
2. **方案可分**：5 皮肤 × 6 强调色下，`Light` 与 `Dark` 的 `background` 必须不同，
   且相对亮度差 > 0.35。
3. **几何单调**：每皮肤 `Shape::for_skin` 的 `radius_sm ≤ md ≤ lg ≤ xl`。
4. **装饰一一对应**：5 个 `Decor` 值与 5 个皮肤一一映射，无重复共享。
5. **对比度 AA**：5 皮肤 × 2 方案 × 6 强调色，断言 11 对文字/底面配对 ≥ 4.5:1
   （含 `danger_foreground`/`danger` 与 `brand_foreground`/`brand` 两组回归防线）；
   边狱卡头标题额外对程序化色带单独断言。
6. **tag 不变量**：tag 唯一、落在保留区间、每皮肤下 9 个兼容常量解析非透明、
   且 tag 路径保留调用方 alpha；未知字面量原样透传。
7. **规模守卫**：源码扫描断言（a）无 6 档之外的字号字面量，
   （b）`src/components` 下无 `.rounded_sm/md/lg/xl()` 硬编码。
8. **既有测试全绿**：`theme.rs`、`assets.rs`、`components/mod.rs` 等
   现有断言随新签名同步更新，无断言被删除。

### 7.2 人工（截图）

用 `gpui-app/scripts/capture_visual.ps1` 生成 **5 皮肤 × 2 方案** 的主控台 / 设置 / 队伍截图，
归档到 `artifacts/visual/skins/<skin>-<scheme>/`，逐项核对：

- 无位图拉伸变形（血污、栅格、边框均等比或程序化）；
- 卡片层级可见（有投影或描边，不再糊成一片）；
- 圆角语言统一；
- 强调色与框架色不冲突；
- 明暗切换在 5 个皮肤下都有可见变化；
- 中文/英文均无截断（导航改名后长度变化）。

---

## 8. 明确不做 / 后续可选

| 项 | 原因 |
|---|---|
| 强调色运行时推导（OKLCH + WCAG 自动挑前景） | 需要把 `AccentId` 从枚举改为结构体，牵动 `AppSettings.accentId` 契约与全部测试；视觉收益低于本轮 5 皮肤，单独立项 |
| 自定义 hex 强调色与屏幕取色 | 依赖上一项 |
| OS 级 Mica / 窗口模糊背景（`window.set_background_appearance(Blurred/MicaBackdrop)`） | API 存在且可运行时切换，但 Win10 与不支持 Mica 的环境会退化为「窗口全透明」，在无法实机验证的前提下默认不启用。Glass 皮肤改用「不透明底 + 分层半透卡面 + 顶部高光 + 强调色微光」，效果确定且可截图回归。后续可作为实验开关（`AHAB_MICA=1`）另行验证 |
| 页面布局重排（Bento 首页等） | 超出主题范围，且会破坏既有截图基线 |
| 150 处 `render_rgb` 存量调用一次性迁移 | 高风险机械改动；本轮改为 tag 化 + 不变量测试，把「静默错配」降级为「编译期/测试期可发现」，迁移按文件渐进 |

---

## 9. 实施顺序（已完成）

| # | 步骤 | 状态 |
|---|---|---|
| 1 | `tokens.rs`：`SkinId`/`Decor`/`ShadowLevel`/`Shape`/`AccentTokens.decor` | ✅ |
| 2 | `palette.rs`：5 皮肤 × 2 方案 + 新 token | ✅ |
| 3 | `style/mod.rs` 不变量与对比度测试 | ✅ |
| 4 | `runtime.rs`：`shape_rounded`/`ShapeExt`/`apply_card_shadow`/tag 化兼容常量 | ✅ |
| 5 | `base.rs` + `controls/*`：几何与装饰统一 | ✅ |
| 6 | `app/render.rs`：`skin_background` | ✅ |
| 7 | `overlays.rs` / `title_bar` / `home/*`：装饰接线 | ✅ |
| 8 | `pages/settings/*`：卡头改造 + 预览卡皮肤选择器 | ✅ |
| 9 | `assets.rs` + 生成脚本 + 资源清理 | ✅ |
| 10 | i18n 文案与字号归一化 | ✅ |
| 11 | `cargo test` + `cargo clippy` + `cargo fmt` | ✅ |
| 12 | 逐皮肤截图归档到 `artifacts/visual/skins/` | ✅（30 张，5 皮肤 × 明暗 × 首页/设置/队伍） |
| 13 | 截图复核发现并修复 D13–D15 | ✅ |
| 14 | 按 §3.2 行数阈值拆分超长模块 | ✅（见 §5.8） |

## 10. 验证命令

```pwsh
cd gpui-app
cargo fmt --check
cargo clippy --all-targets
cargo test

# 5 皮肤 × 2 方案的视觉基线（需图形环境）
./scripts/capture_visual.ps1 -Skins default,glass,archive,limbus,mist `
    -Pages home,settings,teams -OutputDirectory ../artifacts/visual/skins
```

`cargo test` 中的主题测试会逐项验证：皮肤 id 往返、明暗可分、几何单调、
装饰一一对应、tag 唯一性、6 档字号、组件层无硬编码圆角、页面根节点不自铺底，以及
**5×2×6=60 组配色下 11 对文字/底面配对的 WCAG AA 对比度**。

## 11. 风险与回退

| 改动 | 影响面 | 回退方式 |
|---|---|---|
| 暗色方案 `brand_foreground` 改深字（D11） | 默认皮肤深色下按钮对比观感 | 改 `AccentTokens::for_scheme` 一处表格 |
| 深色 `border` 从透明改为 `rgba(FFFFFF1A)`（D5） | 所有深色卡片多出 1px 描边 | `Palette` 中恢复 `ColorToken::rgba(0)` |
| 卡头统一为 44px 固定高 | 现代皮肤卡头 +2px | `card_header` 的 `Decor::Plain` 分支恢复 `py_3` |
| 导航「主题包」→「镜牢主题包」 | 文案长度变化 | i18n `nav_themes` 一处 |
| `seal-red.png` 重新生成（随机流变化） | 装饰印章图案 | `git checkout` 该文件（不影响代码） |
| 删除 `tagband.png` / `frame.png` | 无 | 无需回退：已由程序化几何替代，效果不可退回位图 |
| `bg.png` 移除底部警示斜纹（D13） | 背景底板 | 重跑生成脚本（已移除的代码不会回来） |
| 边狱浅色不绘制背景底板（D14） | 边狱浅色 | `skin_background` 去掉 `is_dark()` 分支 |
