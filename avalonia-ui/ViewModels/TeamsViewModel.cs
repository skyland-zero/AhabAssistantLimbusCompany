using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Services;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace AhabAssistant.Avalonia.ViewModels;

public partial class TeamsViewModel : ObservableObject
{
    public MockBackend Backend => MockBackend.Instance;

    public ObservableCollection<TeamDetail> Teams { get; } = new();
    public ObservableCollection<SinnerInfo> Sinners { get; } = new();

    [ObservableProperty] private string _activeTab = "all";
    [ObservableProperty] private TeamDetail? _deleteTarget;

    public TeamsViewModel()
    {
        foreach (var s in Backend.Sinners) Sinners.Add(s);
        Reload();
    }

    public void Reload()
    {
        Teams.Clear();
        foreach (var t in Backend.Teams) Teams.Add(t);
        RefreshCounts();
    }

    /// <summary>卡片显示包装（含徽章与人格位次）。</summary>
    public TeamCardVm MakeCard(TeamDetail t)
    {
        var mc = t.MirrorConfig;
        return new TeamCardVm
        {
            Team = t,
            Purpose = PurposeLabel(t.Purpose),
            Scheme = SchemeLabel(t.AccessoryScheme),
            MemberCountText = Localization.IsEnglish
                ? $"{t.Sinners.Count} sinners"
                : $"{t.Sinners.Count} 名人格",
            HasStarlight = mc?.OpeningBonus is { } b && b.Any(x => x > 0),
            HasSecondSystem = mc?.SecondSystem == true,
            DiscardCount = mc?.DiscardSystems.Values.Count(v => v) ?? 0,
            HasSoloPass = mc?.DefenseForSolo == true,
            HasTeamCode = mc?.UseTeamCode == true,
            Chips = t.Sinners.Select((id, idx) => new SinnerChip(idx + 1, SinnerName(id))).ToList(),
        };
    }

    public IEnumerable<TeamCardVm> FilteredCards => FilteredTeams.Select(MakeCard);

    /* 用途筛选计数 */
    public int AllCount => Teams.Count;
    public int MirrorCount => Teams.Count(t => t.Purpose == "mirror");
    public int LuxcavationCount => Teams.Count(t => t.Purpose == "luxcavation");
    public int GeneralCount => Teams.Count(t => t.Purpose == "general");

    public IEnumerable<TeamDetail> FilteredTeams =>
        ActiveTab == "all" ? Teams : Teams.Where(t => t.Purpose == ActiveTab);

    public bool IsEmpty => !FilteredTeams.Any();
    public bool IsNoTeams => Teams.Count == 0;
    public string EmptyTitle => Localization.T(IsNoTeams ? "暂无队伍" : "当前分类暂无队伍");
    public string EmptyDescription => Localization.T(IsNoTeams ? "创建一个队伍用于镜牢、经验本等任务" : "点击上方“新建队伍”创建一个队伍");

    partial void OnActiveTabChanged(string value)
    {
        OnPropertyChanged(nameof(FilteredCards));
        OnPropertyChanged(nameof(IsEmpty));
        OnPropertyChanged(nameof(IsNoTeams));
        OnPropertyChanged(nameof(EmptyTitle));
        OnPropertyChanged(nameof(EmptyDescription));
    }

    private void RefreshCounts()
    {
        OnPropertyChanged(nameof(AllCount));
        OnPropertyChanged(nameof(MirrorCount));
        OnPropertyChanged(nameof(LuxcavationCount));
        OnPropertyChanged(nameof(GeneralCount));
        OnPropertyChanged(nameof(FilteredCards));
        OnPropertyChanged(nameof(IsEmpty));
        OnPropertyChanged(nameof(IsNoTeams));
        OnPropertyChanged(nameof(EmptyTitle));
        OnPropertyChanged(nameof(EmptyDescription));
    }

    public static string PurposeLabel(string purpose) => Localization.T(purpose switch
    {
        "mirror" => "镜牢",
        "luxcavation" => "经验本",
        _ => "通用",
    });

    public static string SchemeLabel(string scheme) => Localization.T(scheme switch
    {
        "burn" => "烧伤", "bleed" => "流血", "tremor" => "震颤", "rupture" => "破裂",
        "sinking" => "沉沦", "poise" => "呼吸", "charge" => "充能", "slash" => "斩击",
        "pierce" => "突刺", "blunt" => "打击", _ => "烧伤",
    });

    public string SinnerName(string id) => Backend.Sinners.Find(s => s.Id == id)?.Name ?? id;

    public bool HasStarlight(TeamDetail team) =>
        team.MirrorConfig?.OpeningBonus is { } bonus && bonus.Any(b => b > 0);

    public int DiscardCount(TeamDetail team) =>
        team.MirrorConfig?.DiscardSystems.Values.Count(v => v) ?? 0;

    [RelayCommand]
    private void SetTab(string tab)
    {
        ActiveTab = tab;
    }

    public event Action<TeamDetail?>? RequestEditModal;

    [RelayCommand]
    private void NewTeam()
    {
        var team = new TeamDetail
        {
            Id = "",
            Name = "",
            Purpose = ActiveTab == "all" ? "general" : ActiveTab,
            AccessoryScheme = "burn",
            Enabled = true,
            MirrorConfig = TeamMirrorConfig.CreateDefault(),
        };
        RequestEditModal?.Invoke(team);
    }

    [RelayCommand]
    private void EditTeam(TeamDetail team) => RequestEditModal?.Invoke(team);

    /// <summary>将编辑窗口返回的工作副本写回后端，并刷新列表。</summary>
    public void SaveTeam(TeamDetail team)
    {
        if (string.IsNullOrWhiteSpace(team.Name)) return;

        if (string.IsNullOrWhiteSpace(team.Id))
            team.Id = $"team-{Guid.NewGuid():N}";

        var index = Backend.Teams.FindIndex(t => t.Id == team.Id);
        if (index >= 0)
            Backend.Teams[index] = team;
        else
            Backend.Teams.Add(team);

        Backend.SaveTeams();
        Reload();
    }

    [RelayCommand]
    private void AskDelete(TeamDetail team) => DeleteTarget = team;

    [RelayCommand]
    private void CancelDelete() => DeleteTarget = null;

    [RelayCommand]
    private void ConfirmDelete()
    {
        if (DeleteTarget == null) return;
        Backend.Teams.RemoveAll(t => t.Id == DeleteTarget.Id);
        Backend.SaveTeams();
        DeleteTarget = null;
        Reload();
    }
}

public record SinnerChip(int Index, string Name);

public class TeamCardVm
{
    public TeamDetail Team { get; init; } = null!;
    public string Purpose { get; init; } = "";
    public string Scheme { get; init; } = "";
    public string MemberCountText { get; init; } = "";
    public bool HasStarlight { get; init; }
    public bool HasSecondSystem { get; init; }
    public int DiscardCount { get; init; }
    public bool HasDiscard => DiscardCount > 0;
    public bool HasSoloPass { get; init; }
    public bool HasTeamCode { get; init; }
    public bool Disabled => !Team.Enabled;
    public List<SinnerChip> Chips { get; init; } = new();
}
