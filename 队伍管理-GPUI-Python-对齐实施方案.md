# 队伍管理 GPUI / Python 对齐实施方案

> 目标：修正队伍管理的 GPUI–Python sidecar 契约，使镜牢队伍和经验本队伍的职责清晰、保存结果可靠，并保持旧版 `config.yaml` 与 `TeamSetting` 的兼容性。
>
> 编制日期：2026-08-29

## 一、现状与问题结论

当前调用链已经成立：

```text
GPUI TeamsState
  -> RpcGateway / WebSocketClient
  -> 本机 Python sidecar（WebSocket + JSON-RPC）
  -> RpcDispatcher
  -> BackendApplication.team_list / team_save
  -> cfg.config.teams（Python TeamSetting）
  -> config.yaml
```

当前 `TeamDetail` 和 `TeamMirrorConfig` 是 Python `TeamSetting` 的 RPC 投影，不是同一个存储类型。投影中已经覆盖大部分字段，但存在以下行为问题：

1. 经验本流程只读取 `daily_teams`、`EXP_day_*` 并选择游戏内队伍编号，不读取 `TeamSetting.purpose` 或 `mirrorConfig`。
2. GPUI 编辑器对所有用途都显示五个镜牢配置 Tab，并为经验本队伍创建完整 `mirrorConfig`。
3. `enabled` 实际映射到镜牢队列 `teams_active_queue`，经验本队伍切换该开关会误改镜牢队列。
4. Python 保存了 `sinner_order`，但 `team.list` 当前只按 `chosen_sinners` 的固定人格顺序返回，GPUI 保存后的队伍顺序会改变。
5. sidecar 保存路径不等待 `team.save` 响应。新建队伍的空 ID 未接收 Python 分配的 `team-N`，后续再次保存可能被当作新队伍。
6. Python `TeamSetting` 中的 `shopping_strategy*`、`opening_items*`、`reward_cards*` 等字段没有进入 GPUI DTO；其中 `shopping_strategy*` 仍被镜牢商店执行器读取。
7. Rust 任务 DTO 的默认值、`set_win_position` 表示法以及 Python 的部分 `int | bool` 字段尚未完全统一。

## 二、目标契约与职责划分

### 2.1 存储模型保持兼容

继续以 Python `TeamSetting` 作为旧配置和镜牢执行器的存储模型，不删除旧字段，不把 YAML 迁移成新的嵌套格式。`purpose` 继续作为队伍用途元数据保存：旧配置缺少该字段时默认 `mirror`。

### 2.2 RPC DTO 的职责

Rust 侧继续使用以下 DTO：

- `TeamDetail`：队伍身份、名称、人格顺序、用途、显示状态。
- `TeamMirrorConfig`：镜牢队伍配置；只在镜牢用途需要编辑时展示。
- `SinnerInfo`：人格 ID 与显示名。

`team.list` 返回统一的队伍列表；`team.save` 返回后端规范化后的队伍对象，以便 GPUI 获取真实 ID、人格顺序和启用状态。

目标队伍对象：

```json
{
  "schemaVersion": 1,
  "id": "team-1",
  "name": "燃烧队",
  "sinners": ["yi_sang", "faust"],
  "purpose": "mirror",
  "accessoryScheme": "burn",
  "enabled": true,
  "mirrorConfig": {}
}
```

经验本用途的队伍仍使用同一个顶层对象，但 `mirrorConfig` 返回 `null`，`enabled` 固定表示“不参与镜牢队列”，避免把它误认为经验本启用开关：

```json
{
  "schemaVersion": 1,
  "id": "team-2",
  "name": "经验本斩击队",
  "sinners": ["..."],
  "purpose": "luxcavation",
  "accessoryScheme": "burn",
  "enabled": false,
  "mirrorConfig": null
}
```

### 2.3 用途行为

- `mirror`：显示并编辑完整镜牢配置；`enabled` 操作 `teams_active_queue`。
- `luxcavation`：只显示基础信息和人格顺序，不显示镜牢商店、战斗、星光、观测等配置；保存时不修改镜牢配置，并从镜牢队列移除。
- `general`：为兼容现有队伍，继续显示完整镜牢配置并允许加入镜牢队列。后续若确认 `general` 不应参与镜牢，再单独调整，不在本次改变其语义。

`purpose` 不参与 Python 经验本实际选队逻辑；经验本仍由 Home 页的日常任务配置决定。用途分类只负责队伍管理页面的筛选和编辑器展示，不能替代 `daily_teams` / `EXP_day_*`。

## 三、实施内容

### 3.1 Python sidecar 与后端映射

修改 `module/backend_application.py`：

1. `team_list` / `_team_detail`：
   - 始终返回 `schemaVersion`。
   - 从配置 key 生成稳定的 `team-N` ID。
   - 从 `sinner_order` 重建 `sinners` 顺序；顺序缺失、重复或越界时，按合法顺序输出，并把未排序的已选人格按固定 ID 顺序补到末尾。
   - `purpose == "luxcavation"` 时返回 `mirrorConfig: null`、`enabled: false`。
   - `purpose == "mirror"` 或 `"general"` 时返回完整镜牢 DTO。

2. `team_save`：
   - 接受空 ID，并按当前最大队伍编号分配新的 `team-N`。
   - 校验人格 ID 不重复、最多 12 个；按照输入列表顺序写入 `chosen_sinners` 和 `sinner_order`。
   - 先确定用途，再按用途处理镜牢配置：经验本用途忽略本次提交的镜牢字段，保留存量隐藏配置；镜牢/通用用途正常 patch。
   - 经验本用途自动调用 `remove_team_from_queue`；镜牢/通用用途才处理 `enabled`。
   - 返回保存后的规范化 `TeamDetail`，而不是只返回 `true`。迁移期间客户端仍兼容 `true` 响应，并回退执行一次 `team.list`。

3. 把镜牢 DTO 字段映射补齐。除存储/统计字段外，至少补充：
   - `reward_cards`、`reward_cards_select`；
   - `shopping_strategy`、`shopping_strategy_select`；
   - `opening_items`、`opening_items_select`、`opening_items_system`。

   `_write_mirror_setting` 继续采用 patch/merge，不覆盖未提交字段；新增字段必须进行明确的布尔、整数、列表元素和长度校验，不能直接把未经校验的值写入 Pydantic 模型。

### 3.2 Rust DTO 与状态层

修改 `gpui-app/src/model/teams.rs`：

1. 为新增镜牢字段补充 Rust 类型，并为可选/旧字段提供 `serde(default)`。
2. 保持现有 JSON 字段名：顶层使用 `schemaVersion`、`accessoryScheme`、`mirrorConfig`，镜牢字段保持 Python 现有 snake_case 名称。
3. `mirrorConfig: Option<TeamMirrorConfig>` 作为用途隔离的现有兼容点；不引入新的 tagged-union wire 格式。

修改 `gpui-app/src/state/teams/persistence.rs`：

1. 增加 `saving` 状态，保存期间保留编辑器草稿并禁用重复保存。
2. sidecar 路径等待 `team.save` 响应：
   - 成功且返回对象：用后端返回对象替换列表中的对应队伍，关闭编辑器；
   - 成功但返回 `true`：重新请求 `team.list`，用规范化列表刷新；
   - 失败：保留编辑器内容，展示错误，不更新本地列表。
3. 删除操作也改为成功后再从本地列表移除；删除失败时恢复删除确认状态。
4. JSON 导入/导出保留 `schemaVersion`，镜牢字段使用完整 DTO；导入经验本用途时不把镜牢字段呈现为可编辑配置。

修改 `gpui-app/src/state/teams/mirror.rs` 和 `gpui-app/src/state/teams/types.rs`：

- 只有存在 `mirrorConfig` 时允许更新镜牢字段；经验本编辑器调用这些方法时直接拒绝或不渲染入口。
- 保留互斥逻辑：避免三技能/优先三技能、首回合防御/单通防御仍互斥。
- 将镜牢配置默认值改为与 Python `TeamSetting()` 和 `team.list` 完全一致。

### 3.3 GPUI 队伍页面

修改 `gpui-app/src/pages/teams/overlay.rs` 及编辑器：

1. 根据 `team.purpose` 决定 Tab：
   - `mirror`、`general`：Basic、Shop、Combat、Starlight、Advanced；
   - `luxcavation`：仅 Basic。
2. Basic 页对经验本隐藏：饰品体系、编队码、固定镜牢用途、镜牢启用开关。保留名称、用途、人格顺序。
3. 列表卡片对经验本不显示镜牢专属徽章、镜牢队列状态和镜牢统计。
4. 新建队伍时：
   - 从 `Luxcavation` 分类创建时生成 `purpose=luxcavation`、`mirrorConfig=null`；
   - 从其他分类创建时生成对应用途和与 Python 一致的默认值。
5. 编辑器的 `config` 获取逻辑不再对 `null` 无条件 `unwrap_or_default()` 后渲染镜牢 Tab；默认配置只能用于镜牢/通用编辑器。

### 3.4 日常任务队伍选择

修改 `gpui-app/src/pages/home/task_details/daily.rs`：

- 选项值使用 `team-N` 中的真实编号，而不是 `Vec` 的 `enumerate() + 1`，避免删除队伍后编号错位。
- 保持 `daily_teams`、`EXP_day_*`、`thread_day_*` 写入 `TasksConfig` 的现有协议。
- 不把 `TeamPurpose` 当作 Python 执行器的选择条件；如 UI 需要分类显示，只改变选项展示，不改变保存的队伍编号。

### 3.5 TasksConfig 契约统一

修改 `gpui-app/src/model/tasks.rs` 与 `module/backend_application.py`：

1. 以 Python `tasks.getConfig` 的 JSON 响应为生产契约，统一 Rust 默认值：
   - `set_win_position: "free"`；
   - 日常次数、连续作战、镜牢次数、体力设置和结束动作采用后端缺省值。
2. Python 返回 `hard_mirror`、`no_weekly_bonuses` 时统一转换为 JSON boolean，不能把旧配置中的 `0/1` 直接返回给 Rust `bool`。
3. 所有 `tasks.setConfig` 字段继续由后端做类型和范围校验；Rust 侧的 UI 限制只作为交互保护，不能替代后端校验。

## 四、兼容与迁移策略

1. `RPC_SCHEMA_VERSION` 暂时保持 `1`。本次字段补齐和 `mirrorConfig: null` 都利用已有可选结构，不改变旧字段含义。
2. 旧队伍没有 `purpose` 时仍按 `mirror` 读取，不自动把历史队伍改成经验本用途。
3. 旧队伍中未被 GPUI 展示的镜牢字段必须通过 Python 的深拷贝 patch 保留；导出/导入时也不得静默清除。
4. 旧客户端若收到 `team.save: true`，后端兼容窗口可继续支持；GPUI 优先使用规范化队伍对象，无法解码时回退 `team.list`。
5. 已被错误标记为 `luxcavation` 且存在于 `teams_active_queue` 的队伍，在保存用途或启动后端归一化时移出镜牢队列；不删除队伍配置本身。
6. 不修改经验本 Python 执行流程的队伍选择语义，避免把管理页用途字段误接入自动化流程。

## 五、测试方案

### Python 单元与契约测试

新增/补充 `tests/unit/module/test_backend_application.py`：

- `team.list` 返回 `schemaVersion`、完整用途和规范化 ID；
- 输入有自定义 `sinner_order` 时，列表返回顺序保持不变；
- 重复人格、未知人格、超过 12 人被拒绝；
- 新建队伍返回真实 `team-N`；
- `team.save` 返回规范化队伍对象；
- 经验本保存不写入镜牢 patch，并从镜牢队列移除；
- 镜牢隐藏字段（尤其 `shopping_strategy*`）保存后仍可回读；
- 旧配置缺少 `purpose`、顺序字段或新增字段时可正常读取。

### Rust 单元测试

新增/补充 GPUI 测试：

- 完整 Python 队伍 fixture 可反序列化为 `TeamDetail`；
- `mirrorConfig=null` 的经验本队伍只打开 Basic 编辑器；
- 完整 `TeamMirrorConfig` 序列化后字段名与 JSON 契约一致；
- sidecar 保存成功使用后端返回的真实 ID；
- sidecar 保存失败时编辑器和本地列表不被错误清除；
- 日常队伍下拉框使用真实队伍编号而非列表位置；
- TasksConfig 默认值与 `tasks.getConfig` fixture 一致。

### 集成与回归

继续运行：

```powershell
uv run pytest -q tests/unit tests/integration
cargo test --manifest-path gpui-app/Cargo.toml
```

至少增加一次真实 WebSocket sidecar 的 `team.list -> team.save -> team.list` 闭环测试，并验证：保存结果、ID、人格顺序、用途隔离和错误响应均可被 GPUI DTO 解码。

## 六、验收标准

1. GPUI 生产模式通过真实 sidecar 完成队伍列表加载、保存、删除，保存失败不会产生假成功状态。
2. 新建队伍保存后立即显示后端分配的真实 `team-N`，重复保存不会生成重复队伍。
3. 自定义人格顺序保存并重新加载后保持一致。
4. 经验本队伍编辑器不显示任何镜牢专属设置，保存不会改变镜牢配置或镜牢队列。
5. 镜牢/通用队伍现有配置可完整回读，未在页面展示的兼容字段不会丢失。
6. Home 日常任务仍按 `daily_teams`、`EXP_day_*` 和 `thread_day_*` 选择游戏内队伍，不依赖 `purpose`。
7. Rust 全量测试、Python 全量测试和真实 sidecar 闭环测试通过。

## 七、实施顺序

1. 先补 Python DTO 映射、人格顺序、用途隔离和 `team.save` 规范化返回值。
2. 再补 Rust DTO、异步保存状态机和真实 ID 回写。
3. 再按用途裁剪 GPUI 编辑器与列表展示。
4. 最后统一 TasksConfig 默认值、日常队伍编号映射和完整测试 fixture。

完成以上步骤后，再考虑是否把 `general` 进一步拆成独立的非镜牢队伍类型；该变化不属于本次兼容性修复范围。
