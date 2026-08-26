using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Documents;
using Avalonia.Media;

namespace AhabAssistant.Avalonia.Controls;

/// <summary>
/// 项目内置的线性图标控件。图标路径来自 Lucide SVG 的 path 数据，运行时不依赖图标组件库。
/// </summary>
public partial class AppIcon : UserControl
{
    public static readonly StyledProperty<string> IconProperty =
        AvaloniaProperty.Register<AppIcon, string>(nameof(Icon), string.Empty);

    public static readonly StyledProperty<IBrush?> StrokeProperty =
        AvaloniaProperty.Register<AppIcon, IBrush?>(nameof(Stroke), inherits: true);

    public static readonly StyledProperty<double> StrokeThicknessProperty =
        AvaloniaProperty.Register<AppIcon, double>(nameof(StrokeThickness), 1.5);

    public static readonly StyledProperty<Geometry?> IconGeometryProperty =
        AvaloniaProperty.Register<AppIcon, Geometry?>(nameof(IconGeometry));

    static AppIcon()
    {
        IconProperty.Changed.AddClassHandler<AppIcon>((icon, _) => icon.UpdateGeometry());
        TextElement.ForegroundProperty.Changed.AddClassHandler<AppIcon>((icon, _) =>
        {
            if (!icon.IsSet(StrokeProperty))
            {
                icon.SetCurrentValue(StrokeProperty, icon.GetValue(TextElement.ForegroundProperty));
            }
        });
    }

    public AppIcon()
    {
        InitializeComponent();
        UpdateGeometry();
    }

    public string Icon
    {
        get => GetValue(IconProperty);
        set => SetValue(IconProperty, value);
    }

    public IBrush? Stroke
    {
        get => GetValue(StrokeProperty) ?? GetValue(TextElement.ForegroundProperty);
        set => SetValue(StrokeProperty, value);
    }

    public double StrokeThickness
    {
        get => GetValue(StrokeThicknessProperty);
        set => SetValue(StrokeThicknessProperty, value);
    }

    public Geometry? IconGeometry
    {
        get => GetValue(IconGeometryProperty);
        private set => SetValue(IconGeometryProperty, value);
    }

    private void UpdateGeometry() => IconGeometry = IconCatalog.Get(Icon);

    protected override void OnAttachedToVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnAttachedToVisualTree(e);
        if (!IsSet(StrokeProperty))
        {
            SetCurrentValue(StrokeProperty, GetValue(TextElement.ForegroundProperty));
        }
    }
}
