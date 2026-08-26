using System;
using System.Collections.ObjectModel;
using System.Linq;
using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Services;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace AhabAssistant.Avalonia.ViewModels;

public partial class ThemePacksViewModel : ObservableObject
{
    public MockBackend Backend => MockBackend.Instance;

    public ObservableCollection<PackCardVm> Packs { get; } = new();

    [ObservableProperty] private bool _sortByWeight;
    [ObservableProperty] private bool _hardMirrorActive;

    public int TotalWeight => Packs.Where(p => p.Enabled).Sum(p => (int)p.Weight);

    public ThemePacksViewModel()
    {
        HardMirrorActive = Backend.ThemePackState.HardMirrorActive;
        Rebuild();
    }

    private void Rebuild()
    {
        var list = Backend.ThemePackState.Packs.ToList();
        if (SortByWeight) list = list.OrderByDescending(p => p.Weight).ToList();

        Packs.Clear();
        foreach (var p in list)
        {
            var vm = new PackCardVm(p, this);
            Packs.Add(vm);
        }
        OnPropertyChanged(nameof(TotalWeight));
    }

    public void PatchPack(ThemePack pack)
    {
        Backend.SaveThemePacks();
        Rebuild();
    }

    [RelayCommand]
    private void ToggleSort() { SortByWeight = !SortByWeight; Rebuild(); }

    [RelayCommand]
    private void EnableAll() => SetAll(true);

    [RelayCommand]
    private void DisableAll() => SetAll(false);

    private void SetAll(bool enabled)
    {
        foreach (var p in Backend.ThemePackState.Packs) p.Enabled = enabled;
        Backend.SaveThemePacks();
        Rebuild();
    }

    [RelayCommand]
    private void ResetWeights()
    {
        int[] defaults = { 5, 3, 4, 2, 6, 1, 3, 7 };
        for (var i = 0; i < Backend.ThemePackState.Packs.Count; i++)
            Backend.ThemePackState.Packs[i].Weight = i < defaults.Length ? defaults[i] : 3;
        Backend.SaveThemePacks();
        Rebuild();
        MainWindow.Toast(Localization.T("已恢复默认权重"));
    }

    partial void OnHardMirrorActiveChanged(bool value) => OnPropertyChanged(nameof(HardMirrorBannerVisible));
    public bool HardMirrorBannerVisible => HardMirrorActive;
}

public class PackCardVm : ObservableObject
{
    private readonly ThemePacksViewModel _owner;

    public ThemePack Pack { get; }

    public string Name => Pack.Name;
    public string Tier => Pack.Tier;

    public bool Enabled
    {
        get => Pack.Enabled;
        set
        {
            if (Pack.Enabled == value) return;
            Pack.Enabled = value;
            _owner.PatchPack(Pack);
            OnPropertyChanged(nameof(Enabled));
            OnPropertyChanged(nameof(Weight));
        }
    }
    public double Weight
    {
        get => Pack.Weight;
        set
        {
            if (Pack.Weight == (int)value) return;
            Pack.Weight = (int)Math.Clamp(value, 0, 10);
            _owner.PatchPack(Pack);
            OnPropertyChanged(nameof(WeightText));
        }
    }
    public string WeightText => ((int)Weight).ToString();

    public PackCardVm(ThemePack pack, ThemePacksViewModel owner)
    {
        Pack = pack;
        _owner = owner;
    }

    public void SetEnabled(bool enabled)
    {
        Pack.Enabled = enabled;
        OnPropertyChanged(nameof(Enabled));
    }
}
