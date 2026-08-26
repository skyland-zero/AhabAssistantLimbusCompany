using System;
using System.Collections.Generic;
using AhabAssistant.Avalonia.Services;
using AhabAssistant.Avalonia.Views;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media;
using Avalonia.Threading;

namespace AhabAssistant.Avalonia;

public partial class MainWindow : Window
{
    private readonly Dictionary<string, Control> _pageCache = new();
    private string _currentPage = "home";
    private DispatcherTimer? _toastTimer;

    public MainWindow()
    {
        InitializeComponent();
        Localization.ApplyStatic(this);
        App.LanguageChanged += OnLanguageChanged;
        App.ApplyTheme();
        UpdateThemeIcon();

        DragRegion.PointerPressed += OnDragRegionPointerPressed;
        DragRegion.DoubleTapped += (_, _) => OnToggleMaximize(this, null!);

        // 无边框窗口：自绘标题栏
        ExtendClientAreaToDecorationsHint = true;
        ExtendClientAreaChromeHints = global::Avalonia.Platform.ExtendClientAreaChromeHints.NoChrome;
        ExtendClientAreaTitleBarHeightHint = 40;

        // F10 / F11 全局热键（窗口级）
        KeyDown += OnWindowKeyDown;

        Navigate("home");
        AttachedToVisualTree += (_, _) => Localization.ApplyStatic(this);
        PositionChanged += (_, _) => App.SavePrefs();
    }

    /* ==================== 窗口控制 ==================== */

    private void OnDragRegionPointerPressed(object? sender, PointerPressedEventArgs e)
    {
        if (e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
            BeginMoveDrag(e);
    }

    private void OnMinimize(object? sender, RoutedEventArgs e) => WindowState = WindowState.Minimized;

    private void OnToggleMaximize(object? sender, RoutedEventArgs e)
    {
        WindowState = WindowState == WindowState.Maximized ? WindowState.Normal : WindowState.Maximized;
        MaxBtn.Content = WindowState == WindowState.Maximized ? "\uE923" : "\uE922";
    }

    private void OnClose(object? sender, RoutedEventArgs e)
    {
        App.LanguageChanged -= OnLanguageChanged;
        Close();
    }

    private void OnWindowKeyDown(object? sender, KeyEventArgs e)
    {
        var hk = MockBackend.Instance.Hotkey;
        if (!hk.Enabled) return;
        var combo = FormatCombo(e);
        if (combo == hk.StartStop)
        {
            var be = MockBackend.Instance;
            if (be.ExecutionState == "idle") { if (be.CanStart(out _)) be.StartExecution(); }
            else be.StopExecutionByUser();
        }
        else if (combo == hk.PauseResume)
        {
            var be = MockBackend.Instance;
            if (be.ExecutionState == "running") be.PauseExecution();
            else if (be.ExecutionState == "paused") be.ResumeExecution();
        }
    }

    internal static string FormatCombo(KeyEventArgs e)
    {
        if (e.Key is Key.LeftCtrl or Key.RightCtrl or Key.LeftShift or Key.RightShift
            or Key.LeftAlt or Key.RightAlt or Key.LWin or Key.RWin or Key.Escape)
            return "";
        var parts = new List<string>();
        if (e.KeyModifiers.HasFlag(KeyModifiers.Control)) parts.Add("Ctrl");
        if (e.KeyModifiers.HasFlag(KeyModifiers.Alt)) parts.Add("Alt");
        if (e.KeyModifiers.HasFlag(KeyModifiers.Shift)) parts.Add("Shift");
        var keyStr = e.Key.ToString();
        parts.Add(keyStr.Length == 1 ? keyStr.ToUpperInvariant() : keyStr);
        return string.Join("+", parts);
    }

    /* ==================== 主题切换 ==================== */

    private void OnToggleTheme(object? sender, RoutedEventArgs e)
    {
        App.ThemeMode = App.ThemeMode switch
        {
            "light" => "dark",
            "dark" => "system",
            _ => "light",
        };
        App.ApplyTheme();
        UpdateThemeIcon();
        App.SavePrefs();
    }

    private void UpdateThemeIcon()
    {
        ThemeIcon.Text = App.ThemeMode switch
        {
            "system" => "🖥",
            "dark" => "🌙",
            _ => "☀️",
        };
    }

    /* ==================== 页面导航 ==================== */

    private void OnLanguageChanged()
    {
        // 页面中的固定文案在创建时本地化；切换语言后重建当前页，确保
        // DataTemplate 和帮助正文也同步刷新。
        _pageCache.Clear();
        PageHost.Content = null;
        Navigate(_currentPage);
        Localization.ApplyStatic(this);
    }

    private void OnNavClick(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string page }) Navigate(page);
    }

    public void Navigate(string page)
    {
        _currentPage = page;

        foreach (var name in new[] { "NavHome", "NavTeams", "NavThemes", "NavToolbox", "NavResources", "NavHelp" })
        {
            if (this.FindControl<Button>(name) is { } btn)
                btn.Classes.Set("active", (string?)btn.Tag == page);
        }
        SettingsBtn.Classes.Set("active", page == "settings");

        PageHost.Content = GetPage(page);
    }

    private Control GetPage(string page) => _pageCache.TryGetValue(page, out var cached)
        ? cached
        : _pageCache[page] = page switch
        {
            "home" => new HomePage(),
            "teams" => new TeamsPage(),
            "themes" => new ThemePacksPage(),
            "toolbox" => new ToolboxPage(),
            "resources" => new ResourcesPage(),
            "help" => new HelpPage(),
            "settings" => new SettingsPage(),
            _ => new HomePage(),
        };

    /* ==================== Toast ==================== */

    public static MainWindow? Current => App.Current?.ApplicationLifetime switch
    {
        global::Avalonia.Controls.ApplicationLifetimes.IClassicDesktopStyleApplicationLifetime d => d.MainWindow as MainWindow,
        _ => null,
    };

    public void ShowToast(string message, string type = "success")
    {
        ToastText.Text = message;
        ToastIcon.Text = type switch
        {
            "warning" => "⚠️",
            "error" => "❌",
            "info" => "💡",
            _ => "✅",
        };
        ToastHost.IsVisible = true;
        _toastTimer?.Stop();
        _toastTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(2.6) };
        _toastTimer.Tick += (_, _) =>
        {
            _toastTimer?.Stop();
            ToastHost.IsVisible = false;
        };
        _toastTimer.Start();
    }

    public static void Toast(string message, string type = "success") => Current?.ShowToast(message, type);
}
