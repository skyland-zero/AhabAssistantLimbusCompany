using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using System.Threading.Tasks;
using AhabAssistant.Avalonia.Models;
using Avalonia.Threading;

namespace AhabAssistant.Avalonia.Services;

/// <summary>
/// 内存版 Mock 后端（对齐 ui/src/services/ipc/mock/server.ts）。
/// 提供与未来真实 Python sidecar 一致的请求/事件语义。
/// </summary>
public sealed class MockBackend
{
    public static MockBackend Instance { get; } = new();

    /* ------------------------------ 假数据 ------------------------------ */

    public List<SinnerInfo> Sinners { get; } = new()
    {
        new() { Id = "yi_sang", Name = "李箱" },
        new() { Id = "faust", Name = "浮士德" },
        new() { Id = "don_quixote", Name = "堂吉诃德" },
        new() { Id = "ryoshu", Name = "良秀" },
        new() { Id = "meursault", Name = "默尔索" },
        new() { Id = "hong_lu", Name = "鸿璐" },
        new() { Id = "heathcliff", Name = "希斯克利夫" },
        new() { Id = "ishmael", Name = "以实玛利" },
        new() { Id = "rodion", Name = "罗佳" },
        new() { Id = "sinclair", Name = "辛克莱" },
        new() { Id = "outis", Name = "奥提斯" },
        new() { Id = "gregor", Name = "格雷戈尔" },
    };

    public List<TeamDetail> Teams { get; private set; } = new();
    public TasksConfig TasksConfig { get; set; } = DefaultTasksConfig();
    public HotkeyConfig Hotkey { get; set; } = new() { StartStop = "F10", PauseResume = "F11", Enabled = true };
    public SystemSettingsConfig SystemSettings { get; set; } = new();
    public ThemePackState ThemePackState { get; set; } = DefaultThemePacks();
    public List<ResourceGroup> Resources { get; set; } = new();
    public List<DeviceInfo> Devices { get; } = new()
    {
        new() { Id = "win-limbus", Name = "Limbus Company", Detail = "1920×1080 · 窗口化" },
        new() { Id = "mumu-instance", Name = "MuMu 模拟器", Detail = "1280×720 · 端口 16384" },
    };

    private string _executionState = "idle";
    private string? _currentRunningTask;

    public string ExecutionState => _executionState;
    public string? CurrentTaskId => _currentRunningTask;

    private readonly Dictionary<string, bool> _toolRunning = new();
    private DispatcherTimer? _execTimer;
    private readonly Queue<(string Name, string Id, int Duration)> _pendingSteps = new();

    private string _deviceId = "win-limbus";
    private string _deviceStatus = "connected";

    /* ------------------------------ 事件 ------------------------------ */

    public event Action<ExecutionStatusPayload>? ExecutionStatus;
    public event Action<MirrorProgressPayload>? MirrorProgress;
    public event Action<LogEntryPayload>? LogEntry;
    public event Action<ToolStatusPayload>? ToolStatus;
    public event Action<string, string>? DeviceStatus; // deviceId, status
    public event Action<SyncProgressPayload>? SyncProgress;

    private static long NowMs => DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();

    public void Log(string level, string message) =>
        LogEntry?.Invoke(new LogEntryPayload { Ts = NowMs, Level = level, Message = message });

    private void SetExecutionState(string state, string? taskId)
    {
        _executionState = state;
        _currentRunningTask = taskId;
        ExecutionStatus?.Invoke(new ExecutionStatusPayload { State = state, CurrentTaskId = taskId });
    }

    /* ------------------------------ 持久化 ------------------------------ */

    private static string SettingsDir
    {
        get
        {
            var dir = Path.Combine(AppContext.BaseDirectory, "..");
            return Path.GetFullPath(dir);
        }
    }

    private static string DataFile(string name) => Path.Combine(SettingsDir, name);

    public void LoadPersisted()
    {
        TasksConfig = TryLoad<TasksConfig>("tasks-config.json") ?? DefaultTasksConfig();
        SystemSettings = TryLoad<SystemSettingsConfig>("system-settings.json") ?? new SystemSettingsConfig();
        Hotkey = TryLoad<HotkeyConfig>("hotkey.json") ?? new HotkeyConfig();

        var savedTeams = TryLoad<List<TeamDetail>>("teams.json");
        Teams = savedTeams ?? DefaultTeams();

        ThemePackState = TryLoad<ThemePackState>("theme-packs.json") ?? DefaultThemePacks();
        Resources = new List<ResourceGroup>
        {
            new() { Id = "templates", Name = "模板资源", LocalVersion = "v2025.06.1", RemoteVersion = null, LastSyncAt = NowMs - 86_400_000 },
            new() { Id = "models", Name = "ONNX 模型", LocalVersion = "v1.2.0", RemoteVersion = null, LastSyncAt = null },
        };
    }

    private static T? TryLoad<T>(string file) where T : class
    {
        try
        {
            if (!File.Exists(DataFile(file))) return default;
            var json = File.ReadAllText(DataFile(file));
            var typeInfo = GetTypeInfo(typeof(T));
            return typeInfo == null ? default : JsonSerializer.Deserialize(json, typeInfo) as T;
        }
        catch { return default; }
    }

    public void SaveTasksConfig() => Save("tasks-config.json", TasksConfig);
    public void SaveTeams() => Save("teams.json", Teams);
    public void SaveThemePacks() => Save("theme-packs.json", ThemePackState);
    public void SaveSystemSettings() => Save("system-settings.json", SystemSettings);
    public void SaveHotkey()
    {
        Save("hotkey.json", Hotkey);
        HotkeyUpdated?.Invoke(Hotkey);
    }

    public event Action<Models.HotkeyConfig>? HotkeyUpdated;

    private static JsonTypeInfo? GetTypeInfo(Type type)
    {
        if (type == typeof(TasksConfig)) return Models.AalcJsonContext.Default.TasksConfig;
        if (type == typeof(SystemSettingsConfig)) return Models.AalcJsonContext.Default.SystemSettingsConfig;
        if (type == typeof(HotkeyConfig)) return Models.AalcJsonContext.Default.HotkeyConfig;
        if (type == typeof(List<TeamDetail>)) return Models.AalcJsonContext.Default.ListTeamDetail;
        if (type == typeof(ThemePackState)) return Models.AalcJsonContext.Default.ThemePackState;
        return null;
    }

    private static void Save<T>(string file, T data)
    {
        try
        {
            var typeInfo = GetTypeInfo(typeof(T));
            if (typeInfo != null)
                File.WriteAllText(DataFile(file), JsonSerializer.Serialize(data, typeInfo));
        }
        catch { /* 忽略持久化失败 */ }
    }

    /* ------------------------------ 默认数据 ------------------------------ */

    public static TasksConfig DefaultTasksConfig() => new();

    private static List<TeamDetail> DefaultTeams() => new()
    {
        new TeamDetail
        {
            Id = "team-1", Name = "编队 1 (震颤)", Purpose = "mirror",
            Sinners = new() { "faust", "ishmael", "ryoshu", "hong_lu" },
            AccessoryScheme = "tremor", Enabled = true,
            MirrorConfig = WithMirror(m => { m.TeamSystem = 2; m.DiscardSystems["sinking"] = true; m.DiscardSystems["poise"] = true; m.OpeningBonus = new() { 2, 2, 1, 1, 0, 0, 0, 0, 0, 0 }; }),
        },
        new TeamDetail
        {
            Id = "team-2", Name = "编队 2 (烧伤)", Purpose = "luxcavation",
            Sinners = new() { "heathcliff", "rodion", "gregor" },
            AccessoryScheme = "burn", Enabled = true,
            MirrorConfig = WithMirror(_ => { }),
        },
        new TeamDetail
        {
            Id = "team-3", Name = "编队 3 (呼吸)", Purpose = "general",
            Sinners = new() { "yi_sang", "don_quixote", "meursault", "sinclair", "outis" },
            AccessoryScheme = "poise", Enabled = false,
            MirrorConfig = WithMirror(m => m.TeamSystem = 5),
        },
    };

    private static TeamMirrorConfig WithMirror(Action<TeamMirrorConfig> patch)
    {
        var cfg = TeamMirrorConfig.CreateDefault();
        patch(cfg);
        return cfg;
    }

    private static ThemePackState DefaultThemePacks() => new()
    {
        HardMirrorActive = true,
        Packs = new List<ThemePack>
        {
            new() { Id = "pk-1", Name = "黑云会", Weight = 5, Enabled = true, Tier = "T1" },
            new() { Id = "pk-2", Name = "拇指", Weight = 3, Enabled = true, Tier = "T2" },
            new() { Id = "pk-3", Name = "利刃兄弟会", Weight = 4, Enabled = true, Tier = "T2" },
            new() { Id = "pk-4", Name = "厄伍商会", Weight = 2, Enabled = false, Tier = "T3" },
            new() { Id = "pk-5", Name = "二十区福利机构", Weight = 6, Enabled = true, Tier = "T1" },
            new() { Id = "pk-6", Name = "技术科学解放者联盟", Weight = 1, Enabled = false, Tier = "T4" },
            new() { Id = "pk-7", Name = "公司总部", Weight = 3, Enabled = true, Tier = "T3" },
            new() { Id = "pk-8", Name = "残响乐团", Weight = 7, Enabled = true, Tier = "T1" },
        },
    };

    /* ------------------------------ 执行模拟 ------------------------------ */

    public bool CanStart(out string warn)
    {
        warn = "";
        var e = TasksConfig.EnabledTasks;
        if (!e.DailyTask && !e.GetReward && !e.BuyEnkephalin && !e.Mirror)
        {
            warn = "请至少勾选一个要执行的任务！";
            return false;
        }
        return true;
    }

    public void StartExecution()
    {
        StopExecutionTimer();
        SetExecutionState("running", null);
        Log("info", "Link Start! 开始执行所有已勾选任务");

        _pendingSteps.Clear();
        var e = TasksConfig.EnabledTasks;
        if (e.DailyTask) _pendingSteps.Enqueue(("日常任务 (经验本 & 纽本)", "daily_task", 2500));
        if (e.GetReward) _pendingSteps.Enqueue(("领取奖励", "get_reward", 1500));
        if (e.BuyEnkephalin) _pendingSteps.Enqueue(("狂气换体", "buy_enkephalin", 1500));
        if (e.Mirror) _pendingSteps.Enqueue(("坐牢任务 (镜牢探索)", "mirror", 4000));

        if (_pendingSteps.Count == 0)
        {
            Log("warn", "未勾选任何执行任务，流程结束");
            StopExecution();
            return;
        }

        RunNextStep();
    }

    private void RunNextStep()
    {
        if (_executionState != "running") return;
        if (_pendingSteps.Count == 0)
        {
            Log("info", "所有任务已完成！");
            var after = TasksConfig.AfterCompletion;
            if (after.Actions.Count > 0 || after.PowerAction != "none")
                Log("info", $"执行收尾动作：{string.Join(", ", after.Actions)} / {after.PowerAction}");
            StopExecution();
            return;
        }

        var cur = _pendingSteps.Peek();
        SetExecutionState("running", cur.Id);
        Log("info", $">> 开始执行：{cur.Name}");

        if (cur.Id == "mirror")
        {
            var total = TasksConfig.Mirror.InfiniteDungeons ? 9999 : TasksConfig.Mirror.SetMirrorCount;
            MirrorProgress?.Invoke(new MirrorProgressPayload
            {
                Current = 1, Total = total,
                IsHard = TasksConfig.Mirror.HardMirror,
                IsInfinite = TasksConfig.Mirror.InfiniteDungeons,
            });
        }

        _execTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(cur.Duration) };
        _execTimer.Tick += (_, _) =>
        {
            _execTimer?.Stop();
            _execTimer = null;
            if (_executionState != "running") return;
            Log("info", $"完成：{cur.Name}");
            _pendingSteps.Dequeue();
            RunNextStep();
        };
        _execTimer.Start();
    }

    public void PauseExecution()
    {
        if (_executionState != "running") return;
        _execTimer?.Stop();
        _execTimer = null;
        SetExecutionState("paused", _currentRunningTask);
        Log("info", "任务执行已暂停");
    }

    public void ResumeExecution()
    {
        if (_executionState != "paused") return;
        SetExecutionState("running", _currentRunningTask);
        Log("info", "任务执行已恢复");

        // 若当前步骤是 mirror，恢复时重新发出进度事件；然后继续当前步骤计时
        var remaining = _pendingSteps.Peek();
        if (remaining.Id == "mirror")
        {
            var total = TasksConfig.Mirror.InfiniteDungeons ? 9999 : TasksConfig.Mirror.SetMirrorCount;
            MirrorProgress?.Invoke(new MirrorProgressPayload
            {
                Current = 1, Total = total,
                IsHard = TasksConfig.Mirror.HardMirror,
                IsInfinite = TasksConfig.Mirror.InfiniteDungeons,
            });
        }
        RunCurrentAgain(remaining.Duration / 2);
    }

    private void RunCurrentAgain(int duration)
    {
        var cur = _pendingSteps.Peek();
        _execTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(Math.Max(300, duration)) };
        _execTimer.Tick += (_, _) =>
        {
            _execTimer?.Stop();
            _execTimer = null;
            if (_executionState != "running") return;
            Log("info", $"完成：{cur.Name}");
            _pendingSteps.Dequeue();
            RunNextStep();
        };
        _execTimer.Start();
    }

    public void StopExecution()
    {
        StopExecutionTimer();
        var wasBusy = _executionState != "idle";
        SetExecutionState("idle", null);
        if (wasBusy) { /* 日志由调用方补充 */ }
    }

    public void StopExecutionByUser()
    {
        StopExecutionTimer();
        SetExecutionState("idle", null);
        Log("warn", "用户手动停止了任务执行");
    }

    private void StopExecutionTimer()
    {
        _execTimer?.Stop();
        _execTimer = null;
    }

    /* ------------------------------ 工具箱 ------------------------------ */

    public void ToolStart(string toolId)
    {
        _toolRunning[toolId] = true;
        ToolStatus?.Invoke(new ToolStatusPayload { ToolId = toolId, Running = true });
        Log("info", $"[mock] 工具启动：{toolId}");
    }

    public void ToolStop(string toolId)
    {
        _toolRunning[toolId] = false;
        ToolStatus?.Invoke(new ToolStatusPayload { ToolId = toolId, Running = false });
        Log("info", $"[mock] 工具停止：{toolId}");
    }

    public bool IsToolRunning(string toolId) => _toolRunning.TryGetValue(toolId, out var r) && r;

    public void TakeScreenshot() => Log("info", "[mock] 截图完成，已保存至 AALC/screenshots 目录");

    /* ------------------------------ 资源同步模拟 ------------------------------ */

    private DispatcherTimer? _syncTimer;

    public void StartResourceSync(string scope)
    {
        Log("info", $"[mock] 开始同步资源：{scope}");
        int progress = 0;
        _syncTimer?.Stop();
        _syncTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(300) };
        _syncTimer.Tick += (_, _) =>
        {
            progress += 25;
            SyncProgress?.Invoke(new SyncProgressPayload { Scope = scope, Progress = progress });
            if (progress >= 100)
            {
                _syncTimer.Stop();
                _syncTimer = null;
                foreach (var r in Resources.Where(r => scope == "all" || r.Id == scope))
                {
                    r.LocalVersion = "v2025.06.2";
                    r.LastSyncAt = NowMs;
                }
                Log("info", $"[mock] 资源同步完成：{scope}");
            }
        };
        _syncTimer.Start();
    }

    public void CheckResourceUpdate()
    {
        Resources[0].RemoteVersion = "v2025.06.2";
        Resources[1].RemoteVersion = "v1.3.0";
        Log("info", "[mock] 已检查远端资源版本");
    }

    /* ------------------------------ 设备连接 ------------------------------ */

    public void ConnectDevice(string id)
    {
        _deviceId = id;
        _deviceStatus = "connected";
        DeviceStatus?.Invoke(id, "connected");
        Log("info", $"已连接设备：{id}");
    }

    public void DisconnectDevice()
    {
        _deviceId = "";
        _deviceStatus = "disconnected";
        DeviceStatus?.Invoke("", "disconnected");
        Log("info", "设备已断开连接");
    }

    public string DeviceStatusNow => _deviceStatus;
}
