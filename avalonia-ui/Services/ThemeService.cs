using System;
using System.Collections.Generic;
using Avalonia;
using Avalonia.Media;
using Avalonia.Styling;

namespace AhabAssistant.Avalonia.Services;

public record AccentPreset(
    string Id,
    string Name,
    string LightBrand,
    string LightHover,
    string LightBrandLight,
    string DarkBrand,
    string DarkHover,
    string DarkBrandLight);

/// <summary>
/// 主题系统（对齐 ui/src/themes/index.ts）：明暗模式 + 强调色预设。
/// 通过覆盖 Application 动态资源实现运行时切换。
/// </summary>
public static class ThemeService
{
    public static readonly List<AccentPreset> AccentPresets = new()
    {
        new("crimson", "赤红", "#c8354f", "#a92b42", "#fbe9ec", "#e05a72", "#c8354f", "#3a1e24"),
        new("blue", "深蓝", "#2563eb", "#1d4ed8", "#dbeafe", "#60a5fa", "#3b82f6", "#1e293b"),
        new("amber", "琥珀", "#d97706", "#b45309", "#fef3c7", "#fbbf24", "#f59e0b", "#3b2f14"),
        new("emerald", "翠绿", "#059669", "#047857", "#d1fae5", "#34d399", "#10b981", "#14332a"),
        new("violet", "紫罗兰", "#7c3aed", "#6d28d9", "#ede9fe", "#a78bfa", "#8b5cf6", "#2b2247"),
    };

    public const string DefaultAccent = "crimson";

    private static readonly Dictionary<string, (
        string Bg,
        string Fg,
        string Card,
        string CardFg,
        string Popover,
        string PopoverFg,
        string Primary,
        string PrimaryFg,
        string Secondary,
        string SecondaryFg,
        string Muted,
        string MutedFg,
        string Accent,
        string AccentFg,
        string Border,
        string Input,
        string InputBorder,
        string Ring,
        string Destructive,
        string DestructiveFg,
        string Success,
        string SuccessLight,
        string Warning,
        string WarningLight)> Palettes = new()
    {
        ["light"] = (
            "#F5F2F2", "#0A0A0A", "#FFFFFF", "#0A0A0A", "#FFFFFF", "#0A0A0A",
            "#171717", "#FAFAFA", "#F5F5F5", "#171717", "#F5F5F5", "#737373",
            "#F5F5F5", "#171717", "#00000000", "#00000000", "#D9D9D9", "#A1A1A1",
            "#E7000B", "#FFFFFF", "#18A349", "#DCFCE6", "#D6791D", "#FFF4CB"),
        ["dark"] = (
            "#0A0707", "#FAFAFA", "#181515", "#FAFAFA", "#262626", "#FAFAFA",
            "#E5E5E5", "#171717", "#262626", "#FAFAFA", "#262626", "#A1A1A1",
            "#404040", "#FAFAFA", "#00000000", "#0BFFFFFF", "#26FFFFFF", "#737373",
            "#FF6467", "#FFFFFF", "#4CDB7C", "#0C341E", "#EAB312", "#432A08"),
    };

    /// <summary>应用主题到全局资源。</summary>
    public static void ApplyTheme(string mode, string accentId)
    {
        var resolved = mode == "system" ? ResolveSystemMode() : mode;
        var palette = Palettes[resolved == "dark" ? "dark" : "light"];
        var accent = AccentPresets.Find(a => a.Id == accentId) ?? AccentPresets[0];
        var brand = resolved == "dark" ? accent.DarkBrand : accent.LightBrand;
        var brandHover = resolved == "dark" ? accent.DarkHover : accent.LightHover;
        var brandLight = resolved == "dark" ? accent.DarkBrandLight : accent.LightBrandLight;

        var res = Application.Current!.Resources;
        void Set(string key, string value) => res[key] = new SolidColorBrush(Color.Parse(value));

        Set("BgBrush", palette.Bg);
        Set("CardBrush", palette.Card);
        Set("CardFgBrush", palette.CardFg);
        Set("PopoverBrush", palette.Popover);
        Set("PopoverFgBrush", palette.PopoverFg);
        Set("FgBrush", palette.Fg);
        Set("PrimaryBrush", palette.Primary);
        Set("PrimaryFgBrush", palette.PrimaryFg);
        Set("MutedBrush", palette.Muted);
        Set("MutedFgBrush", palette.MutedFg);
        Set("SecondaryBrush", palette.Secondary);
        Set("SecondaryFgBrush", palette.SecondaryFg);
        Set("AccentBrush", palette.Accent);
        Set("AccentFgBrush", palette.AccentFg);
        Set("BorderBrush", palette.Border);
        Set("InputBrush", palette.Input);
        Set("InputBorderBrush", palette.InputBorder);
        Set("RingBrush", palette.Ring);
        Set("DestructiveBrush", palette.Destructive);
        Set("DestructiveFgBrush", palette.DestructiveFg);
        Set("SuccessBrush", palette.Success);
        Set("SuccessLightBrush", palette.SuccessLight);
        Set("WarningBrush", palette.Warning);
        Set("WarningLightBrush", palette.WarningLight);
        Set("SubtleSurfaceBrush", resolved == "dark" ? "#1F262626" : "#33F5F5F5");
        Set("SubtleBorderBrush", resolved == "dark" ? "#40737373" : "#40D9D9D9");
        Set("BrandBrush", brand);
        Set("BrandHoverBrush", brandHover);
        Set("BrandLightBrush", brandLight);
        Set("BrandSurfaceBrush", brandLight);
        Set("DestructiveLightBrush", resolved == "dark" ? "#3B1518" : "#FEE2E2");
        Set("BrandFgBrush", "#FFFFFF");
        Set("TabBarBrush", resolved == "dark" ? "#4D181515" : "#4DFFFFFF");

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
