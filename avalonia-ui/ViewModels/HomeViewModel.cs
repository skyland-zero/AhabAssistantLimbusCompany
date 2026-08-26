using System;
using System.Collections.ObjectModel;
using System.Linq;
using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Services;
using Avalonia.Controls;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace AhabAssistant.Avalonia.ViewModels;

public record PreviewTag(string Label, string Value, bool Highlight);

public partial class HomeViewModel : ObservableObject
{
    public MockBackend Backend => MockBackend.Instance;
    public TasksConfig Config => Backend.TasksConfig;

    /* 折叠状态 */
    [ObservableProperty] private bool _setWindowsExpanded;
    [ObservableProperty] private bool _dailyExpanded;
    [ObservableProperty] private bool _rewardExpanded;
    [ObservableProperty] private bool _enkephalinExpanded;
    [ObservableProperty] private bool _mirrorExpanded;

    /* 执行状态 */
    [ObservableProperty] private string _executionState = "idle";
    [ObservableProperty] private string? _currentTaskId;

    public bool IsBusy => ExecutionState != "idle";

    public string StatusBadgeText => ExecutionState switch
    {
        "running" when CurrentTaskId != null => $"{Localization.T("正在执行任务：")}{TaskTitle(CurrentTaskId)}",
        "running" => Localization.T("正在执行任务…"),
        "paused" => Localization.T("执行已暂停"),
        _ => Localization.T("待机中"),
    };

    public bool IsRunningDot => ExecutionState == "running" || ExecutionState == "paused";

    [ObservableProperty] private MirrorProgressPayload? _mirrorProgress;

    /* 右侧面板 */
    [ObservableProperty] private double _rightPanelWidth = App.RightPanelWidth;
    [ObservableProperty] private bool _rightPanelCollapsed = App.RightPanelCollapsed;

    partial void OnRightPanelWidthChanged(double value)
    {
        App.RightPanelWidth = value;
        App.SavePrefs();
        OnPropertyChanged(nameof(RightPanelActualWidth));
    }

    partial void OnRightPanelCollapsedChanged(bool value)
    {
        App.RightPanelCollapsed = value;
        App.SavePrefs();
        OnPropertyChanged(nameof(RightPanelActualWidth));
    }

    public GridLength RightPanelActualWidth => RightPanelCollapsed ? new GridLength(0) : new GridLength(Math.Max(240, RightPanelWidth));

    /* 结束后操作摘要 */
    [ObservableProperty] private string _afterSummary = "";

    /* 预览标签 */
    [ObservableProperty] private List<PreviewTag> _windowTags = new();
    [ObservableProperty] private List<PreviewTag> _dailyTags = new();
    [ObservableProperty] private List<PreviewTag> _rewardTags = new();
    [ObservableProperty] private List<PreviewTag> _enkephalinTags = new();
    [ObservableProperty] private List<PreviewTag> _mirrorTags = new();

    public ObservableCollection<LogEntryPayload> Logs { get; } = new();
    public ObservableCollection<TeamDetail> Teams { get; } = new();
    public ObservableCollection<string> TeamNames { get; } = new();

    public event Action? RequestLogAutoScroll;

    public HomeViewModel()
    {
        foreach (var t in Backend.Teams) Teams.Add(t);
        if (Teams.Count == 0) TeamNames.Add(Localization.T("编队 1"));
        else foreach (var t in Teams) TeamNames.Add(t.Name);
        ExecutionState = Backend.ExecutionState;
        CurrentTaskId = Backend.CurrentTaskId;
        RefreshAll();

        Backend.ExecutionStatus += OnExecStatus;
        Backend.DeviceStatus += (id, status) =>
        {
            ConnectionStatus = status;
            SelectedDevice = Devices.FirstOrDefault(d => d.Id == id);
        };
        ConnectionStatus = Backend.DeviceStatusNow;
        SelectedDevice = Devices.FirstOrDefault();
        Backend.MirrorProgress += p =>
        {
            MirrorProgress = p;
            OnPropertyChanged(nameof(MirrorProgress));
        };
        Backend.LogEntry += entry =>
        {
            Logs.Add(entry);
            while (Logs.Count > 300) Logs.RemoveAt(0);
            RequestLogAutoScroll?.Invoke();
        };
    }

    private void OnExecStatus(ExecutionStatusPayload payload)
    {
        ExecutionState = payload.State;
        CurrentTaskId = payload.CurrentTaskId;
        if (payload.State == "idle") { MirrorProgress = null; OnPropertyChanged(nameof(MirrorProgress)); }
        OnPropertyChanged(nameof(IsBusy));
        OnPropertyChanged(nameof(StatusBadgeText));
        OnPropertyChanged(nameof(IsRunningDot));
        PauseResumeText = payload.State == "paused" ? Localization.T("继续") : Localization.T("暂停");
    }

    [ObservableProperty] private string _pauseResumeText = Localization.T("暂停");

    public static string TaskTitle(string id) => Localization.T(id switch
    {
        "set_windows" => "窗口设置",
        "daily_task" => "日常任务",
        "get_reward" => "领取奖励",
        "buy_enkephalin" => "狂气换体",
        "mirror" => "坐牢设置 (镜牢)",
        "resonate_with_Ahab" => "亚哈共鸣",
        _ => id,
    });

    /// <summary>配置发生任意变更后调用：持久化并刷新预览。</summary>
    public void NotifyConfigChanged()
    {
        Backend.SaveTasksConfig();
        RefreshAll();
    }

    public void RefreshAll()
    {
        var c = Config;
        WindowTags = new List<PreviewTag>
        {
            new(Localization.T("分辨率"), $"{(int)c.SetWindows.SetWinSize}P", false),
            new(Localization.T("异步输入"), c.SetWindows.UsePostMessage ? Localization.T("开") : Localization.T("关"), c.SetWindows.UsePostMessage),
        };
        DailyTags = new List<PreviewTag>
        {
            new(Localization.T("经验本标签"), $"×{c.DailyTask.SetExpCount}", false),
            new(Localization.T("纽本"), $"×{c.DailyTask.SetThreadCount}", false),
            new(Localization.T("连战"), c.DailyTask.UseContinuousCombat ? $"×{c.DailyTask.UseContinuousCombatSelect}" : Localization.T("关"), c.DailyTask.UseContinuousCombat),
        };
        RewardTags = new List<PreviewTag>
        {
            new(Localization.T("模式"), Localization.T(c.GetReward.SetGetPrize switch { 1 => "狂气/通行证", 2 => "邮件", _ => "全部" }), false),
        };
        EnkephalinTags = new List<PreviewTag>
        {
            new(Localization.T("换体"), $"{c.BuyEnkephalin.SetLunacyToEnkephalin}{Localization.T("次")}", false),
            new(Localization.T("葛朗台"), c.BuyEnkephalin.DrGrandetMode ? Localization.T("开") : Localization.T("关"), c.BuyEnkephalin.DrGrandetMode),
        };
        MirrorTags = new List<PreviewTag>
        {
            new(Localization.T("坐牢"), c.Mirror.InfiniteDungeons ? "∞" : $"{c.Mirror.SetMirrorCount}{Localization.T("次")}", c.Mirror.InfiniteDungeons),
            new(Localization.T("难度"), c.Mirror.HardMirror ? Localization.T("困难") : Localization.T("普通"), c.Mirror.HardMirror),
        };
        ResonateTags = new List<PreviewTag>
        {
            new(Localization.T("语录"), c.EnabledTasks.ResonateWithAhab ? Localization.T("开启") : Localization.T("关闭状态"), c.EnabledTasks.ResonateWithAhab),
        };
        AfterSummary = FormatAfterSummary(c.AfterCompletion);

        OnPropertyChanged(nameof(ExpMinusEnabled));
        OnPropertyChanged(nameof(ExpPlusEnabled));
        OnPropertyChanged(nameof(ThreadMinusEnabled));
        OnPropertyChanged(nameof(ThreadPlusEnabled));
        OnPropertyChanged(nameof(ContMinusEnabled));
        OnPropertyChanged(nameof(ContPlusEnabled));
        OnPropertyChanged(nameof(BuyMinusEnabled));
        OnPropertyChanged(nameof(BuyPlusEnabled));
        OnPropertyChanged(nameof(MirrorMinusEnabled));
        OnPropertyChanged(nameof(MirrorPlusEnabled));
    }

    public bool ExpMinusEnabled => Config.DailyTask.SetExpCount > 0;
    public bool ExpPlusEnabled => Config.DailyTask.SetExpCount < 99;
    public bool ThreadMinusEnabled => Config.DailyTask.SetThreadCount > 0;
    public bool ThreadPlusEnabled => Config.DailyTask.SetThreadCount < 99;
    public bool ContMinusEnabled => Config.DailyTask.UseContinuousCombatSelect > 1;
    public bool ContPlusEnabled => Config.DailyTask.UseContinuousCombatSelect < 10;
    public bool BuyMinusEnabled => Config.BuyEnkephalin.SetLunacyToEnkephalin > 0;
    public bool BuyPlusEnabled => Config.BuyEnkephalin.SetLunacyToEnkephalin < 10;
    public bool MirrorMinusEnabled => Config.Mirror.SetMirrorCount > 1;
    public bool MirrorPlusEnabled => Config.Mirror.SetMirrorCount < 99;

    [ObservableProperty] private List<PreviewTag> _resonateTags = new();

    public static string FormatAfterSummary(AfterCompletionConfig cfg)
    {
        var parts = new List<string>();
        if (cfg.Actions.Contains("exit_game")) parts.Add(Localization.T("游戏"));
        if (cfg.Actions.Contains("exit_emulator")) parts.Add(Localization.T("模拟器"));
        if (cfg.Actions.Contains("exit_aalc")) parts.Add("AALC");

        var powerText = cfg.PowerAction switch
        {
            "sleep" => Localization.T("睡眠"),
            "hibernate" => Localization.T("休眠"),
            "lock" => Localization.T("锁屏"),
            "shutdown" => Localization.T("关机"),
            _ => "",
        };
        var modeText = cfg.KeepAfterCompletion ? Localization.T("默认") : Localization.T("本次");

        if (parts.Count == 0 && powerText == "")
            return $"{Localization.T("什么也不干")} ({modeText})";

        string text;
        if (parts.Count > 0)
        {
            text = $"{Localization.T("退出")}{string.Join(Localization.T("与"), parts)}";
            if (powerText != "") text += $"{Localization.T("后")}{powerText}";
        }
        else text = powerText;

        return $"{text} ({modeText})";
    }

    /* ==================== 任务启用开关 ==================== */

    [RelayCommand]
    private void ToggleDailyEnabled(bool? v) { Config.EnabledTasks.DailyTask = v == true; NotifyConfigChanged(); }

    [RelayCommand]
    private void ToggleRewardEnabled(bool? v) { Config.EnabledTasks.GetReward = v == true; NotifyConfigChanged(); }

    [RelayCommand]
    private void ToggleEnkephalinEnabled(bool? v) { Config.EnabledTasks.BuyEnkephalin = v == true; NotifyConfigChanged(); }

    [RelayCommand]
    private void ToggleMirrorEnabled(bool? v) { Config.EnabledTasks.Mirror = v == true; NotifyConfigChanged(); }

    [RelayCommand]
    private void ToggleResonateEnabled(bool? v) { Config.EnabledTasks.ResonateWithAhab = v == true; NotifyConfigChanged(); }

    /* ==================== 步进器 ==================== */

    [RelayCommand]
    private void Step(string param)
    {
        var parts = param.Split(':');
        int delta = int.Parse(parts[1]);
        switch (parts[0])
        {
            case "exp": Config.DailyTask.SetExpCount = Math.Clamp(Config.DailyTask.SetExpCount + delta, 0, 99); break;
            case "thread": Config.DailyTask.SetThreadCount = Math.Clamp(Config.DailyTask.SetThreadCount + delta, 0, 99); break;
            case "cont": Config.DailyTask.UseContinuousCombatSelect = Math.Clamp(Config.DailyTask.UseContinuousCombatSelect + delta, 1, 10); break;
            case "buy": Config.BuyEnkephalin.SetLunacyToEnkephalin = Math.Clamp(Config.BuyEnkephalin.SetLunacyToEnkephalin + delta, 0, 10); break;
            case "mirror": Config.Mirror.SetMirrorCount = Math.Clamp(Config.Mirror.SetMirrorCount + delta, 1, 99); break;
        }
        NotifyConfigChanged();
    }

    /* ==================== 工具栏命令 ==================== */

    [RelayCommand]
    private void SelectAll()
    {
        if (IsBusy) return;
        Config.EnabledTasks.DailyTask = true;
        Config.EnabledTasks.GetReward = true;
        Config.EnabledTasks.BuyEnkephalin = true;
        Config.EnabledTasks.Mirror = true;
        NotifyConfigChanged();
    }

    [RelayCommand]
    private void ClearAll()
    {
        if (IsBusy) return;
        Config.EnabledTasks.DailyTask = false;
        Config.EnabledTasks.GetReward = false;
        Config.EnabledTasks.BuyEnkephalin = false;
        Config.EnabledTasks.Mirror = false;
        NotifyConfigChanged();
    }

    [RelayCommand]
    private void Start()
    {
        if (!Backend.CanStart(out var warn))
        {
            MainWindow.Toast(warn, "warning");
            return;
        }
        Backend.StartExecution();
    }

    [RelayCommand]
    private void Stop() => Backend.StopExecutionByUser();

    [RelayCommand]
    private void PauseResume()
    {
        if (ExecutionState == "running") Backend.PauseExecution();
        else if (ExecutionState == "paused") Backend.ResumeExecution();
    }

    [RelayCommand]
    private void ClearLogs()
    {
        Logs.Clear();
    }

    public event Action? RequestAfterCompletionModal;

    [RelayCommand]
    private void OpenAfterCompletion() => RequestAfterCompletionModal?.Invoke();

    /* ==================== ComboBox 索引包装（配置值 ↔ 选中索引） ==================== */

    public int WinSizeIndex { get => Config.SetWindows.SetWinSize switch { 720 => 0, 1440 => 2, _ => 1 }; set { Config.SetWindows.SetWinSize = value switch { 0 => 720, 2 => 1440, _ => 1080 }; NotifyConfigChanged(); } }
    public int WinPosIndex { get => int.TryParse(Config.SetWindows.SetWinPosition, out var p) ? p : 0; set { Config.SetWindows.SetWinPosition = value.ToString(); NotifyConfigChanged(); } }
    public int ShotIntervalIndex { get => Config.SetWindows.ScreenshotInterval switch { 0.2 => 0, 1 => 2, _ => 1 }; set { Config.SetWindows.ScreenshotInterval = value switch { 0 => 0.2, 2 => 1, _ => 0.5 }; NotifyConfigChanged(); } }
    public int MouseIntervalIndex { get => Config.SetWindows.MouseActionInterval switch { 0.1 => 0, 0.5 => 2, _ => 1 }; set { Config.SetWindows.MouseActionInterval = value switch { 0 => 0.1, 2 => 0.5, _ => 0.3 }; NotifyConfigChanged(); } }
    public int RewardModeIndex { get => Config.GetReward.SetGetPrize; set { Config.GetReward.SetGetPrize = value; NotifyConfigChanged(); } }
    public int DailyTeamIndex { get => Math.Max(0, Config.DailyTask.DailyTeams - 1); set { Config.DailyTask.DailyTeams = value + 1; NotifyConfigChanged(); } }

    public int ExpDay12Index { get => Config.DailyTask.ExpDay12 - 1; set { Config.DailyTask.ExpDay12 = value + 1; NotifyConfigChanged(); } }
    public int ExpDay34Index { get => Config.DailyTask.ExpDay34 - 1; set { Config.DailyTask.ExpDay34 = value + 1; NotifyConfigChanged(); } }
    public int ExpDay56Index { get => Config.DailyTask.ExpDay56 - 1; set { Config.DailyTask.ExpDay56 = value + 1; NotifyConfigChanged(); } }
    public int ExpDay7Index { get => Config.DailyTask.ExpDay7 - 1; set { Config.DailyTask.ExpDay7 = value + 1; NotifyConfigChanged(); } }
    public int ThreadDay1Index { get => Config.DailyTask.ThreadDay1 - 1; set { Config.DailyTask.ThreadDay1 = value + 1; NotifyConfigChanged(); } }
    public int ThreadDay2Index { get => Config.DailyTask.ThreadDay2 - 1; set { Config.DailyTask.ThreadDay2 = value + 1; NotifyConfigChanged(); } }
    public int ThreadDay3Index { get => Config.DailyTask.ThreadDay3 - 1; set { Config.DailyTask.ThreadDay3 = value + 1; NotifyConfigChanged(); } }
    public int ThreadDay4Index { get => Config.DailyTask.ThreadDay4 - 1; set { Config.DailyTask.ThreadDay4 = value + 1; NotifyConfigChanged(); } }
    public int ThreadDay5Index { get => Config.DailyTask.ThreadDay5 - 1; set { Config.DailyTask.ThreadDay5 = value + 1; NotifyConfigChanged(); } }
    public int ThreadDay6Index { get => Config.DailyTask.ThreadDay6 - 1; set { Config.DailyTask.ThreadDay6 = value + 1; NotifyConfigChanged(); } }
    public int ThreadDay7Index { get => Config.DailyTask.ThreadDay7 - 1; set { Config.DailyTask.ThreadDay7 = value + 1; NotifyConfigChanged(); } }

    /* 卡片内部 Tab（general / advanced） */
    [ObservableProperty] private string _setWindowsTab = "general";
    [ObservableProperty] private string _dailyTab = "general";
    [ObservableProperty] private string _enkephalinTab = "general";
    [ObservableProperty] private string _mirrorTab = "general";

    /* ==================== 设备连接 ==================== */

    public ObservableCollection<DeviceInfo> Devices => Backend.Devices.ToObservableCollection();

    [ObservableProperty] private DeviceInfo? _selectedDevice;
    [ObservableProperty] private string _connectionStatus = "disconnected";
    [ObservableProperty] private bool _scanning;

    [RelayCommand]
    private void ConnectDevice(DeviceInfo? device)
    {
        if (device == null || ConnectionStatus == "connecting") return;
        Backend.ConnectDevice(device.Id);
    }

    [RelayCommand]
    private async Task RescanDevicesAsync()
    {
        Scanning = true;
        OnPropertyChanged(nameof(Devices));
        await Task.Delay(400);
        Scanning = false;
    }

    [RelayCommand]
    private void Disconnect() => Backend.DisconnectDevice();
}

public static class EnumerableExtensions
{
    public static ObservableCollection<T> ToObservableCollection<T>(this IEnumerable<T> source)
        => new(source);
}
