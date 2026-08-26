using System;
using System.Collections.Generic;
using AhabAssistant.Avalonia.Controls;
using AhabAssistant.Avalonia.Services;
using AhabAssistant.Avalonia.Views;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media;
using Avalonia.Threading;

namespace AhabAssistant.Avalonia;

public partial class MainWindow : MotionWindow
{
    private readonly Dictionary<string, Control> _pageCache = new();
    private string _currentPage = "home";
    private DispatcherTimer? _toastTimer;
    private DispatcherTimer? _navigationTimer;
    private int _toastGeneration;
    private int _modalDepth;
    private bool _isClosing;
    private bool _navigationLocked;
    private string? _pendingNavigationPage;
    private bool _pendingLanguageRefresh;

    private static readonly string[] NavigationPages =
    {
        "home", "teams", "themes", "toolbox", "resources", "help", "settings",
    };

    // MotionContentHost cancels an old animation when a new one starts. Keep
    // navigation calls serialized here so cached controls are never reparented
    // while the previous transition is still unwinding.
    private static readonly TimeSpan NavigationSettleDuration =
        TimeSpan.FromMilliseconds(UiMotion.StandardDuration.TotalMilliseconds + 40);

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
        ExtendClientAreaTitleBarHeightHint = 40;

        // F10 / F11 全局热键（窗口级）
        KeyDown += OnWindowKeyDown;

        _navigationTimer = new DispatcherTimer { Interval = NavigationSettleDuration };
        _navigationTimer.Tick += OnNavigationTimerTick;

        Navigate("home");
        AttachedToVisualTree += (_, _) => Localization.ApplyStatic(this);
        PositionChanged += (_, _) => App.SavePrefs();
        Closed += (_, _) =>
        {
            _toastTimer?.Stop();
            _toastTimer = null;
            StopNavigationQueue();
            if (_navigationTimer is not null)
            {
                _navigationTimer.Tick -= OnNavigationTimerTick;
                _navigationTimer = null;
            }
            App.LanguageChanged -= OnLanguageChanged;
        };
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
        MaxIcon.Icon = WindowState == WindowState.Maximized ? "copy" : "square";
    }

    private void OnClose(object? sender, RoutedEventArgs e)
    {
        if (_isClosing) return;
        _isClosing = true;
        App.LanguageChanged -= OnLanguageChanged;
        _toastTimer?.Stop();
        _toastTimer = null;
        StopNavigationQueue();
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
        ThemeIcon.Icon = App.ThemeMode switch
        {
            "system" => "monitor",
            "dark" => "moon",
            _ => "sun",
        };
    }

    /* ==================== 页面导航 ==================== */

    private void OnLanguageChanged()
    {
        if (!Dispatcher.UIThread.CheckAccess())
        {
            Dispatcher.UIThread.Post(OnLanguageChanged);
            return;
        }

        if (_isClosing) return;

        // 页面中的固定文案在创建时本地化；切换语言后重建当前页，确保
        // DataTemplate 和帮助正文也同步刷新。
        // 如果页面切换动画仍在进行，延迟清缓存，避免 TransitionTo 同时
        // 读取旧控件并把新控件挂载到同一个宿主。
        _pendingLanguageRefresh = true;
        if (_navigationLocked)
        {
            _pendingNavigationPage = _currentPage;
        }
        else
        {
            _pendingLanguageRefresh = false;
            _pageCache.Clear();
            NavigateNow(_currentPage, forceRefresh: true);
        }

        Localization.ApplyStatic(this);
    }

    private void OnNavClick(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string page }) Navigate(page);
    }

    public void Navigate(string page)
    {
        if (!Dispatcher.UIThread.CheckAccess())
        {
            Dispatcher.UIThread.Post(() => Navigate(page));
            return;
        }

        if (_isClosing) return;

        page = NormalizePage(page);
        if (_navigationLocked)
        {
            // 只保留最后一次点击；这能把连续点击压缩成一个稳定目标，
            // 同时保持页面缓存和 TransitionTo 的调用顺序。
            _pendingNavigationPage = page;
            return;
        }

        NavigateNow(page, forceRefresh: false);
    }

    private void NavigateNow(string page, bool forceRefresh)
    {
        if (_isClosing) return;

        page = NormalizePage(page);
        var previous = _currentPage;
        var isSamePage = string.Equals(previous, page, StringComparison.Ordinal);

        if (!forceRefresh && isSamePage && PageHost.CurrentContent is not null)
        {
            UpdateNavigationState(page);
            return;
        }

        if (forceRefresh)
        {
            _pageCache.Clear();
        }

        var direction = PageHost.CurrentContent is null || isSamePage
            ? MotionDirection.None
            : GetNavigationIndex(page) >= GetNavigationIndex(previous)
                ? MotionDirection.Forward
                : MotionDirection.Backward;

        var content = GetPage(page);
        try
        {
            // TransitionTo 必须成功后才提交 _currentPage；否则一次失败的
            // 导航不会让缓存状态与屏幕内容分离。
            PageHost.TransitionTo(content, direction);
        }
        catch (InvalidOperationException)
        {
            // 旧版本动画中断时，Avalonia 可能仍短暂保留缓存控件的 Parent。
            // 丢弃这个有问题的实例，清空宿主后用全新页面安全恢复；恢复
            // 路径关闭动画，避免同一帧再次触发相同的挂载冲突。
            if (!_pageCache.TryGetValue(page, out var cached) ||
                !ReferenceEquals(cached, content))
            {
                throw;
            }

            _pageCache.Remove(page);
            PageHost.TransitionTo(null, MotionDirection.None, animate: false);
            content = CreatePage(page);
            _pageCache[page] = content;
            PageHost.TransitionTo(content, MotionDirection.None, animate: false);
        }

        _currentPage = page;
        UpdateNavigationState(page);
        StartNavigationLock();
    }

    private void UpdateNavigationState(string page)
    {

        foreach (var name in new[] { "NavHome", "NavTeams", "NavThemes", "NavToolbox", "NavResources", "NavHelp" })
        {
            if (this.FindControl<Button>(name) is { } btn)
                btn.Classes.Set("active", (string?)btn.Tag == page);
        }
        SettingsBtn.Classes.Set("active", page == "settings");
    }

    private void StartNavigationLock()
    {
        if (!UiMotion.IsEnabled || _navigationTimer is null)
        {
            return;
        }

        _navigationLocked = true;
        _navigationTimer.Stop();
        _navigationTimer.Start();
    }

    private void OnNavigationTimerTick(object? sender, EventArgs e)
    {
        _navigationTimer?.Stop();
        _navigationLocked = false;

        if (_isClosing)
        {
            _pendingNavigationPage = null;
            _pendingLanguageRefresh = false;
            return;
        }

        var nextPage = _pendingNavigationPage;
        var refresh = _pendingLanguageRefresh;
        _pendingNavigationPage = null;
        _pendingLanguageRefresh = false;

        if (nextPage is null && !refresh)
        {
            return;
        }

        NavigateNow(nextPage ?? _currentPage, forceRefresh: refresh);
    }

    private void StopNavigationQueue()
    {
        _navigationTimer?.Stop();
        _navigationLocked = false;
        _pendingNavigationPage = null;
        _pendingLanguageRefresh = false;
    }

    private static string NormalizePage(string page) =>
        Array.IndexOf(NavigationPages, page) >= 0 ? page : "home";

    private static Control CreatePage(string page) => page switch
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

    private static int GetNavigationIndex(string page)
    {
        var index = Array.IndexOf(NavigationPages, page);
        return index < 0 ? 0 : index;
    }

    /* ==================== 模态遮罩 ==================== */

    /// <summary>
    /// Marks the main window as hosting an independent modal window. Nested callers
    /// are reference-counted so one dialog cannot remove another dialog's scrim.
    /// </summary>
    public void BeginModal()
    {
        _modalDepth++;
        ModalScrim.IsShown = true;
        ModalScrim.IsHitTestVisible = true;
    }

    /// <summary>Releases one modal owner and hides the scrim when the last one exits.</summary>
    public void EndModal()
    {
        if (_modalDepth > 0) _modalDepth--;
        if (_modalDepth != 0) return;

        ModalScrim.IsShown = false;
        ModalScrim.IsHitTestVisible = false;
    }

    private Control GetPage(string page) => _pageCache.TryGetValue(page, out var cached)
        ? cached
        : _pageCache[page] = CreatePage(page);

    /* ==================== Toast ==================== */

    public static MainWindow? Current => App.Current?.ApplicationLifetime switch
    {
        global::Avalonia.Controls.ApplicationLifetimes.IClassicDesktopStyleApplicationLifetime d => d.MainWindow as MainWindow,
        _ => null,
    };

    public void ShowToast(string message, string type = "success")
    {
        var generation = ++_toastGeneration;
        ToastText.Text = message;
        ToastIcon.Icon = type switch
        {
            "warning" => "triangle-alert",
            "error" => "circle-x",
            "info" => "lightbulb",
            _ => "circle-check",
        };
        ToastIcon.Stroke = type switch
        {
            "warning" => (IBrush)Application.Current!.Resources["WarningBrush"]!,
            "error" => (IBrush)Application.Current!.Resources["DestructiveBrush"]!,
            "info" => (IBrush)Application.Current!.Resources["BrandBrush"]!,
            _ => (IBrush)Application.Current!.Resources["SuccessBrush"]!,
        };
        // 更新已有 Toast 时保持其可见状态，避免连续通知造成闪烁。
        ToastHost.IsShown = true;
        _toastTimer?.Stop();
        var timer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(2.6) };
        timer.Tick += (_, _) =>
        {
            timer.Stop();
            // 每次 ShowToast 都有自己的代际；旧 timer 只能结束自己，不能
            // 提前关闭新 Toast 或停止新 timer。
            if (generation != _toastGeneration) return;
            if (ReferenceEquals(_toastTimer, timer)) _toastTimer = null;
            ToastHost.IsShown = false;
        };
        _toastTimer = timer;
        _toastTimer.Start();
    }

    public static void Toast(string message, string type = "success") => Current?.ShowToast(message, type);
}
