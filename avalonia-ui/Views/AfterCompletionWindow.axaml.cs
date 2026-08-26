using System.Linq;
using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Services;
using Avalonia.Controls;
using Avalonia.Interactivity;

namespace AhabAssistant.Avalonia.Views;

public partial class AfterCompletionWindow : Window
{
    private readonly AfterCompletionConfig _original;
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

    private void CloseWith(bool keep)
    {
        Result = BuildResult(keep);
        Saved = true;
        Close(true);
    }

    private void OnApplyOnce(object? sender, RoutedEventArgs e) => CloseWith(false);

    private void OnSaveDefault(object? sender, RoutedEventArgs e) => CloseWith(true);
}
