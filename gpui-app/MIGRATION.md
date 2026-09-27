# GPUI Kit 迁移：剩余工作

本文档记录尚未迁移到 GPUI Kit 的三项，以及每项的确切做法和已知陷阱。
分析结论来自实际尝试，不是推测。

已完成的部分见 git 历史（`feat/gpui-kit-migration` 分支）；组件层、
图标、主题桥接、TextInput 均已迁移并通过视觉验证。

## 已经打好的地基

**页面 render 已能拿到 `window`。** `pages::render(page, app, window, cx)`。
这是下面三项的共同前置条件，已完成（提交 `effd0f58`，视觉 0.00% 差异）。

**主题桥接已就位。** `components/style/kit/` 把 `Palette` 投影到
`ThemeColor` + `ThemeTokens`。GPUI Kit 控件自动跟随皮肤，页面**不需要**
再逐个推送调色板。`sync_input_palettes` 就是因此删除的。

**图标按需嵌入 + 有测试守护。** `kit_assets::AppAssets` 组合了应用用到的
50 个图标与 GPUI Kit 的默认包。**未嵌入的图标会静默渲染成空白**，无报错
无日志 —— `every_referenced_icon_is_embedded` 会扫源码发现。

---

## 1. Select（3 处，推荐先做）

### 迁移点

| 文件 | 函数 |
|---|---|
| `pages/home/controls.rs` | `home_select` |
| `pages/settings/cards/updates.rs` | `update_card` 内的更新源选择 |
| `pages/teams/mod.rs` | `team_select` |

43 个调用点全部经由这三个封装，**不需要动**。

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

### 已完成：两个内容面板

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
- **日常统计明细是同一个问题**（`open_stats_details` 自己发请求），
  所以它**保持内联遮罩**，没有迁到 sheet。
- **队伍编辑器含 `InputState` 实体**，快照要连实体句柄一起存，是更大的工程。

**正确解法不是加长 sleep，而是用 entity 支撑的视图**：sheet 的
`.child(...)` 放一个实现了 `Render` 的 `Entity<View>`，该视图的 render
在 `AhabApp::render` 返回之后才跑，因此可以安全 `app.read(cx)`，再用
`cx.observe` 跟随数据更新。这是余下三个面板该走的路。

### 副作用（顺带清理）

`scroll_area_with_id` 的 `&mut AhabApp` 参数从未被读取（形参写作 `_app`）。
builder 借不到 app 才让这个死参数变成障碍，现已删除，8 个调用点同步更新。
`mirror_history_body` 也被同一次改动带出未使用的 `app` 参数。

### 剩余：三个内容面板

```
pages/home/stats/details.rs     日常统计明细（异步数据，需 entity 视图）
pages/home/completion_editor.rs 结束动作编辑器
pages/teams/overlay/render.rs   队伍编辑器（含 InputState 实体）
```

### 视觉状态覆盖

`teams-delete`、`teams-stats-clear`、`teams-preset-overwrite` 三个状态已接入
`capture_visual.ps1`。加状态时注意：**状态必须真的走到弹窗那条分支**。
`teams-preset-overwrite` 第一次写成了空槽位路径，截图拍到一个没有弹窗的
正常页面 —— 看起来通过，实际什么都没验证。

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
