using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Documents;
using Avalonia.Layout;
using Avalonia.Media;
using AhabAssistant.Avalonia.Services;

namespace AhabAssistant.Avalonia.Views;

public partial class HelpPage : UserControl
{
    private record Heading(string Title, Control Anchor);

    private readonly List<Heading> _headings = new();

    public HelpPage()
    {
        InitializeComponent();
        Localization.ApplyStatic(this);
        RenderHelp();
        Localization.ApplyStatic(this);
    }

    private void RenderHelp()
    {
        var lang = App.Language;
        var source = ReadHelp(lang == "en-US" ? "help-en.md" : "help-zh.md");
        var lines = source.Replace("\r\n", "\n").Split('\n');

        var content = new StackPanel { Spacing = 0, MaxWidth = 660 };
        _headings.Clear();
        TocList.Items.Clear();

        foreach (var raw in lines)
        {
            var line = raw.TrimEnd();
            if (line.StartsWith("# ") && !line.StartsWith("## "))
                continue; // 隐藏 h1，与参考实现一致

            if (line.StartsWith("## "))
            {
                var title = line[3..].Trim();
                var heading = new Border
                {
                    BorderThickness = new Thickness(0, 0, 0, 1),
                    BorderBrush = new SolidColorBrush(Color.Parse("#40888888")),
                    Margin = new Thickness(0, 28, 0, 8),
                    Padding = new Thickness(0, 0, 0, 4),
                    Child = new TextBlock { Text = title, FontSize = 15, FontWeight = FontWeight.SemiBold },
                };
                content.Children.Add(heading);
                _headings.Add(new Heading(title, heading));
                AddTocItem(title, _headings.Count - 1);
                continue;
            }

            if (line.StartsWith("### "))
            {
                content.Children.Add(new TextBlock
                {
                    Text = line[4..].Trim(),
                    FontSize = 13,
                    FontWeight = FontWeight.SemiBold,
                    Margin = new Thickness(0, 18, 0, 5),
                });
                continue;
            }

            if (string.IsNullOrWhiteSpace(line))
                continue;

            // 列表项 / 普通段落（简单处理 **bold** 内联）
            bool ordered = System.Text.RegularExpressions.Regex.IsMatch(line, @"^\d+\.\s");
            if (line.StartsWith("- ") || ordered)
            {
                var text = ordered
                    ? System.Text.RegularExpressions.Regex.Replace(line, @"^(\d+)\.\s", "$1. ")
                    : "• " + line[2..];
                var p = BuildParagraph(text);
                p.Margin = new Thickness(0, 1, 0, 1);
                content.Children.Add(p);
            }
            else
            {
                var p = BuildParagraph(line);
                p.Margin = new Thickness(0, 7, 0, 7);
                content.Children.Add(p);
            }
        }

        HelpScroll.Content = content;
    }

    private static TextBlock BuildParagraph(string text)
    {
        var tb = new TextBlock { TextWrapping = TextWrapping.Wrap, FontSize = 12.5, LineHeight = 21 };
        // 处理 **bold** 和 `inline code` 行内样式
        var parts = System.Text.RegularExpressions.Regex.Split(text, @"(\*\*[^*]+\*\*|`[^`]+`)");
        foreach (var part in parts)
        {
            if (part.Length == 0) continue;
            if (part.StartsWith("**") && part.EndsWith("**") && part.Length > 4)
                tb.Inlines?.Add(new Run(part[2..^2]) { FontWeight = FontWeight.SemiBold });
            else if (part.StartsWith("`") && part.EndsWith("`") && part.Length > 2)
                tb.Inlines?.Add(new Run(part[1..^1]) { FontFamily = new FontFamily("Consolas, Cascadia Mono, monospace") });
            else
                tb.Inlines?.Add(new Run(part));
        }
        return tb;
    }

    private void AddTocItem(string title, int index)
    {
        var btn = new Button
        {
            Content = new TextBlock
            {
                Text = title,
                FontSize = 11.5,
                Foreground = (IBrush)global::Avalonia.Application.Current!.Resources["MutedFgBrush"]!,
                TextTrimming = TextTrimming.CharacterEllipsis,
            },
            HorizontalContentAlignment = HorizontalAlignment.Left,
            HorizontalAlignment = HorizontalAlignment.Stretch,
            Padding = new Thickness(8, 5),
            CornerRadius = new CornerRadius(6),
            Background = Brushes.Transparent,
            Tag = index,
        };
        btn.Click += (_, _) => JumpTo(index);
        TocList.Items.Add(btn);
    }

    private void JumpTo(int index)
    {
        if (index < 0 || index >= _headings.Count) return;
        var anchor = _headings[index].Anchor;
        var pos = anchor.TranslatePoint(default, HelpScroll);
        if (pos.HasValue)
            HelpOuter.Offset = new global::Avalonia.Vector(0, Math.Max(0, pos.Value.Y - 8));
    }

    private static string ReadHelp(string file)
    {
        try
        {
            using var stream = global::Avalonia.Platform.AssetLoader.Open(
                new Uri($"avares://AhabAssistant.Avalonia/Assets/{file}"));
            using var reader = new StreamReader(stream);
            return reader.ReadToEnd();
        }
        catch
        {
            return "";
        }
    }
}
