import os
from typing import Tuple, Union

import cv2
import numpy as np
from PIL import Image

from module.config import cfg
from module.logger import log
from utils.image_utils import ImageUtils
from utils.path_manager import path_manager


class TextBootstrapManager:
    """静态文本/按钮的运行时自举模板管理器。

    工作机制：
    1. 首次遇到未自举的静态文本，利用 OCR 识别其坐标 (bbox)。
    2. OCR 命中后，自动将该区域从当前屏幕截图中切出，并以 1440p 参考分辨率归一化存盘至
       assets/images/{theme}/{lang}/bootstrap/{key}.png。
    3. 后续运行直接使用 OpenCV 模板匹配 (耗时 < 1ms)，未命中时才回退至 OCR 并自动刷新自举模板。
    """

    BOOTSTRAP_SUBDIR = "bootstrap"

    @classmethod
    def get_template_rel_path(cls, key: str) -> str:
        """获取适用于 auto.find_element 的相对模板路径。"""
        return f"{cls.BOOTSTRAP_SUBDIR}/{key}.png"

    @classmethod
    def has_template(cls, key: str) -> bool:
        """检查当前主题和语言环境下是否已存在该自举模板。"""
        rel_path = cls.get_template_rel_path(key)
        existing = ImageUtils.existing_image_paths(rel_path)
        return bool(existing)

    @classmethod
    def get_save_abs_path(cls, key: str) -> str:
        """获取自举模板保存的目标物理绝对路径（根据当前主题与语言自动分流）。"""
        lang = path_manager.current_language or "zh_cn"
        theme = path_manager.current_theme or "default"
        save_dir = os.path.abspath(f"./assets/images/{theme}/{lang}/{cls.BOOTSTRAP_SUBDIR}")
        os.makedirs(save_dir, exist_ok=True)
        return os.path.join(save_dir, f"{key}.png")

    @classmethod
    def harvest_from_frame(
        cls,
        source_frame: Union[Image.Image, np.ndarray],
        bbox: Tuple[float, float, float, float],
        key: str,
        padding: int = 5,
    ) -> bool:
        """从截图中裁剪文本区域，将其缩放到 1440p 基准分辨率后存盘。

        Args:
            source_frame: 当前屏幕截图（PIL.Image 或 np.ndarray）。
            bbox: 文本边界框 (xmin, ymin, xmax, ymax)。
            key: 自举唯一标识名（无后缀）。
            padding: 四周内边距，确保文字边缘和背景特征完整。
        """
        try:
            img_arr = np.asarray(source_frame)
            h, w = img_arr.shape[:2]
            xmin, ymin, xmax, ymax = bbox

            # 加入 padding 并限制在图像范围内
            x1 = max(0, int(round(xmin)) - padding)
            y1 = max(0, int(round(ymin)) - padding)
            x2 = min(w, int(round(xmax)) + padding)
            y2 = min(h, int(round(ymax)) + padding)

            if x2 <= x1 or y2 <= y1:
                log.warning(f"[BOOTSTRAP] 非法裁剪坐标: ({x1}, {y1}, {x2}, {y2})")
                return False

            cropped = img_arr[y1:y2, x1:x2]

            # 项目的 assets 资产标准基准为 2560x1440 (win_size=1440)
            # 若当前截图分辨率不同，需反向缩放至 1440p，保证后续 ImageUtils.load_image 加载时缩放比例对称
            win_h = getattr(cfg, "set_win_size", 1080) or 1080
            if win_h != 1440 and win_h > 0:
                scale = 1440.0 / float(win_h)
                cropped = cv2.resize(cropped, None, fx=scale, fy=scale, interpolation=cv2.INTER_CUBIC)

            # 统一为 BGR 格式保存为 PNG
            if cropped.ndim == 3:
                if cropped.shape[2] == 4:
                    cropped = cv2.cvtColor(cropped, cv2.COLOR_RGBA2BGR)
                elif cropped.shape[2] == 3:
                    # PIL 数组是 RGB，cv2.imwrite 需要 BGR
                    cropped = cv2.cvtColor(cropped, cv2.COLOR_RGB2BGR)

            save_path = cls.get_save_abs_path(key)
            cv2.imwrite(save_path, cropped)
            log.info(f"[BOOTSTRAP] 成功自举生成模板: {cls.get_template_rel_path(key)} -> {save_path} (尺寸: {cropped.shape[1]}x{cropped.shape[0]})")
            return True
        except Exception as e:
            log.warning(f"[BOOTSTRAP] 自举模板保存失败: {e}")
            return False
