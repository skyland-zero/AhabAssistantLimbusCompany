using System;
using System.Collections.ObjectModel;
using System.Linq;
using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Services;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace AhabAssistant.Avalonia.ViewModels;

public partial class SettingsViewModel : ObservableObject
{
    public MockBackend Backend => MockBackend.Instance;

    public ObservableCollection<AccentPreset> Accents => new(ThemeService.AccentPresets);

    /* 外观 */
    [ObservableProperty] private string _themeMode = App.ThemeMode;
    [ObservableProperty] private string _accentId = App.AccentId;
    [ObservableProperty] private string _language = App.Language;

    public bool ThemeModeLight => ThemeMode == "light";
    public bool ThemeModeDark => ThemeMode == "dark";
    public bool ThemeModeSystem => ThemeMode == "system";

    public bool IsZhLang => Language == "zh-CN";
    public bool IsEnLang => Language == "en-US";

    /* 热键 */
    [ObservableProperty] private HotkeyConfig _hotkey;
    [ObservableProperty] private string? _capturingTarget;

    public string StartStopDisplay => CapturingTarget == "startStop" ? Localization.T("点击输入框后按下快捷键") : (string.IsNullOrEmpty(Hotkey.StartStop) ? Localization.T("启动 / 停止热键") : Hotkey.StartStop);
    public string PauseResumeDisplay => CapturingTarget == "pauseResume" ? Localization.T("点击输入框后按下快捷键") : (string.IsNullOrEmpty(Hotkey.PauseResume) ? Localization.T("暂停 / 继续热键") : Hotkey.PauseResume);

    /* 系统设置 */
    [ObservableProperty] private Models.SystemSettingsConfig _sys;

    public bool ShowSimulatorOptions => Sys.Simulator;
    public bool ShowMuMuTimeout => Sys.Simulator && Sys.SimulatorType == 0;
    public bool ShowMirrorChyanCdk => Sys.UpdateSource == "MirrorChyan";

    // UI 使用 0/1 的选中索引，配置协议使用 0/10（MuMu/其他模拟器）。
    public int SimulatorTypeIndex
    {
        get => Sys.SimulatorType == 10 ? 1 : 0;
        set
        {
            Sys.SimulatorType = value == 1 ? 10 : 0;
            SaveSysSettings();
            OnPropertyChanged(nameof(ShowMuMuTimeout));
        }
    }

    public string VersionText { get; } = "v" + (Assembly.GetEntryAssembly()
        ?.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion ?? "1.0.0")
        .Split('+')[0];

    public string FooterText => $"{Language} · Ahab Assistant Limbus Company {VersionText}";

    public SettingsViewModel()
    {
        _hotkey = Backend.Hotkey;
        _sys = Backend.SystemSettings;
        Backend.HotkeyUpdated += hk =>
        {
            Hotkey = hk;
            OnPropertyChanged(nameof(StartStopDisplay));
            OnPropertyChanged(nameof(PauseResumeDisplay));
        };
    }

    partial void OnThemeModeChanged(string value)
    {
        OnPropertyChanged(nameof(ThemeModeLight));
        OnPropertyChanged(nameof(ThemeModeDark));
        OnPropertyChanged(nameof(ThemeModeSystem));
        App.ThemeMode = value;
        App.ApplyTheme();
        App.SavePrefs();
    }

    partial void OnAccentIdChanged(string value)
    {
        App.AccentId = value;
        App.ApplyTheme();
        App.SavePrefs();
    }

    partial void OnLanguageChanged(string value)
    {
        OnPropertyChanged(nameof(IsZhLang));
        OnPropertyChanged(nameof(IsEnLang));
        App.Language = value;
        App.SavePrefs();
        OnPropertyChanged(nameof(FooterText));
    }

    partial void OnSysChanged(Models.SystemSettingsConfig value) { }

    [RelayCommand]
    private void SetThemeMode(string mode) => ThemeMode = mode;

    [RelayCommand]
    private void SetAccent(string id) => AccentId = id;

    [RelayCommand]
    private void SetLanguage(string lang) => Language = lang;

    public void SaveHotkey() { Backend.Hotkey = Hotkey; Backend.SaveHotkey(); }

    public void SetHotkey(string target, string combo)
    {
        if (target == "startStop") Hotkey.StartStop = combo;
        else Hotkey.PauseResume = combo;
        SaveHotkey();
        CapturingTarget = null;
        OnPropertyChanged(nameof(StartStopDisplay));
        OnPropertyChanged(nameof(PauseResumeDisplay));
    }

    public void SaveSysSettings()
    {
        Backend.SystemSettings = Sys;
        Backend.SaveSystemSettings();
        OnPropertyChanged(nameof(ShowSimulatorOptions));
        OnPropertyChanged(nameof(ShowMuMuTimeout));
        OnPropertyChanged(nameof(ShowMirrorChyanCdk));
    }

    [RelayCommand]
    private void CheckUpdate() => MainWindow.Toast($"{Localization.T("当前已是最新版本（")}{VersionText}）");

    [RelayCommand]
    private void OpenRepo()
    {
        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = "https://github.com/KIYI671/AhabAssistantLimbusCompany",
                UseShellExecute = true,
            });
        }
        catch { /* 忽略 */ }
    }
}
