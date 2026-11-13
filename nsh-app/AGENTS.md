# NSH App 开发约束

本文档适用于 `nsh-app/` 及其所有子目录。`nsh-app` 是 AALC 的新前端：**后端用 Rust（`native/`），前端用 TSX**，跑在 [Native Shell](https://nsh.dev) 上。新增、修改、重构时必须遵守本文档；代码与文档冲突时，先调整结构或更新本文档，不得默默恢复旧模式。

## 1. 项目目标

- 界面与交互 1:1 照搬迁移前的客户端（参考图归档在 `docs/reference/gpui/`），不新增功能、不改用户可见行为。
- Rust 拥有后端与状态，TSX 拥有页面与样式，两侧通过生成的 TypeScript 声明强类型对接。
- Mock 与 Python sidecar 共用同一套 JSON-RPC 契约（仓库根 `contract/contract.json`）。
- 浅色/深色、7 个强调色、7 个皮肤由 `theme.json` 的 token 驱动，页面里不出现硬编码色值；
  颜色主题（`themeSeed`）在皮肤之上再覆盖品牌色。

## 2. 目录职责

```text
nsh-app/
├── nsh.json            应用清单：id/version/窗口/图标/主题/capabilities/rust crate
├── App.tsx             入口：mount(App)，只做窗口壳与路由编排
├── theme.json          默认主题（shadcn cssVars，light + dark）
├── theme/<skin>.json   其余皮肤的 token，供 setTheme() 运行时切换
├── shell/              TSX：标题栏、侧栏、Toast 等窗口级 UI
├── components/         TSX：设计系统（按钮、控件、弹层、文本输入、图标）
├── pages/              TSX：home / teams / settings / theme_packs / help
├── native/             Rust 后端 crate 工作区（nsh.json `"rust": "native"`）
│   ├── Cargo.toml      工作区根包 `aalc-native`，members 为 `crates/*`
│   ├── crates/
│   │   ├── contract/   `aalc-contract`：RPC 信封、方法/事件名、结构化错误（只依赖 serde）
│   │   └── model/      `aalc-model`：契约 payload，纯 serde 数据，不依赖 nsh-core
│   └── src/
│       ├── lib.rs      唯一的 #[native_module("aalc/native")] 注册点
│       ├── ipc/        网关、WebSocket 传输、sidecar 生命周期、mock（契约见 crates/contract）
│       ├── state/      状态容器、状态变更、持久化
│       ├── i18n/       双语文案表
│       ├── assets.rs   资源身份表（assets!() 内嵌）
│       └── app/        生命周期、调度、设备、预览等编排
├── assets/             全部随包资源（图标、立绘、状态图、主题、帮助文档）
└── locales/            若启用 NSH 自带 i18n 的翻译文件
```

职责边界：

- `native/` 不 `use` 任何渲染类型（`nsh-core` 之外的 GPUI/平台 crate），只依赖 `nsh-core`、`nsh-base` 与纯 Rust 库。
- `crates/model/` 与 `crates/contract/` 不依赖 `nsh-core`：payload 与契约的改动、契约一致性测试可以用 `cargo test -p aalc-model -p aalc-contract` 秒级验证，不需要 UI 栈。
- `model/` 不发起 RPC，不持有信号，不渲染。
- `state/` 负责状态变更、RPC 调用与持久化，不构建元素。
- `pages/` 只把状态渲染成 UI；事件闭包调用 `state`/`native` 的方法，不得在页面内复制业务状态。
- `components/` 不知道具体页面业务，不直接访问 `AppSession`、`TeamsStore` 或 mock 状态。
- `shell/` 只处理窗口级 UI 与全局反馈。
- 资源只能来自 `assets/`（`asset!()` 或 TSX 的 `import`），页面不得拼接仓库绝对路径。

## 3. Rust ↔ TSX 边界

规则：**每一份状态只有一个 owner，另一侧只持镜像并单向请求变更。**

| 属于 Rust（`native/`） | 属于 TSX（`pages/` `components/` `shell/`） |
| --- | --- |
| 领域规则、校验、解析、金额/统计计算 | 页面组合、路由、导航栈 |
| 触碰文件、socket、进程，或 CPU 密集的逻辑 | 文案、布局、样式、实验性界面 |
| 必须跨 TSX 热更新存活的状态（执行状态、会话、队伍数据） | 屏幕内的瞬时状态（弹层开关、筛选、草稿、当前 tab） |
| 需要 `cargo test` 就能验证的逻辑 | 表单步骤顺序 |

导出方式：`#[module]`（无状态函数/异步调用）、`#[class]`（TSX 持有生命周期的状态）、`#[component]`（逐帧绘制或复杂交互的视图，例如日志面板、预览画面）。

两条必须避免的错误：

- **JSX 里直接读 Rust getter 不会更新**：`<Text>{session.status}</Text>` 读到的值不在响应式图里，永远不变。必须 `const status = signal(session.status)` + `session.onChange((s) => status.set(s))`。
- **两侧同时写同一个值**：TSX 直接 `set()` 而 Rust 也在改，必然漂移。只能调方法，让返回值或监听器更新信号。

## 4. 文件规模与拆分

- 新 TSX 文件目标 **300 行以内**，超过 400 行必须按职责拆分。
- 新 Rust 文件目标 **300 行以内**，超过 500 行原则上禁止合入（纯静态表、生成代码除外，需在文件顶部注明原因）。
- `App.tsx` 只做挂载、窗口壳与路由；页面逻辑进 `pages/`。
- 拆分后用 re-export 保持调用方路径稳定。

## 5. 零外部引用（硬约束）

- 不引用迁移前客户端的源码、路径或环境变量；只使用归档在 `docs/reference/gpui/` 的截图。
- 资源一律从 `assets/` 读，不得回退到 checkout 目录（不用 `CARGO_MANIFEST_DIR` 拼运行时路径）。
- `python3 scripts/check_no_legacy_client_refs.py` 必须在提交前通过。

## 6. 验证

```bash
nsh check nsh-app            # TSX 对着 .nsh/types 里的 Rust 声明做类型检查
nsh test nsh-app             # App.test.tsx：无头跑真实 UI
cargo test --manifest-path nsh-app/native/Cargo.toml   # 工作区全部成员
nsh run nsh-app --target gpui
```

`cargo test` 在 `native` 工作区里跑 `aalc-native` 与 `crates/*`；只改契约或 payload 时用
`cargo test -p aalc-model -p aalc-contract`，不编译 UI 栈。

页面改动的验收标准是与原客户端截图 1:1 对照（见 `MIGRATION.md` 的进度表），而不是「看起来差不多」。

截图（Linux 无 GPU 也可以，Xvfb + Mesa lavapipe）：

```bash
python3 scripts/capture_nsh.py     # → nsh-app/docs/shots/p0-desktop.png
```

## 7. 构建速度

改 Rust 后 `nsh run` 要重新编译并链接一个完整 host，慢的是链接与重复的依赖编译，不是应用
代码（`aalc-native` 本身几秒就能编完）。仓库已按这一点做了四件事，改动时不要退回去：

| 机制 | 位置 | 作用 |
| --- | --- | --- |
| 统一 target 目录 | 根 `.cargo/config.toml` 的 `build.target-dir` | native crate 与 host 共用 `nsh-app/target`，不再把 tokio/rustls/`nsh-core` 再编一遍；`nsh` 自己传 `--target-dir`，以它的为准 |
| `debuginfo=1` | 同上 `build.rustflags` | dev host 的调试信息从约 1.6 GB 降到几百 MB，链接时间随之下降；保留行号，没有变量值 |
| 链接器 | `scripts/build-env.sh` / `scripts/build-env.bat`（由 `run-nsh.sh` / `run-nsh*.bat` 调用） | Linux：装了 `mold` 就默认用它（实测重链 2m08s → 1m18s），没有则试 `ld.lld`；Windows：PATH 上有 `lld-link` 就用它。`AALC_LINKER=default` 回到默认，`AALC_LINKER=lld`/`<路径>` 换人；`AALC_DEBUGINFO=2` 临时恢复完整调试信息 |
| 并发上限 | 同上，`AALC_JOBS`（默认 4） | 八个 rustc 进程抢同一块磁盘是「编译时整机卡死」的主因；CI 不用这两个脚本，保持满并发 |

`native/scrcpy_decoder` 是例外：产物固定在自己的 `target/release`（`scripts/build.py` 显式传
`CARGO_TARGET_DIR`），因为 Python 侧按这个路径加载 DLL。

拆分边界同理：`crates/model`、`crates/contract` 只放纯 serde 数据（不依赖 `nsh-core`），
`src/` 里的 state/ipc 依赖 UI 栈。往 `crates/` 里加 `nsh-core` 依赖会让快编译路径失效。

### 7.1 机械硬盘 / 内存紧张的机器

上面的仓库内设置只解决一半：HDD 上真正让机器「卡死」的是写回风暴与换页。系统层建议（Ubuntu
用 `/etc/sysctl.d/90-aalc-build.conf`）：

```ini
vm.dirty_ratio = 10
vm.dirty_background_ratio = 5
vm.swappiness = 10
```

默认的 20% 脏页上限意味着一次大链接会把几 GB 脏页攒在一起，随后的回写会卡住所有进程；
`swappiness=60` 在只有 10 GB 内存的机器上会把编译工作集换到同一块盘上。

磁盘调度器改为 `mq-deadline`（HDD 上比 `none` 少饥饿）：

```bash
echo mq-deadline | sudo tee /sys/block/sda/queue/scheduler
```

日常开发用 `./run-nsh.sh`（`--mock` 对面是 `run-nsh-mock.bat`）：它会 source
`scripts/build-env.sh` 把并发降到 4、默认用 mold（机器上没有就回退），并以
`ionice -c 2 -n 6 nice -n 5` 运行构建，桌面不被编译抢死。`AALC_DRY_RUN=1 ./run-nsh.sh`
可以只看环境与命令而不真构建。

## 8. 参考：项目级 Agent Skills

仓库内置了 Native Shell 官方 Agent Skills（`.agents/skills/`，随仓库提交）：

- `nsh`：Rust/TSX/混合三模式、响应式、组件、样式、工具链与测试。**混合模式的边界规则先读 `references/mixed.md`。**
- `nsh-design-guides`：层级、色彩 token、间距、密度、交互状态、动效规范，以及改完 UI 必走的完成检查清单。

改 `nsh-app/` 的 Rust 或 TSX 代码前先读对应 skill；它们来自上游，不要手改（见 `.agents/skills/README.md`）。
