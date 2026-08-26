//! zigui: AhabAssistantLimbusCompany 主控台 UI 的 Native SDK 演示版。
//!
//! 架构与 `ui/`（Tauri + React + zustand）一一对应：
//!   - Model            ↔ appStore.ts（currentPage/themeMode/...）
//!   - Msg + update     ↔ 组件里的 setState / IPC 请求
//!   - zigui.native     ↔ HomePage.tsx 的 JSX
//!   - fx.startTimer    ↔ 前端 mock IPC 推送的 execution.status / log.entry
//!
//! 视图在 `zigui.native`；本文件是全部逻辑：Model、Msg、update。
//! 中文文案集中放在 Labels（通过 {l.xxx} 绑定进视图）——SDK 内置字体
//! 只覆盖拉丁字符，字面 CJK 文本会被 tofu guard 拒绝，且必须注册中文字体
//! 才能真正渲染（见 tokensFromModel / app_fonts）。

const std = @import("std");
const builtin = @import("builtin");
const runner = @import("runner");
const native_sdk = @import("native_sdk");

pub const panic = std.debug.FullPanic(native_sdk.debug.capturePanic);

const canvas = native_sdk.canvas;
const geometry = native_sdk.geometry;

const canvas_label = "ahab-canvas";
const window_width: f32 = 1180;
const window_height: f32 = 760;
/// 隐藏式标题栏下，头部行的自然高度（chrome 通道会以系统标题栏带高为下限）
pub const header_natural_height: f32 = 52;

/// 日志环形容量（对齐 ui 里 logs.slice(-300) 的有界窗口思路）
const logs_capacity: usize = 80;
const max_log_text: usize = 120;

/// 执行模拟定时器的 key（fx 的 key 空间）
const exec_timer_key: u64 = 77;
/// 每次 tick 推进的进度（约 8 秒跑完一个任务）
const progress_step: f32 = 0.05;

// ------------------------------------------------------------------ model

/// 固定任务卡片的 id（对齐 ui TasksConfig.enabledTasks）
pub const task_windows: u32 = 1;
pub const task_daily: u32 = 2;
pub const task_reward: u32 = 3;
pub const task_enkephalin: u32 = 4;
pub const task_mirror: u32 = 5;
pub const task_resonate: u32 = 6;

pub const LogLevel = enum { info, warn, err };

pub const LogEntry = struct {
    id: u32 = 0,
    level: LogLevel = .info,
    hh: u32 = 0,
    mm: u32 = 0,
    ss: u32 = 0,
    text_storage: [max_log_text]u8 = [_]u8{0} ** max_log_text,
    text_len: usize = 0,

    pub fn text(entry: *const LogEntry) []const u8 {
        return entry.text_storage[0..entry.text_len];
    }
    /// markup 里按级别上色用的谓词绑定
    pub fn isErr(entry: *const LogEntry) bool {
        return entry.level == .err;
    }
    pub fn isWarn(entry: *const LogEntry) bool {
        return entry.level == .warn;
    }
};

/// 任务卡片行：由 `cards()` 在每次视图构建时派生（derive, don't store）
pub const CardRow = struct {
    id: u32,
    title: []const u8,
    tag: []const u8,
    enabled: bool,
    can_disable: bool,
    expanded: bool,
    executing: bool,

    pub fn showProgress(row: *const CardRow) bool {
        return row.executing and row.id != task_resonate;
    }
};

/// 全部中文文案。字段即绑定路径：markup 里写 {l.xxx}。
pub const Labels = struct {
    app_title: []const u8 = "Ahab Assistant · Limbus Company",
    task_section_title: []const u8 = "任务配置",
    selected_fmt: []const u8 = "已选",
    idle_status: []const u8 = "空闲",
    running_status: []const u8 = "运行中",
    paused_status: []const u8 = "已暂停",

    // 卡片标题
    t_windows: []const u8 = "窗口设置",
    t_daily: []const u8 = "日常任务",
    t_reward: []const u8 = "领取奖励",
    t_enkephalin: []const u8 = "狂气换体",
    t_mirror: []const u8 = "坐牢设置 (镜牢)",
    t_resonate: []const u8 = "亚哈共鸣",

    // 窗口设置选项
    resolution: []const u8 = "分辨率",
    async_input: []const u8 = "异步输入 (PostMessage)",

    // 日常任务选项
    exp_count: []const u8 = "经验次数",
    thread_count: []const u8 = "线程次数",
    continuous: []const u8 = "连续战斗",

    // 领奖选项
    reward_mode: []const u8 = "领取模式",
    reward_all: []const u8 = "全部领取",
    reward_pass: []const u8 = "月券+邮件",
    reward_mail: []const u8 = "仅领取邮件",

    // 狂气换体选项
    swap_count: []const u8 = "兑换次数",
    grandet: []const u8 = "葛朗台模式",

    // 镜牢选项
    mirror_count: []const u8 = "镜牢次数",
    infinite: []const u8 = "无限坐牢",
    hard_mirror: []const u8 = "困难镜牢",

    // 共鸣说明
    resonate_hint: []const u8 = "开启后执行时随机播放亚哈语音（演示）",

    // 工具栏
    select_all: []const u8 = "全选",
    clear_all: []const u8 = "清空",
    start: []const u8 = "开始执行",
    pause_label: []const u8 = "暂停",
    resume_label: []const u8 = "继续",
    stop: []const u8 = "停止",
    start_shortcut: []const u8 = "F10",

    // 右侧面板
    connection: []const u8 = "设备连接",
    disconnected: []const u8 = "未连接",
    connect_btn: []const u8 = "连接模拟器",
    screenshot_title: []const u8 = "实时画面",
    screenshot_pending: []const u8 = "等待设备接入",
    logs_title: []const u8 = "运行日志",
    logs_clear: []const u8 = "清空",
    no_logs: []const u8 = "暂无日志",
};

pub const ExecState = enum { idle, running, paused };

pub const Msg = union(enum) {
    // 执行控制
    start,
    pause_task,
    resume_task,
    stop,
    /// 执行模拟定时器（fx.startTimer），不进视图
    tick: native_sdk.EffectTimer,
    // 批量操作
    select_all,
    clear_all,
    clear_logs,
    // 卡片勾选 / 展开（id 为 task_* 常量）
    card_toggle: u32,
    card_expand: u32,
    // 窗口设置
    size_720,
    size_1080,
    size_1600,
    toggle_postmsg,
    // 日常任务
    dec_exp,
    inc_exp,
    dec_thread,
    inc_thread,
    toggle_continuous,
    // 领奖模式
    set_reward_all,
    set_reward_pass,
    set_reward_mail,
    // 狂气换体
    dec_swap,
    inc_swap,
    toggle_grandet,
    // 镜牢
    dec_mirror_n,
    inc_mirror_n,
    toggle_infinite,
    toggle_hard,
    pub const view_unbound = .{"tick"};
};

pub const Model = struct {
    // ---- 任务开关（set_windows 常开，无开关）----
    daily_enabled: bool = true,
    reward_enabled: bool = true,
    enkephalin_enabled: bool = false,
    mirror_enabled: bool = true,
    resonate_enabled: bool = false,

    // ---- 展开状态 ----
    expand_windows: bool = false,
    expand_daily: bool = false,
    expand_reward: bool = false,
    expand_enkephalin: bool = false,
    expand_mirror: bool = false,

    // ---- 窗口设置 ----
    win_size: u32 = 2, // 0=720P 1=1080P 2=1600P（对齐默认配置）
    post_message: bool = true,

    // ---- 日常任务 ----
    exp_count: u32 = 3,
    thread_count: u32 = 3,
    continuous: bool = true,

    // ---- 领奖 ----
    reward_mode: u32 = 0, // 0=全部 1=月券+邮件 2=仅邮件

    // ---- 狂气换体 ----
    swap_count: u32 = 5,
    grandet: bool = false,

    // ---- 镜牢 ----
    mirror_count: u32 = 3,
    infinite_mirror: bool = false,
    hard_mirror: bool = true,

    // ---- 执行状态 ----
    exec: ExecState = .idle,
    current_task: u32 = 0, // task_* 常量；0 = 无
    progress: f32 = 0,
    milestones_done: u32 = 0,

    // ---- 日志 ----
    logs: [logs_capacity]LogEntry = [_]LogEntry{.{}} ** logs_capacity,
    log_count: usize = 0,
    next_log_id: u32 = 1,

    // ---- chrome 几何 ----
    /// 中文文案表
    l: Labels = .{},

    pub const view_unbound = .{
        // 这些字段只被 cards()/update 内部消费，markup 不直接绑定
        "daily_enabled",      "reward_enabled",     "enkephalin_enabled",
        "mirror_enabled",     "resonate_enabled",
        "expand_windows",     "expand_daily",       "expand_reward",
        "expand_enkephalin",  "expand_mirror",
        "exec",               "current_task",       "logs",
        "anyEnabled",         "currentTitle",
        "next_log_id",        "milestones_done",
    };

    // ------------------------------------------------ 派生（derived）

    pub fn busy(model: *const Model) bool {
        return model.exec != .idle;
    }
    pub fn isRunning(model: *const Model) bool {
        return model.exec == .running;
    }
    pub fn isPaused(model: *const Model) bool {
        return model.exec == .paused;
    }

    fn enabledAt(model: *const Model, id: u32) bool {
        return switch (id) {
            task_windows => true,
            task_daily => model.daily_enabled,
            task_reward => model.reward_enabled,
            task_enkephalin => model.enkephalin_enabled,
            task_mirror => model.mirror_enabled,
            task_resonate => model.resonate_enabled,
            else => false,
        };
    }

    pub fn anyEnabled(model: *const Model) bool {
        var id: u32 = task_windows;
        while (id <= task_mirror) : (id += 1) {
            if (model.enabledAt(id)) return true;
        }
        return false;
    }

    pub fn selectedCount(model: *const Model) usize {
        var count: usize = 0;
        var id: u32 = task_windows;
        while (id <= task_resonate) : (id += 1) {
            if (model.enabledAt(id)) count += 1;
        }
        return count;
    }

    pub fn canStart(model: *const Model) bool {
        return model.exec == .idle and model.anyEnabled();
    }

    pub fn currentTitle(model: *const Model) []const u8 {
        return switch (model.current_task) {
            task_windows => model.l.t_windows,
            task_daily => model.l.t_daily,
            task_reward => model.l.t_reward,
            task_enkephalin => model.l.t_enkephalin,
            task_mirror => model.l.t_mirror,
            else => "",
        };
    }

    /// 头部状态徽标文本（arena 标量派生绑定）
    pub fn statusText(model: *const Model, arena: std.mem.Allocator) []const u8 {
        return switch (model.exec) {
            .idle => std.fmt.allocPrint(arena, "{s} · {s} {d}", .{
                model.l.idle_status, model.l.selected_fmt, model.selectedCount(),
            }) catch model.l.idle_status,
            .running => std.fmt.allocPrint(arena, "{s} · {s}", .{
                model.l.running_status, model.currentTitle(),
            }) catch model.l.running_status,
            .paused => model.l.paused_status,
        };
    }

    /// 卡片预览 tag（对应 ui 里 PreviewTag 计算，这里直接格式化成一行）
    fn cardTag(model: *const Model, arena: std.mem.Allocator, id: u32) []const u8 {
        return switch (id) {
            task_windows => std.fmt.allocPrint(arena, "{d}P · PM {s}", .{
                @as(u32, switch (model.win_size) {
                    0 => 720,
                    1 => 1080,
                    else => 1600,
                }),
                if (model.post_message) "开" else "关",
            }) catch "",
            task_daily => std.fmt.allocPrint(arena, "EXP×{d} · 连续 {s}", .{
                model.exp_count, if (model.continuous) "开" else "关",
            }) catch "",
            task_reward => switch (model.reward_mode) {
                0 => "全部领取",
                1 => "月券+邮件",
                else => "仅邮件",
            },
            task_enkephalin => std.fmt.allocPrint(arena, "{d}次 · 葛朗台 {s}", .{
                model.swap_count, if (model.grandet) "开" else "关",
            }) catch "",
            task_mirror => std.fmt.allocPrint(arena, "{s} · {s}", .{
                if (model.infinite_mirror) "∞" else "次数",
                if (model.hard_mirror) "困难" else "普通",
            }) catch "",
            task_resonate => if (model.resonate_enabled) "已开启" else "未开启",
            else => "",
        };
    }

    /// 六张固定任务卡片的行数据（构建期拷入 arena，绝不存回模型）
    pub fn cards(model: *const Model, arena: std.mem.Allocator) []const CardRow {
        const out = arena.alloc(CardRow, 6) catch return &.{};
        const ids = [_]u32{ task_windows, task_daily, task_reward, task_enkephalin, task_mirror, task_resonate };
        for (ids, 0..) |id, i| {
            out[i] = .{
                .id = id,
                .title = switch (id) {
                    task_windows => model.l.t_windows,
                    task_daily => model.l.t_daily,
                    task_reward => model.l.t_reward,
                    task_enkephalin => model.l.t_enkephalin,
                    task_mirror => model.l.t_mirror,
                    else => model.l.t_resonate,
                },
                .tag = model.cardTag(arena, id),
                .enabled = model.enabledAt(id),
                .can_disable = id != task_windows,
                .expanded = switch (id) {
                    task_windows => model.expand_windows,
                    task_daily => model.expand_daily,
                    task_reward => model.expand_reward,
                    task_enkephalin => model.expand_enkephalin,
                    task_mirror => model.expand_mirror,
                    else => false,
                },
                .executing = model.current_task == id and model.exec != .idle,
            };
        }
        return out;
    }

    /// 日志窗口（最旧的在前）。空切片时 markup 走 <for> 的 <else> 空状态。
    pub fn logRows(model: *const Model) []const LogEntry {
        return model.logs[0..model.log_count];
    }

    // ------------------------------------------------ 操作

    pub fn addLog(model: *Model, level: LogLevel, comptime fmt: []const u8, args: anytype) void {
        var buf: [max_log_text]u8 = undefined;
        const text = std.fmt.bufPrint(&buf, fmt, args) catch return;
        model.pushLog(level, text);
    }

    fn pushLog(model: *Model, level: LogLevel, text: []const u8) void {
        if (model.log_count >= logs_capacity) {
            // 丢最旧的一条，保持插入顺序（新日志追加在尾部）
            std.mem.copyForwards(LogEntry, model.logs[0 .. logs_capacity - 1], model.logs[1..logs_capacity]);
            model.log_count = logs_capacity - 1;
        }
        const entry = &model.logs[model.log_count];
        const wall_ms = native_sdk.nowMs();
        const day_secs: u64 = @intCast(@mod(@divTrunc(wall_ms, 1000), 24 * 3600));
        entry.* = .{
            .id = model.next_log_id,
            .level = level,
            .hh = @intCast(day_secs / 3600),
            .mm = @intCast(@mod(day_secs / 60, 60)),
            .ss = @intCast(day_secs % 60),
        };
        const len = @min(text.len, max_log_text);
        @memcpy(entry.text_storage[0..len], text[0..len]);
        entry.text_len = len;
        model.log_count += 1;
        model.next_log_id += 1;
    }

    /// 下一个启用的任务 id；没有则返回 0
    fn nextEnabledAfter(model: *const Model, id: u32) u32 {
        var next = id + 1;
        while (next <= task_mirror) : (next += 1) {
            if (model.enabledAt(next)) return next;
        }
        return 0;
    }

    fn firstEnabled(model: *const Model) u32 {
        return model.nextEnabledAfter(0);
    }

    /// 执行推进（tick）：假进度 + 阶段日志，跑完当前任务后接下一个
    fn advanceExecution(model: *Model, fx: *Effects) void {
        model.progress += progress_step;
        const quarter: u32 = @intFromFloat(model.progress * 4.0);
        if (quarter > model.milestones_done and quarter < 4) {
            model.milestones_done = quarter;
            model.addLog(.info, "[{s}] 进度 {d}%", .{ model.currentTitle(), quarter * 25 });
        }
        if (model.progress >= 1.0) {
            model.addLog(.info, "{s} 完成 ✓", .{model.currentTitle()});
            const next = model.nextEnabledAfter(model.current_task);
            if (next == 0) {
                model.finish(fx);
            } else {
                model.current_task = next;
                model.progress = 0;
                model.milestones_done = 0;
                model.addLog(.info, "开始 {s}", .{model.currentTitle()});
            }
        }
    }

    fn finish(model: *Model, fx: *Effects) void {
        fx.cancelTimer(exec_timer_key);
        model.exec = .idle;
        model.current_task = 0;
        model.progress = 0;
        model.milestones_done = 0;
        model.addLog(.info, "全部任务执行完成", .{});
    }
};

// ---------------------------------------------------------------- update

pub fn update(model: *Model, msg: Msg, fx: *Effects) void {
    switch (msg) {
        .start => {
            if (model.busy()) return;
            if (!model.anyEnabled()) {
                model.addLog(.warn, "请至少勾选一个任务再开始", .{});
                return;
            }
            model.exec = .running;
            model.current_task = model.firstEnabled();
            model.progress = 0;
            model.milestones_done = 0;
            model.addLog(.info, "开始执行任务流程（共模拟）", .{});
            fx.startTimer(.{
                .key = exec_timer_key,
                .interval_ms = 400,
                .mode = .repeating,
                .on_fire = Effects.timerMsg(.tick),
            });
        },
        .pause_task => {
            if (model.exec != .running) return;
            model.exec = .paused;
            fx.cancelTimer(exec_timer_key);
            model.addLog(.warn, "执行已暂停", .{});
        },
        .resume_task => {
            if (model.exec != .paused) return;
            model.exec = .running;
            fx.startTimer(.{
                .key = exec_timer_key,
                .interval_ms = 400,
                .mode = .repeating,
                .on_fire = Effects.timerMsg(.tick),
            });
            model.addLog(.info, "继续执行", .{});
        },
        .stop => {
            if (model.exec == .idle) return;
            fx.cancelTimer(exec_timer_key);
            model.exec = .idle;
            model.current_task = 0;
            model.progress = 0;
            model.milestones_done = 0;
            model.addLog(.err, "执行已被手动停止", .{});
        },
        .tick => |timer| switch (timer.outcome) {
            .fired => {
                if (model.exec == .running) model.advanceExecution(fx);
            },
            else => {},
        },

        .select_all => {
            model.daily_enabled = true;
            model.reward_enabled = true;
            model.enkephalin_enabled = true;
            model.mirror_enabled = true;
            model.resonate_enabled = true;
        },
        .clear_all => {
            model.daily_enabled = false;
            model.reward_enabled = false;
            model.enkephalin_enabled = false;
            model.mirror_enabled = false;
            model.resonate_enabled = false;
        },
        .clear_logs => {
            model.log_count = 0;
        },

        .card_toggle => |id| switch (id) {
            task_daily => model.daily_enabled = !model.daily_enabled,
            task_reward => model.reward_enabled = !model.reward_enabled,
            task_enkephalin => model.enkephalin_enabled = !model.enkephalin_enabled,
            task_mirror => model.mirror_enabled = !model.mirror_enabled,
            task_resonate => model.resonate_enabled = !model.resonate_enabled,
            else => {}, // 窗口设置常开
        },
        .card_expand => |id| switch (id) {
            task_windows => model.expand_windows = !model.expand_windows,
            task_daily => model.expand_daily = !model.expand_daily,
            task_reward => model.expand_reward = !model.expand_reward,
            task_enkephalin => model.expand_enkephalin = !model.expand_enkephalin,
            task_mirror => model.expand_mirror = !model.expand_mirror,
            else => {},
        },

        .size_720 => model.win_size = 0,
        .size_1080 => model.win_size = 1,
        .size_1600 => model.win_size = 2,
        .toggle_postmsg => model.post_message = !model.post_message,

        .dec_exp => model.exp_count = saturatingDec(model.exp_count),
        .inc_exp => model.exp_count = @min(model.exp_count + 1, 99),
        .dec_thread => model.thread_count = saturatingDec(model.thread_count),
        .inc_thread => model.thread_count = @min(model.thread_count + 1, 99),
        .toggle_continuous => model.continuous = !model.continuous,

        .set_reward_all => model.reward_mode = 0,
        .set_reward_pass => model.reward_mode = 1,
        .set_reward_mail => model.reward_mode = 2,

        .dec_swap => model.swap_count = saturatingDec(model.swap_count),
        .inc_swap => model.swap_count = @min(model.swap_count + 1, 20),
        .toggle_grandet => model.grandet = !model.grandet,

        .dec_mirror_n => model.mirror_count = saturatingDec(model.mirror_count),
        .inc_mirror_n => model.mirror_count = @min(model.mirror_count + 1, 99),
        .toggle_infinite => model.infinite_mirror = !model.infinite_mirror,
        .toggle_hard => model.hard_mirror = !model.hard_mirror,

    }
}

fn saturatingDec(v: u32) u32 {
    return if (v > 0) v - 1 else 0;
}

/// chrome 通道：隐藏式标题栏的覆盖几何（Windows 上是回收的 caption 带）

// ------------------------------------------------------------------ view

pub const markup_src = @embedFile("zigui.native");

/// release：comptime 编译整个视图，二进制里不带 markup 解析器；
/// debug：额外保留运行时引擎并监视 src/zigui.native 热重载。
pub const CompiledView = canvas.CompiledMarkupView(Model, Msg, markup_src);

// ------------------------------------------------------------------- app

const dev_markup_reload_app = builtin.mode == .Debug;
const ZApp = native_sdk.UiAppWithFeatures(Model, Msg, .{ .runtime_markup = dev_markup_reload_app });
const Effects = ZApp.Effects;

/// 注册中文字体（SimHei，TrueType/glyf）。内置 Geist 只有拉丁字符，
/// 不注册的话所有中文都是 notdef 豆腐块。
const cjk_font_id: canvas.FontId = canvas.min_registered_font_id;
const app_fonts = [_]ZApp.FontRegistration{.{
    .id = cjk_font_id,
    .name = "simhei.ttf",
    .ttf = @embedFile("fonts/simhei.ttf"),
}};

/// 深色 house 主题 + 把正文字体指向注册的中文字面
fn tokensFromModel(model: *const Model) canvas.DesignTokens {
    _ = model;
    var tokens = canvas.DesignTokens.theme(.{ .color_scheme = .dark });
    tokens.typography.font_id = cjk_font_id;
    return tokens;
}

const app_permissions = [_][]const u8{ native_sdk.security.permission_command, native_sdk.security.permission_view };

const shell_views = [_]native_sdk.ShellView{
    .{ .label = canvas_label, .kind = .gpu_surface, .fill = true, .role = "Ahab main canvas", .accessibility_label = "Ahab 主控台画布", .gpu_backend = .metal, .gpu_pixel_format = .bgra8_unorm, .gpu_present_mode = .timer, .gpu_alpha_mode = .@"opaque", .gpu_color_space = .srgb, .gpu_vsync = true },
};
const shell_windows = [_]native_sdk.ShellWindow{.{
    .label = "main",
    .title = "Ahab Assistant · Limbus Company",
    .width = window_width,
    .height = window_height,
    .min_width = 980,
    .min_height = 620,
    .views = &shell_views,
}};
pub const shell_scene: native_sdk.ShellConfig = .{ .windows = &shell_windows };

pub fn initialModel() Model {
    var model = Model{};
    model.addLog(.info, "UI 就绪（Native SDK 原生渲染演示版）", .{});
    model.addLog(.info, "数据为内存 Mock，尚未对接 Python 后端 IPC", .{});
    return model;
}

pub fn main(init: std.process.Init) !void {
    // create：堆上分配 app 结构并就地构造 Model——两者都不走栈
    const app_state = try ZApp.create(std.heap.page_allocator, .{
        .name = "zigui",
        .scene = shell_scene,
        .canvas_label = canvas_label,
        .update_fx = update,
        .tokens_fn = tokensFromModel,
        .fonts = &app_fonts,
        .view = CompiledView.build,
        .markup = if (dev_markup_reload_app)
            .{ .source = markup_src, .watch_path = "src/zigui.native", .io = init.io }
        else
            null,
    });
    defer app_state.destroy();
    app_state.model = initialModel();

    try runner.runWithOptions(app_state.app(), .{
        .app_name = "zigui",
        .window_title = "Ahab Assistant · Limbus Company",
        .bundle_id = "com.ahab.zigui",
        .default_frame = geometry.RectF.init(0, 0, window_width, window_height),
        .js_window_api = false,
        .security = .{
            .permissions = &app_permissions,
            .navigation = .{ .allowed_origins = &.{ "zero://inline", "zero://app" } },
        },
    }, init);
}
