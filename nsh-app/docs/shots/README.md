# 截图基线

`nsh-app` 在 GPUI 开发宿主里的真实渲染结果。**统一 820×680** —— 与旧客户端的默认窗口
（`gpui-app/src/main.rs`：`Bounds::centered(None, size(px(820.), px(680.)), cx)`，最小 800×560）
以及 `docs/reference/gpui/` 的对照图同一个视口，`nsh.json` 的 `window` 已对齐。

| 文件 | 内容 |
| --- | --- |
| `home-light.png` / `home-dark.png` | 主控台（P3 待迁入的占位） |
| `gallery-light.png` / `gallery-dark.png` | 组件对照页：变体 / 尺寸 / 状态 / 图标 / token 色板 / 圆角 |
| `skin-archive-light.png` | Archive 皮肤（纸张与墨线，无阴影） |
| `skin-glass-dark.png` | Glass 皮肤（半透明层叠、发丝高光、大圆角） |
| `skin-limbus-dark.png` | Limbus 皮肤（暗色羊皮纸、黄铜角框、红色印章） |
| `skin-mist-dark.png` | Mist 皮肤 |
| `titlebar-light.png` | 标题栏：品牌、6 个导航项、状态点、外观切换、设置、窗口按钮 |

## 重新生成（Linux，无需 GPU）

```bash
python3 scripts/capture_nsh.py --size 820x680 --out nsh-app/docs/shots/home-light.png
```

`capture_nsh.py` 会自己起 Xvfb（屏幕尺寸 = 窗口尺寸，产物无桌面留白）、设置
`VK_ICD_FILENAMES` 指向 Mesa 的 lavapipe 软件 Vulkan、等宿主 `connected` 后抓 X11 根窗口，
结束清理进程组。

**换页面 / 外观 / 皮肤**目前靠临时改 `store.ts` 的初始值（拍完 `git checkout` 还原）：

```bash
# page: home | teams | theme-packs | toolbox | resources | help | settings | gallery
sed -i 's|const page = signal<PageId>("home");|const page = signal<PageId>("gallery");|' nsh-app/store.ts
# appearance: light | dark | system
sed -i 's|const appearance = signal<Appearance>("system");|const appearance = signal<Appearance>("dark");|' nsh-app/store.ts
# skin: default | glass | archive | limbus | mist
sed -i 's|const skin = signal<SkinName>("default");|const skin = signal<SkinName>("limbus");|' nsh-app/store.ts

python3 scripts/capture_nsh.py --size 820x680 --out nsh-app/docs/shots/skin-limbus-dark.png
git checkout nsh-app/store.ts
```

> P3 之前值得加一个正式的捕获入口（`nsh test` 步骤驱动导航 + `app.screenshot()`，
> 或者让 `store.ts` 的初始值可从外部覆盖），这样就不用手改源码了。

## 与原客户端的对照

`docs/reference/gpui/` 存着旧客户端的 7 页 × 浅/深（同为 820×680）参考图，
P3 逐页迁移时按同名文件两两对照（`home-light.png` ↔ `reference/gpui/home-light.png` 等）。
参考图生成方式见 `docs/reference/gpui/README.md`。

## 已知问题（P2/P3 要修）

- **`font-semibold` 在 Linux 上回退**：宿主日志 `failed to load font ... ".SystemUIFont" ... weight 600`，
  标题没有真正加粗。定下字体族后显式声明，或改用宿主存在的字重。
- 页面内容还是占位：P3 逐页迁入，底座与标题栏已就位。
