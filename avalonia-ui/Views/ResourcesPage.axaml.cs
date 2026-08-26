using System;
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
        LayoutUpdated += OnLayoutUpdated;
    }

    private void OnSizeChanged(object? sender, SizeChangedEventArgs e) => UpdateCardLayout();

    private void OnLayoutUpdated(object? sender, EventArgs e)
    {
        if (UpdateCardLayout()) LayoutUpdated -= OnLayoutUpdated;
    }

    private bool UpdateCardLayout()
    {
        if (ResourceItems.ItemsPanelRoot is not WrapPanel panel || panel.Bounds.Width <= 0)
            return false;

        var twoColumns = Bounds.Width >= 1024;
        panel.ItemWidth = twoColumns
            ? Math.Max(0, (panel.Bounds.Width - 24) / 2)
            : Math.Max(0, panel.Bounds.Width - 12);
        return true;
    }
}
