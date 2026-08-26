using System;
using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Services;
using AhabAssistant.Avalonia.ViewModels;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Threading;

namespace AhabAssistant.Avalonia.Views;

public partial class TeamsPage : UserControl
{
    private TeamsViewModel Vm => (TeamsViewModel)DataContext!;

    public TeamsPage()
    {
        InitializeComponent();
        Localization.ApplyStatic(this);
        DataContext = new TeamsViewModel();
        Vm.RequestEditModal += OpenEditModal;
        Vm.PropertyChanged += OnViewModelPropertyChanged;
        LayoutUpdated += OnLayoutUpdated;
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
        if (owner != null) await win.ShowDialog<bool>(owner);
        else win.Show();

        if (win.Saved && win.Result != null) Vm.SaveTeam(win.Result);
    }
}
