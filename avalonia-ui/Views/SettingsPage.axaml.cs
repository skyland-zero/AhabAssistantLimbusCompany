using System;
using AhabAssistant.Avalonia.Services;
using AhabAssistant.Avalonia.ViewModels;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;

namespace AhabAssistant.Avalonia.Views;

public partial class SettingsPage : UserControl
{
    private SettingsViewModel Vm => (SettingsViewModel)DataContext!;
    private bool _capturing;

    public SettingsPage()
    {
        InitializeComponent();
        Localization.ApplyStatic(this);
        DataContextChanged += (_, _) => InitCombo();
        PointerPressed += (_, _) =>
        {
            if (_capturing)
            {
                _capturing = false;
                Vm.CapturingTarget = null;
            }
        };
        KeyDown += OnCaptureKeyDown;
        DataContext = new SettingsViewModel();
    }

    private void InitCombo()
    {
        UpdateSourceCombo.SelectedIndex = Vm.Sys.UpdateSource == "MirrorChyan" ? 1 : 0;
    }

    /* ==================== 热键捕获 ==================== */

    public void StartHotkeyCapture(string target)
    {
        _capturing = true;
        Vm.CapturingTarget = target;
        Focus();
    }

    private void CaptureStartStopTapped(object? sender, TappedEventArgs e)
    {
        e.Handled = true;
        StartHotkeyCapture("startStop");
    }

    private void CapturePauseResumeTapped(object? sender, TappedEventArgs e)
    {
        e.Handled = true;
        StartHotkeyCapture("pauseResume");
    }

    private void OnCaptureKeyDown(object? sender, KeyEventArgs e)
    {
        if (!_capturing) return;
        e.Handled = true;
        var combo = MainWindow.FormatCombo(e);
        if (combo.Length == 0) return; // 单独按修饰键不生效
        Vm.SetHotkey(Vm.CapturingTarget ?? "startStop", combo);
        _capturing = false;
    }

    private void ClearStartStop(object? sender, RoutedEventArgs e) => Vm.SetHotkey("startStop", "");
    private void ClearPauseResume(object? sender, RoutedEventArgs e) => Vm.SetHotkey("pauseResume", "");

    private void HotkeySwitchToggled(object? sender, RoutedEventArgs e) => Vm.SaveHotkey();

    /* ==================== 系统设置 ==================== */

    private void SysSwitchToggled(object? sender, RoutedEventArgs e) => Vm.SaveSysSettings();

    private void SimTypeChanged(object? sender, SelectionChangedEventArgs e) => Vm.SaveSysSettings();

    private void PortLostFocus(object? sender, RoutedEventArgs e)
    {
        if (int.TryParse((sender as TextBox)?.Text, out var n))
            Vm.Sys.SimulatorPort = Math.Clamp(n, 0, 65535);
        else
            Vm.Sys.SimulatorPort = 16384;
        Vm.SaveSysSettings();
        if (sender is TextBox tb) tb.Text = Vm.Sys.SimulatorPort.ToString();
    }

    private void TimeoutLostFocus(object? sender, RoutedEventArgs e)
    {
        if (int.TryParse((sender as TextBox)?.Text, out var n))
            Vm.Sys.StartEmulatorTimeout = Math.Max(10, n);
        else
            Vm.Sys.StartEmulatorTimeout = 60;
        Vm.SaveSysSettings();
        if (sender is TextBox tb) tb.Text = Vm.Sys.StartEmulatorTimeout.ToString();
    }

    private void UpdateSourceChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (!IsInitialized) return;
        Vm.Sys.UpdateSource = UpdateSourceCombo.SelectedIndex == 1 ? "MirrorChyan" : "GitHub";
        Vm.SaveSysSettings();
    }

    private void CdkLostFocus(object? sender, RoutedEventArgs e) => Vm.SaveSysSettings();
}
