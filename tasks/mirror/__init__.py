import os

# 商店资源根目录随图片资源布局迁移过：`assets/images/share` 已不存在，
# 实际资源在 `assets/images/default/share`。旧实现写死旧路径，导致下面三个
# 列表恒为空，商店的“必买 / 必弃 / 合成产物”判断全部空转。
# 参考上游 8196e72f。
_SHOP_BASE = os.path.join("assets", "images", "default", "share")


def _collect(subdir: str) -> list[str]:
    """收集某个商店子目录下的全部图片引用（形如 `mirror/shop/<sub>/<file>.png`）。"""
    directory = os.path.join(_SHOP_BASE, *subdir.split("/"))
    if not os.path.isdir(directory):
        return []
    return [
        f"{subdir}/{name}" for name in sorted(os.listdir(directory)) if os.path.isfile(os.path.join(directory, name))
    ]


must_purchase = _collect("mirror/shop/must_purchase")
must_be_abandoned = _collect("mirror/shop/must_be_abandoned")
fusion_result = _collect("mirror/shop/fusion_result")
