# PP-OCRv6 Small 与 DirectML 升级实施方案

本方案旨在将边狱公司自动化助手（AhabAssistantLimbusCompany）现有的 **PP-OCRv4 Mobile (CPU)** 架构，升级为 **PP-OCRv6 Small + DirectML (GPU 加速)** 架构，且核心依赖库均升级并对齐至 **最新版本**（`rapidocr>=3.9.2` 与 `onnxruntime-directml>=1.24.4`）。

---

## 一、 方案背景与目标收益

### 1.1 核心痛点与升级驱动
* **消除延迟瓶颈**：当前线上 PP-OCRv4 (CPU) 全图推理需 **5.7 秒**，局部 ROI 需 **1.2 秒**；升级后预期局部降至 **~440ms**（提速 **2.75 倍**），全图降至 **~1.9s**（提速 **2.9 倍**）。
* **攻克英文精度短板**：彻底解决老旧中文模型对英文词条的拼写错误与断词缺陷（如 `Weekly Projection Cap`、`[NORMAL]`、`Floor 1` 等）。
* **释放 CPU 算力**：将繁重的矩阵乘法与卷积运算卸载到 GPU（基于 Windows DirectX 12 DirectML），消除 OCR 与游戏前台（模拟器/Steam端）的 CPU 争抢。

### 1.2 核心依赖版本选型（最新版对齐）

| 依赖组件 | 当前版本 | 目标升级版本 | 选型考量与说明 |
| :--- | :--- | :--- | :--- |
| **`rapidocr`** | `>=3.4.1` (实装 3.9.2) | **`>=3.9.2` (最新版)** | 官方最新版本，原生支持 PP-OCRv6 多语言路由与模型解析 |
| **`onnxruntime` 后端** | `onnxruntime==1.29.0` (纯 CPU) | **`onnxruntime-directml>=1.24.4` (最新版)** | PyPI 最新 DirectML 发行版，内置 DirectX 12 硬件加速支持，兼容 Python 3.14 |
| **OCR 模型版本** | PP-OCRv4 (mobile) | **PP-OCRv6 (small)** | 兼顾 442ms 极速响应与全量复杂字词的工业级稳定度 |
| **方向分类器 (Cls)** | PP-OCRv4 (mobile) | **PP-OCRv4 (mobile)** | 官方未训练独立 v6 分类器，严格保留 v4 避免运行时崩溃 |

---

## 二、 涉及改动的文件与清单

| 序号 | 目标文件路径 | 改动类型 | 详细说明 |
| :---: | :--- | :---: | :--- |
| **1** | `pyproject.toml` | 依赖定义 | 移除 `onnxruntime`，加入最新 `onnxruntime-directml`，更新 `rapidocr>=3.9.2` |
| **2** | `assets/config/default_rapidocr.yaml` | 运行时配置 | 开启 `use_dml: true`，将 Det/Rec 升级为 `PP-OCRv6` + `small`，Cls 锁定 `PP-OCRv4` |
| **3** | `module/ocr/ocr.py` | 代码逻辑 | 更新 RapidOCR 参数枚举，并在 `__init__` 末尾注入 DirectML Shader 静默预热逻辑 |
| **4** | `main_backend.spec` | 打包规则 | 引入 `collect_dynamic_libs("onnxruntime")` 自动打包 `DirectML.dll` 及依赖动态库 |
| **5** | `requirements.txt` & `uv.lock` | 依赖锁文件 | 通过 `uv lock` 与导出脚本同步锁定版本 |

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

### 步骤 4：更新打包构建规则（`main_backend.spec`）

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
3. **无 DirectX 12 设备的平滑降级（Graceful Fallback）**：
   * RapidOCR 内置了 Provider 检测机制，若用户的显卡驱动异常或运行在纯 CPU 环境，DirectML 会自动打印 Warning 并**静默降级为 `CPUExecutionProvider`**，脚本不会闪退。
4. **离线包构建前预缓存模型**：
   * 在执行 PyInstaller 打包生成发布版 `.exe` 前，需确保本地 Python 环境的 `site-packages/rapidocr/models/` 目录下已存在下列 3 个模型文件：
     1. `PP-OCRv6_det_small.onnx`（9.9 MB）
     2. `PP-OCRv6_rec_small.onnx`（21.2 MB）
     3. `ch_ppocr_mobile_v2.0_cls_mobile.onnx`（585 KB）
   * 首次运行 `OCR()` 时 RapidOCR 会自动通过 ModelScope 下载并缓存。

---

## 五、 验收与验证标准

实施完成后，按照以下标准进行系统验收：

1. **自动化测试套件通过**：
   ```bash
   pytest tests/ -k "ocr or vision or theme_pack"
   ```
   验证全部既有 OCR 相关单测（坐标偏移、多语言回退、去空格匹配、缓存）通过。
2. **日志启动态核验**：
   启动主后端进程，检查 `debugLog.log` 是否出现 DirectML 加载成功的特征行：
   ```text
   [INFO] Windows 10 or above detected, try to use DirectML as primary provider
   [INFO] Using ...\PP-OCRv6_det_small.onnx
   [INFO] Using ...\PP-OCRv6_rec_small.onnx
   [DEBUG] OCR DirectML 引擎初始化及预热完成
   ```
3. **耗时指标验收**：
   检查日志中的 `[VISION-OCR]` 打点，确认局部 ROI 耗时稳定在 **400ms ~ 500ms** 范围（原基准为 1200ms），性能达标。
