using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Services;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace AhabAssistant.Avalonia.ViewModels;

public partial class ToolboxViewModel : ObservableObject
{
    public MockBackend Backend => MockBackend.Instance;

    public ObservableCollection<ToolCardVm> Tools { get; } = new();

    public ToolboxViewModel()
    {
        Tools.Add(new ToolCardVm("infinite_battle", "target", Localization.T("自动战斗"), Localization.T("循环执行战斗直至手动停止"), this));
        Tools.Add(new ToolCardVm("enkephalin", "pill", Localization.T("体力换饼"), Localization.T("自动将狂气转换为体力并合成脑啡肽模块，防止体力溢出"), this));
        Tools.Add(new ToolCardVm("screenshot", "camera", Localization.T("辅助截图"), Localization.T("截取当前游戏窗口画面并保存到 AALC 目录"), this));

        Backend.ToolStatus += s =>
        {
            foreach (var t in Tools)
                if (t.Id == s.ToolId)
                    t.Running = s.Running;
        };
    }

    [RelayCommand]
    private void ToggleTool(ToolCardVm tool)
    {
        if (tool.IsScreenshot) return;
        if (tool.Running) Backend.ToolStop(tool.Id);
        else Backend.ToolStart(tool.Id);
    }

    [RelayCommand]
    private void TakeScreenshot()
    {
        Backend.TakeScreenshot();
        MainWindow.Toast(Localization.T("截图完成，已保存到 AALC 目录"));
    }
}

public partial class ToolCardVm : ObservableObject
{
    private readonly ToolboxViewModel _owner;

    public string Id { get; }
    public string Icon { get; }
    public string Title { get; }
    public string Desc { get; }
    public bool IsScreenshot => Id == "screenshot";
    public string IsScreenshotText => IsScreenshot ? "—" : Localization.T("待机");

    [ObservableProperty] private bool _running;

    partial void OnRunningChanged(bool value)
    {
        OnPropertyChanged(nameof(ShowRunButton));
        OnPropertyChanged(nameof(ShowStopButton));
        OnPropertyChanged(nameof(IsScreenshotText));
    }

    public bool ShowRunButton => !Running && !IsScreenshot;
    public bool ShowStopButton => Running && !IsScreenshot;

    public ToolCardVm(string id, string icon, string title, string desc, ToolboxViewModel owner)
    {
        Id = id;
        Icon = icon;
        Title = title;
        Desc = desc;
        _owner = owner;
    }

    [RelayCommand]
    private void Run() => _owner.ToggleToolCommand.Execute(this);
}
