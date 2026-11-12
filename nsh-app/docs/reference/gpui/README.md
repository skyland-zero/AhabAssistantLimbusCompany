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
| 皮肤 | `AHAB_VISUAL_SKIN=default`（默认皮肤；另外 4 个皮肤见下方「扩充」） |
| 渲染 | Xvfb + Mesa lavapipe（软件 Vulkan），抓 X11 根窗口，真实 GPUI 渲染 |
| 页面 | `home` `teams` `themes` `toolbox` `resources` `help` `settings` —— 7 页 × 浅/深 = 14 张 |

## 重新生成

这些图只能在 P5 删除旧客户端之前重新生成；现在它们只作为归档基准。
迁移期间使用的命令是：

```bash
python3 scripts/capture_gpui.py --all --size 820x680 --out-dir nsh-app/docs/reference/gpui
```

## 扩充

`capture_gpui.py` 还支持旧客户端自带的 22 个具名状态和 5 个皮肤、6 个强调色，需要时按同一视口补图：

```bash
# 单个具名状态：home-expanded / home-running / teams-editor / settings-hotkey / …
python3 scripts/capture_gpui.py --state teams-editor --theme dark \
  --out-dir nsh-app/docs/reference/gpui

# 换皮肤（default / glass / archive / limbus / mist）
python3 scripts/capture_gpui.py --page home --theme dark --skin limbus \
  --out-dir nsh-app/docs/reference/gpui

# 换语言
python3 scripts/capture_gpui.py --page settings --theme light --language en-US \
  --out-dir nsh-app/docs/reference/gpui
```

注意 `capture_gpui.py` 的默认输出目录是 `artifacts/visual/gpui/`（仓库忽略，随手实验用）；
只有用 `--out-dir` 指到本目录的图才会入库。

## 已知限制

- 这些图是**旧客户端**的渲染，字体是 Linux 宿主上的字体（`Inter` 缺失时回退到系统字体），
  和 Windows 上旧客户端的字形有细微差别 —— 对照布局、间距、配色、层级可靠，对字形细节仅供参考。
- 长列表/滚动位置默认在顶部；需要滚动态时用对应的具名状态（如 `help-scrolled`）。
