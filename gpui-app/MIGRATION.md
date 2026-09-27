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

## 2. Root 弹窗（9 处遮罩）

### 迁移点

`bg(rgba(0x00000080))` 的遮罩共 9 处：

```
pages/home/completion_editor.rs:233
pages/home/stats/details.rs:125, 242
pages/teams/editors/team_stats.rs:199
pages/teams/overlay/preset.rs:198, 329
pages/teams/overlay/render.rs:34, 306, 355
```

### 收益

每处都手写了：遮罩、点击外部关闭、`capture_key_down` 捕获 Esc、
`stop_propagation` 阻止穿透。**当前完全没有焦点捕获**（WCAG 要求），
`Root::open_dialog` 提供焦点捕获、Esc、层叠、动画。

### 做法

`Root::update(window, cx, ...)` 从任何持有 window 的地方都可调用，包括
`apply_visual_state(&mut self, window, cx)`。

**关键点：`on_ok` / `on_cancel` 拿到的是 `&mut App`，不是
`Context<AhabApp>`**（`dialog.rs:369,380`），所以回写页面状态要经过
`WeakEntity`：

```rust
let app = cx.entity().downgrade();          // WeakEntity<AhabApp>
let name = team.name.clone();

self.teams.request_delete(team);            // 状态照旧，供其他逻辑读取
Root::update(window, cx, move |root, window, cx| {
    root.open_dialog(
        move |dialog, _window, _cx| {
            let cancel_app = app.clone();
            let ok_app = app.clone();
            dialog
                .title(text("确认删除队伍？", "Delete this team?"))
                .child(name.clone())
                .on_cancel(move |_, _, cx| {
                    let _ = cancel_app.update(cx, |view, cx| {
                        view.teams.cancel_delete();
                        cx.notify();
                    });
                    true                        // true = 关闭弹窗
                })
                .on_ok(move |_, _, cx| {
                    let _ = ok_app.update(cx, |view, cx| {
                        let _ = view.teams.confirm_delete();
                        cx.notify();
                    });
                    true
                })
        },
        window,
        cx,
    );
});
```

回调返回 `bool`：`true` 关闭弹窗，`false` 保持打开（用于校验失败时）。

### 陷阱（最严重的一项）

**视觉回归套件依赖声明式状态。** `AHAB_VISUAL_STATE=teams-delete` 是通过
`self.teams.request_delete(team)` 设置状态来显示遮罩的
（`app/lifecycle/construct.rs`）。改成命令式 `open_dialog` 后，
**恢复状态不会再显示弹窗**，`capture_visual.ps1 -States teams-delete` 等
用例会失败。

所以迁移必须同时更新 `apply_visual_state` 里的对应分支去调用
`open_dialog`，并且：

- **不要在 render 里做"flag → 弹窗"的同步副作用** —— 在渲染帧内打开弹窗
  需要 `window.refresh()`，会多推一帧；这与为 TextInput 移除的坏味道同类。
- 应当在**事件处理里**推入弹窗，在弹窗的 `on_close`/`on_cancel` 里清 flag。

### 顺序建议

从 `teams/overlay/render.rs` 的删除确认开始（单一 flag：`delete_target`，
开/关路径清晰），跑通后再推广。

---

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
