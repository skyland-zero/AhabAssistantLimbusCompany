using Avalonia;
using Avalonia.Controls;
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
        AvaloniaProperty.Register<AppIcon, IBrush?>(nameof(Stroke));

    public static readonly StyledProperty<double> StrokeThicknessProperty =
        AvaloniaProperty.Register<AppIcon, double>(nameof(StrokeThickness), 2);

    private Geometry? _iconGeometry;

    public static readonly DirectProperty<AppIcon, Geometry?> IconGeometryProperty =
        AvaloniaProperty.RegisterDirect<AppIcon, Geometry?>(nameof(IconGeometry), icon => icon.IconGeometry);

    static AppIcon()
    {
        IconProperty.Changed.AddClassHandler<AppIcon>((icon, _) => icon.UpdateGeometry());
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
        get => GetValue(StrokeProperty);
        set => SetValue(StrokeProperty, value);
    }

    public double StrokeThickness
    {
        get => GetValue(StrokeThicknessProperty);
        set => SetValue(StrokeThicknessProperty, value);
    }

    public Geometry? IconGeometry
    {
        get => _iconGeometry;
        private set => SetAndRaise(IconGeometryProperty, ref _iconGeometry, value);
    }

    private void UpdateGeometry() => IconGeometry = IconCatalog.Get(Icon);
}
