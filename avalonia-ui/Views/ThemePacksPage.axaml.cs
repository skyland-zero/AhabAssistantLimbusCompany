using AhabAssistant.Avalonia.Services;
using AhabAssistant.Avalonia.ViewModels;
using Avalonia.Controls;

namespace AhabAssistant.Avalonia.Views;

public partial class ThemePacksPage : UserControl
{
    public ThemePacksPage()
    {
        InitializeComponent();
        Localization.ApplyStatic(this);
        DataContext = new ThemePacksViewModel();
    }
}
