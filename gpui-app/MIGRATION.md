# GPUI Kit 迁移：进度与剩余工作

本文档既记录已完成的部分，也给出剩下那一项的确切做法与已知陷阱。
分析结论全部来自实际尝试（或标注为「尚未实测」），不是推测。

**接手方式**：先看「当前进度」与「下次开工顺序」两节。
只剩 Select 一项、且它是取舍而不是技术问题；其余章节的价值在于**已踩过的坑**：
每一节的陷阱都是实际撞过并已解决的（弹窗 builder 不能读 app、子视图却可以、
静态控件需要句柄、截图工具的三种假象）。每次改完照「验证清单」跑一遍，
弹窗/侧栏类改动补一个 `VisualState` 截图。

---

## 当前进度（截至 `6db4e6d9`，30 个提交）

分支 `feat/gpui-kit-migration`，基线 `upstream-sync/2026-08-29`。
（旧稿写「已推送 `fork`」；这个 clone 的远端叫 `origin`，对应
`origin/feat/gpui-kit-migration`。）工作区干净，195 测试通过，clippy / rustfmt 干净，
应用可启动。

| 项 | 状态 |
|---|---|
| 框架切换：gpui 源码 → `gpui-pre 0.3.6` + `gpui-component 0.6.6` | 完成 |
| 主题桥接 `Palette` → `ThemeColor` + `ThemeTokens` | 完成 |
| 按钮/徽章、开关、图标、加载态、标签页、文本输入 | 完成 |
| `window` 穿透进页面 render | 完成 |
| 三个确认弹窗 → `Root` dialog | 完成（有截图）|
| 预设选择器、镜牢明细 → `Root` sheet | 完成（有截图）|
| 日常统计明细 → `Root` sheet（entity 视图）| 完成（有截图）|
| 结束后操作编辑器 → `Root` dialog（entity 视图）| 完成（有截图）|
| 队伍编辑器 → `Root` dialog（entity 视图）| 完成（有截图）|
| Select（设置页 3 处）| **阻断**，见第 1 节末尾 |

页面自己画的遮罩已经一个不剩（`pages::render_overlay` 已删除）。

### 下次开工顺序

1. **只剩 Select 一项需要你拍板**（三个选项在第 1 节末尾）。
2. 两个可选收尾（都不阻塞）：把镜牢明细也换成 entity 视图，以补上
   `home-mirror-details` 只能拍空状态的缺口；以及给截图工具加一个
   「不要兜底点击」的开关（见「视觉验证」末）。

### 每次改完的验证清单

```sh
cargo +nightly-2026-08-26 fmt --check
cargo +nightly-2026-08-26 clippy --all-targets    # 必须 0 警告
cargo +nightly-2026-08-26 test                    # 195 测试
cargo +nightly-2026-08-26 build
AHAB_BACKEND=mock timeout 12 ./target/debug/ahab-gpui-app.exe   # exit 124 = 正常
```

**本机环境的两个坑**（2026-09-28 实测，踩过的：）

1. **cargo 直连 crates.io 会卡在 `Updating crates.io index`** —— 代理会
   `transfer too slow` 反复重试，加 `CARGO_HTTP_TIMEOUT` 反而让它一次挂 5 分钟。
   走 rsproxy 镜像即可（不需要改配置文件）：

   ```sh
   cargo build --config 'source.crates-io.replace-with="rsproxy"' \
               --config 'source.rsproxy.registry="sparse+https://rsproxy.cn/index/"'
   ```
   注意：换 source 会让 cargo 用另一个 registry 目录，**中途切回默认源等于重新编译全部依赖**。
2. **截图脚本缺 `pyautogui`**（`capture_window.py` 需要它，报错只是 ModuleNotFoundError）。
   直连 PyPI 会被代理的 SSL 打断，用国内镜像装：

   ```sh
   HTTPS_PROXY= HTTP_PROXY= pip install pyautogui -i https://pypi.tuna.tsinghua.edu.cn/simple
   ```

截图验证见文末「视觉验证」节。**弹窗/侧栏类改动必须补一个 `VisualState`，
否则无法区分「能用」与「静默不出现」。**

---

## 已经打好的地基

**页面 render 已能拿到 `window`。** `pages::render(page, app, window, cx)`。
这是后来三个 entity 视图面板的共同前置条件，已完成（提交 `effd0f58`，视觉 0.00% 差异）。

**主题桥接已就位。** `components/style/kit/` 把 `Palette` 投影到
`ThemeColor` + `ThemeTokens`。GPUI Kit 控件自动跟随皮肤，页面**不需要**
再逐个推送调色板。`sync_input_palettes` 就是因此删除的。

**图标按需嵌入 + 有测试守护。** `kit_assets::AppAssets` 组合了应用用到的
50 个图标与 GPUI Kit 的默认包。**未嵌入的图标会静默渲染成空白**，无报错
无日志 —— `every_referenced_icon_is_embedded` 会扫源码发现。

**entity 视图 + `app_listener` 是打通剩余面板的钥匙。** 见 2.5（为何子视图能读 app）
与 2.7（如何把只吃 `Context<AhabApp>` 的控件改造成句柄驱动）。两个新面板
（`AfterCompletionView` / `TeamEditorView`）都是照这个模式做的，队伍编辑器那份
连 `InputState` 实体一起渲染，是现成模板。

**页面遵罩已清零。** 页面不再给自己画遵罩，`pages::render_overlay` 已删除；
所有弹层都归 `Root` 的 dialog / sheet 层。新增弹窗照「已完成：三个确认/取消型」
那一节的形态抄。

---

## 1. Select（3 处）—— 迁移的最后一项，需要拍板

### 迁移点

| 文件 | 函数 | 调用点 |
|---|---|---|
| `pages/home/controls.rs` | `home_select` | 6（其中 `daily_team_select` 只是它的一层包装）|
| `pages/teams/mod.rs` | `team_select` | 10（全在队伍编辑器的 5 个标签页里）|
| `pages/settings/cards/updates.rs` | `update_card` 内的更新源选择 | 1 —— **没有封装**，直接在 `updates.rs` 里拼 `select_trigger` + `select_popup`，迁移时不要漏 |

（旧稿写「43 个调用点经由这三个封装」，与现状不符：实测是 6 + 10 + 1。
`components/` 里的 `select_trigger` / `select_popup` / `select_option` 是共用外观，
不算调用点。）

**调用点本身不用动**，但两个封装现在各有两副面孔（见 2.7）：
`home_select(app, cx, config)` / `home_select_for_view(root, open, config)`，
`team_select(root, app, config)`。换成 GPUI Kit 的 `Select` 时两副都要覆盖，
或者干脆合并成只吃 `(root, window, …)` 的单一版本 —— 子视图的 `render` 本来就
拿得到 `window`，而 `use_keyed_state` 正需要它。

### 好消息：委托不用自己写

`gpui_component::searchable_list::vec` 里，**`Vec<T>` 本身就实现了
`SearchableListDelegate`**，且 `String` / `SharedString` / `&'static str`
都实现了 `SearchableListItem`。纯标签列表直接：

```rust
let items: Vec<SharedString> = vec!["GitHub".into(), "Mirror 酱".into()];
SelectState::new(items, Some(IndexPath::default().row(index)), window, cx)
```

### 做法：用 window 的 keyed state 拿稳定实体

**不要在 render 里 `cx.new`** —— 每帧都会新建，选中态直接丢失。
`window.use_keyed_state` 按元素 id 复用同一个实体：

```rust
let state = window
    .use_keyed_state(("select", id), cx, move |window, cx| {
        SelectState::new(items, Some(IndexPath::default().row(selected)), window, cx)
    })
    .read(cx)
    .clone();
```

### 阻断点（已实测，必须先解决）

**`SelectState` 无法程序化打开下拉。** `select.rs:431` 的
`fn set_open` 是私有的（无 `pub`），`is_open()` 只有 getter。

这直接阻断迁移：`AHAB_VISUAL_STATE=settings-select` 正是靠
`settings_page.open_select = Some(SettingsSelect::UpdateSource)`
（`app/lifecycle/construct.rs:193`）来展开下拉并截图。换成 GPUI Kit 的
`Select` 后**该视觉状态无法复现**，`capture_visual.ps1 -States
settings-select` 会拍到关闭状态的控件。

三选一：

1. 接受丢失该状态的覆盖，重新建立基线（需要明确同意 —— 这是用测试覆盖
   换组件替换）；
2. 向 longbridge/gpui-kit 提 issue 请求公开 `set_open`；
3. 暂时保留自研 select。

**在解决之前不要开始这项迁移。** 我实测到这里，已回滚。

### 其余陷阱（已验证）

1. **订阅会泄漏。** `Select` 通过 `SelectState` 实体上的
   `SelectEvent::Confirm(Option<Value>)` 上报选择（`select.rs:70`），页面用
   `cx.subscribe` 接。但 keyed 实体跨帧存活，**每次 render 都订阅会无限累积**。
   需要按实体 id 去重（例如在 `AhabApp` 里存
   `HashMap<ElementId, Subscription>`），或只在首次创建时订阅。

2. **模型 → UI 同步会多一帧。** 选中态由页面 state 拥有，用户点击走
   `on_confirm` → 写模型。反向（如配置从磁盘恢复）需要
   `set_selected_index`，而它会 `cx.notify()`，触发一次额外渲染。因为更新后
   不再不匹配，循环会自行终止 —— 但**必须先比较再设置**，否则每帧自增渲染。

3. **`items` 变化（设备列表刷新）要 `set_items`**，同样注意第 2 点的比较。

4. 现有实现手写了左/右方向键循环、Esc、`on_mouse_down_out` 关闭、
   `deferred(popup).priority(10)` 层叠 —— `Select` 全都提供，可以整块删除。

---

## 2. Root 弹窗

### 已完成：三个"确认/取消"型

| 提交 | 内容 |
|---|---|
| `ec750ffc` | 删除队伍确认 |
| `f7d5fe8d` | 清除历史统计数据确认 |
| `93332c9d` | 覆盖预设确认 |

三者共用同一形态（见 `open_delete_confirmation` / `open_clear_stats_confirmation` /
`open_preset_overwrite_confirmation`）：状态标志留在 state 层供领域校验，
弹窗只驱动两个转移。**新增确认时照抄其中一个即可。**

### 踩过的四个坑（都已解决，别再踩一遍）

**1. `Root` 只托管 dialog 层，不渲染它。** 它自己的 `Render` 只画
文本选择、tooltip、菜单三层；`render_dialog_layer` 是**应用必须自己放置**的
公开方法。少了这一行，`open_dialog` 会推到一个没人绘制的实体上 ——
弹窗创建成功、不报错、永远不出现。已在 `AhabApp::render` 接好。

**2. 该层需要绝对定位的包裹。** 它本身只是 `div()`，作为 flex 列的子元素
会走正常布局流，`size_full()` 的遮罩会以一个高度为 0 的盒子为基准。

**3. 渲染帧内打开不生效。** `apply_visual_state` 在 `AhabApp::render` 里跑，
帧内推入 Root 层的弹窗不会进入该帧。用 `cx.defer_in` 推迟。

**4. `Dialog` 不自带 OK/Cancel。** `render_ok` / `render_cancel` 只被
`AlertDialog` 调用，普通 `Dialog` 是内容容器，动作行要用 `.footer(...)` 自建。
另外 `on_ok` / `on_cancel` 拿到的是 `&mut App` 而非 `Context<AhabApp>`，
回写状态要经 `WeakEntity::update`。

### 已完成：两个内容面板（快照式）

| 提交 | 内容 | 尺寸 |
|---|---|---|
| `faa3de24` | 预设选择器 | 右侧 55% |
| `993f0546` | 镜牢统计明细 | 右侧 60% |

### 内容面板的关键约束：**sheet builder 跑在宿主 render 内部**

`Root::render_sheet_layer` 会在 `AhabApp::render` **仍在栈上时**调用 builder。
所以 builder 里既不能 `update` 也不能 `read` 宿主：

```
panicked at gpui-pre/src/app/entity_map.rs:
cannot update ahab_gpui_app::app::AhabApp while it is already being updated
```

三个确认弹窗没撞上这个坑**纯属侥幸** —— 它们的 builder 只捕获
`WeakEntity`，留到点击回调里才用，那时 render 早已结束。

**解法是快照**：在打开 sheet 的地方（此时不在 render）把内容需要的字符串
全部解析好。`preset_picker_body` / `mirror_history_body` 于是都是
「快照 + 可选 `WeakEntity`」的纯函数，卡片用普通闭包而非 `cx.listener`。

### 快照模型的边界：**数据必须在打开那一刻就绪**

这是决定性的一条，余下面板卡在这里：

- **异步加载的数据不适合。** 镜牢查看器的记录来自启动时的后台拉取，
  而 sheet 只在打开瞬间取一次快照 —— 于是**不存在**「记录已到 + sheet 仍开着」
  的那一帧。试过三种办法，都失败：
  - 固定 400ms 延迟：数据还没到就开了；
  - 轮询到有数据再开：**更糟** —— 一旦轮询落败，拍到的是一张普通主页截图，
    看起来通过、实际什么都没验证；
  - 往 mock 里播一条记录：没进到 UI，已回退，不留未经验证的夹具数据。
- **日常统计明细是同一个问题**（`open_stats_details` 自己发请求），已按 2.5 迁走。
- **队伍编辑器含 `InputState` 实体**，entity 视图也要把实体句柄一起传，是更大的工程。

**正确解法不是加长 sleep，而是用 entity 支撑的视图** —— 见 2.5。

### 2.5 entity 支撑的视图（已落地，机制已核对）

快照模型的边界（见上）意味着这类面板必须换方案。**不是加长 sleep。**

**为什么成立**（在 `gpui-pre 0.3.6` 源码里核对，不再是推理）：
`ViewElement::request_layout` → `request_layout_view`（`src/view.rs:422`）先调
`let mut element = render(window, cx)`，那一步返回即释放该实体的 lease，**之后**才
`element.request_layout(window, cx)` 递归布局子元素。子视图的 `render` 就发生在那次递归里，
所以它跑在父级借用期之外，`app.read(cx)` 合法。

对照：sheet 的 builder 由 `Root::render_sheet_layer`（`gpui-component/src/root.rs:215`）
在 `AhabApp::render` **内部同步**调用，那时 lease 还握着，所以读写 app 必 panic。
「builder 不能碰 app」与「子视图可以读 app」来自同一份源码的两条不同路径，
两者已经被实测各自证实一次。

样板实现：`DailyDetailsView`（`pages/home/stats/details.rs`）。

```rust
pub(crate) struct DailyDetailsView {
    root: WeakEntity<AhabApp>,
    // 不能丢：订阅是「数据晚到也能进到已打开的 sheet」的唯一来源
    _app_events: gpui::Subscription,
}

impl DailyDetailsView {
    pub(crate) fn new(root: Entity<AhabApp>, cx: &mut Context<Self>) -> Self {
        let app_events = cx.observe(&root, |_, _, cx| cx.notify());
        Self { root: root.downgrade(), _app_events: app_events }
    }
}

impl Render for DailyDetailsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(root) = self.root.upgrade() else {
            return div().into_any_element();
        };
        let app = root.read(cx);   // 安全：见上面的机制
        // …拷出所需字段，再交给纯函数 body
    }
}
```

打开处（`AhabApp::open_stats_details`）：

```rust
let app = cx.entity();
let view = cx.new(|cx| DailyDetailsView::new(app.clone(), cx));
window.open_sheet(cx, move |sheet, _, _| {
    let close_app = close_app.clone();   // builder 是 Fn，每次调用都要重新 clone
    sheet
        .title(...)
        .size(gpui::relative(0.6))
        .on_close(move |_, _, cx| {
            let _ = close_app.update(cx, |v, cx| v.close_stats_details(cx));
        })
        .child(view.clone())
});
```

三条要点：

1. **订阅必须存在视图里**（`_app_events`）。`Context::observe` 返回的 `Subscription`
   一 drop 就退订；暂存到局部变量会被立即丢掉，而且**不会报错** —— 只表现为
   「sheet 一直停在加载中」。视图持有 `Entity<AhabApp>` 才能在 `new` 里订阅，
   所以 `new` 收强句柄、内部转弱。
2. **`WeakEntity::update` + 普通闭包**替代 `cx.listener`：子视图拿不到
   `Context<AhabApp>`，body 里的点击写成
   `move |_, _, cx| { if let Some(root) = row_root.upgrade() { root.update(cx, |view, cx| { … }) } }`。
   `daily_details_body` 已改成这个形态（去掉了 `&mut AhabApp` 参数）。
3. **观察 app 等于任意 app notify 都重绘**。当前面板小、只在打开时存在，可以接受；
   要省的话在闭包里比较快照，只在相关字段变化时 `cx.notify()`（像 `StatsView::sync_snapshot`）。

#### 实测到什么程度

- **已实测**：`AHAB_VISUAL_STATE=home-daily-details` 能拍到 sheet，内容是从活的 app
  读出来的（标题、汇总卡、表头、行都在）。子视图 render 里 `app.read(cx)` **不 panic** ——
  2.5 的生死题答案是「成立」。
- **已实测**：`app.read` 之外，`apply_daily_stats` 的默认选中行也生效了
  （截图里第一行有高亮底色）。
- **尚未实测**：数据在 sheet 打开**之后**才到的重绘。`_app_events` 就是为它存在的，
  但截图分不出「第一帧就有数据」和「后面重绘进来的」—— mock 的 `request_async` 走
  `ready_receiver`（`ipc/backend.rs:226`），几乎肯定在第一帧之前就绪。
  要实测得给 mock 加延迟，或者接真实 sidecar。
- **已改夹具**：mock 的 `stats.getDailySummary` 原本固定返回 `days: []`，
  于是这个状态只能拍到空表——和「sheet 根本没渲染」长得一模一样。现改为三条固定日期，
  该 payload 只有这个 sheet 消费，不会渗到别的页面。

**顺带的机会**：镜牢明细仍用快照，所以 `home-mirror-details` 只能拍空状态。
换成 entity 视图就能拍到真实记录，把这条覆盖补上。

### 2.6 已完成：结束后操作编辑器（dialog + entity 视图）

| 内容 | 位置 |
|---|---|
| 结束后操作 | 居中 dialog，宽 512 |

从「页面自己的内联遮罩」换成 `Root` dialog：居中模态的外观和原来一致，但
关闭键、焦点捕获、Esc 都归 `Root` 了。同一批改动里 Home 已经没有任何页面级
遮罩，`pages/home/render_overlay` 随之下线（`pages::render_overlay` 只剩 Teams 用）。

**这项真正的发现不是 dialog，而是里面两个控件只吃 `Context<AhabApp>`。**
sheet/dialog 的 body 是子视图，拿不到它，于是一个都换不上去。解法是给两个控件
各加一个句柄版入口、共用同一份实现：

- `home_select_for_view(root, open, config)` 与 `task_option_switch_for_view(...)`
- 原入口 `home_select(app, cx, config)` 保留，内部转调 —— 改动当时 **17 个**
  既有调用点（7 + 10）一行未改。之后结束动作编辑器自己也在子视图里，把
  `home_select` 的其中一个调用点换成了 `_for_view` 变体，所以今天看到的
  数字会少一个，不是回归。
- `open` 由调用方传入（视图已经从 app 读过 `is_select_open`），这两个封装于是
  完全不再需要 `&AhabApp`

内部实现把每一个 `cx.listener` 换成 `WeakEntity::update` + 普通闭包。两个陷：

1. **内层闭包不能写 `move`**。外层是 `Fn`，内层若 `move` 会把
   `key_values` / `key_current` 移出外层闭包，直接编译不过。
2. **`&mut Window` 要先重借用**（`let window = &mut *window;`）再进内层闭包，
   否则外层闭包就不再是 `Fn`。

**这是队伍编辑器的前置条件**：它也要在子视图里用 `team_select`，照这个模式加
一个 `_for_view` 变体即可。

**已实测**：dialog 里的 power select 能展开，且弹层不被 dialog 的
`overflow_y_scrollbar` 裁掉 —— `deferred(popup).priority(10)` 会逃出容器裁切，
这本来就是它能用的原因。

**切页要显式关 dialog**（`select_page` → `close_after_completion`）：以前遮罩
只画在 Home 页，换页自然就消失；`Root` 的弹窗不吃这一套。用
`after_completion_open` 做守卫，避免 `close_dialog` 弹掉别的 dialog。

### 2.7 已完成：队伍编辑器（dialog + entity 视图，35 处控件改造）

| 内容 | 位置 |
|---|---|
| 队伍编辑器（5 个标签页）| 居中 dialog，宽 680，正文高度固定 520 |

这是最大的一块：编辑器本身 300 行，但它带着 5 个标签页的整套表单
（`editors/*` 共 29 个 `cx.listener`，加 `teams/mod.rs` 里的 `team_select` / `mirror_switch`），
它们全部只吃 `Context<AhabApp>`。

**解法是把 2.6 的手工改造提炼成一个通用桥：**
`components/bridge.rs::app_listener`：

```rust
pub fn app_listener<E, F>(root: &WeakEntity<AhabApp>, handler: F)
    -> impl Fn(&E, &mut Window, &mut App) + 'static
where F: Fn(&mut AhabApp, &E, &mut Window, &mut Context<AhabApp>) + 'static;
```

`cx.listener(handler)` 与 `app_listener(root, handler)` 的**闭包体完全一样**，
所以 35 处改造是纯机械的（`cx.listener(` → `app_listener(root, ` + 把 `root` 往下传）。
两个坑已在 helper 里处理掉，调用方不用知道：`&mut Window` 的重借用，
以及 handler 必须按共享引用调用（否则外层闭包不再是 `Fn`）。

**新的两个设计点：**

1. **body 自己把 dialog 关掉。** 保存成功后是异步完成路径调的
   `apply_saved_team`，那里**没有 window**，无法调 `Root::close_dialog`。
   所以 `TeamEditorView` 发现 `teams.editor` 为 `None` 时就
   `cx.defer_in(window, ...)` 请求一关，下一帧执行（那时不在 `Root` 的布局期间）。
2. **`TeamsState::editor_dialog_open` 是幂等开关。** `close_dialog` 弹的是**栈顶**，
   不是「我这个 dialog」，而 dialog 会从两个方向被关（`Root` 自己，以及上面的 body）。
   没有这个标志，`on_close` 之后 body 还会再请求关一次，就把**下面那个** dialog 弹掉。

**视觉状态有顺序要求**：`teams-stats-clear` 要「编辑器在下、确认在上」，
两个 dialog 必须在**同一次** deferred 回调里按序 push
（先 `open_existing_team` + 设 tab，再 `open_clear_stats_confirmation`）。
分开 defer 顺序就不确定了。

**正文高度写死 520。** dialog 高度是内容撑的，不写死会随标签页高低跳。

**顺带清掉最后一处遗留**：`pages::render_overlay` 连同 Home/Teams 的页面遮罩
一起删除，`app/render.rs` 里那一行也去掉了。至此页面不再自画任何遮罩。

### 副作用（顺带清理）

`scroll_area_with_id` 的 `&mut AhabApp` 参数从未被读取（形参写作 `_app`）。
builder 借不到 app 才让这个死参数变成障碍，现已删除，8 个调用点同步更新。
`mirror_history_body` 也被同一次改动带出未使用的 `app` 参数。

### 剩余：无（只差 Select 的取舍）

```
无待迁移面板。页面遮罩已全部移除。
```

可以做的收尾：

1. 镜牢明细换成 entity 视图（它用的是快照，所以 `home-mirror-details`
   只能拍空状态）；`TeamEditorView` 是最好的模板 —— 它是唯一一个
   渲染完整活表单（含 `InputState` 实体）的 body。
2. 截图工具的两个假通过问题（兜底点击、拍到别的窗口），见文末。

### 视觉状态覆盖

| 状态 | 覆盖 |
|---|---|
| `teams-delete` | 删除队伍确认 dialog（列表页）|
| `teams-stats-clear` | 编辑器 + **叠在它上面的**清空确认（两个 dialog 的堆叠顺序）|
| `teams-preset-overwrite` | 覆盖预设确认 dialog（列表页）|
| `teams-preset-picker` | 预设选择器 sheet |
| `teams-editor` / `teams-shop-editor` / `teams-combat-editor` / `teams-starlight-editor` / `teams-advanced-editor` | 队伍编辑器的 5 个标签页（dialog + 完整活表单，含 `InputState`）|
| `home-mirror-details` | 镜牢明细 sheet（**仅空状态**，见 2.5）|
| `home-daily-details` | 每日刷本明细 sheet（**真实行**）|
| `home-after-completion` | 结束后操作 dialog |
| `home-after-completion-power` | 同上 + 电源动作下拉展开（唯一能证明 select 在 dialog 里可用的状态）|
| `home-select` / `teams-select` / `settings-select` | **Select 迁移的阻断点**（第 1 节）：`SelectState::set_open` 是私有的，这三个状态用 GPUI Kit 的 `Select` 复现不了 |

加状态的铁律：**状态必须真的走到那条分支**。踩过三次：

- `teams-preset-overwrite` 第一次用了空槽位路径，`select_preset` 直接应用预设
  而不弹确认 —— 拍到一张没有弹窗的正常页面；
- `home-mirror-details` 的轮询版本落败时，拍到一张普通主页截图；
- `home-after-completion-power` 第一次拍摄时，截图工具的激活兜底点击落在
  对话框遮罩上，把 dialog 关掉了 —— 又一张「干净主页」。

三次都是**看起来通过、实际什么都没验证**。加完状态要问自己：这张图里
有没有一个「只有走对分支才会出现」的东西？没有就再加。

正面例子两个：`home-daily-details` 的表格行只能是 fetch 回来后渲染的；
`home-after-completion-power` 的下拉只能由 `open_select` + 子视图重绘产生。

另外：弹窗/侧栏都要 `cx.defer_in` 打开（渲染帧内推 Root 不生效），
且 `Root` 的 dialog / sheet 层需要**应用自己放置**（`app/render.rs` 已接）。

## 3. 滚动区（有意保留，除非改变取舍）

**不建议迁移**，理由已写在 `components/overlays.rs`：

- GPUI 已有原生滚动条，且调用点已经在设 `.scrollbar_width(px(6.))`
  并用 `.track_scroll(&handle)` 跟踪。GPUI Kit 的 `Scrollable` 只换来
  不同的滚动条外观，不是缺失的能力。
- `Scrollable` 把 `ScrollHandle` 放进 window 的 keyed state，**会夺走帮助页
  的句柄**。该页用 `scroll_to_top_of_item()` 做目录跳转、用 `top_item()`
  跟踪当前小节高亮 —— 丢掉句柄是功能回退。

若确实需要 GPUI Kit 的滚动条外观，正确的做法是先让帮助页改用
`window.use_keyed_state(HELP_SCROLL_ID, ...)` 取句柄（与 `Scrollable`
用同一个 id，从而共享同一实例），再替换滚动区。

---

## 视觉验证

每次改动后跑：

```powershell
pwsh -File gpui-app/scripts/capture_visual.ps1 `
  -Executable gpui-app/target/debug/ahab-gpui-app.exe `
  -OutputDirectory artifacts/visual/<step> -Sizes 900x680 `
  -Languages zh-CN -Themes dark,light -Skins default,limbus
```

纯重构应当是 **0.00% 显著差异**（`thread the window` 那次就是）。
有差异时先用颜色直方图判断是布局变化还是抗锯齿噪声 —— 直接数"非零差异
像素"会把 ±1 的抗锯齿噪声算进去，曾让我误判过一次 15% 的"差异"。

只拍一两个状态时用 `-States`（它会接管页面选择，不必再传 `-Pages`）：

```powershell
pwsh -File gpui-app/scripts/capture_visual.ps1 `
  -Executable gpui-app/target/debug/ahab-gpui-app.exe `
  -OutputDirectory artifacts/visual/daily-details -Sizes 900x680 `
  -Languages zh-CN -Themes dark -Skins default `
  -States home-daily-details,home-mirror-details
```

2026-09-28 的 entity 视图这一步拍到的（`artifacts/` 已被 gitignore）：
`home-daily-details` 深浅两套主题都有三行真实数据且首行高亮，
`home-mirror-details` 与改动前一致（仍是空状态），
`home`（无状态）作为 `render_overlay` 的回归对照，均正常。
队伍编辑器那一步的 12 张在 `artifacts/visual/team-editor-dialog/`。

**截图工具有三种会误导人的失败模式**（2026-09-28 全部遇到过，下单前先看图）：

| 现象 | 看起来像 | 原因 |
|---|---|---|
| 拍到**干净主页** | 通过 | `capture_window.py` 在 `SetForegroundWindow` 失败时会补一次合成点击（窗口左上角附近）。`Root` 的 dialog 遮罩覆盖整窗、默认点外关闭，这一击就把 dialog 关掉了 |
| 拍到**另一个应用** | 通过（图像很"丰富"，更难认）| 脚本截的是**屏幕上一块区域**。自己的窗口被别的窗口盖住时（本次是微信），截到的就是那个窗口，**而脚本照样打印 captured、退出码 0** |
| `TimeoutError` /「capture failed」| 代码坏了 | 窗口没起来，或 30s 内没能找到并激活。本次是临时脚本给 `APPDATA` 传了 MSYS 风格路径 `/tmp/...`，应用起不到主窗口 —— 与代码无关，但当时看着很像崩溃 |

前两种就是 `home-after-completion-power` 第一张空图、以及随后一整批图全是别的
窗口的原因。判据仍是那条：**图里有没有一个「只有走对分支才会出现」的东西。**

本次因此改用一份临时脚本（在 `/tmp`，未入库）：`SetWindowPos(HWND_TOPMOST)`
拉置顶（拍完还原）+ 不点击 + 直接 `pyautogui.screenshot`，逐张目视核对内容。
要入库的话，给 `capture_window.py` 加两个参数就是正解：「不兜底点击」与
「拍前拉置顶」—— 跳过点击会让激活失败变成抛错，而不是留下一张假通过图。
