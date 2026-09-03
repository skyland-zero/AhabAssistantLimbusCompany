# PP-OCRv6 Small 与 DirectML 统一升级实施方案
## （涵盖 OCR 与镜牢地图节点检测 best.onnx 双模型 GPU 加速）

本方案旨在将边狱公司自动化助手（AhabAssistantLimbusCompany）现有的深度学习推理体系整体升级为 **Windows DirectML (DirectX 12 GPU 加速)** 架构，涵盖项目内两大核心神经网络模型：
1. **OCR 文本识别引擎**：升级为 **PP-OCRv6 Small + DirectML**（最新版 `rapidocr>=3.9.2`）；
2. **镜牢地图节点检测模型**：将 `assets/model/best.onnx`（YOLOv8，544×960）接入 **DirectML GPU 加速**；
3. **底层统一推理依赖**：全面对齐至 PyPI 最新版 **`onnxruntime-directml>=1.24.4`**。

---

## 一、 方案背景与目标收益

### 1.1 核心痛点与双模型加速驱动
* **OCR 识别延迟痛点**：当前线上 PP-OCRv4 (CPU) 全图推理需 **5.7 秒**，局部 ROI 需 **1.2 秒**；升级后预期局部降至 **~440ms**（提速 **2.75 倍**），全图降至 **~1.9s**（提速 **2.9 倍**），且彻底解决老旧中文模型对英文词条的拼写断词问题。
* **寻路节点检测（第二大 AI 任务）**：`assets/model/best.onnx` 是输入高达 `544×960`、包含 10,710 个候选锚框的 YOLOv8 目标检测模型，在 CPU 上单次推理约 **50ms**。既然环境已引入 `onnxruntime-directml`，将其一并切换至 GPU 运算可压至 **10~15ms**，实现零额外依赖、零新增复杂度的顺手提速。
* **释放 CPU 算力**：将全部浮点密集矩阵运算卸载至 GPU（NVIDIA/AMD/Intel 独显及核显均支持），彻底避免后台助手与游戏前台争抢 CPU 核心。

### 1.2 核心组件版本与指标对齐

| 核心组件 | 当前基准 | 升级后目标 | 耗时变化与收益 |
| :--- | :--- | :--- | :--- |
| **`onnxruntime` 后端** | `1.29.0` (纯 CPU) | **`onnxruntime-directml>=1.24.4`** | 全局接入 DirectX 12 DML 驱动 |
| **`rapidocr` 框架** | `>=3.4.1` (实装 3.9.2) | **`>=3.9.2`** | 完整支持 PP-OCRv6 路由与多语言模型 |
| **OCR 模型 (Det+Rec)** | PP-OCRv4 (mobile) | **PP-OCRv6 (small)** | 局部耗时由 **1214ms $\rightarrow$ 442ms** |
| **节点检测 (`best.onnx`)**| YOLOv8 (CPU) | **YOLOv8 (DirectML)** | 节点检测由 **50ms $\rightarrow$ 10~15ms** |
| **显存/共享内存总占用** | 0 MB | **~180 MB** (OCR ~150M + YOLO ~30M) | 负荷极轻，不影响前台游戏 |

---

## 二、 涉及改动的文件与清单

| 序号 | 目标文件路径 | 改动类型 | 详细说明 |
| :---: | :--- | :---: | :--- |
| **1** | `pyproject.toml` | 依赖定义 | 移除 `onnxruntime`，加入最新 `onnxruntime-directml`，更新 `rapidocr>=3.9.2` |
| **2** | `assets/config/default_rapidocr.yaml` | OCR 配置 | 开启 `use_dml: true`，Det/Rec 设为 `PP-OCRv6` + `small`，Cls 锁定 `PP-OCRv4` |
| **3** | `module/ocr/ocr.py` | OCR 核心代码 | 更新枚举参数，并在单例 `__init__` 末尾注入 DirectML Shader 预热逻辑 |
| **4** | `tasks/mirror/search_road.py` | 寻路节点检测 | 为 `best.onnx` 指定 `providers=["DmlExecutionProvider", ...]` 并加入首帧预热 |
| **5** | `main_backend.spec` | 打包规则 | 引入 `collect_dynamic_libs("onnxruntime")` 确保 `DirectML.dll` 自动打入二进制 |
| **6** | `requirements.txt` & `uv.lock` | 依赖锁文件 | 通过 `uv lock` 与导出脚本同步锁定最新依赖版本 |

---

## 三、 分步实施细节与代码改动规范

### 步骤 1：修改 `pyproject.toml` 依赖声明

修改根目录下的 `pyproject.toml`，引入最新版的 `onnxruntime-directml` 与 `rapidocr`：

```toml
# pyproject.toml
dependencies = [
    "pywin32",
    "PyAutoGUI",
    "pynput",
    "pyperclip",
    "playsound3",
    "requests",
    "packaging",
    "markdown-it-py",
    "mdit-py-plugins",
    "linkify-it-py",
    "ruamel.yaml",
    "numpy",
    "opencv-python-headless",
    "pillow",
    "psutil",
-   "onnxruntime==1.29.0",
+   "onnxruntime-directml>=1.24.4",
    "concurrent-log-handler>=0.9.28",
    "colorlog>=6.9.0",
-   "rapidocr>=3.4.1",
+   "rapidocr>=3.9.2",
    "tzdata>=2025.2",
    "adbutils>=2.10.2",
    "windows-toasts>=1.3.1",
    "pydantic>=2.12.5",
    "websockets>=15.0",
    "cryptography>=42.0",
]
```

执行依赖同步与 requirements 导出命令：
```bash
uv lock
uv sync
python scripts/export-requirements-from-uv-lock.py
```

---

### 步骤 2：更新 OCR 配置文件（`assets/config/default_rapidocr.yaml`）

修改配置文件中的推理引擎配置与模型配置：

```yaml
# assets/config/default_rapidocr.yaml

EngineConfig:
    onnxruntime:
        intra_op_num_threads: 2
        inter_op_num_threads: 2
        enable_cpu_mem_arena: false

        cpu_ep_cfg:
            arena_extend_strategy: "kSameAsRequested"

        use_cuda: false
        cuda_ep_cfg:
            device_id: 0
            arena_extend_strategy: "kNextPowerOfTwo"
            cudnn_conv_algo_search: "EXHAUSTIVE"
            do_copy_in_default_stream: true

-       use_dml: false
+       use_dml: true
        dml_ep_cfg: null

Det:
    engine_type: "onnxruntime"
    lang_type: "ch"
-   model_type: "mobile"
-   ocr_version: "PP-OCRv4"
+   model_type: "small"
+   ocr_version: "PP-OCRv6"

    task_type: "det"

Cls:
    engine_type: "onnxruntime"
    lang_type: "ch"
    model_type: "mobile"
    ocr_version: "PP-OCRv4"    # 重点注意：必须锁定 PP-OCRv4，禁止改为 v6

Rec:
    engine_type: "onnxruntime"
    lang_type: "ch"
-   model_type: "mobile"
-   ocr_version: "PP-OCRv4"
+   model_type: "small"
+   ocr_version: "PP-OCRv6"

    task_type: "rec"
```

---

### 步骤 3：改造 OCR 核心模块（`module/ocr/ocr.py`）

1. 更新单例初始化参数：使用 `ModelType.SMALL` 和 `OCRVersion.PPOCRV6`；
2. 注入 DirectML 首帧预热（Warmup）机制，消化 HLSL Compute Shader 的一次性编译开销（~2.5s）：

```python
# module/ocr/ocr.py
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
                "EngineConfig.onnxruntime.use_dml": True,
                "Det.engine_type": EngineType.ONNXRUNTIME,
                "Det.lang_type": LangDet.CH,
                "Det.model_type": ModelType.SMALL,
                "Det.ocr_version": OCRVersion.PPOCRV6,
                "Cls.engine_type": EngineType.ONNXRUNTIME,
                "Cls.ocr_version": OCRVersion.PPOCRV4,
                "Cls.model_type": ModelType.MOBILE,
                "Rec.engine_type": EngineType.ONNXRUNTIME,
                "Rec.lang_type": LangRec.CH,
                "Rec.model_type": ModelType.SMALL,
                "Rec.ocr_version": OCRVersion.PPOCRV6,
            },
            config_path=r"assets\config\default_rapidocr.yaml",
        )
        # CLAHE 配置固定，避免每次 OCR 请求重复创建 OpenCV 对象。
        self._clahe = createCLAHE(clipLimit=2.0, tileGridSize=(8, 8))

        # 静默预热 DirectML 着色器编译，将首帧 2.5s 开销提前在启动期消化
        try:
            _warmup_img = np.zeros((64, 64, 3), dtype=np.uint8)
            self.engine(_warmup_img)
            self.logger.debug("OCR DirectML 引擎初始化及预热完成")
        except Exception as e:
            self.logger.warning(f"OCR 预热警告（将按需回退）: {e}")
```

---

### 步骤 4：改造镜牢寻路节点检测（`tasks/mirror/search_road.py`）

将 `assets/model/best.onnx` 的 `InferenceSession` 接入 DirectML，并增加首帧 Shader 预热：

```python
# tasks/mirror/search_road.py 约 80 行

def _get_node_detector():
    """Create the ONNX session once and reload it only when the model changes."""
    global _node_detector_input_name, _node_detector_session, _node_detector_signature

    try:
        model_stat = os.stat(_NODE_MODEL_PATH)
        model_signature = (model_stat.st_mtime_ns, model_stat.st_size)
    except OSError:
        model_signature = None

    with _node_detector_lock:
        if _node_detector_session is None or model_signature != _node_detector_signature:
            import onnxruntime as ort

-           _node_detector_session = ort.InferenceSession(_NODE_MODEL_PATH)
+           # 优先使用 DirectML GPU 加速，若显卡不支持则自动平滑回退至 CPU
+           providers = ["DmlExecutionProvider", "CPUExecutionProvider"]
+           _node_detector_session = ort.InferenceSession(_NODE_MODEL_PATH, providers=providers)
            _node_detector_input_name = _node_detector_session.get_inputs()[0].name
            _node_detector_signature = model_signature

+           # 静默预热 YOLOv8 544x960 卷积核着色器编译，消除寻路首次点击卡顿
+           try:
+               dummy = np.zeros((1, 3, 544, 960), dtype=np.float32)
+               _node_detector_session.run(None, {_node_detector_input_name: dummy})
+           except Exception as e:
+               log.warning("镜牢寻路节点检测 DirectML 预热警告: %s", e)

        return _node_detector_session, _node_detector_input_name
```

---

### 步骤 5：更新打包构建规则（`main_backend.spec`）

DirectML 依赖 `DirectML.dll` 与 `onnxruntime_providers_shared.dll`，需确保 PyInstaller 在静态分析阶段将其作为二进制依赖自动收集：

```python
# main_backend.spec
from pathlib import Path
-from PyInstaller.utils.hooks import collect_data_files, collect_submodules
+from PyInstaller.utils.hooks import collect_data_files, collect_submodules, collect_dynamic_libs

root = Path(SPECPATH).resolve()
hiddenimports = []
# ... hiddenimports 收集保持不变

datas = collect_data_files("rapidocr")
datas.append((str(root / "assets" / "binary" / "scrcpy-server.jar"), "assets/binary"))

+# 收集 onnxruntime 动态链接库（包含 DirectML.dll）
+binaries = collect_dynamic_libs("onnxruntime")

a = Analysis(
    [str(root / "main_backend.py")],
    pathex=[str(root)],
-   binaries=[],
+   binaries=binaries,
    datas=datas,
    hiddenimports=sorted(set(hiddenimports)),
    hookspath=[],
    # ... 其余配置保持不变
)
```

---

## 四、 核心技术避坑与容错防护规则

1. **`ModelType` 参数枚举冲突**：
   * PP-OCRv4 规格名为 `MOBILE`；
   * PP-OCRv6 规格名为 `SMALL` / `TINY` / `MEDIUM`；
   * **绝对不可**在 PP-OCRv6 下传入 `ModelType.MOBILE`，否则 RapidOCR 会在路由层直接抛出 `ValueError: Unsupported det.lang_type='ch' for PP-OCRv6 mobile model.`。
2. **方向分类器 `Cls` 严禁设为 v6**：
   * Paddle 官方未单独提供 PP-OCRv6 的 Cls 模型；
   * 若配置 `Cls.ocr_version: "PP-OCRv6"`，RapidOCR 会报 `KeyError: <OCRVersion.PPOCRV6: 'PP-OCRv6'>` 闪退。Cls 必须保持 `PP-OCRv4`。
3. **双模型自动平滑降级（Graceful Fallback）**：
   * RapidOCR 与 `ort.InferenceSession` 均配置了 `providers=["DmlExecutionProvider", "CPUExecutionProvider"]`；
   * 若用户的显卡驱动异常或运行在纯 CPU / 虚拟机环境，两处模型均会自动静默降级为 `CPUExecutionProvider`，自动化主流程稳定不崩溃。
4. **离线发布包构建前预缓存模型**：
   * 在执行 PyInstaller 打包生成发布版 `.exe` 前，需确保本地 Python 环境的 `site-packages/rapidocr/models/` 目录下已存在下列 3 个 OCR 模型文件（`best.onnx` 原本已在 `assets/model/` 下）：
     1. `PP-OCRv6_det_small.onnx`（9.9 MB）
     2. `PP-OCRv6_rec_small.onnx`（21.2 MB）
     3. `ch_ppocr_mobile_v2.0_cls_mobile.onnx`（585 KB）

---

## 五、 验收与验证标准

实施完成后，按照以下标准进行系统验收：

1. **自动化测试套件回归**：
   ```bash
   pytest tests/ -k "ocr or vision or theme_pack or search_road"
   ```
   验证全部 OCR 相关单测与镜牢寻路节点检测单测全部通过。
2. **日志启动与双模型硬件加速核验**：
   启动主后端进程，检查 `debugLog.log` 是否确认两大模型均由 DirectML 接管：
   ```text
   [INFO] Windows 10 or above detected, try to use DirectML as primary provider
   [INFO] Using ...\PP-OCRv6_det_small.onnx
   [INFO] Using ...\PP-OCRv6_rec_small.onnx
   [DEBUG] OCR DirectML 引擎初始化及预热完成
   ```
3. **延迟验收标准**：
   * **OCR 局部 ROI 耗时**：稳定在 **400ms ~ 500ms**（原基准为 1200ms）；
   * **寻路节点检测耗时**：稳定在 **10ms ~ 20ms**（原基准为 50ms）。
