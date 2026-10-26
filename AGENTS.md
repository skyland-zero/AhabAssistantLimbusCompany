# 仓库通用约定

## 1. 镜牢 E.G.O 饰品图标（从 wiki 取图）

wiki.gg 的 E.G.O 饰品原图通常是 **256×256 RGBA**，不要直接放进仓库用于模板匹配。

`auto.find_element()` 的图像匹配没有多尺度搜索，`ImageUtils.load_image()` 只会按窗口大小整体缩放。仓库现有饰品模板的实际最大边是 **96px** 左右；直接使用 256px 原图会导致模板远大于游戏内图标，图标匹配基本失效，只能依赖 OCR 名称兜底。

### 处理规范

仓库现有 `assets/images/default/share/mirror/ego_gifts/*.png` 的统一处理方式是：

1. 以 RGBA 打开 wiki 原图；
2. 按 alpha 通道裁掉透明边（`image.getbbox()`）；
3. 等比缩放到最大边 **96px**；
4. 使用 **LANCZOS** 重采样；
5. 保存为 PNG，保留透明通道。

等价代码：

```python
from PIL import Image

image = Image.open(source).convert("RGBA")
image = image.crop(image.getbbox())
width, height = image.size
scale = 96 / max(width, height)
image = image.resize(
    (max(1, round(width * scale)), max(1, round(height * scale))),
    Image.Resampling.LANCZOS,
)
image.save(target, optimize=True)
```

该规则已用 `darkflame_smoking_pipe`、`hoarfrost_footprint`、`sharp_needle_and_thread`、`red_necktie`、`lunar_memory` 与 wiki 原图反推验证，处理结果与仓库现有文件像素一致。

### 放置位置

路线饰品的默认资源路径是：

```text
assets/images/default/share/mirror/ego_gifts/{gift_id}.png
```

`GiftRouteTarget.asset` 默认使用 `mirror/ego_gifts/{gift_id}.png`。如果某个饰品使用其他路径，应在路线数据中显式指定 `asset`。

### 验证

`tests/unit/module/test_mirror_routes.py` 中的
`test_spiderweb_normal_farm_gift_icons_match_repository_template_rules`
会检查蜘蛛巢普牢速刷路线饰品的图标：

- 文件存在；
- 模式为 RGBA；
- 最大边为 96；
- 图像不是空图。

新增路线饰品图标后，应补充同类测试或复用该检查。注意：现有 House of Spiders 路线仍有少量历史图标缺失（例如 `bloodtinged_ichthyic_odor`、`into_certain_library_book`），不要直接写成“所有路线图标必须存在”的全量测试，除非先补齐这些资源。

## 2. 分支策略

仓库约定以下分支分工：

| 分支 | 角色 | 说明 |
| --- | --- | --- |
| `dev-gpui` | **日常开发分支** | 所有本地改动、提交都提交到这里，并推送到 `origin/dev-gpui` |
| `main` | **上游同步分支** | 只用来拉取上游更新，保持与 `origin/main` 一致，**不在 `main` 上提交任何本地改动** |
| `feat/gpui-kit-0.8` | 创建 `dev-gpui` 时的起点 | 只是当时最新提交所在的分支，`dev-gpui` 从它拉出；它不是长期基线，不需要特殊维护 |

约定：

- 开始工作前先确认当前在 `dev-gpui` 分支，不要直接在 `main` 上修改代码。
- 同步上游流程：切到 `main` 执行 `git pull`（或 `git fetch`）后，再切回 `dev-gpui` 合并或变基，`main` 本身不产生新提交。
- 如果 `main` 出现任何本地提交或未提交改动，应先处理干净再继续，保持其作为纯上游镜像。
