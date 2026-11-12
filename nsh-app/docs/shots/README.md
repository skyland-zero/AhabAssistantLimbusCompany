# 截图基线

`nsh-app` 在 GPUI 开发宿主里的真实渲染结果。**统一 820×680** —— 与旧客户端的默认窗口
（`gpui-app/src/main.rs`：`Bounds::centered(None, size(px(820.), px(680.)), cx)`，最小 800×560）
以及 `docs/reference/gpui/` 的对照图同一个视口，`nsh.json` 的 `window` 已对齐。

| 文件 | 内容 |
| --- | --- |
| `home-light.png` / `home-dark.png` | 主控台（P3 已迁入；动态部分等 P1） |
| `gallery-light.png` / `gallery-dark.png` | 组件对照页：变体 / 尺寸 / 状态 / 图标 / token 色板 / 圆角 |
| `p2-settings-*.png` | P2 设置页基线 |
| `p3-*.png` | P3 逐页迁移后的基线（队伍编辑器的 Windows 实机图见 `artifacts/nsh/`，不入库） |
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

**换页面 / 外观 / 皮肤**：Linux 无 GPU 截图仍靠临时改 `store.ts` 初始值；Windows 用
`nsh-app/visual.test.tsx` 的 `open(..., { colorScheme })` 和逐页 `press` 驱动，不再手改源码。
`store.ts` 的初始值保持默认（`home` / `system` / `default`）。

## 在原客户端上对照

`docs/reference/gpui/` 存着旧客户端的 7 页 × 浅/深（同为 820×680）参考图，
P3 逐页迁移时按同名文件两两对照（`home-light.png` ↔ `reference/gpui/home-light.png` 等）。
参考图生成方式见 `docs/reference/gpui/README.md`。

## Windows 实机截图

```powershell
New-Item -ItemType Directory -Force artifacts/nsh | Out-Null
nsh test nsh-app/visual.test.tsx --target windows
```

`visual.test.tsx` 在真实 Windows 宿主里逐页（浅/深）和队伍编辑器五个页签截图，统一 820×680，
输出到 `artifacts/nsh/`（仓库忽略）。同一测试在无头 `nsh test` 里会跳过截图并保持通过。

> `app.screenshot()` 只抓应用绘制面；Windows 的系统文本框是独立 HWND，所以输入框里的文字不会
> 出现在截图里。真实窗口正常，截图对照时把输入框内容视为已知差异。

2026-11-12 的对照与性能数据见 `MIGRATION.md` §1.6。

## 已知问题

- **`font-semibold` 在 Linux 上回退**：宿主日志 `failed to load font ... ".SystemUIFont" ... weight 600`，
  标题没有真正加粗。定下字体族后显式声明，或改用宿主存在的字重。
- **Windows 截图不含系统文本框文字**：`app.screenshot()` 只抓应用绘制面；真实窗口正常。
- **Linux 与 Windows 的像素差**：字体栅格化不同，逐像素对照看布局/颜色/层级，不看字形。
