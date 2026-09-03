# GPUI 缺失设置项与迁移缺口排查清单

## 1. 概述与背景

在 AhabAssistant 从旧版 PyQt5 架构全面重构迁移至 Rust/GPUI 前端架构的过程中，界面交互进行了大幅现代化重构。
其中，**模拟器高级配置（`simulator_port`、`simulator_type`、`start_emulator_timeout` 等）已由全新的设备扫描枚举面板接管，属于设计主动精简，不在遗漏之列**。

本清单梳理了除设备连接外，在本次全量比对（旧版 Qt 界面 `cb9294a0^` vs 后端配置 `config.example.yaml` / `config_typing.py` vs 当前 GPUI 前端）中发现的其他**遗漏或尚未接入 UI 的设置项**，为后续迭代补齐提供权威参考。

---

## 2. 缺失设置项分类清单

### 2.1 核心控制与队伍选择（优先级：高）

#### 1. `select_team_by_order`（选择队伍方式）
- **配置项字段**：`select_team_by_order: bool`（默认 `False`）
- **功能描述**：控制进入战斗/镜牢编队时，是使用 OCR 识别队伍名称（`False`），还是按 1~40 物理槽位序号盲点（`True`）。对于自定义队伍名称（如“破裂队”、“良秀单通”）的用户而言是核心功能。
- **旧版 Qt 表现**：在【设置】$\rightarrow$【游戏设置】提供下拉选择框（`使用队伍名` / `使用队伍序号`）。
- **当前 GPUI 现状**：**完全缺失**，既无 UI 渲染，也未接入 IPC 协议。
- **建议补齐位置**：
  - 方案 A：在【队伍管理】页面顶部提供切换选项；
  - 方案 B：在【设置】$\rightarrow$【系统与防护】或【运行参数】中提供“队伍选择方式”下拉选择框。

#### 2. `win_input_type` 与 `background_click`（Windows 键鼠操控方式）
- **配置项字段**：`win_input_type: str`（可选 `"background"`, `"foreground"`, `"window_move"`）
- **功能描述**：
  - `background`：经典后台模式（PostMessage 模拟输入，不占用鼠标，游戏不可最小化）；
  - `foreground`：前台置顶模式（PyAutoGUI 驱动，最稳定但独占鼠标）；
  - `window_move`：窗口移出可视区模式（规避鼠标抢夺，适合多任务办公）。
- **旧版 Qt 表现**：【游戏设置】中拥有核心 ComboBox 卡片，并配有不同模式的说明与风险提示。
- **当前 GPUI 现状**：在 `windows.rs` 中仅提供了 `use_post_message`（异步 PostMessage）开关，**丢失了最根本的三种输入模式切换下拉框**。
- **建议补齐位置**：主页 $\rightarrow$ 窗口设置（`SetWindows`）$\rightarrow$【常规】或【高级】标签页中增加“操控模式”下拉选择框。

#### 3. `game_path` 与 `auto_set_game_path`（游戏路径与自启动）
- **配置项字段**：`game_path: str`，`auto_set_game_path: bool`
- **功能描述**：记录 Steam 版游戏可执行文件 `LimbusCompany.exe` 的路径，支持自动检测与手动文件浏览，用于脚本一键冷启动游戏。
- **旧版 Qt 表现**：拥有专门的【游戏路径】卡片组，点击“修改”可调起 Windows 文件选择器，支持自动探测。
- **当前 GPUI 现状**：**完全缺失**。GPUI 目前仅能连接已启动的游戏窗口，无冷启动能力。
- **建议补齐位置**：【设置】$\rightarrow$【系统与防护】中增加“游戏可执行文件路径”浏览选择与自动检测。

---

### 2.2 自动化调度与任务增强（优先级：中 ~ 高）

#### 4. `autodaily` 系列（4 组定时日常任务）
- **配置项字段**：
  - `autodaily`, `autodaily2`, `autodaily3`, `autodaily4: bool`（开关）
  - `autodaily_time`, `autodaily_time2`, `autodaily_time3`, `autodaily_time4: str`（时间 `HH:mm`）
  - `autodaily_task` 系列（各组勾选执行的任务：日常本、经验本、纽本、镜牢等）
  - `autodaily_task_exit` 系列（各组执行完毕后的动作：关游戏/关模拟器/休眠/关机等）
- **功能描述**：支持多达 4 组独立的每日定时任务调度，实现真正无人值守挂机。
- **旧版 Qt 表现**：拥有独立整块【定时执行】界面，可按时间点配置各任务组。
- **当前 GPUI 现状**：**完全缺失**。GPUI 仅在主控台提供了执行后动作（`AfterCompletion`），但完全缺失定时触发器 UI。
- **建议补齐位置**：在主导航栏或主控台增加【计划调度】（Scheduling）抽屉或独立面板。

#### 5. `auto_hard_mirror` 与 `hard_mirror_chance`（周四自动切换困难镜牢）
- **配置项字段**：`auto_hard_mirror: bool`，`hard_mirror_chance: int`
- **功能描述**：每周四镜牢奖励重置时，自动开启困难镜牢并打完剩余奖励加成次数，随后自动切回普通镜牢。
- **旧版 Qt 表现**：【游戏设置】中有独立开关与剩余次数展示。
- **当前 GPUI 现状**：**完全缺失**。GPUI 只有手动单选“困难镜牢”并选 5层/15层，没有周四自动轮换逻辑入口。
- **建议补齐位置**：主页 $\rightarrow$ 镜牢任务展开详情 $\rightarrow$【高级选项】中增加“周四自动困难镜牢”开关。

---

### 2.3 性能调优与输入细节（优先级：低 ~ 中）

#### 6. `mouse_down_duration`（鼠标按下持续时间）
- **配置项字段**：`mouse_down_duration: float`（默认 `0.1` 秒）
- **功能描述**：控制 Windows 点击时鼠标按住的毫秒数，可有效解决部分低帧率或卡顿环境下由于点击过快导致游戏未响应的问题。
- **当前 GPUI 现状**：Rust 结构体 `SetWindowsConfig` 已定义此字段，但在 `windows.rs` 的高级选项网格中**漏掉了此控件**（仅有截图间隔与鼠标操作间隔）。
- **建议补齐位置**：主页 $\rightarrow$ 窗口设置 $\rightarrow$【高级】标签页中补齐数值微调或下拉框（0.05s / 0.1s / 0.2s）。

#### 7. Scrcpy 高级视频流参数
- **配置项字段**：
  - `scrcpy_max_fps: int`（视频最大帧率，默认 15）
  - `scrcpy_video_bit_rate: int`（视频码率，默认 8000000 即 8Mbps）
  - `scrcpy_use_luma_gray: bool`（业务灰度识别直接读取 Y/luma 平面加速，默认 True）
  - `scrcpy_gesture_settle: float`（触控手势结束后的稳定等待时间，默认 0.05s）
- **功能描述**：对 Scrcpy 的视频串流与图像预处理提供底层调优能力。
- **当前 GPUI 现状**：配置已落地后端并有性能优化文档，但前端界面无任何调节入口。
- **建议补齐位置**：【设置】$\rightarrow$【系统与防护】$\rightarrow$【高级图形/流设置】。

---

### 2.4 系统维护与辅助工具（优先级：低）

#### 8. `clean_logs`（自动清理过期日志）
- **配置项字段**：`clean_logs: bool`
- **功能描述**：每周自动清理一周前的过期运行日志，防止 `logs/` 目录过度膨胀占用磁盘空间。
- **旧版 Qt 表现**：【日志设置】中有“自动清理一周前的日志”卡片开关。
- **当前 GPUI 现状**：**完全缺失**（GPUI 目前仅有主页底部的日志抽屉和打开日志文件夹功能）。
- **建议补齐位置**：【设置】$\rightarrow$【系统与防护】中增加日志自动清理开关。

#### 9. `image_resource_source`（图片资源同步源选择）
- **配置项字段**：`image_resource_source: str`（可选 `"Auto"`, `"Gitee"`, `"GitHub"`）
- **功能描述**：在网络受限或 GitHub 无法直连时，切换使用 Gitee 镜像同步图片资源。
- **旧版 Qt 表现**：更新设置中有同步源选择卡片。
- **当前 GPUI 现状**：`resources.rs` 资源中心页面支持检查与一键同步，但**没有同步源切换下拉框**。
- **建议补齐位置**：【资源中心】页面工具栏或【设置】$\rightarrow$【更新】中补齐同步源下拉组件。

#### 10. 社群入口与辅助工具
- **社群链接**：旧版包含官方交流 QQ 群（`946227774`）与 Discord 链接；当前 GPUI 的关于卡片仅有 GitHub 仓库。
- **截图基准测试（`screenshot_benchmark`）**：旧版提供一键测 10 次截图平均耗时（ms）的功能，辅助用户评估当前设备画面延迟；当前 GPUI 工具箱内暂未恢复。

---

## 3. 建议恢复路线图

1. **第一阶段（伴随本次队伍优化）**：
   - 在前端界面暴露 **`select_team_by_order`**，让用户可自由在“按名称 OCR”与“按序号直点”之间切换；
   - 在 `windows.rs` 中补回 **`win_input_type`** 操控模式下拉框；
   - 在 `windows.rs` 中补上遗漏渲染的 **`mouse_down_duration`**。
2. **第二阶段（自动化补完）**：
   - 镜牢卡片补齐 **`auto_hard_mirror`**（周四自动困难镜牢）；
   - 规划 **`autodaily`** 定时日常调度界面的重构实现。
3. **第三阶段（系统与体验微调）**：
   - 资源中心增加 **`image_resource_source`** 源选择；
   - 设置中加入 **`clean_logs`** 自动清理；
   - 关于页面补齐社群入口。
