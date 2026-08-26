using System;
using System.IO;
using System.Text.Json;
using AhabAssistant.Avalonia.Services;
using Avalonia;
using global::Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;

namespace AhabAssistant.Avalonia;

public partial class App : Application
{
    /* 应用级外观偏好（对齐 zustand persist "ahab-ui-settings"） */
    public static string ThemeMode { get; set; } = "system";
    public static string AccentId { get; set; } = ThemeService.DefaultAccent;
    public static double RightPanelWidth { get; set; } = 280;
    public static bool RightPanelCollapsed { get; set; }

    private static string _language = "zh-CN";
    public static string Language
    {
        get => _language;
        set
        {
            var next = value is "en-US" ? value : "zh-CN";
            if (_language == next) return;
            _language = next;
            LanguageChanged?.Invoke();
        }
    }

    public static event Action? LanguageChanged;

    private static readonly string PrefsFile =
        Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "ahab-ui-settings.json"));

    public override void Initialize()
    {
        AvaloniaXamlLoader.Load(this);
    }

    public override void OnFrameworkInitializationCompleted()
    {
        LoadPrefs();
        MockBackend.Instance.LoadPersisted();
        ActualThemeVariantChanged += (_, _) =>
        {
            if (ThemeMode == "system") ThemeService.ApplyTheme(ThemeMode, AccentId);
        };

        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            desktop.MainWindow = new MainWindow();
        }

        base.OnFrameworkInitializationCompleted();
    }

    public static void ApplyTheme() => ThemeService.ApplyTheme(ThemeMode, AccentId);

    public static void SavePrefs()
    {
        try
        {
            var data = new Models.UiPreferences
            {
                ThemeMode = ThemeMode,
                AccentId = AccentId,
                RightPanelWidth = RightPanelWidth,
                RightPanelCollapsed = RightPanelCollapsed,
                Language = Language,
            };
            File.WriteAllText(PrefsFile, JsonSerializer.Serialize(data, Models.AalcJsonContext.Default.UiPreferences));
        }
        catch { /* 忽略 */ }
    }

    private static void LoadPrefs()
    {
        try
        {
            if (!File.Exists(PrefsFile)) return;
            var prefs = JsonSerializer.Deserialize(
                File.ReadAllText(PrefsFile), Models.AalcJsonContext.Default.UiPreferences);
            if (prefs == null) return;
            ThemeMode = prefs.ThemeMode is "light" or "dark" or "system" ? prefs.ThemeMode : "system";
            AccentId = prefs.AccentId;
            RightPanelWidth = Math.Clamp(prefs.RightPanelWidth, 240, 800);
            RightPanelCollapsed = prefs.RightPanelCollapsed;
            Language = prefs.Language;
        }
        catch { /* 忽略 */ }
    }
}
