using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;
using AhabAssistant.Avalonia.Services;

namespace AhabAssistant.Avalonia.Models;

/* ============================ 任务与执行数据模型（对齐 services/ipc/types.ts） ============================ */

public class AfterCompletionConfig
{
    public List<string> Actions { get; set; } = new() { "exit_game" }; // exit_game | exit_emulator | exit_aalc
    public string PowerAction { get; set; } = "none";  // none|sleep|hibernate|lock|shutdown
    public bool KeepAfterCompletion { get; set; } = true;
}

public class SetWindowsConfig
{
    public double SetWinSize { get; set; } = 1080;
    public string SetWinPosition { get; set; } = "0";
    public bool SetReduceMiscontact { get; set; } = true;
    public double ScreenshotInterval { get; set; } = 0.5;
    public double MouseActionInterval { get; set; } = 0.3;
    public double MouseDownDuration { get; set; } = 0.1;
    public bool UsePostMessage { get; set; }
}

public class DailyTaskConfig
{
    public int SetExpCount { get; set; } = 3;
    public int SetThreadCount { get; set; } = 3;
    public int DailyTeams { get; set; } = 1;
    public bool UseContinuousCombat { get; set; } = true;
    public int UseContinuousCombatSelect { get; set; } = 3;
    public bool TargetedTeamingExp { get; set; }
    public int ExpDay12 { get; set; } = 1;
    public int ExpDay34 { get; set; } = 1;
    public int ExpDay56 { get; set; } = 1;
    public int ExpDay7 { get; set; } = 1;
    public bool TargetedTeamingThread { get; set; }
    public int ThreadDay1 { get; set; } = 1;
    public int ThreadDay2 { get; set; } = 1;
    public int ThreadDay3 { get; set; } = 1;
    public int ThreadDay4 { get; set; } = 1;
    public int ThreadDay5 { get; set; } = 1;
    public int ThreadDay6 { get; set; } = 1;
    public int ThreadDay7 { get; set; } = 1;
}

public class GetRewardConfig
{
    public int SetGetPrize { get; set; } // 0 全部 / 1 狂气与通行证 / 2 仅邮件
}

public class BuyEnkephalinConfig
{
    public int SetLunacyToEnkephalin { get; set; } = 2;
    public bool DrGrandetMode { get; set; } = true;
    public bool SkipEnkephalin { get; set; }
}

public class MirrorConfig
{
    public int SetMirrorCount { get; set; } = 3;
    public bool InfiniteDungeons { get; set; }
    public bool HardMirror { get; set; }
    public bool NoWeeklyBonuses { get; set; }
    public bool Floor3Exit { get; set; }
    public bool SaveRewards { get; set; }
    public bool HardMirrorSingleBonuses { get; set; }
    public bool SelectEventPack { get; set; }
    public bool SkipEventPack { get; set; }
    public bool ReClaimRewards { get; set; }
    public bool NotSkipWhitegossypium { get; set; }
    public bool FightToLastMan { get; set; }
    public bool MirrorKeyboardNavigation { get; set; }
    public bool MirrorKeyboardSimplePathfinding { get; set; }
}

public class EnabledTasks
{
    public bool DailyTask { get; set; } = true;
    public bool GetReward { get; set; } = true;
    public bool BuyEnkephalin { get; set; }
    public bool Mirror { get; set; } = true;
    public bool ResonateWithAhab { get; set; } = true;
}

public class TasksConfig
{
    public EnabledTasks EnabledTasks { get; set; } = new();
    public SetWindowsConfig SetWindows { get; set; } = new();
    public DailyTaskConfig DailyTask { get; set; } = new();
    public GetRewardConfig GetReward { get; set; } = new();
    public BuyEnkephalinConfig BuyEnkephalin { get; set; } = new();
    public MirrorConfig Mirror { get; set; } = new();
    public AfterCompletionConfig AfterCompletion { get; set; } = new();
}

public class ExecutionStatusPayload
{
    public string State { get; set; } = "idle"; // idle | running | paused
    public string? CurrentTaskId { get; set; }
}

public class MirrorProgressPayload
{
    public int Current { get; set; }
    public int Total { get; set; }
    public bool IsHard { get; set; }
    public bool IsInfinite { get; set; }

    [JsonIgnore]
    public string TotalText => IsInfinite ? "∞" : Total.ToString();

    [JsonIgnore]
    public string ModeText => Localization.T(IsHard ? "困难" : "普通");
}

/* ============================ 队伍与罪人 ============================ */

public class TeamMirrorConfig
{
    public int TeamSystem { get; set; } // 0 burn ... 9 blunt
    public int ShopStrategy { get; set; }
    public Dictionary<string, bool> DiscardSystems { get; set; } = new();
    public bool DoNotHeal { get; set; }
    public bool DoNotBuy { get; set; }
    public bool DoNotFuse { get; set; }
    public bool DoNotSell { get; set; }
    public bool DoNotEnhance { get; set; }
    public bool OnlyAggressiveFuse { get; set; }
    public bool DoNotSystemFuse { get; set; }
    public bool OnlySystemFuse { get; set; }
    public bool AggressiveAlsoEnhance { get; set; }
    public bool AggressiveSaveSystems { get; set; }
    public bool AfterLevelIv { get; set; }
    public int AfterLevelIvSelect { get; set; }
    public List<bool> IgnoreShop { get; set; } = new() { false, false, false, false, false };
    public int MaxKeywordRefresh { get; set; } = 1;
    public int MaxNormalRefresh { get; set; } = 1;
    public bool SecondSystem { get; set; }
    public int SecondSystemSelect { get; set; }
    public int SecondSystemSetting { get; set; } = 2;
    public bool SecondSystemFuseIv { get; set; } = true;
    public bool SecondSystemBuy { get; set; } = true;
    public bool SecondSystemSelectReward { get; set; } = true;
    public bool SecondSystemPowerUp { get; set; } = true;
    public bool AvoidSkill3 { get; set; }
    public bool PrioritizeSkill3 { get; set; }
    public bool ReFormationEachFloor { get; set; }
    public bool DefenseFirstRound { get; set; }
    public bool DefenseForSolo { get; set; }
    public int DefenseForSoloTurns { get; set; } = 5;
    public bool SkillReplacement { get; set; }
    public int SkillReplacementSelect { get; set; }
    public int SkillReplacementMode { get; set; }
    public bool UseStarlight { get; set; }
    public List<int> OpeningBonus { get; set; } = new() { 1, 1, 1, 1, 0, 0, 0, 0, 0, 0 };
    public bool FixedTeamUse { get; set; }
    public int FixedTeamUseSelect { get; set; }
    public bool UseTeamCode { get; set; }
    public string TeamCode { get; set; } = "";
    public bool UseCustomThemePackWeight { get; set; }
    public bool ObserveEgoGift { get; set; }
    public List<string> ObserveEgoGiftSelected { get; set; } = new();

    public static TeamMirrorConfig CreateDefault()
    {
        var cfg = new TeamMirrorConfig();
        foreach (var k in new[] { "burn", "bleed", "tremor", "rupture", "sinking", "poise", "charge", "slash", "pierce", "blunt" })
            cfg.DiscardSystems[k] = false;
        return cfg;
    }
}

public class TeamDetail
{
    public string Id { get; set; } = "";
    public string Name { get; set; } = "";
    public List<string> Sinners { get; set; } = new();
    public string Purpose { get; set; } = "general"; // mirror | luxcavation | general
    public string AccessoryScheme { get; set; } = "burn";
    public bool Enabled { get; set; } = true;
    public TeamMirrorConfig? MirrorConfig { get; set; }
}

public class SinnerInfo
{
    public string Id { get; set; } = "";
    public string Name { get; set; } = "";
}

/* ============================ 工具箱 / 主题包 / 资源中心 ============================ */

public class ToolStatusPayload
{
    public string ToolId { get; set; } = "";
    public bool Running { get; set; }
}

public class ThemePack
{
    public string Id { get; set; } = "";
    public string Name { get; set; } = "";
    public int Weight { get; set; }
    public bool Enabled { get; set; } = true;
    public string Tier { get; set; } = "T3";
}

public class ThemePackState
{
    public bool HardMirrorActive { get; set; } = true;
    public List<ThemePack> Packs { get; set; } = new();
}

public class ResourceGroup
{
    public string Id { get; set; } = "";
    public string Name { get; set; } = "";
    public string LocalVersion { get; set; } = "";
    public string? RemoteVersion { get; set; }
    public long? LastSyncAt { get; set; }

    [JsonIgnore]
    public bool HasUpdate => RemoteVersion != null && RemoteVersion != LocalVersion;

    [JsonIgnore]
    public string StatusText => HasUpdate
        ? $"{Localization.T("可更新到")} {RemoteVersion}"
        : Localization.T("已是最新");

    [JsonIgnore]
    public string RemoteVersionText => RemoteVersion ?? "—";

    [JsonIgnore]
    public string LastSyncText => LastSyncAt is { } ts
        ? DateTimeOffset.FromUnixTimeMilliseconds(ts).LocalDateTime.ToString("yyyy-MM-dd HH:mm:ss")
        : Localization.T("从未同步");
}

public class SyncProgressPayload
{
    public string Scope { get; set; } = "";
    public int Progress { get; set; }
}

public class LogEntryPayload
{
    public long Ts { get; set; }
    public string Level { get; set; } = "info"; // debug|info|warn|error
    public string Message { get; set; } = "";

    [JsonIgnore]
    public string TimeText => DateTimeOffset.FromUnixTimeMilliseconds(Ts).LocalDateTime.ToString("HH:mm:ss");
}

/* ============================ 设置 / 设备连接 ============================ */

public class HotkeyConfig
{
    public string StartStop { get; set; } = "F10";
    public string? PauseResume { get; set; } = "F11";
    public bool Enabled { get; set; } = true;
}

public class SystemSettingsConfig
{
    public bool Simulator { get; set; } = true;
    public int SimulatorType { get; set; } // 0: MuMu, 10: other emulator
    public int SimulatorPort { get; set; } = 16384;
    public int StartEmulatorTimeout { get; set; } = 60;
    public bool MemoryProtection { get; set; } = true;
    public bool MinimizeToTray { get; set; } = true;
    public bool Autostart { get; set; }
    public bool ExperimentalKeepScreenAwake { get; set; } = true;
    public bool ExperimentalHdrWarning { get; set; } = true;
    public bool UpdatePrereleaseEnable { get; set; }
    public string UpdateSource { get; set; } = "GitHub";
    public string MirrorchyanCdk { get; set; } = "";
}

public class UpdateInfo
{
    public bool UpdateAvailable { get; set; }
    public string Latest { get; set; } = "";
}

public class DeviceInfo
{
    public string Id { get; set; } = "";
    public string Name { get; set; } = "";
    public string? Detail { get; set; }
}
