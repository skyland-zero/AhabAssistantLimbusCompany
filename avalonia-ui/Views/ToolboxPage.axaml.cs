using System;
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
        LayoutUpdated += OnLayoutUpdated;
    }

    private void OnSizeChanged(object? sender, SizeChangedEventArgs e) => UpdateCardLayout();

    private void OnLayoutUpdated(object? sender, EventArgs e)
    {
        if (UpdateCardLayout()) LayoutUpdated -= OnLayoutUpdated;
    }

    private bool UpdateCardLayout()
    {
        if (ToolItems.ItemsPanelRoot is not WrapPanel panel || panel.Bounds.Width <= 0)
            return false;

        var columns = Bounds.Width >= 1024 ? 3 : Bounds.Width >= 768 ? 2 : 1;
        panel.ItemWidth = Math.Max(0, (panel.Bounds.Width - columns * 12) / columns);
        return true;
    }
}
