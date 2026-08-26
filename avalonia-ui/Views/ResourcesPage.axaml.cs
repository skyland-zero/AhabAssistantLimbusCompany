using AhabAssistant.Avalonia.Services;
using AhabAssistant.Avalonia.ViewModels;
using Avalonia.Controls;

namespace AhabAssistant.Avalonia.Views;

public partial class ResourcesPage : UserControl
{
    public ResourcesPage()
    {
        InitializeComponent();
        Localization.ApplyStatic(this);
        DataContext = new ResourcesViewModel();
    }
}
