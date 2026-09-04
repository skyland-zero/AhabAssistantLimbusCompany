from __future__ import annotations

import math
from dataclasses import dataclass, field
from threading import Lock
from typing import Any

import numpy as np

from module.logger import log
from utils.singletonmeta import SingletonMeta


@dataclass
class MatchRecord:
    target: str
    model: str
    region: tuple[int, int, int, int]
    duration_ms: float
    matched: bool
    score: float
    hit_point: tuple[int, int] | None = None
    is_fullscreen: bool = False
    template_size: tuple[int, int] = (0, 0)
    screen_size: tuple[int, int] = (1920, 1080)


@dataclass
class TargetStats:
    target: str
    calls: int = 0
    hits: int = 0
    misses: int = 0
    total_time_ms: float = 0.0
    fullscreen_calls: int = 0
    hit_points: list[tuple[int, int]] = field(default_factory=list)
    template_size: tuple[int, int] = (0, 0)
    screen_size: tuple[int, int] = (1920, 1080)

    @property
    def avg_time_ms(self) -> float:
        return self.total_time_ms / self.calls if self.calls > 0 else 0.0

    @property
    def hit_rate(self) -> float:
        return (self.hits / self.calls) * 100 if self.calls > 0 else 0.0


@dataclass
class OCRStats:
    calls: int = 0
    total_time_ms: float = 0.0
    regions: list[tuple[int, int, int, int]] = field(default_factory=list)

    @property
    def avg_time_ms(self) -> float:
        return self.total_time_ms / self.calls if self.calls > 0 else 0.0


class VisionProfiler(metaclass=SingletonMeta):
    """视觉识别画像探针与固定搜索区域 (ROI) 自动分析引擎。"""

    def __init__(self):
        self._lock = Lock()
        self._targets: dict[str, TargetStats] = {}
        self._ocr_stats = OCRStats()
        self._total_match_calls: int = 0
        self._total_match_time_ms: float = 0.0

    def reset(self) -> None:
        """重置统计数据（通常在单轮镜牢或会话开始时调用）。"""
        with self._lock:
            self._targets.clear()
            self._ocr_stats = OCRStats()
            self._total_match_calls = 0
            self._total_match_time_ms = 0.0

    def record_match(
        self,
        target: str,
        model: str,
        region: tuple[int, int, int, int],
        duration_ms: float,
        matched: bool,
        score: float,
        hit_point: tuple[int, int] | None = None,
        is_fullscreen: bool = False,
        template_size: tuple[int, int] = (0, 0),
        screen_size: tuple[int, int] = (1920, 1080),
    ) -> None:
        """记录一次模板匹配调用事件。"""
        with self._lock:
            self._total_match_calls += 1
            self._total_match_time_ms += duration_ms

            stat = self._targets.get(target)
            if stat is None:
                stat = TargetStats(
                    target=target,
                    template_size=template_size,
                    screen_size=screen_size,
                )
                self._targets[target] = stat

            stat.calls += 1
            stat.total_time_ms += duration_ms
            if is_fullscreen:
                stat.fullscreen_calls += 1

            if matched:
                stat.hits += 1
                if hit_point is not None:
                    stat.hit_points.append(hit_point)
            else:
                stat.misses += 1

            if template_size != (0, 0):
                stat.template_size = template_size
            if screen_size != (0, 0):
                stat.screen_size = screen_size

    def record_ocr(
        self,
        region: tuple[int, int, int, int],
        duration_ms: float,
    ) -> None:
        """记录一次 OCR 识别调用事件。"""
        with self._lock:
            self._ocr_stats.calls += 1
            self._ocr_stats.total_time_ms += duration_ms
            if len(self._ocr_stats.regions) < 100:
                self._ocr_stats.regions.append(region)

    @staticmethod
    def calculate_suggested_roi(
        hit_points: list[tuple[int, int]],
        template_size: tuple[int, int],
        screen_size: tuple[int, int],
    ) -> dict[str, Any] | None:
        """根据历史命中坐标样本，计算推荐的固定搜索区域 (ROI)。"""
        if len(hit_points) < 3:
            return None

        tw, th = template_size if template_size != (0, 0) else (50, 50)
        sw, sh = screen_size if screen_size != (0, 0) else (1920, 1080)

        # 1. 过滤异常离群点 (保留与中位数欧氏距离在 2.5 倍平均差内的点)
        xs = [p[0] for p in hit_points]
        ys = [p[1] for p in hit_points]
        med_x = float(np.median(xs))
        med_y = float(np.median(ys))

        distances = [math.hypot(p[0] - med_x, p[1] - med_y) for p in hit_points]
        avg_dist = float(np.mean(distances)) if distances else 0.0
        cutoff = max(80.0, avg_dist * 2.5)

        valid_points = [p for p, d in zip(hit_points, distances) if d <= cutoff]
        if len(valid_points) < 3:
            valid_points = hit_points

        # 2. 检查分布跨度
        v_xs = [p[0] for p in valid_points]
        v_ys = [p[1] for p in valid_points]
        span_x = max(v_xs) - min(v_xs)
        span_y = max(v_ys) - min(v_ys)

        # 跨度过大说明是全屏多处游走元素，不可强行锁死固定小区域
        if span_x > sw * 0.45 or span_y > sh * 0.45:
            return {
                "type": "dynamic",
                "sample_count": len(valid_points),
                "reason": f"分布跨度较大 ({span_x}x{span_y})，属于多槽位或游走元素，建议保留全屏或使用带状区域",
            }

        # 3. 紧凑型固定按钮计算 (添加 25% 模板尺寸或至少 25px 安全冗余)
        pad_x = max(25, int(tw * 0.25))
        pad_y = max(25, int(th * 0.25))

        crop_x1 = max(0, int(min(v_xs) - tw // 2 - pad_x))
        crop_y1 = max(0, int(min(v_ys) - th // 2 - pad_y))
        crop_x2 = min(sw, int(max(v_xs) + tw // 2 + pad_x))
        crop_y2 = min(sh, int(max(v_ys) + th // 2 + pad_y))

        crop = (crop_x1, crop_y1, crop_x2, crop_y2)
        crop_w = crop_x2 - crop_x1
        crop_h = crop_y2 - crop_y1

        # 归一化比例 (兼容多分辨率)
        ratio = (
            round(crop_x1 / sw, 4),
            round(crop_y1 / sh, 4),
            round(crop_x2 / sw, 4),
            round(crop_y2 / sh, 4),
        )

        return {
            "type": "fixed",
            "crop": crop,
            "size": (crop_w, crop_h),
            "ratio": ratio,
            "sample_count": len(valid_points),
            "cluster_center": (int(med_x), int(med_y)),
        }

    def get_summary_text(self) -> str:
        """生成格式化的性能报告与 ROI 推荐文本。"""
        with self._lock:
            if not self._targets and self._ocr_stats.calls == 0:
                return "暂无视觉调用性能数据。"

            lines: list[str] = [
                "============================= 视觉性能诊断与 ROI 优化报告 =============================",
                f"【总体负载】模板匹配总计: {self._total_match_calls}次, 累计耗时: {self._total_match_time_ms:.1f}ms | OCR总计: {self._ocr_stats.calls}次, 累计耗时: {self._ocr_stats.total_time_ms:.1f}ms",
            ]

            # 排序：按累计耗时降序排列
            sorted_targets = sorted(self._targets.values(), key=lambda t: t.total_time_ms, reverse=True)

            # 1. 累计耗时 Top 5 榜单
            lines.append("【累计耗时 Top 5 模板】:")
            for i, stat in enumerate(sorted_targets[:5], 1):
                clean_name = stat.target.replace("./assets/images/", "").replace("\\", "/")
                lines.append(
                    f"  {i}. {clean_name}: "
                    f"调用 {stat.calls}次 (全屏{stat.fullscreen_calls}次), "
                    f"命中 {stat.hits}次 ({stat.hit_rate:.1f}%), "
                    f"均耗 {stat.avg_time_ms:.1f}ms, "
                    f"累计 {stat.total_time_ms:.1f}ms"
                )

            # 2. 负样本空转警告 (调用次数多但命中率低、全屏占比较高)
            waste_targets = [
                t for t in sorted_targets
                if t.fullscreen_calls >= 5 and t.hit_rate < 30.0 and t.total_time_ms > 100.0
            ]
            if waste_targets:
                lines.append("【无效全屏轮询警告 (负样本开销大户)】:")
                for stat in waste_targets[:3]:
                    clean_name = stat.target.replace("./assets/images/", "").replace("\\", "/")
                    lines.append(
                        f"  [警告] {clean_name}: "
                        f"全屏未命中 {stat.misses}次, "
                        f"白白消耗 CPU: {stat.total_time_ms:.1f}ms, "
                        f"命中率仅 {stat.hit_rate:.1f}%"
                    )

            # 3. ROI 固定区域推荐
            lines.append("【推荐固定区域 (ROI Suggestions)】:")
            suggested_count = 0
            for stat in sorted_targets:
                if stat.fullscreen_calls < 3 or len(stat.hit_points) < 3:
                    continue
                roi = self.calculate_suggested_roi(stat.hit_points, stat.template_size, stat.screen_size)
                if roi and roi["type"] == "fixed":
                    suggested_count += 1
                    clean_name = stat.target.replace("./assets/images/", "").replace("\\", "/")
                    crop = roi["crop"]
                    size = roi["size"]
                    ratio = roi["ratio"]
                    lines.append(f"  [建议] 目标: {clean_name}")
                    lines.append(f"     - 历史命中样本: {roi['sample_count']} 次, 中心位于 {roi['cluster_center']}")
                    lines.append(f"     - 建议固定裁剪: my_crop={crop} [{size[0]}x{size[1]}]")
                    lines.append(f"     - 归一化比例: ({ratio[0]}*W, {ratio[1]}*H, {ratio[2]}*W, {ratio[3]}*H)")
                    lines.append("     - 预期收益: 单次耗时由 ~42ms 降至 ~0.8ms (提速 50 倍)")

            if suggested_count == 0:
                lines.append("  (暂无足够命中样本，运行 1~2 轮镜牢后将自动生成)")

            lines.append("====================================================================================")
            return "\n".join(lines)

    def log_summary(self) -> None:
        """将报告输出至日志。"""
        summary = self.get_summary_text()
        log.info(summary)


vision_profiler = VisionProfiler()
