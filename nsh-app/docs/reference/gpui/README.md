# 旧客户端参考图（1:1 对照基准）

旧 GPUI 客户端在 **P5 删除**，删掉之后就再也生成不出这些图了，所以在这里归档一份：
迁移期间每一页的验收标准就是和这里的图逐像素对照，而不是「看起来差不多」。

| | |
| --- | --- |
| 来源 | 迁移前的 GPUI 客户端（已删除） |
| 视口 | **820×680** —— 旧客户端的默认窗口（最小 800×560） |
| 数据 | `AHAB_BACKEND=mock`，确定性 mock 数据，不需要 Python sidecar |
| 主题 | `AHAB_VISUAL_THEME=light` / `dark` |
| 语言 | `AHAB_VISUAL_LANGUAGE=zh-CN` |
| 皮肤 | `AHAB_VISUAL_SKIN=default`（默认皮肤；另外 4 个皮肤没有归档） |
| 渲染 | Xvfb + Mesa lavapipe（软件 Vulkan），抓 X11 根窗口，真实 GPUI 渲染 |
| 页面 | `home` `teams` `themes` `toolbox` `resources` `help` `settings` —— 7 页 × 浅/深 = 14 张 |

## 状态

这些图**无法重新生成**：`scripts/capture_gpui.py` 与旧客户端一起删除了，`artifacts/visual/gpui/`
那份实验输出也已清理。本目录是只读归档，改页面时用它对照，不要改图。

需要新的对照图时抓的是**当前**客户端：

```bash
python3 scripts/capture_nsh.py     # → nsh-app/docs/shots/
```

## 已知限制

- 这些图是**旧客户端**的渲染，字体是 Linux 宿主上的字体（`Inter` 缺失时回退到系统字体），
  和 Windows 上旧客户端的字形有细微差别 —— 对照布局、间距、配色、层级可靠，对字形细节仅供参考。
- 长列表/滚动位置默认在顶部；归档时没有抓滚动态。
