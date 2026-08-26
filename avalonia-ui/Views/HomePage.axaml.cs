using System;
using System.ComponentModel;
using System.Linq;
using AhabAssistant.Avalonia.Controls;
using AhabAssistant.Avalonia.Services;
using AhabAssistant.Avalonia.ViewModels;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Media;
using Avalonia.Threading;

namespace AhabAssistant.Avalonia.Views;

public partial class HomePage : UserControl
{
    private HomeViewModel Vm => (HomeViewModel)DataContext!;
    private HomeViewModel? _hookedVm;
    private int _statusPulseGeneration;

    public HomePage()
    {
        InitializeComponent();
        Localization.ApplyStatic(this);
        DataContextChanged += (_, _) => HookVm();
        DetachedFromVisualTree += (_, _) => { };
        DataContext = new HomeViewModel();
        AttachedToVisualTree += (_, _) =>
        {
            void DumpLayout()
            {
                Console.Error.WriteLine($"LAYOUT home={Bounds} root={HomeRoot.Bounds} left={LeftPanel.Bounds} splitter={Splitter.Bounds} right={RightPanel.Bounds} conn={RightConnection.Bounds} shot={RightScreenshot.Bounds} logs={RightLogs.Bounds} page={(Parent as Control)?.Bounds}");
            }
            Dispatcher.UIThread.Post(DumpLayout, DispatcherPriority.Loaded);
            Dispatcher.UIThread.Post(DumpLayout, DispatcherPriority.Background);
            Dispatcher.UIThread.Post(DumpLayout, DispatcherPriority.ApplicationIdle);
        };
    }

    private void HookVm()
    {
        if (ReferenceEquals(_hookedVm, DataContext))
        {
            UpdateChevrons();
            return;
        }

        if (_hookedVm is not null)
        {
            _hookedVm.RequestLogAutoScroll -= OnRequestLogAutoScroll;
            _hookedVm.RequestAfterCompletionModal -= OpenAfterCompletionWindow;
            _hookedVm.PropertyChanged -= OnVmPropertyChanged;
        }

        if (DataContext is not HomeViewModel vm) return;

        _hookedVm = vm;
        vm.RequestLogAutoScroll += OnRequestLogAutoScroll;
        vm.RequestAfterCompletionModal += OpenAfterCompletionWindow;
        vm.PropertyChanged += OnVmPropertyChanged;
        UpdateChevrons();
    }

    private void OnRequestLogAutoScroll()
    {
        if (Vm.Logs.Count > 0)
            LogsList.ScrollIntoView(Vm.Logs[^1]);
    }

    private void OnVmPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        switch (e.PropertyName)
        {
            case nameof(HomeViewModel.SetWindowsExpanded):
                UpdateChevron(SetWindowsChevron, Vm.SetWindowsExpanded);
                break;
            case nameof(HomeViewModel.DailyExpanded):
                UpdateChevron(DailyChevron, Vm.DailyExpanded);
                break;
            case nameof(HomeViewModel.RewardExpanded):
                UpdateChevron(RewardChevron, Vm.RewardExpanded);
                break;
            case nameof(HomeViewModel.EnkephalinExpanded):
                UpdateChevron(EnkephalinChevron, Vm.EnkephalinExpanded);
                break;
            case nameof(HomeViewModel.MirrorExpanded):
                UpdateChevron(MirrorChevron, Vm.MirrorExpanded);
                break;
            case nameof(HomeViewModel.ExecutionState):
            case nameof(HomeViewModel.StatusBadgeText):
                PulseStatusBadge();
                break;
        }
    }

    private void UpdateChevrons()
    {
        if (_hookedVm is not { } vm) return;
        UpdateChevron(SetWindowsChevron, vm.SetWindowsExpanded);
        UpdateChevron(DailyChevron, vm.DailyExpanded);
        UpdateChevron(RewardChevron, vm.RewardExpanded);
        UpdateChevron(EnkephalinChevron, vm.EnkephalinExpanded);
        UpdateChevron(MirrorChevron, vm.MirrorExpanded);
    }

    private static void UpdateChevron(AppIcon chevron, bool expanded)
    {
        if (chevron.RenderTransform is RotateTransform rotate)
            rotate.Angle = expanded ? 180 : 0;
    }

    private void PulseStatusBadge()
    {
        var generation = ++_statusPulseGeneration;
        StatusBadge.Opacity = 0.72;
        Dispatcher.UIThread.Post(() =>
        {
            if (generation == _statusPulseGeneration)
                StatusBadge.Opacity = 1;
        }, DispatcherPriority.Render);
    }

    /* ==================== 卡片展开/折叠 ==================== */

    private void StopBubble(object? sender, TappedEventArgs e) => e.Handled = true;

    private void ToggleSetWindows(object? sender, TappedEventArgs e) => Vm.SetWindowsExpanded = !Vm.SetWindowsExpanded;
    private void ToggleDaily(object? sender, TappedEventArgs e) => Vm.DailyExpanded = !Vm.DailyExpanded;
    private void ToggleReward(object? sender, TappedEventArgs e) => Vm.RewardExpanded = !Vm.RewardExpanded;
    private void ToggleEnkephalin(object? sender, TappedEventArgs e) => Vm.EnkephalinExpanded = !Vm.EnkephalinExpanded;
    private void ToggleMirror(object? sender, TappedEventArgs e) => Vm.MirrorExpanded = !Vm.MirrorExpanded;

    private void SetWindowsTabClick(object? sender, RoutedEventArgs e) =>
        Vm.SetWindowsTab = (string)((Button)sender!).Tag!;
    private void DailyTabClick(object? sender, RoutedEventArgs e) =>
        Vm.DailyTab = (string)((Button)sender!).Tag!;
    private void EnkephalinTabClick(object? sender, RoutedEventArgs e) =>
        Vm.EnkephalinTab = (string)((Button)sender!).Tag!;
    private void MirrorTabClick(object? sender, RoutedEventArgs e) =>
        Vm.MirrorTab = (string)((Button)sender!).Tag!;

    /// <summary>所有绑定到 Config 的开关变更后统一持久化并刷新预览。</summary>
    private void ConfigSwitchToggled(object? sender, RoutedEventArgs e)
    {
        // IsChecked 已通过双向绑定写回 Config
        Vm.NotifyConfigChanged();
    }

    /* ==================== 设备连接 ==================== */

    private void DeviceSelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (DeviceCombo.SelectedItem is Models.DeviceInfo device && sender is ComboBox)
            Vm.ConnectDeviceCommand.Execute(device);
    }

    /* ==================== 日志自动滚动 ==================== */

    private void LogsListSelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (sender is ListBox lb) lb.SelectedIndex = -1;
    }

    /* ==================== 右侧面板拖拽调宽（对齐 MXU） ==================== */

    private bool _dragging;

    private void SplitterPressed(object? sender, PointerPressedEventArgs e)
    {
        _dragging = true;
        ((Border)sender!).Cursor = new Cursor(StandardCursorType.SizeWestEast);
        e.Pointer.Capture(sender as IInputElement);
    }

    private void SplitterMoved(object? sender, PointerEventArgs e)
    {
        if (!_dragging) return;
        var x = e.GetPosition(this).X;
        var newWidth = Bounds.Width - x;
        if (newWidth < 160)
        {
            Vm.RightPanelCollapsed = true;
            return;
        }
        var maxWidth = Math.Min(800, Bounds.Width - 460 - 4);
        Vm.RightPanelWidth = Math.Max(240, Math.Min(maxWidth, newWidth));
        Vm.RightPanelCollapsed = false;
    }

    private void SplitterReleased(object? sender, PointerReleasedEventArgs e)
    {
        _dragging = false;
        e.Pointer.Capture(null);
    }

    /* ==================== 结束后操作弹窗 ==================== */

    private async void OpenAfterCompletionWindow()
    {
        var win = new AfterCompletionWindow(Vm.Config.AfterCompletion);
        var owner = global::Avalonia.Controls.Window.GetTopLevel(this) as global::Avalonia.Controls.Window;
        var mainOwner = owner as MainWindow;
        bool saved = false;
        if (owner != null)
        {
            mainOwner?.BeginModal();
            try
            {
                saved = await win.ShowDialog<bool>(owner);
            }
            finally
            {
                mainOwner?.EndModal();
            }
        }

        if (saved && win.Result != null)
        {
            Vm.Config.AfterCompletion = win.Result;
            Vm.NotifyConfigChanged();
        }
    }
}
