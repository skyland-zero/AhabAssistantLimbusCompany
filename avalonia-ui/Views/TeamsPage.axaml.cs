using System;
using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Controls;
using AhabAssistant.Avalonia.Services;
using AhabAssistant.Avalonia.ViewModels;
using Avalonia.Controls;
using Avalonia.Controls.Presenters;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Threading;

namespace AhabAssistant.Avalonia.Views;

public partial class TeamsPage : UserControl
{
    private const int MaxStaggeredCards = 8;
    private TeamsViewModel Vm => (TeamsViewModel)DataContext!;

    public TeamsPage()
    {
        InitializeComponent();
        Localization.ApplyStatic(this);
        DataContext = new TeamsViewModel();
        Vm.RequestEditModal += OpenEditModal;
        Vm.PropertyChanged += OnViewModelPropertyChanged;
        TeamItems.ContainerPrepared += OnTeamContainerPrepared;
        LayoutUpdated += OnLayoutUpdated;
    }

    private static void OnTeamContainerPrepared(object? sender, ContainerPreparedEventArgs e)
    {
        // A filter/reload can materialize a large collection in one pass. Keep
        // the first eight cards eligible for the entrance motion and make all
        // later/recycled cards settle immediately.
        if (e.Container is ContentPresenter { Content: MotionVisibility motion })
            motion.Duration = e.Index < MaxStaggeredCards
                ? UiMotion.StandardDuration
                : TimeSpan.Zero;
    }

    private void OnViewModelPropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(TeamsViewModel.DeleteTarget) && Vm.DeleteTarget != null)
            Dispatcher.UIThread.Post(() => DeleteOverlay.Focus());
    }

    private void OnSizeChanged(object? sender, SizeChangedEventArgs e) => UpdateCardLayout();

    private void OnLayoutUpdated(object? sender, EventArgs e)
    {
        if (UpdateCardLayout()) LayoutUpdated -= OnLayoutUpdated;
    }

    private bool UpdateCardLayout()
    {
        if (TeamItems.ItemsPanelRoot is not WrapPanel panel || panel.Bounds.Width <= 0)
            return false;

        var twoColumns = Bounds.Width >= 1280;
        panel.ItemWidth = twoColumns
            ? Math.Max(280, (panel.Bounds.Width - 16) / 2)
            : Math.Max(280, panel.Bounds.Width - 8);
        return true;
    }

    private void TabClick(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string tab }) Vm.SetTabCommand.Execute(tab);
    }

    private void OnDeleteOverlayPointerPressed(object? sender, PointerPressedEventArgs e)
    {
        if (ReferenceEquals(e.Source, sender)) Vm.CancelDeleteCommand.Execute(null);
    }

    private void OnDeleteOverlayKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key == Key.Escape)
        {
            e.Handled = true;
            Vm.CancelDeleteCommand.Execute(null);
        }
    }

    private async void OpenEditModal(TeamDetail? team)
    {
        var win = new TeamEditWindow(team, Vm);
        var owner = Window.GetTopLevel(this) as Window;
        var mainOwner = owner as MainWindow;

        // 编辑窗口使用真正的模态对话框；主窗口遮罩的生命周期必须覆盖
        // ShowDialog（包括窗口的退出动画），避免窗口关闭后遮罩残留。
        mainOwner?.BeginModal();
        try
        {
            if (owner != null) await win.ShowDialog<bool>(owner);
            else win.Show();
        }
        finally
        {
            mainOwner?.EndModal();
        }

        if (win.Saved && win.Result != null) Vm.SaveTeam(win.Result);
    }
}
