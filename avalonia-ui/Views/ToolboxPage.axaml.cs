using AhabAssistant.Avalonia.Services;
using AhabAssistant.Avalonia.ViewModels;
using Avalonia.Controls;

namespace AhabAssistant.Avalonia.Views;

public partial class ToolboxPage : UserControl
{
    public ToolboxPage()
    {
        InitializeComponent();
        Localization.ApplyStatic(this);
        DataContext = new ToolboxViewModel();
    }
}
