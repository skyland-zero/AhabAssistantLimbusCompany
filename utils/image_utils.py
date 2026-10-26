from __future__ import annotations

import math
import os

import cv2
import numpy as np
from cv2 import createCLAHE
from PIL import Image

from module.config import cfg
from module.logger import log
from utils.path_manager import path_manager


class ImageUtils:
    # ``_assets.png`` templates are full-screen reference frames; their content
    # bbox is stable until the window size / theme / language changes.  Several
    # battle and shop loops used to reload and re-measure the same asset on
    # every iteration, so keep the bbox next to the existing template cache.
    _bbox_cache: dict[tuple, tuple[int, int, int, int] | None] = {}

    # Coarse-to-fine matching limits.  The thresholds are deliberately
    # conservative: the full-resolution verification below the coarse pass is
    # what decides a hit, so a low coarse floor only costs a small extra pass
    # while it protects low-contrast UI elements from false negatives.
    _COARSE_MIN_AREA = 350_000
    _COARSE_MIN_TEMPLATE_SIDE = 28
    _COARSE_MIN_SCALED_SIDE = 14
    _COARSE_MIN_SCORE = 0.25
    _COARSE_MAX_SCALE = 4
    _COARSE_MAX_CANDIDATES = 3
    _COARSE_MAX_MULTI_CANDIDATES = 32

    @staticmethod
    def load_image(image_path, resize=True, return_path=False):
        """
        加载图片，并根据指定区域裁剪图片。
        :param image_path: 图片文件路径。
        :param resize: 是否根据窗口大小调整图片尺寸。
        :param return_path: 是否返回实际加载到的路径名。
        :return: 图片数组；若 return_path=True，则返回 (图片数组, 路径名)。
        """
        try:
            img_path = None
            selected_path = None
            for path in path_manager.pic_path:
                img_path = os.path.join(f"./assets/images/{path}/{image_path}")
                if os.path.exists(img_path):
                    selected_path = path
                    break
            if img_path is None or not os.path.exists(img_path):
                log.error(f"未找到图片： {image_path} ")
                return (None, None) if return_path else None
            # 使用上下文管理器打开图片文件，确保文件对象及时关闭
            with Image.open(img_path) as img:
                image = ImageUtils._prepare_loaded_image(np.array(img), resize)
                if return_path:
                    return image, selected_path
                return image
        except FileNotFoundError:
            log.error(f"未找到图片： {image_path} ")
            return (None, None) if return_path else None
        except IOError:
            log.error(f"无法读取图片： {image_path}")
            return (None, None) if return_path else None
        except Exception as e:
            log.error(f"加载图片时发生错误： {e}")
            return (None, None) if return_path else None

    @staticmethod
    def check_default_path_exists(image_path):
        """检查图片在默认路径（非 dark）中是否存在。"""
        for path in path_manager.pic_path:
            if path_manager.is_path_dark(path):
                continue
            img_path = os.path.join(f"./assets/images/{path}/{image_path}")
            if os.path.exists(img_path):
                return True, path
        return False, None

    @staticmethod
    def existing_image_paths(image_path):
        """返回当前有效路径中存在该图片的路径列表。"""
        paths = []
        for path in path_manager.pic_path:
            img_path = os.path.join(f"./assets/images/{path}/{image_path}")
            if os.path.exists(img_path):
                paths.append(path)
        if path_manager.current_theme == "dark":
            dark_paths = [path for path in paths if path_manager.is_path_dark(path)]
            if dark_paths:
                paths = dark_paths
        elif path_manager.current_theme == "default":
            paths = [path for path in paths if path_manager.is_path_default(path)]

        if path_manager.current_language == "zh_cn":
            zh_cn_paths = [path for path in paths if path_manager.is_path_zh_cn(path)]
            if zh_cn_paths:
                paths = zh_cn_paths
        elif path_manager.current_language == "en":
            en_paths = [path for path in paths if path.endswith("/en")]
            if en_paths:
                paths = en_paths
            else:
                paths = [path for path in paths if path.endswith("/share")]
        return paths

    @staticmethod
    def load_from_specific_path(image_path, target_path, resize=True):
        """从指定路径加载图片。"""
        img_path = os.path.join(f"./assets/images/{target_path}/{image_path}")
        if not os.path.exists(img_path):
            return None
        try:
            with Image.open(img_path) as img:
                return ImageUtils._prepare_loaded_image(np.array(img), resize)
        except Exception as e:
            log.error(f"从指定路径加载图片失败: {e}")
            return None

    @staticmethod
    def _prepare_loaded_image(image, resize):
        """统一处理磁盘加载出来的模板图片。"""
        channel = image.shape[2] if len(image.shape) > 2 else 1
        if channel > 3:
            image = image[:, :, :3].copy()
        if resize:
            win_size = cfg.set_win_size
            # 如果win_size 为2560*1440，则不变，否则将图片缩放到对应的16：9大小
            if win_size < 1440:
                image = cv2.resize(
                    image,
                    None,
                    fx=win_size / 1440,
                    fy=win_size / 1440,
                    interpolation=cv2.INTER_AREA,
                )
            elif win_size > 1440:
                image = cv2.resize(
                    image,
                    None,
                    fx=win_size / 1440,
                    fy=win_size / 1440,
                    interpolation=cv2.INTER_LINEAR,
                )
        if len(image.shape) == 2:
            return image
        if image.shape[2] == 1:
            return image[:, :, 0]
        return cv2.cvtColor(image, cv2.COLOR_RGB2GRAY)

    @staticmethod
    def image_channel(image):
        """
        通过检查图像数组的维度来确定图像的通道数。
        如果图像是三维数组，则假设第一个维度是高度，第二个维度是宽度，
        第三个维度是通道数。如果图像是二维数组，则认为图像是单通道的。
        :param image: 输入的图像数组，可以是二维或三维数组。
        :return int: 图像的通道数。如果是三维数组，则返回第三维的大小；
                    如果是二维数组，则返回0，表示单通道。
        """
        # 如果图像数组是三维的，则返回第三维的大小作为通道数
        # 否则，返回0，表示图像是单通道的
        return image.shape[2] if len(image.shape) == 3 else 0

    @staticmethod
    def get_bbox(image, threshold=0):
        """
        获取图像中有效区域的边界框。
        如果图像是彩色的（具有3个通道），则首先将其转换为灰度图像，然后计算边界框。
        :param image: numpy数组，表示输入的图像。
        :param threshold: float，定义有效像素的阈值，默认为0。
        :return tuple: (xmin, ymin, xmax, ymax) 表示有效区域的边界框坐标。
        """
        # 检查图像是否有3个通道（RGB），如果有，则转换为灰度图像
        if ImageUtils.image_channel(image) == 3:
            image = np.max(image, axis=2)
        # 计算在x轴方向上有效像素的投影，并找到投影大于阈值的像素列
        x = np.where(np.max(image, axis=0) > threshold)[0]
        # 计算在y轴方向上有效像素的投影，并找到投影大于阈值的像素行
        y = np.where(np.max(image, axis=1) > threshold)[0]
        # 返回有效区域的边界框坐标
        return x[0], y[0], x[-1] + 1, y[-1] + 1

    @staticmethod
    def get_bbox_from_path(image_path):
        """Load an asset once and return its content bbox for the active state."""

        state_key = (
            image_path,
            tuple(path_manager.pic_path),
            path_manager.current_theme,
            path_manager.current_language,
            float(getattr(cfg, "set_win_size", 0) or 0),
        )
        if state_key in ImageUtils._bbox_cache:
            return ImageUtils._bbox_cache[state_key]

        image = ImageUtils.load_image(image_path)
        bbox = None
        if image is not None:
            try:
                bbox = tuple(int(value) for value in ImageUtils.get_bbox(image))
            except Exception as error:
                log.debug(f"计算图片边界失败: {image_path} ({error})")
                bbox = None
        ImageUtils._bbox_cache[state_key] = bbox
        return bbox

    @staticmethod
    def clear_bbox_cache() -> None:
        """Drop cached reference-frame bboxes after a resource state change."""

        ImageUtils._bbox_cache.clear()

    @staticmethod
    def crop(image, area, copy=True):
        """
        从图像中裁剪出指定区域。
        如果裁剪区域超出图像边界，函数将自动调整裁剪区域，并在超出边界的部分填充黑色。
        :param image: numpy数组，输入的图像。
        :param area: 四元组，指定裁剪区域的左上角和右下角坐标（x1, y1, x2, y2）。
        :param copy: 是否返回图像的一个副本。默认为True。
        :return numpy数组，裁剪后的图像。
        """
        # 将裁剪区域的坐标转换为整数，以适应图像坐标系。
        x1, y1, x2, y2 = map(int, map(round, area))
        # 获取图像的尺寸。
        h, w = image.shape[:2]
        # 计算裁剪区域是否超出图像边界，如果超出，则记录需要填充的尺寸。
        border = np.maximum((0 - y1, y2 - h, 0 - x1, x2 - w), 0)
        # 确保裁剪区域的坐标不小于0。
        x1, y1, x2, y2 = np.maximum((x1, y1, x2, y2), 0)
        # 从图像中裁剪出指定区域。
        image = image[y1:y2, x1:x2]
        # 如果裁剪区域超出了图像边界，对裁剪后的图像进行边框填充。
        if sum(border) > 0:
            image = cv2.copyMakeBorder(image, *border, borderType=cv2.BORDER_CONSTANT, value=(0, 0, 0))
        # 如果需要，返回图像的一个副本。
        if copy:
            image = image.copy()
        return image

    @staticmethod
    def get_grey_normalized_pic(img_array):
        """
        将传入的图片数组转为灰度模式并进行自适应直方图均衡化
        :param img_array: 图片数组
        :return cl1: 经过自适应直方图均衡化处理后的灰度图像
        """
        # 检查输入图像是否为彩色图像，如果是则转换为灰度图像
        if len(img_array.shape) == 3 and img_array.shape[2] == 3:
            img = cv2.cvtColor(img_array, cv2.COLOR_RGB2GRAY)
        else:
            img = img_array.copy()
        # 创建自适应直方图均衡化对象，设置限制对比度的参数为2.0，划分的网格大小为8x8
        clahe = createCLAHE(clipLimit=2.0, tileGridSize=(8, 8))
        # 应用自适应直方图均衡化处理，以改善图像的对比度
        cl1 = clahe.apply(img)
        return cl1

    @staticmethod
    def _blur_enabled() -> bool:
        return bool(getattr(cfg, "enable_template_blur", False))

    @classmethod
    def _select_match_region(cls, screenshot, bbox, model):
        """Resolve the final search region and its full-screen origin."""

        shape = screenshot.shape
        if len(shape) == 2:
            height, width = shape
        elif len(shape) == 3:
            height, width = shape[:2]
        else:
            return None, (0, 0)

        if bbox is None or model == "aggressive":
            return screenshot, (0, 0)

        offset = 100 if model == "normal" else 30
        x1 = max(int(round(bbox[0])) - offset, 0)
        y1 = max(int(round(bbox[1])) - offset, 0)
        x2 = min(int(round(bbox[2])) + offset, width)
        y2 = min(int(round(bbox[3])) + offset, height)
        if x2 <= x1 or y2 <= y1:
            return None, (0, 0)
        return screenshot[y1:y2, x1:x2], (x1, y1)

    @classmethod
    def _match_region_full(cls, region, template, origin=(0, 0)):
        """Match one template against a region without any downscaling."""

        if (
            region is None
            or template is None
            or not hasattr(region, "shape")
            or not hasattr(template, "shape")
            or region.size == 0
            or template.size == 0
            or len(region.shape) < 2
            or len(template.shape) < 2
        ):
            return None, 0.0
        template_height, template_width = template.shape[:2]
        if region.shape[0] < template_height or region.shape[1] < template_width:
            return None, 0.0

        source = region
        if cls._blur_enabled():
            source = cv2.GaussianBlur(source, (3, 3), 0)
        result = cv2.matchTemplate(source, template, cv2.TM_CCOEFF_NORMED)
        _, max_val, _, max_loc = cv2.minMaxLoc(result)
        center = (
            int(origin[0]) + int(max_loc[0]) + template_width // 2,
            int(origin[1]) + int(max_loc[1]) + template_height // 2,
        )
        return center, float(max_val)

    @classmethod
    def _coarse_scale(cls, region_shape, template_shape) -> int:
        """Choose a conservative downscale factor for coarse matching."""

        region_height, region_width = region_shape[:2]
        template_height, template_width = template_shape[:2]
        area = int(region_height) * int(region_width)
        template_min = min(int(template_height), int(template_width))
        if area < cls._COARSE_MIN_AREA or template_min < cls._COARSE_MIN_TEMPLATE_SIDE:
            return 1

        if area < 900_000:
            scale = 2
        elif area < 2_000_000:
            scale = 3
        else:
            scale = 4
        max_by_template = template_min // cls._COARSE_MIN_SCALED_SIDE
        return max(1, min(scale, max_by_template, cls._COARSE_MAX_SCALE))

    @classmethod
    def _resize_for_coarse(cls, region, template, scale):
        """Return downscaled region/template and exact scale factors."""

        region_height, region_width = region.shape[:2]
        template_height, template_width = template.shape[:2]
        small_width = max(1, int(round(region_width / scale)))
        small_height = max(1, int(round(region_height / scale)))
        small_template_width = max(1, int(round(template_width / scale)))
        small_template_height = max(1, int(round(template_height / scale)))
        if small_width < small_template_width or small_height < small_template_height:
            return None
        small_region = cv2.resize(
            region,
            (small_width, small_height),
            interpolation=cv2.INTER_AREA,
        )
        small_template = cv2.resize(
            template,
            (small_template_width, small_template_height),
            interpolation=cv2.INTER_AREA,
        )
        return (
            small_region,
            small_template,
            float(region_width) / float(small_width),
            float(region_height) / float(small_height),
        )

    @staticmethod
    def _top_peaks(result, count, suppress_width, suppress_height):
        """Return up to ``count`` separated peaks from a template result map."""

        scores = np.array(result, copy=True)
        peaks = []
        for _ in range(max(1, int(count))):
            _, max_val, _, max_loc = cv2.minMaxLoc(scores)
            max_val = float(max_val)
            if not math.isfinite(max_val) or max_val <= -1.0:
                break
            x, y = int(max_loc[0]), int(max_loc[1])
            peaks.append((x, y))
            x1 = max(0, x - max(1, suppress_width) // 2)
            y1 = max(0, y - max(1, suppress_height) // 2)
            x2 = min(scores.shape[1], x + max(1, suppress_width) // 2 + 1)
            y2 = min(scores.shape[0], y + max(1, suppress_height) // 2 + 1)
            scores[y1:y2, x1:x2] = -1.0
        return peaks

    @classmethod
    def _verify_coarse_candidate(cls, region, template, cand_x, cand_y, scale_x, scale_y, origin):
        """Verify one coarse peak in a small full-resolution window."""

        template_height, template_width = template.shape[:2]
        region_height, region_width = region.shape[:2]
        pad_x = max(8, int(round(scale_x * 2)), int(round(template_width * 0.15)))
        pad_y = max(8, int(round(scale_y * 2)), int(round(template_height * 0.15)))
        full_x = int(round(cand_x * scale_x))
        full_y = int(round(cand_y * scale_y))
        full_x = max(0, min(full_x, region_width - template_width))
        full_y = max(0, min(full_y, region_height - template_height))
        x1 = max(0, full_x - pad_x)
        y1 = max(0, full_y - pad_y)
        x2 = min(region_width, full_x + template_width + pad_x)
        y2 = min(region_height, full_y + template_height + pad_y)
        if x2 <= x1 or y2 <= y1:
            return None, 0.0
        return cls._match_region_full(
            region[y1:y2, x1:x2],
            template,
            (origin[0] + x1, origin[1] + y1),
        )

    @classmethod
    def _coarse_match(cls, region, template, scale, threshold):
        """Run one coarse pass and verify its top candidates at full resolution.

        Returns ``(center, score, fallback)``.  ``fallback=True`` asks the
        caller to perform one complete full-resolution match, which preserves
        the old behavior when the coarse peak is ambiguous.
        """

        resized = cls._resize_for_coarse(region, template, scale)
        if resized is None:
            return None, 0.0, True
        small_region, small_template, scale_x, scale_y = resized
        result = cv2.matchTemplate(small_region, small_template, cv2.TM_CCOEFF_NORMED)
        _, coarse_max, _, _ = cv2.minMaxLoc(result)
        coarse_max = float(coarse_max)
        if not math.isfinite(coarse_max):
            return None, 0.0, True
        if coarse_max < cls._COARSE_MIN_SCORE:
            return None, 0.0, False

        peaks = cls._top_peaks(
            result,
            cls._COARSE_MAX_CANDIDATES,
            small_template.shape[1] // 2,
            small_template.shape[0] // 2,
        )
        best_center = None
        best_score = 0.0
        for cand_x, cand_y in peaks:
            center, score = cls._verify_coarse_candidate(
                region,
                template,
                cand_x,
                cand_y,
                scale_x,
                scale_y,
                (0, 0),
            )
            if center is not None and score > best_score:
                best_center = center
                best_score = score
            if threshold is not None and best_score >= threshold:
                return best_center, best_score, False

        if threshold is None:
            return best_center, best_score, False

        fallback_floor = max(0.55, float(threshold) - 0.15)
        if coarse_max >= fallback_floor:
            return None, 0.0, True
        return best_center, best_score, False

    @classmethod
    def match_template(
        cls,
        screenshot,
        template,
        bbox,
        model="clam",
        *,
        threshold=None,
        coarse=True,
    ):
        """Match a template, optionally using a coarse-to-fine search."""

        try:
            if (
                screenshot is None
                or template is None
                or not hasattr(screenshot, "shape")
                or not hasattr(template, "shape")
                or screenshot.size == 0
                or template.size == 0
                or len(screenshot.shape) < 2
                or len(template.shape) < 2
            ):
                return None, 0.0

            region, origin = cls._select_match_region(screenshot, bbox, model)
            if region is None or region.size == 0:
                return None, 0.0
            if region.shape[0] < template.shape[0] or region.shape[1] < template.shape[1]:
                return None, 0.0

            scale = cls._coarse_scale(region.shape, template.shape) if coarse else 1
            if scale > 1 and not cls._blur_enabled() and float(np.std(template)) >= 1.0:
                center, score, fallback = cls._coarse_match(region, template, scale, threshold)
                if not fallback:
                    if center is not None:
                        center = (center[0] + origin[0], center[1] + origin[1])
                    return center, score
            return cls._match_region_full(region, template, origin)
        except Exception as e:
            log.error(f"图片识别出现错误：{e}")
            return None, 0.0

    @staticmethod
    def _component_peaks(result, threshold):
        """Return sorted component peaks from a template result map."""

        match_mask = np.asarray(result >= threshold, dtype=np.uint8)
        if not np.any(match_mask):
            return []
        label_count, labels, stats, _ = cv2.connectedComponentsWithStats(match_mask, connectivity=8)
        points = []
        for label in range(1, label_count):
            x, y, width, height, _ = stats[label]
            component_mask = (labels[y : y + height, x : x + width] == label).astype(np.uint8)
            component_result = result[y : y + height, x : x + width]
            _, _, _, max_loc = cv2.minMaxLoc(component_result, mask=component_mask)
            point_x = int(x + max_loc[0])
            point_y = int(y + max_loc[1])
            points.append((point_x, point_y, float(result[point_y, point_x])))
        points.sort(key=lambda item: item[2], reverse=True)
        return points

    @classmethod
    def _coarse_multi_match(cls, screenshot, template, threshold, min_dist, scale):
        """Coarse multi-target search; returns ``None`` when full fallback is needed."""

        resized = cls._resize_for_coarse(screenshot, template, scale)
        if resized is None:
            return None
        small_screenshot, small_template, scale_x, scale_y = resized
        result = cv2.matchTemplate(small_screenshot, small_template, cv2.TM_CCOEFF_NORMED)
        coarse_max = float(np.max(result)) if result.size else 0.0
        # Downscaling lowers TM_CCOEFF_NORMED scores (especially for noisy or
        # high-frequency art), so the coarse gate must be well below the final
        # threshold.  Verification below still applies the caller's threshold.
        coarse_threshold = max(cls._COARSE_MIN_SCORE, min(0.45, float(threshold) * 0.40))
        points = cls._component_peaks(result, coarse_threshold)
        if not points:
            return []
        template_height, template_width = template.shape[:2]
        verified = []
        for cand_x, cand_y, _ in points[: cls._COARSE_MAX_MULTI_CANDIDATES]:
            center, score = cls._verify_coarse_candidate(
                screenshot,
                template,
                cand_x,
                cand_y,
                scale_x,
                scale_y,
                (0, 0),
            )
            if center is not None and score >= threshold:
                verified.append((center, float(score)))
        if not verified:
            if coarse_max >= max(0.60, float(threshold) - 0.15):
                return None
            return []
        verified.sort(key=lambda item: item[1], reverse=True)
        kept = []
        min_dist_squared = float(min_dist) ** 2
        for center, _ in verified:
            top_left = (center[0] - template_width // 2, center[1] - template_height // 2)
            if all(
                (top_left[0] - kept_point[0]) ** 2 + (top_left[1] - kept_point[1]) ** 2 > min_dist_squared
                for kept_point in kept
            ):
                kept.append(top_left)
        return [
            (int(point[0] + template_width / 2), int(point[1] + template_height / 2))
            for point in kept
        ]

    @classmethod
    def match_template_with_multiple_targets(
        cls,
        screenshot,
        template,
        threshold,
        min_dist=10,
        *,
        coarse=True,
    ):
        """在一张截图中查找多个模板位置。

        ``matchTemplate`` 的结果通常包含同一目标周围的大量相邻像素。
        旧实现会对所有过阈值像素排序，复杂度和临时对象数量会随截图
        大小快速增长。先把过阈值区域分成连通块，每块只取最高点，
        再执行原有的最小距离抑制，可以保留目标级别的结果并避免全量排序。
        大图会先做一次粗搜，再对候选做全分辨率复核。
        """

        if (
            screenshot is None
            or template is None
            or not hasattr(screenshot, "shape")
            or not hasattr(template, "shape")
            or screenshot.size == 0
            or template.size == 0
            or len(screenshot.shape) < 2
            or len(template.shape) < 2
            or screenshot.shape[0] < template.shape[0]
            or screenshot.shape[1] < template.shape[1]
        ):
            return []

        threshold = 0.0 if threshold is None else float(threshold)
        width, height = cls.get_image_info(template)
        scale = cls._coarse_scale(screenshot.shape, template.shape) if coarse else 1
        if scale > 1 and not cls._blur_enabled() and float(np.std(template)) >= 1.0:
            result = cls._coarse_multi_match(screenshot, template, threshold, min_dist, scale)
            if result is not None:
                return result

        # 模板已按开关预糊，仅当开启时对截图高斯
        if cls._blur_enabled():
            blurred_screenshot = cv2.GaussianBlur(screenshot, (3, 3), 0)
            res = cv2.matchTemplate(blurred_screenshot, template, cv2.TM_CCOEFF_NORMED)
        else:
            res = cv2.matchTemplate(screenshot, template, cv2.TM_CCOEFF_NORMED)
        points = cls._component_peaks(res, threshold)
        if not points:
            log.debug(f"未找到匹配项，最高匹配度为：{np.max(res)}")
            return []

        center_points = []
        min_dist_squared = float(min_dist) ** 2
        for point_x, point_y, _ in points:
            if all(
                (point_x - kept_point[0]) ** 2 + (point_y - kept_point[1]) ** 2 > min_dist_squared
                for kept_point in center_points
            ):
                center_points.append((point_x, point_y))

        return [
            (int(point[0] + width / 2), int(point[1] + height / 2))
            for point in center_points
        ]

    @staticmethod
    def get_image_info(image_array):
        """
        获取图片的信息，如尺寸。
        :param image_array: 图片的 numpy 数组。
        :return: 图片的宽度和高度。
        """
        return image_array.shape[::-1]

    @staticmethod
    def feature_descriptors(image):
        """Return ORB descriptors for an image, using the same preprocessing as matching."""
        resized = cv2.resize(image, None, fx=2, fy=2, interpolation=cv2.INTER_LINEAR)
        orb = cv2.ORB_create(nfeatures=1000, scaleFactor=1.2, edgeThreshold=10)
        return orb.detectAndCompute(resized, None)

    @staticmethod
    def feature_matching(
        template_img,
        target_img,
        min_matches=8,
        *,
        template_features=None,
        target_features=None,
    ):
        """Match ORB features, optionally reusing descriptors for a frame."""
        if template_features is None:
            template_features = ImageUtils.feature_descriptors(template_img)
        if target_features is None:
            target_features = ImageUtils.feature_descriptors(target_img)

        kp1, des1 = template_features
        kp2, des2 = target_features
        if des1 is None or des2 is None or not kp1 or not kp2:
            return False, 0

        # 使用FLANN匹配器
        FLANN_INDEX_LSH = 6
        index_params = dict(algorithm=FLANN_INDEX_LSH, table_number=6, key_size=12, multi_probe_level=1)
        search_params = dict(checks=50)
        flann = cv2.FlannBasedMatcher(index_params, search_params)

        try:
            matches = flann.knnMatch(des1, des2, k=2)
        except cv2.error:
            return False, 0

        # 比率测试
        good_matches = []
        for match in matches:
            if len(match) >= 2:
                m, n = match
                if m.distance < 0.7 * n.distance:
                    good_matches.append(m)

        if len(good_matches) >= min_matches:
            # 获取匹配点的坐标
            src_pts = np.float32([kp1[m.queryIdx].pt for m in good_matches])
            dst_pts = np.float32([kp2[m.trainIdx].pt for m in good_matches])

            # 计算单应性矩阵
            M, mask = cv2.findHomography(src_pts, dst_pts, cv2.RANSAC, 5.0)
            if M is None or mask is None:
                return False, len(good_matches)

            # 计算匹配置信度评分
            inlier_ratio = np.count_nonzero(mask) / len(mask)
            if inlier_ratio < 0.3:  # 如果内点比例过低，认为匹配不可靠
                return False, len(good_matches)

            return True, len(good_matches)
        else:
            return False, len(good_matches)

    @staticmethod
    def image_to_blob(image: np.ndarray, scalefactor: float = 1.0 / 255.0) -> np.ndarray:
        """将 HWC uint8 图像转换为 ONNX 常见的 NCHW float32 连续张量并归一化。

        :param image: 输入图像 (H, W, C)
        :param scalefactor: 缩放归一化因子，默认 1/255.0
        :return: (1, C, H, W) 形状的 float32 C 连续内存数组
        """
        if image.ndim != 3:
            raise ValueError(f"输入图像必须为 3 维 (H, W, C)，当前为: {image.shape}")
        blob = np.empty((1, image.shape[2], image.shape[0], image.shape[1]), dtype=np.float32)
        np.multiply(image.transpose(2, 0, 1), np.float32(scalefactor), out=blob[0], dtype=np.float32)
        return blob

    @staticmethod
    def non_max_suppression(
        boxes: list[list[float]] | np.ndarray,
        scores: list[float] | np.ndarray,
        score_threshold: float = 0.0,
        nms_threshold: float = 0.4,
    ) -> list[int]:
        """纯 NumPy 实现的高性能非极大值抑制 (NMS)，行为对齐 cv2.dnn.NMSBoxes。

        :param boxes: 候选框集合，格式为 [x, y, w, h] (左上角坐标与宽高)。
        :param scores: 对应的置信度分数集合。
        :param score_threshold: 置信度阈值，低于此阈值的框被过滤。
        :param nms_threshold: 重叠抑制阈值 (IoU)。
        :return: 保留框在原始输入序列中的索引列表 (按照置信度降序排序)。
        """
        if len(boxes) == 0:
            return []

        boxes_arr = np.asarray(boxes, dtype=np.float32)
        scores_arr = np.asarray(scores, dtype=np.float32)

        if boxes_arr.ndim != 2 or boxes_arr.shape[1] != 4:
            raise ValueError(f"boxes 形状异常，期望 (N, 4)，实际为: {boxes_arr.shape}")
        if scores_arr.ndim != 1 or len(scores_arr) != len(boxes_arr):
            raise ValueError("scores 长度必须与 boxes 数量一致")

        valid = scores_arr >= score_threshold
        if not np.any(valid):
            return []

        indices = np.where(valid)[0]
        b = boxes_arr[valid]
        s = scores_arr[valid]

        x1 = b[:, 0]
        y1 = b[:, 1]
        x2 = x1 + b[:, 2]
        y2 = y1 + b[:, 3]
        areas = np.maximum(0.0, b[:, 2]) * np.maximum(0.0, b[:, 3])

        order = s.argsort()[::-1]
        keep: list[int] = []

        while order.size > 0:
            i = order[0]
            keep.append(int(indices[i]))
            if order.size == 1:
                break

            other = order[1:]
            xx1 = np.maximum(x1[i], x1[other])
            yy1 = np.maximum(y1[i], y1[other])
            xx2 = np.minimum(x2[i], x2[other])
            yy2 = np.minimum(y2[i], y2[other])

            w = np.maximum(0.0, xx2 - xx1)
            h = np.maximum(0.0, yy2 - yy1)
            inter = w * h

            union = areas[i] + areas[other] - inter
            iou = np.divide(inter, union, out=np.zeros_like(inter), where=union > 0)

            remaining = np.where(iou <= nms_threshold)[0]
            order = other[remaining]

        return keep

