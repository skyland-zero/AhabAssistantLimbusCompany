# 截图基线

P0 骨架在 GPUI 开发宿主里的真实渲染结果，作为 P2/P3 逐页重写的对照基准。

| 文件 | 尺寸 | 怎么来的 |
| --- | --- | --- |
| `p0-desktop.png` | 1100×720 | `nsh run nsh-app --target gpui --window-size 1100x720`，在 Xvfb + lavapipe（软件 Vulkan）里跑，抓 X11 根窗口 |

重新生成（Linux，无需 GPU）：

```bash
python3 scripts/capture_nsh.py                        # → nsh-app/docs/shots/p0-desktop.png
python3 scripts/capture_nsh.py --size 1400x900 --out /tmp/big.png
```

跨平台（Windows/macOS，需要能显示窗口的会话）用 nsh 自带的截图 API：

```bash
printf 'await app.screenshot("nsh-app/docs/shots/p0-test-host.png")\n' \
  | nsh test nsh-app/App.tsx --target gpui
```

注意：`nsh test` 的宿主窗口尺寸固定（390×800），桌面尺寸的对照图要走 `nsh run --window-size`。

## 已知问题（P2 要修）

- **标题栏配色是硬编码的**（`shell/TitleBar.tsx` 里的 `#1b1114` / `#d8d0bc` / `#462c2c`，来自 limbus 皮肤），
  浅色主题下与页面背景对比突兀。P2 换成主题 token（`bg-card` / `border-border` / `text-foreground`）。
- **`font-semibold` 在 Linux 上回退**：宿主日志 `failed to load font ... ".SystemUIFont" ... weight 600`，
  标题没有真正加粗。P2 定下字体族后要显式声明，或改用宿主存在的字重。
- 侧栏没有选中态、内容区留白过多：P0 占位，P2/P3 落实。
