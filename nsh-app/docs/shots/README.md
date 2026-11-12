# 截图基线

P0 骨架在 GPUI 开发宿主里的真实渲染结果，作为 P2/P3 逐页重写的对照基准。

尺寸统一为 **820×680** —— 原 GPUI 客户端的默认窗口（`gpui-app/src/main.rs`：`size(px(820.), px(680.))`，最小 800×560），
`nsh-app/nsh.json` 的 `window` 已对齐。

| 文件 | 尺寸 | 怎么来的 |
| --- | --- | --- |
| `p0-desktop.png` | 820×680 | `nsh run nsh-app --target gpui --window-size 820x680`，在 Xvfb + lavapipe（软件 Vulkan）里跑，抓 X11 根窗口 |

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

## 原客户端的对照图

同一台机器上原 GPUI 客户端也能无头出图（mock 后端，不需要 Python sidecar）：

```bash
cargo build --manifest-path gpui-app/Cargo.toml     # 首次 ~11 分钟
export PATH="$HOME/.local/bin:$PATH"
python3 scripts/capture_gpui.py --all               # → artifacts/visual/gpui/{page}-{light,dark}.png
python3 scripts/capture_gpui.py --page home --state home-running
```

原客户端自带一套可视化捕获开关（`AHAB_VISUAL_PAGE` / `_STATE` / `_THEME` / `_LANGUAGE` / `_SKIN` / `_ACCENT`），
Windows 版驱动脚本是 `gpui-app/scripts/capture_visual.ps1`，`scripts/capture_gpui.py` 是它的 Linux 等价实现。

一组 14 张（7 页 × 浅/深，820×680）已归档进仓库：**`nsh-app/docs/reference/gpui/`**，
`gpui-app/` 删掉后仍然可以对照。随手实验的图仍写到 `artifacts/`（仓库忽略）。

## P0 截图暴露的问题（转 P2）

- **窗口壳结构不对**：原客户端是**顶部标签栏**（主控台 / 队伍管理 / 镜牢主题包 / 工具箱 / 资源中心 / 帮助，
  右侧是设置、主题与窗口按钮）+ 页面内底部操作栏；P0 占位用的是左侧栏。P2 按 `docs/reference/gpui/*.png` 重做。
- **标题栏配色硬编码**（`shell/TitleBar.tsx` 里的 `#1b1114` / `#d8d0bc` / `#462c2c`，取自 limbus 皮肤），
  浅色主题下与页面背景冲突。P2 改走主题 token（`bg-card` / `border-border` / `text-foreground`）。
- **`font-semibold` 在 Linux 上回退**：宿主日志 `failed to load font ... ".SystemUIFont" ... weight 600`，
  标题没有真正加粗。P2 定下字体族后显式声明，或改用宿主存在的字重。
- 侧栏无选中态、内容区留白过多：P0 占位，P2/P3 落实。
