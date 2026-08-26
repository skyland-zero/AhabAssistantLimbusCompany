using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Services;
using AhabAssistant.Avalonia.ViewModels;
using Avalonia.Controls;
using Avalonia.Interactivity;

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
    }

    private void TabClick(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string tab }) Vm.SetTabCommand.Execute(tab);
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
