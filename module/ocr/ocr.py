import logging

import cv2
import numpy as np
from cv2 import createCLAHE
from PIL import Image
from rapidocr import EngineType, LangDet, LangRec, ModelType, OCRVersion, RapidOCR
from rapidocr.utils.output import RapidOCROutput

from utils.singletonmeta import SingletonMeta


class OCR(metaclass=SingletonMeta):
    def __init__(self, logger: logging.Logger):
        self.logger = logger
        self.engine = RapidOCR(
            params={
                "Det.engine_type": EngineType.ONNXRUNTIME,
                "Det.lang_type": LangDet.CH,
                "Det.model_type": ModelType.MOBILE,
                "Det.ocr_version": OCRVersion.PPOCRV4,
                "Rec.engine_type": EngineType.ONNXRUNTIME,
                "Rec.lang_type": LangRec.CH,
                "Rec.model_type": ModelType.MOBILE,
                "Rec.ocr_version": OCRVersion.PPOCRV4,
            },
            config_path=r"assets\config\default_rapidocr.yaml",
        )
        # CLAHE 配置固定，避免每次 OCR 请求重复创建 OpenCV 对象。
        self._clahe = createCLAHE(clipLimit=2.0, tileGridSize=(8, 8))

    def run(self, image: Image.Image | np.ndarray | str) -> RapidOCROutput:
        """执行OCR识别，支持Image对象、文件路径和np.ndarray对象"""
        try:
            if isinstance(image, str):
                with Image.open(image) as image_file:
                    image_array = np.array(image_file)
            elif isinstance(image, Image.Image):
                image_array = np.array(image)
            elif isinstance(image, np.ndarray):
                image_array = image
            else:
                image_array = np.array(image)

            if image_array.ndim == 2:
                img_cv_gray = image_array
            elif image_array.ndim == 3:
                channel_count = image_array.shape[2]
                if channel_count == 1:
                    img_cv_gray = image_array[:, :, 0]
                elif channel_count == 3:
                    img_cv = cv2.cvtColor(image_array, cv2.COLOR_RGB2BGR)
                    img_cv_gray = cv2.cvtColor(img_cv, cv2.COLOR_BGR2GRAY)
                elif channel_count == 4:
                    img_cv = cv2.cvtColor(image_array, cv2.COLOR_RGBA2BGR)
                    img_cv_gray = cv2.cvtColor(img_cv, cv2.COLOR_BGR2GRAY)
                else:
                    raise ValueError(f"不支持的图像通道数: {channel_count}")
            else:
                raise ValueError(f"不支持的图像维度: {image_array.ndim}")

            if image_array.size == 0 or img_cv_gray.shape[0] == 0 or img_cv_gray.shape[1] == 0:
                return RapidOCROutput()

            # P0止血：大图限幅缩放，长边>900时等比缩小再OCR，结束后将boxes按比例还原
            # 1865x576(8s) -> ~900x278(~0.9s) 实测降低5-8倍，对文字中心点精度影响<2px
            h, w = img_cv_gray.shape[:2]
            scale = 1.0
            max_side = max(h, w)
            if max_side > 900:
                scale = 900.0 / float(max_side)
                new_w = max(1, int(round(w * scale)))
                new_h = max(1, int(round(h * scale)))
                img_cv_gray = cv2.resize(img_cv_gray, (new_w, new_h), interpolation=cv2.INTER_AREA)

            # 自适应均衡化(均值化后更亮)
            processed_image = self._clahe.apply(img_cv_gray)
            results = self.engine(processed_image)
            # 将缩放后的boxes还原到原图坐标系，保持上层my_crop偏移逻辑不变
            if scale != 1.0 and getattr(results, "boxes", None) is not None:
                inv = 1.0 / scale
                scaled_boxes = []
                for box in results.boxes:
                    # box为4x2 ndarray/list，需逐点还原
                    try:
                        arr = np.asarray(box, dtype=float) * inv
                        scaled_boxes.append(arr)
                    except Exception:
                        scaled_boxes.append(box)
                # RapidOCROutput为dataclass，boxes可直接替换
                try:
                    results.boxes = scaled_boxes  # type: ignore[attr-defined]
                except Exception:
                    pass
            self.log_results(results)
            return results
        except Exception as e:
            self.logger.error(e)
            return RapidOCROutput()

    def log_results(self, ocr_results: RapidOCROutput) -> None:
        """记录OCR识别记录"""
        self.logger.debug(f"OCR识别结果：{ocr_results.txts}")
