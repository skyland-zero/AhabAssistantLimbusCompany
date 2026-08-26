using System;
using System.Collections.ObjectModel;
using System.Linq;
using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Services;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace AhabAssistant.Avalonia.ViewModels;

public partial class ResourcesViewModel : ObservableObject
{
    public MockBackend Backend => MockBackend.Instance;

    public ObservableCollection<ResourceGroup> Groups { get; } = new();

    /// <summary>null = 空闲；否则为同步进度 0-100</summary>
    [ObservableProperty] private int? _syncProgress;

    partial void OnSyncProgressChanged(int? value)
    {
        OnPropertyChanged(nameof(IsSyncing));
        OnPropertyChanged(nameof(SyncText));
        OnPropertyChanged(nameof(SyncButtonEnabled));
    }

    public bool IsSyncing => SyncProgress != null;
    public bool SyncButtonEnabled => !IsSyncing;
    public string SyncText => $"{Localization.T("同步中…")} {SyncProgress ?? 0}%";

    public ResourcesViewModel()
    {
        Reload();
        Backend.SyncProgress += p =>
        {
            SyncProgress = p.Progress;
            if (p.Progress >= 100)
            {
                // 进度走完后刷新最终状态并复位
                var timer = new System.Timers.Timer(300) { AutoReset = false };
                timer.Elapsed += (_, _) =>
                    global::Avalonia.Threading.Dispatcher.UIThread.Post(() =>
                    {
                        Reload();
                        SyncProgress = null;
                        MainWindow.Toast(Localization.T("资源同步完成"));
                    });
                timer.Start();
            }
        };
    }

    private void Reload()
    {
        Groups.Clear();
        foreach (var g in Backend.Resources) Groups.Add(g);
    }

    [RelayCommand]
    private void CheckUpdate()
    {
        Backend.CheckResourceUpdate();
        Reload();
    }

    [RelayCommand]
    private void SyncNow()
    {
        if (IsSyncing) return;
        Backend.StartResourceSync("all");
    }
}
