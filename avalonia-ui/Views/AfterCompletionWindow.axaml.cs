using System.Linq;
using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Controls;
using AhabAssistant.Avalonia.Services;
using Avalonia.Input;
using Avalonia.Interactivity;

namespace AhabAssistant.Avalonia.Views;

public partial class AfterCompletionWindow : MotionWindow
{
    private readonly AfterCompletionConfig _original;
    private bool _closeRequested;
    public bool Saved { get; private set; }
    public AfterCompletionConfig? Result { get; private set; }

    private static readonly string[] PowerKeys = { "none", "sleep", "hibernate", "lock", "shutdown" };

    // 保留无参构造函数，供 Avalonia 设计器和运行时 XAML loader 识别。
    public AfterCompletionWindow() : this(new AfterCompletionConfig()) { }

    public AfterCompletionWindow(AfterCompletionConfig config)
    {
        InitializeComponent();
        Localization.ApplyStatic(this);
        _original = config;

        SwExitGame.IsChecked = config.Actions.Contains("exit_game");
        SwExitEmulator.IsChecked = config.Actions.Contains("exit_emulator");
        SwExitAalc.IsChecked = config.Actions.Contains("exit_aalc");
        PowerCombo.SelectedIndex = System.Math.Max(0, PowerKeys.ToList().IndexOf(config.PowerAction));
    }

    private AfterCompletionConfig BuildResult(bool keep)
    {
        var actions = new System.Collections.Generic.List<string>();
        if (SwExitGame.IsChecked == true) actions.Add("exit_game");
        if (SwExitEmulator.IsChecked == true) actions.Add("exit_emulator");
        if (SwExitAalc.IsChecked == true) actions.Add("exit_aalc");
        return new AfterCompletionConfig
        {
            Actions = actions,
            PowerAction = PowerKeys[System.Math.Max(0, PowerCombo.SelectedIndex)],
            KeepAfterCompletion = keep,
        };
    }

    private async Task CloseWithAsync(bool keep)
    {
        if (_closeRequested) return;
        Result = BuildResult(keep);
        Saved = true;
        _closeRequested = true;
        await RequestCloseAsync(true);
    }

    private async void OnApplyOnce(object? sender, RoutedEventArgs e) => await CloseWithAsync(false);

    private async void OnSaveDefault(object? sender, RoutedEventArgs e) => await CloseWithAsync(true);

    private async void OnCancel(object? sender, RoutedEventArgs e) => await CloseWithAnimationAsync(false);

    private async void OnKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key == Key.Escape)
        {
            e.Handled = true;
            await CloseWithAnimationAsync(false);
        }
    }

    private async Task CloseWithAnimationAsync(bool dialogResult)
    {
        if (_closeRequested) return;
        _closeRequested = true;
        await RequestCloseAsync(dialogResult);
    }
}
