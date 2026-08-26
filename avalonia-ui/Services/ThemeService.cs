using System;
using System.Collections.Generic;
using Avalonia;
using Avalonia.Media;
using Avalonia.Styling;

namespace AhabAssistant.Avalonia.Services;

public record AccentPreset(string Id, string Name, string LightBrand, string LightHover, string DarkBrand, string DarkHover);

/// <summary>
/// 主题系统（对齐 ui/src/themes/index.ts）：明暗模式 + 强调色预设。
/// 通过覆盖 Application 动态资源实现运行时切换。
/// </summary>
public static class ThemeService
{
    public static readonly List<AccentPreset> AccentPresets = new()
    {
        new("crimson", "赤红", "#c8354f", "#a92b42", "#e05a72", "#c8354f"),
        new("blue", "深蓝", "#2563eb", "#1d4ed8", "#60a5fa", "#3b82f6"),
        new("amber", "琥珀", "#d97706", "#b45309", "#fbbf24", "#f59e0b"),
        new("emerald", "翠绿", "#059669", "#047857", "#34d399", "#10b981"),
        new("violet", "紫罗兰", "#7c3aed", "#6d28d9", "#a78bfa", "#8b5cf6"),
    };

    public const string DefaultAccent = "crimson";

    private static readonly Dictionary<string, (string Bg, string Fg, string Card, string Muted, string MutedFg,
        string Secondary, string SecondaryFg, string InputBorder, string Destructive,
        string Success, string SuccessLight, string Warning, string WarningLight)> Palettes = new()
    {
        ["light"] = ("#F5F4F4", "#232223", "#FFFFFF", "#F4F3F3", "#8A8A8A",
            "#ECEBEA", "#232223", "#DDDCDC", "#DC2626",
            "#16A34A", "#DCF5E5", "#D97706", "#FBF0DA"),
        ["dark"] = ("#1B1918", "#FAFAFA", "#272525", "#353332", "#ABABAB",
            "#353332", "#FAFAFA", "#454342", "#F25555",
            "#46D17F", "#25332C", "#FBBF24", "#33302A"),
    };

    /// <summary>应用主题到全局资源。</summary>
    public static void ApplyTheme(string mode, string accentId)
    {
        var resolved = mode == "system" ? ResolveSystemMode() : mode;
        var palette = Palettes[resolved == "dark" ? "dark" : "light"];
        var accent = AccentPresets.Find(a => a.Id == accentId) ?? AccentPresets[0];
        var brand = resolved == "dark" ? accent.DarkBrand : accent.LightBrand;
        var brandHover = resolved == "dark" ? accent.DarkHover : accent.LightHover;

        var res = Application.Current!.Resources;
        void Set(string key, string value) => res[key] = new SolidColorBrush(Color.Parse(value));

        Set("BgBrush", palette.Bg);
        Set("CardBrush", palette.Card);
        Set("FgBrush", palette.Fg);
        Set("MutedBrush", palette.Muted);
        Set("MutedFgBrush", palette.MutedFg);
        Set("SecondaryBrush", palette.Secondary);
        Set("SecondaryFgBrush", palette.SecondaryFg);
        Set("InputBorderBrush", palette.InputBorder);
        Set("DestructiveBrush", palette.Destructive);
        Set("SuccessBrush", palette.Success);
        Set("SuccessLightBrush", palette.SuccessLight);
        Set("WarningBrush", palette.Warning);
        Set("WarningLightBrush", palette.WarningLight);
        Set("BrandBrush", brand);
        Set("BrandHoverBrush", brandHover);
        Set("BrandFgBrush", "#FFFFFF");
        Set("TabBarBrush", resolved == "dark" ? "#A6272525" : "#A6FFFFFF");

        if (Application.Current.ActualThemeVariant != (resolved == "dark" ? ThemeVariant.Dark : ThemeVariant.Light))
            Application.Current.RequestedThemeVariant = resolved == "dark" ? ThemeVariant.Dark : ThemeVariant.Light;
    }

    public static string ResolveSystemMode()
    {
        // Avalonia 平台亮暗检测
        var variant = Application.Current?.ActualThemeVariant;
        return variant == ThemeVariant.Dark ? "dark" : "light";
    }
}
