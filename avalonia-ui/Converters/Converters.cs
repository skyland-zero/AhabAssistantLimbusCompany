using System;
using System.Collections.ObjectModel;
using System.Globalization;
using System.Linq;
using Avalonia.Data.Converters;
using Avalonia.Media;

namespace AhabAssistant.Avalonia.Converters;

/// <summary>bool → IVisibility 转换（可选反转）。</summary>
public class BoolToVis : IValueConverter
{
    public bool Invert { get; set; }

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var b = value is true;
        if (Invert) b = !b;
        return b;
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

public class NullToVis : IValueConverter
{
    public bool Invert { get; set; }

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var notNull = value != null;
        if (value is string s) notNull = !string.IsNullOrEmpty(s);
        if (value is int i) notNull = i > 0;
        if (Invert) notNull = !notNull;
        return notNull;
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

/// <summary>PreviewTag.Highlight → 徽章底色。</summary>
public class HighlightBg : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var res = global::Avalonia.Application.Current!.Resources;
        return value is true ? res["BrandBrush"] : res["SecondaryBrush"];
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

/// <summary>PreviewTag.Highlight → 徽章前景色。</summary>
public class HighlightFg : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var res = global::Avalonia.Application.Current!.Resources;
        return value is true ? res["BrandFgBrush"] : res["MutedFgBrush"];
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

/// <summary>连接状态字符串 → 状态徽章颜色。</summary>
public class ConnectionStatusBrush : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var res = global::Avalonia.Application.Current!.Resources;
        return value switch
        {
            "connected" => res["SuccessLightBrush"],
            "connecting" => res["WarningLightBrush"],
            _ => res["SecondaryBrush"],
        };
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

public class LogLevelBrush : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var res = global::Avalonia.Application.Current!.Resources;
        return value switch
        {
            "error" => res["DestructiveBrush"],
            "warn" => res["WarningBrush"],
            _ => res["MutedFgBrush"],
        };
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

public class LogLevelIcon : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
        => value switch { "error" => "circle-x", "warn" => "triangle-alert", _ => "info" };

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

public class TaskExecutingBrush : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var current = value?.ToString();
        return string.Equals(current, parameter?.ToString(), StringComparison.Ordinal)
            ? global::Avalonia.Application.Current!.Resources["BrandBrush"]
            : Brushes.Transparent;
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

public class StringEqualsConv : IValueConverter
{
    public static readonly StringEqualsConv Default = new();

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
        => string.Equals(value?.ToString(), parameter?.ToString(), StringComparison.Ordinal);

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

/// <summary>MirrorProgressPayload → 进度百分比 (0~100)；无限模式返回 -1 表示满宽。</summary>
public class MirrorPercent : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        if (value is Models.MirrorProgressPayload p)
            return p.IsInfinite ? 10000.0 : Math.Min(100.0, p.Current * 100.0 / Math.Max(1, p.Total));
        return 0.0;
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

public class AccentPresetBrush : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        if (value is Services.AccentPreset preset)
            return new SolidColorBrush(Color.Parse(preset.LightBrand));
        return Brushes.Transparent;
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

/// <summary>体系 ID → 状态图标 Bitmap。</summary>
public class SchemeIcon : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        if (value is string id && !string.IsNullOrEmpty(id))
            return new global::Avalonia.Media.Imaging.Bitmap($"avares://AhabAssistant.Avalonia/Assets/status_effects/{id}.png");
        return null;
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}

/// <summary>bool → 1.0 / 0.6 透明度（停用队伍卡片）。</summary>
public class EnabledOpacity : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
        => value is true ? 1.0 : 0.6;

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => throw new NotSupportedException();
}
