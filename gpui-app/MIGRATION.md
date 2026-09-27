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

### 剩余：五个内容面板

```
pages/home/completion_editor.rs:233   结束动作编辑器
pages/home/stats/details.rs:125       日常统计明细
pages/home/stats/details.rs:242       镜牢统计明细
pages/teams/overlay/preset.rs:198     预设选择器
pages/teams/overlay/render.rs:274     队伍编辑器
```

这些**不是确认框，而是内容面板**，所以不该照抄上面的形态：

- 队伍编辑器（`render.rs:274`）和预设选择器都是带滚动内容与多标签的
  大面板，更贴 `Sheet`（侧栏）或保持整页遮罩，用 `Dialog` 会得到一个
  被 `margin_top = 视口/10` 推到屏幕上方、且宽度受限于 448px 的盒子。
- 两个统计明细面板同理，是"查看器"而非"确认"。

**建议**：先看 `gpui_component::sheet` 的能力，或明确接受它们继续用内联
遮罩（它们是页面内容，不是模态确认）。不要为了"迁完"而把它们塞进
`Dialog`。

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
