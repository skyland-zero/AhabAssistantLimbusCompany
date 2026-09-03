# OpenCV 精简与 DNN 解耦技术排查指南

本文档记录了项目中关于 **OpenCV 瘦身优化** 与 **`cv2.dnn` 代码解耦** 的技术背景、改动细节、性能基准及故障排查指引，供后续维护与问题排查时参考。

---

## 1. 背景与优化目标

### 1.1 现状痛点
1. **体积臃肿**：`opencv-python-headless` 安装目录高达 **112MB**，主要由以下两部分构成：
   * `opencv_videoio_ffmpeg*.dll`（约 30MB）：视频解码动态库。
   * `cv2.pyd`（约 82MB）：OpenCV 核心 C++ 扩展，内置了庞大的 DNN 深度学习推理引擎（Caffe/ONNX/Torch/TF 解析器）、Video、G-API、ML 等未用到的模块。
2. **职责重叠**：
   * 视频流解码：项目有专属的 `assets/binary/scrcpy-ffmpeg/` 原生 Scrcpy 运行时，根本不需要 OpenCV 的 FFmpeg 插件。
   * 模型推理：项目统一采用独立的 `onnxruntime` 引擎。
   * `cv2.dnn` 的使用仅局限在 `tasks/mirror/search_road.py` 内部的两处辅助工具调用（`blobFromImage` 与 `NMSBoxes`）。
3. **阻碍定制精简**：
   若代码中残留对 `cv2.dnn` 的硬调用，后续在编译或使用定制剪裁版 OpenCV 时，会直接报 `AttributeError: module 'cv2' has no attribute 'dnn'` 崩溃。

---

## 2. 改动清单与文件定位

| 文件路径 | 改动阶段 | 改动内容与说明 |
| :--- | :--- | :--- |
| `main_backend.spec` | 阶段一 | 打包层增加 `a.binaries` 过滤，排除 `opencv_videoio_ffmpeg*.dll`。 |
| `utils/image_utils.py` | 阶段二 | 静态方法增加 `image_to_blob`（纯 NumPy NCHW 转换）与 `non_max_suppression`（纯 NumPy NMS）。 |
| `tasks/mirror/search_road.py` | 阶段二 | 将 `cv2.dnn.blobFromImage` 替换为 `ImageUtils.image_to_blob`；将 `cv2.dnn.NMSBoxes` 替换为 `ImageUtils.non_max_suppression`，并消除多余的 `.tolist()` 列表转换。 |
| `tests/unit/utils/test_image_utils.py` | 阶段二 | 新增 `test_image_to_blob_formats_contiguous_nchw_tensor` 和 `test_non_max_suppression_*` 单元测试。 |

---

## 3. 核心实现与数学等价性

### 3.1 图像前处理：`image_to_blob`
* **原方案**：`cv2.dnn.blobFromImage(model_input, scalefactor=1/255, size=(W, H), swapRB=False)`
* **新方案**：
  ```python
  blob = np.empty((1, image.shape[2], image.shape[0], image.shape[1]), dtype=np.float32)
  np.multiply(image.transpose(2, 0, 1), np.float32(scalefactor), out=blob[0], dtype=np.float32)
  ```
* **等价性保证**：
  1. 维度由 `(H, W, C)` 映射为 `(1, C, H, W)`。
  2. 数值范围归一化至 `[0.0, 1.0]`（float32）。
  3. `np.empty` 确保张量在内存中严格满足 **C 连续（C-Contiguous）**，可无缝零拷贝传入 ONNX Runtime。
  4. 浮点误差：与 OpenCV 输出的最大绝对误差（Max Absolute Error）为 **0.0**。

### 3.2 目标框后处理：`non_max_suppression`
* **原方案**：
  `boxes_array.tolist()` + `scores.tolist()` 传给 `cv2.dnn.NMSBoxes`，返回后再将索引拆包。
* **新方案**：
  基于经典贪心非极大值抑制算法实现：
  1. 根据 `score_threshold` 对候选框和置信度进行向量化预筛选；
  2. 提取左上坐标 $(x_1, y_1)$ 与右下坐标 $(x_2, y_2)$，向量化预计算所有框的面积 $Area$；
  3. 按分数降序迭代，针对剩余候选框，利用 `np.maximum` 与 `np.minimum` 批量计算相交区域 $Inter$，并计算交并比 $IoU = \frac{Inter}{Area_1 + Area_2 - Inter}$；
  4. 剔除 $IoU > nms\_threshold$ 的候选框，直到候选池为空。
* **等价性保证**：
  在不同候选框数量（1 ~ 100）和随机置信度的跨测试验证中，自研 NMS 输出的索引列表与 `cv2.dnn.NMSBoxes` **100% 完全一致**。

---

## 4. 性能测试数据（Benchmark）

在真实 640×640 图像与镜牢节点检测场景（10~30 个候选框）实测：

| 测试项 | 原 OpenCV 方案 | 新 NumPy 自研方案 | 差异分析 |
| :--- | :--- | :--- | :--- |
| **`image_to_blob`** | 3.38 ms | **1.42 ms** |  **提速 ~58%**（免除 Python-C++ 跨语言对象包装和中间分配） |
| **`NMS` (10 候选框)** | 0.03 ms | **0.14 ms** | 微秒级差异（多花费约 110 微秒） |
| **整体前后处理综合耗时** | **~3.41 ms** | **~1.56 ms** |  **整体加速约 1.85 ms** |
| **内存对象分配** | 频繁创建上百个 Python List/Float 小对象 | 纯连续 NumPy 数组原地运算 | 极大减轻 Python GC 压力，利于长时间稳定运行 |

---

## 5. 故障排查（Troubleshooting）

如果镜牢自动寻路中的“节点识别（Node Detection）”出现异常，请按以下步骤定位：

### 症状 1：识别不到任何道路节点（返回 `None`）
* **排查步骤**：
  1. 检查日志中是否有 `节点检测模型输出形状异常` 等报错。
  2. 检查 `assets/model/best.onnx` 是否存在且完整。
  3. 检查当前置信度阈值 `confidence_threshold`（默认在 `search_road.py` 内部计算，或受到配置文件覆盖）。
  4. 运行单元测试验证基础 NMS 是否正常：
     ```bash
     .venv/Scripts/pytest.exe tests/unit/tasks/test_search_road.py tests/unit/utils/test_image_utils.py
     ```

### 症状 2：ONNX Runtime 报内存布局错误（Layout / Contiguity Error）
* **原因**：某些推理后端要求传入的 NumPy 数组必须是连续内存。
* **排查**：检查 `blob.flags['C_CONTIGUOUS']` 是否为 `True`。当前 `ImageUtils.image_to_blob` 使用了 `np.empty(..., dtype=np.float32)` 作为底层连续缓冲区，默认满足连续性要求。

### 症状 3：需要临时对比回退到 OpenCV 原版行为进行验证
若怀疑新算法造成了漏检或误检，可在 `tasks/mirror/search_road.py` 中临时加回旧逻辑进行对拍比对：
```python
# 对拍验证代码段：
result_boxes_new = ImageUtils.non_max_suppression(boxes_array, candidate_scores, confidence_threshold, 0.4)
result_boxes_old = cv2.dnn.NMSBoxes(boxes_array.tolist(), candidate_scores.astype(float).tolist(), confidence_threshold, 0.4)
old_flat = [int(np.asarray(x).reshape(-1)[0]) for x in result_boxes_old]
if result_boxes_new != old_flat:
    log.warning("NMS 对拍不一致！新: %s, 旧: %s", result_boxes_new, old_flat)
```

---

## 6. 阶段三：专属轻量 Wheel 编译与实机接入结果

通过 `.github/workflows/build_opencv_slim.yml` 成功在云端构建出专属的 `opencv-python-headless-slim-win64` Wheel，并在本地环境实装测试通过。

### 6.1 体积缩减三级跳对比

| 指标 | 初始状态 (OpenCV 5.0 Headless) | 阶段一 (排除视频 DLL) | 阶段三 (接入专属裁剪 Wheel) | 累计优化幅度 |
| :--- | :--- | :--- | :--- | :--- |
| **Wheel 安装包体积** | ~44 MB | ~44 MB | **9.5 MB** | 📉 **-78.4%** |
| **`cv2.pyd` 核心扩展库** | 82 MB | 82 MB | **24 MB** | 📉 **-70.7%** |
| **`opencv_videoio_ffmpeg`** | 30 MB | 30 MB (打包排除) | **0 MB (完全未编译)** | 📉 **-100%** |
| **site-packages/cv2 目录**| 113 MB | 113 MB | **~25 MB** | 📉 **-77.9% (-88MB)** |
| **最终打包 `AALC Backend.exe`** | **141 MB** | **129 MB** | **108 MB** | 📉 **净缩减 33 MB (-23.4%)** |

### 6.2 关键兼容性修复记录
1. **Windows + Ninja 平台参数冲突**：
   * 官方 `setup.py` 硬编码注入 `-DCMAKE_GENERATOR_PLATFORM=x64`，与 Ninja 生成器冲突报错；CI 中通过自动化 Patch 剔除该平台参数。
2. **打包自检找不到 FFmpeg DLL**：
   * 官方 `setup.py` 在打包最后一步硬编码要求检查 `opencv_videoio_ffmpeg*.dll`，CI 中使用正则将该依赖检查替换为 `[]`。
3. **NumPy 2.x C-API ABI 适配**：
   * 初始构建使用了 `numpy<2.0`，导致生成的 Wheel 无法在 Python 3.14 / NumPy 2.x 运行时加载；升级为 `numpy>=2.0.0` 编译后实现原生双向兼容。

### 6.3 验证结果
* **模块检查**：`dnn`、`VideoCapture` 等已彻底禁用；`matchTemplate`、`cvtColor`、`resize`、`GaussianBlur`、`ORB`、`FLANN`、`findHomography`、`connectedComponents` 完好保留。
* **RapidOCR 验证**：文字检测与识别推理正常。
* **自动化测试**：全量 **183 个单元测试全部通过**。
* **可执行文件**：`AALC Backend.exe --help` 启动正常。

---

## 7. 最终成果总结

1. **打包体积断崖式下降**：
   `AALC Backend.exe` 单文件从 **141 MB** 降至 **108 MB**，解压运行时目录直接节省约 **88 MB**。
2. **OpenCV DNN 彻底解耦**：
   代码中不再存在对 `cv2.dnn` 的任何依赖，由纯 NumPy 向量化替代，前/后处理综合耗时缩短约 1.8ms。
3. **独立可重复的构建流水线**：
   拥有专属的 `.github/workflows/build_opencv_slim.yml`，后续如需升级 OpenCV 版本只需一键触发 Action 即可自动打包交付。
