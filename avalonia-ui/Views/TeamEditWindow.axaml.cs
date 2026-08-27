using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using AhabAssistant.Avalonia.Controls;
using AhabAssistant.Avalonia.Models;
using AhabAssistant.Avalonia.Services;
using AhabAssistant.Avalonia.ViewModels;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Input;
using global::Avalonia.Input.Platform;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using global::Avalonia.Media.Imaging;

namespace AhabAssistant.Avalonia.Views;

public partial class TeamEditWindow : MotionWindow
{
    private readonly TeamDetail _team;          // 工作副本
    private TeamMirrorConfig Mc => _team.MirrorConfig!;
    private readonly List<SinnerInfo> _sinners;
    private int _activeTabIndex = -1;
    private bool _closeRequested;
    public bool Saved { get; private set; }
    public TeamDetail? Result { get; private set; }

    private static readonly (string Id, string Label)[] Systems =
    {
        ("burn", "烧伤"), ("bleed", "流血"), ("tremor", "震颤"), ("rupture", "破裂"), ("sinking", "沉沦"),
        ("poise", "呼吸"), ("charge", "充能"), ("slash", "斩击"), ("pierce", "突刺"), ("blunt", "打击"),
    };

    private static readonly (int Cost, string Zh, string En, string Desc)[] StarlightItems =
    {
        (10, "初始之星", "Star of the Beginning", "初始经费增加，卡包/饰品展出+1，免费普通刷新"),
        (10, "积聚的星云", "Cumulating Starcloud", "进阶经费利息+10%~30%，售卖饰品经费加成"),
        (20, "星际漫游", "Interstellar Travel", "卡包出现+1，卡包刷新+2~4，未记录卡包等级提升"),
        (20, "流星", "Star-shower", "初始经费+400~700，初始饰品可选择数+1"),
        (30, "双星商店", "Binary Star-shop", "展出饰品+1，战斗经费+20%~40%，高阶饰品概率提升"),
        (30, "卫星商店", "Moon Star-shop", "免费关键词刷新，进入第1层送1~3件1级饰品"),
        (40, "星云的宠爱", "Favor of the Starcloud", "进入第1层人格等级+3，通关阶段人格等级提升"),
        (40, "星芒的引导", "Guidance of the Starlight", "最大速度+2~3，拼点威力/伤害强化/守护提升"),
        (50, "偶然的彗星", "Accidental Comet", "进商店赠送合成/售卖专用饰品，赠送对应关键词3/4级饰品"),
        (60, "全面的可能性", "All-round Possibility", "开局自选3级饰品，获得残影饰品"),
    };

    // 保留无参构造函数，供 Avalonia 设计器和运行时 XAML loader 识别。
    public TeamEditWindow() : this(null, null) { }

    public TeamEditWindow(TeamDetail? team, TeamsViewModel? vm)
    {
        InitializeComponent();
        Localization.ApplyStatic(this);
        _sinners = MockBackend.Instance.Sinners;

        _team = team == null || string.IsNullOrEmpty(team.Id)
            ? new TeamDetail { MirrorConfig = TeamMirrorConfig.CreateDefault() }
            : Clone(team);

        if (_team.MirrorConfig == null) _team.MirrorConfig = TeamMirrorConfig.CreateDefault();

        TitleText.Text = Localization.T(string.IsNullOrEmpty(_team.Id) ? "新建队伍" : "编辑队伍");
        BuildTabs();
        SelectTab(TabBasic, animate: false);
        AttachedToVisualTree += (_, _) => Localization.ApplyStatic(this);
    }

    private static TeamDetail Clone(TeamDetail t) => JsonSerializer.Deserialize(
        JsonSerializer.Serialize(t, Models.AalcJsonContext.Default.TeamDetail),
        Models.AalcJsonContext.Default.TeamDetail)!;

    /* ==================== UI 构建辅助 ==================== */

    private static TextBlock Label(string text, double size = 12, IBrush? fg = null, FontWeight weight = FontWeight.Normal)
        => new() { Text = text, FontSize = size, Foreground = fg ?? (IBrush)global::Avalonia.Application.Current!.Resources["FgBrush"]!, FontWeight = weight };

    private static TextBlock FieldLabel(string text, double size = 12)
        => Label(text, size, weight: FontWeight.Medium);

    private static TextBlock SubsectionTitle(string text, double size = 12)
        => Label(text, size, weight: FontWeight.SemiBold);

    private static TextBlock Muted(string text, double size = 11)
        => new() { Text = text, FontSize = size, Foreground = (IBrush)global::Avalonia.Application.Current!.Resources["MutedFgBrush"]!, TextWrapping = TextWrapping.Wrap };

    private static Border Section(params Control[] children)
    {
        var sp = new StackPanel { Spacing = 10 };
        foreach (var c in children) sp.Children.Add(c);
        return new Border
        {
            Classes = { "card" },
            Padding = new Thickness(14),
            Margin = new Thickness(0, 0, 0, 14),
            Child = sp,
        };
    }

    private static ToggleSwitch MakeSwitch(bool initial, Action<bool> onChanged)
    {
        var sw = new ToggleSwitch { Classes = { "compact" }, IsChecked = initial };
        sw.IsCheckedChanged += (_, _) => onChanged(sw.IsChecked == true);
        return sw;
    }

    private static Grid Row(Control left, Control right)
    {
        Grid.SetColumn(right, 1);
        return new Grid
        {
            ColumnDefinitions = ColumnDefinitions.Parse("*,Auto"),
            Children = { left, right },
        };
    }
    
    private static void SetCol(global::Avalonia.Controls.Panel g, int col, global::Avalonia.Controls.Control c)
    {
        Grid.SetColumn(c, col);
        g.Children.Add(c);
    }

    private ComboBox MakeCombo(string[] options, int index, Action<int> onChanged, double width = 180)
    {
        var cb = new ComboBox { Classes = { "app-select" }, MinWidth = width };
        foreach (var o in options) cb.Items.Add(Localization.T(o));
        cb.SelectedIndex = index;
        cb.SelectionChanged += (_, _) => onChanged(cb.SelectedIndex);
        return cb;
    }

    private TextBox MakeInput(string text, string watermark, Action<string> onChanged, double minWidth = 160)
    {
        var tb = new TextBox
        {
            Classes = { "app-input" },
            Text = text,
            PlaceholderText = watermark,
            MinWidth = minWidth,
            MinHeight = 30,
        };
        tb.TextChanged += (_, _) => onChanged(tb.Text ?? "");
        return tb;
    }

    private static Bitmap? LoadSchemeIcon(string id)
    {
        try
        {
            using var stream = global::Avalonia.Platform.AssetLoader.Open(
                new Uri($"avares://AhabAssistant.Avalonia/Assets/status_effects/{id}.png"));
            return new Bitmap(stream);
        }
        catch
        {
            return null;
        }
    }

    private Image SchemeIcon(string id) => new()
    {
        Source = LoadSchemeIcon(id),
        Width = 18, Height = 18,
    };

    /* ==================== Tab 内容 ==================== */

    private void BuildTabs()
    {
        // 各 Tab 惰性构建
    }

    private void OnTab(object? sender, RoutedEventArgs e)
    {
        if (sender is not Button btn) return;
        SelectTab(btn, animate: true);
    }

    private void SelectTab(Button btn, bool animate)
    {
        foreach (var b in new[] { TabBasic, TabShop, TabCombat, TabStarlight, TabAdvanced })
            b.Classes.Set("active", b == btn);

        var tabIndex = btn == TabBasic ? 0
            : btn == TabShop ? 1
            : btn == TabCombat ? 2
            : btn == TabStarlight ? 3
            : 4;
        var direction = tabIndex >= _activeTabIndex
            ? MotionDirection.Forward
            : MotionDirection.Backward;
        var content = btn.Tag switch
        {
            "basic" => BuildBasic(),
            "shop" => BuildShop(),
            "combat" => BuildCombat(),
            "starlight" => BuildStarlight(),
            _ => BuildAdvanced(),
        };

        Localization.ApplyStatic(content);
        TabHost.TransitionTo(
            content,
            direction,
            animate && _activeTabIndex >= 0 && tabIndex != _activeTabIndex && UiMotion.IsEnabled);
        _activeTabIndex = tabIndex;
    }

    private Control BuildBasic()
    {
        var root = new StackPanel { Spacing = 14 };

        // 名称 + 用途
        var nameBox = MakeInput(_team.Name, "输入队伍名称", v => _team.Name = v);
        nameBox.MinWidth = 240;
        var purposeCb = MakeCombo(new[] { "镜牢", "经验本", "通用" }, _team.Purpose switch
        {
            "mirror" => 0, "luxcavation" => 1, _ => 2,
        }, i => _team.Purpose = i switch { 0 => "mirror", 1 => "luxcavation", _ => "general" });
        var topGrid = new Grid { ColumnDefinitions = ColumnDefinitions.Parse("*,*,Auto") };
        var nameStack = new StackPanel { Spacing = 4 };
        nameStack.Children.Add(FieldLabel("队伍名称"));
        nameStack.Children.Add(nameBox);
        var purposeStack = new StackPanel { Spacing = 4 };
        purposeStack.Children.Add(FieldLabel("用途"));
        purposeStack.Children.Add(purposeCb);
        SetCol(topGrid, 0, nameStack);
        SetCol(topGrid, 1, purposeStack);
        root.Children.Add(topGrid);

        // 主体系
        var schemeStack = new StackPanel { Spacing = 6 };
        schemeStack.Children.Add(FieldLabel("主体系"));
        var schemeWrap = new WrapPanel();
        for (var i = 0; i < Systems.Length; i++)
        {
            var idx = i;
            var sys = Systems[i];
            var panel = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
            panel.Children.Add(SchemeIcon(sys.Id));
            panel.Children.Add(Label(sys.Label));
            var btn = new Button
            {
                Classes = { "app-btn" },
                Content = panel,
                Padding = new Thickness(10, 7),
                CornerRadius = new CornerRadius(8),
                Tag = idx,
            };
            UpdateSchemeStyle(btn, sys.Id);
            btn.Click += (_, _) =>
            {
                _team.AccessoryScheme = sys.Id;
                Mc.TeamSystem = idx;
                foreach (var child in schemeWrap.Children.OfType<Button>())
                    UpdateSchemeStyle(child, Systems[(int)child.Tag!].Id);
            };
            schemeWrap.Children.Add(btn);
        }
        schemeStack.Children.Add(schemeWrap);
        root.Children.Add(Section(schemeStack.Children.ToArray()));

        // 人格编成
        var sinnerSection = new StackPanel { Spacing = 8 };
        var selectedText = Localization.IsEnglish
            ? $"{_team.Sinners.Count}/12 selected"
            : $"{_team.Sinners.Count}/12 已选";
        var headerGrid = Row(SubsectionTitle("人格编成"), Label(selectedText));
        sinnerSection.Children.Add(headerGrid);
        sinnerSection.Children.Add(Muted("点击选择/取消，按点击顺序分配出战位次 #1~#12"));
        var sinnerWrap = new WrapPanel { Name = "SinnerWrap" };
        RebuildSinners(sinnerWrap);
        sinnerSection.Children.Add(sinnerWrap);
        root.Children.Add(Section(sinnerSection.Children.ToArray()));

        // 编队码 & 固定用途
        var twoCols = new UniformGrid { Columns = 2 };
        var codePanel = new StackPanel { Spacing = 6, Margin = new Thickness(0, 0, 8, 0) };
        codePanel.Children.Add(Row(FieldLabel("使用编队码"), MakeSwitch(Mc.UseTeamCode, v => { Mc.UseTeamCode = v; RefreshTab(); })));
        codePanel.Children.Add(Muted("直接填入游戏内编队码自动配队"));
        if (Mc.UseTeamCode)
            codePanel.Children.Add(MakeInput(Mc.TeamCode, "输入或粘贴游戏内编队码", v => Mc.TeamCode = v));

        var fixedPanel = new StackPanel { Spacing = 6, Margin = new Thickness(8, 0, 0, 0) };
        fixedPanel.Children.Add(Row(FieldLabel("固定队伍用途"), MakeSwitch(Mc.FixedTeamUse, v => { Mc.FixedTeamUse = v; RefreshTab(); })));
        fixedPanel.Children.Add(Muted("限制此队伍仅在特定镜牢难度下使用"));
        if (Mc.FixedTeamUse)
            fixedPanel.Children.Add(MakeCombo(new[] { "困难镜牢专用", "普通镜牢专用", "全部通用" }, Mc.FixedTeamUseSelect,
                i => Mc.FixedTeamUseSelect = i));
        SetCol(twoCols, 0, codePanel);
        SetCol(twoCols, 1, fixedPanel);
        root.Children.Add(Section(twoCols));

        // 启用开关
        root.Children.Add(Row(FieldLabel("启用"), MakeSwitch(_team.Enabled, v => _team.Enabled = v)));
        return root;
    }

    private void UpdateSchemeStyle(Button btn, string id)
    {
        var res = global::Avalonia.Application.Current!.Resources;
        var selected = _team.AccessoryScheme == id;
        btn.Background = selected ? (IBrush)res["BrandBrush"]! : (IBrush)res["CardBrush"]!;
        foreach (var child in ((StackPanel)btn.Content!).Children)
        {
            if (child is TextBlock tb)
            {
                tb.Foreground = selected ? Brushes.White : (IBrush)res["MutedFgBrush"]!;
                tb.FontWeight = selected ? FontWeight.SemiBold : FontWeight.Normal;
            }
        }
    }

    private void RebuildSinners(WrapPanel wrap)
    {
        wrap.Children.Clear();
        foreach (var s in _sinners)
        {
            var info = s;
            var orderIdx = _team.Sinners.IndexOf(info.Id);
            var selected = orderIdx >= 0;

            var stack = new StackPanel { Spacing = 5 };
            var img = new Image
            {
                Source = new Bitmap($"avares://AhabAssistant.Avalonia/Assets/sinners/{info.Id}.png"),
                Width = 48, Height = 48,
            };
            stack.Children.Add(img);
            stack.Children.Add(new TextBlock
            {
                Text = info.Name,
                FontSize = 11,
                FontWeight = selected ? FontWeight.SemiBold : FontWeight.Normal,
                HorizontalAlignment = HorizontalAlignment.Center,
                Foreground = selected ? (IBrush)global::Avalonia.Application.Current!.Resources["FgBrush"]! : (IBrush)global::Avalonia.Application.Current!.Resources["MutedFgBrush"]!,
            });

            var btn = new Button
            {
                Classes = { "app-btn" },
                Content = stack,
                Padding = new Thickness(8, 8),
                CornerRadius = new CornerRadius(8),
                Background = selected ? (IBrush)global::Avalonia.Application.Current!.Resources["BrandSurfaceBrush"]! : (IBrush)global::Avalonia.Application.Current!.Resources["CardBrush"]!,
                Margin = new Thickness(0, 0, 8, 8),
            };
            if (selected)
            {
                var badge = new Border
                {
                    Background = (IBrush)global::Avalonia.Application.Current!.Resources["BrandBrush"]!,
                    CornerRadius = new CornerRadius(10),
                    Width = 20, Height = 20,
                    Child = new TextBlock
                    {
                        Text = $"#{orderIdx + 1}",
                        FontSize = 10,
                        FontWeight = FontWeight.Bold,
                        FontFamily = (global::Avalonia.Application.Current!.Resources["MonoFont"] as FontFamily)!,
                        Foreground = Brushes.White,
                        HorizontalAlignment = HorizontalAlignment.Center,
                        VerticalAlignment = VerticalAlignment.Center,
                    },
                };
                var overlay = new Panel();
                overlay.Children.Add(stack);
                Grid.SetRow(badge, 0);
                badge.HorizontalAlignment = HorizontalAlignment.Right;
                badge.VerticalAlignment = VerticalAlignment.Top;
                overlay.Children.Add(badge);
                btn.Content = overlay;
            }
            btn.Click += (_, _) =>
            {
                if (_team.Sinners.Contains(info.Id)) _team.Sinners.Remove(info.Id);
                else if (_team.Sinners.Count < 12) _team.Sinners.Add(info.Id);
                RebuildSinners(wrap);
            };
            wrap.Children.Add(btn);
        }
    }

    private Control BuildShop()
    {
        var root = new StackPanel();

        root.Children.Add(Section(
            WrapVertical("商店策略",
                MakeCombo(new[] { "默认策略", "保守策略", "激进策略" }, Mc.ShopStrategy, i => Mc.ShopStrategy = i))));

        // 舍弃体系
        var discardStack = new StackPanel { Spacing = 8 };
        discardStack.Children.Add(FieldLabel("舍弃的饰品体系 (多选)"));
        discardStack.Children.Add(Muted("在商店与战斗掉落中避开或售卖选中的体系饰品"));
        var discardWrap = new WrapPanel();
        foreach (var (id, label) in Systems)
        {
            var key = id;
            var chip = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6 };
            chip.Children.Add(SchemeIcon(key));
            chip.Children.Add(Label(label));
            var isDiscarded = Mc.DiscardSystems.TryGetValue(key, out var d) && d;
            var border = new Border
            {
                Child = chip,
                Padding = new Thickness(9, 7),
                CornerRadius = new CornerRadius(8),
                Margin = new Thickness(0, 0, 8, 8),
                BorderThickness = new Thickness(1),
            };
            UpdateDiscardStyle(border, isDiscarded);
            border.PointerPressed += (_, _) =>
            {
                var now = !(Mc.DiscardSystems.TryGetValue(key, out var dv) && dv);
                Mc.DiscardSystems[key] = now;
                UpdateDiscardStyle(border, now);
            };
            discardWrap.Children.Add(border);
        }
        discardStack.Children.Add(discardWrap);
        root.Children.Add(Section(discardStack.Children.ToArray()));

        // 基础操作限制
        var restrict = SwitchGroup(new[]
        {
            ("不治疗罪人", Mc.DoNotHeal, (Action<bool>)(v => Mc.DoNotHeal = v)),
            ("不购买饰品", Mc.DoNotBuy, v => Mc.DoNotBuy = v),
            ("不合成饰品", Mc.DoNotFuse, v => Mc.DoNotFuse = v),
            ("不出售饰品", Mc.DoNotSell, v => Mc.DoNotSell = v),
            ("不升级饰品", Mc.DoNotEnhance, v => Mc.DoNotEnhance = v),
        });
        root.Children.Add(Section(WrapVertical("基础操作限制", restrict)));

        // 进阶合成策略
        var fusion = SwitchGroup(new[]
        {
            ("只激进合成", Mc.OnlyAggressiveFuse, (Action<bool>)(v => Mc.OnlyAggressiveFuse = v)),
            ("不使用公式合成", Mc.DoNotSystemFuse, v => Mc.DoNotSystemFuse = v),
            ("只使用公式合成", Mc.OnlySystemFuse, v => Mc.OnlySystemFuse = v),
            ("激进合成期间也升级饰品", Mc.AggressiveAlsoEnhance, v => Mc.AggressiveAlsoEnhance = v),
            ("激进合成保留体系饰品", Mc.AggressiveSaveSystems, v => Mc.AggressiveSaveSystems = v),
        });
        var fusionPanel = WrapVertical("进阶合成策略", fusion);
        fusionPanel.Children.Add(Separator());
        fusionPanel.Children.Add(Row(
            WrapHorizontal(MakeSwitch(Mc.AfterLevelIv, v => { Mc.AfterLevelIv = v; RefreshTab(); }), FieldLabel("合成四级饰品后行为")),
            Mc.AfterLevelIv
                ? MakeCombo(new[] { "停止合成", "继续合成其他", "转为升级已有饰品" }, Mc.AfterLevelIvSelect, i => Mc.AfterLevelIvSelect = i, 170)
                : new Control()));
        root.Children.Add(Section(fusionPanel.Children.ToArray()));

        // 刷新上限 + 忽略楼层
        var grid = new UniformGrid { Columns = 2 };
        var refreshPanel = new StackPanel { Spacing = 8, Margin = new Thickness(0, 0, 8, 0) };
        refreshPanel.Children.Add(SubsectionTitle("商店刷新与忽略"));
        refreshPanel.Children.Add(Row(FieldLabel("定向刷新上限"), NumInput(Mc.MaxKeywordRefresh, v => Mc.MaxKeywordRefresh = v)));
        refreshPanel.Children.Add(Row(FieldLabel("普通刷新上限"), NumInput(Mc.MaxNormalRefresh, v => Mc.MaxNormalRefresh = v)));
        SetCol(grid, 0, refreshPanel);

        var floorPanel = new StackPanel { Spacing = 8, Margin = new Thickness(8, 0, 0, 0) };
        floorPanel.Children.Add(FieldLabel("忽略指定楼层商店"));
        var floors = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6 };
        for (var f = 0; f < 5; f++)
        {
            var idx = f;
            var ignored = idx < Mc.IgnoreShop.Count && Mc.IgnoreShop[idx];
            var btn = new Button { Classes = { "app-btn" }, Content = $"{idx + 1}F", Padding = new Thickness(12, 5), Tag = idx };
            UpdateFloorStyle(btn, ignored);
            btn.Click += (_, _) =>
            {
                while (Mc.IgnoreShop.Count <= idx) Mc.IgnoreShop.Add(false);
                Mc.IgnoreShop[idx] = !Mc.IgnoreShop[idx];
                UpdateFloorStyle(btn, Mc.IgnoreShop[idx]);
            };
            floors.Children.Add(btn);
        }
        floorPanel.Children.Add(floors);
        SetCol(grid, 1, floorPanel);
        root.Children.Add(Section(grid));
        return root;
    }

    private TextBox NumInput(int value, Action<int> set)
    {
        var tb = new TextBox
        {
            Classes = { "app-input" },
            Text = value.ToString(),
            Width = 64,
            MinHeight = 28,
            TextAlignment = TextAlignment.Right,
        };
        tb.TextChanged += (_, _) => { if (int.TryParse(tb.Text, out var n)) set(Math.Clamp(n, 0, 10)); };
        return tb;
    }

    private void UpdateFloorStyle(Button btn, bool ignored)
    {
        var res = global::Avalonia.Application.Current!.Resources;
        btn.Background = ignored ? (IBrush)res["DestructiveLightBrush"]! : (IBrush)res["CardBrush"]!;
        btn.Foreground = ignored ? (IBrush)res["DestructiveBrush"]! : (IBrush)res["MutedFgBrush"]!;
        btn.FontWeight = ignored ? FontWeight.Bold : FontWeight.Normal;
    }

    private void UpdateDiscardStyle(Border border, bool discarded)
    {
        var res = global::Avalonia.Application.Current!.Resources;
        border.BorderBrush = discarded ? (IBrush)res["DestructiveBrush"]! : (IBrush)res["InputBorderBrush"]!;
        border.Background = discarded ? (IBrush)res["DestructiveLightBrush"]! : (IBrush)res["CardBrush"]!;
        if (border.Child is StackPanel sp)
            foreach (var c in sp.Children)
                if (c is TextBlock tb)
                {
                    tb.Foreground = discarded ? (IBrush)res["DestructiveBrush"]! : (IBrush)res["MutedFgBrush"]!;
                    tb.FontWeight = discarded ? FontWeight.SemiBold : FontWeight.Normal;
                }
    }

    private Control BuildCombat()
    {
        var root = new StackPanel();

        // 二体系
        var secondPanel = WrapVertical(null,
            Row(WrapVertical("第二体系设置", Muted("在指定楼层后引入次要饰品体系与合成策略")),
                MakeSwitch(Mc.SecondSystem, v => { Mc.SecondSystem = v; RefreshTab(); })));
        if (Mc.SecondSystem)
        {
            secondPanel.Children.Add(Separator());
            var selGrid = new UniformGrid { Columns = 2 };
            var sel1 = WrapVertical(FieldLabel("次要体系"),
                MakeCombo(Systems.Select(s => s.Label).ToArray(), Mc.SecondSystemSelect, i => Mc.SecondSystemSelect = i, 150));
            var sel2 = WrapVertical(FieldLabel("起始启用楼层"),
                MakeCombo(new[] { "第2层", "第3层", "第4层", "第5层" }, Mc.SecondSystemSetting - 2, i => Mc.SecondSystemSetting = i + 2, 150));
            SetCol(selGrid, 0, sel1);
            SetCol(selGrid, 1, sel2);
            secondPanel.Children.Add(selGrid);
            secondPanel.Children.Add(FieldLabel("第二体系联动动作"));
            secondPanel.Children.Add(SwitchGroup(new[]
            {
                ("合成四级", Mc.SecondSystemFuseIv, (Action<bool>)(v => Mc.SecondSystemFuseIv = v)),
                ("购买饰品", Mc.SecondSystemBuy, v => Mc.SecondSystemBuy = v),
                ("选取胜利奖励", Mc.SecondSystemSelectReward, v => Mc.SecondSystemSelectReward = v),
                ("升级四级", Mc.SecondSystemPowerUp, v => Mc.SecondSystemPowerUp = v),
            }));
        }
        root.Children.Add(Section(secondPanel.Children.ToArray()));

        // 战斗与技能策略
        var combat = WrapVertical("战斗与技能策略", SwitchGroup(new[]
        {
            ("链接战避免使用三技能", Mc.AvoidSkill3, (Action<bool>)(v =>
            {
                Mc.AvoidSkill3 = v;
                if (v) Mc.PrioritizeSkill3 = false;
            })),
            ("链接战优先使用三技能", Mc.PrioritizeSkill3, v =>
            {
                Mc.PrioritizeSkill3 = v;
                if (v) Mc.AvoidSkill3 = false;
            }),
            ("每楼层重新编队", Mc.ReFormationEachFloor, v => Mc.ReFormationEachFloor = v),
        }));
        root.Children.Add(Section(combat.Children.ToArray()));

        // 防御与特殊机制
        var defense = WrapVertical("防御与特殊机制",
            SwitchGroup(new[]
            {
                ("链接战首回合全员防御", Mc.DefenseFirstRound, (Action<bool>)(v =>
                {
                    Mc.DefenseFirstRound = v;
                    if (v) Mc.DefenseForSolo = false;
                })),
            }));
        var soloRow = new StackPanel { Spacing = 4 };
        soloRow.Children.Add(Row(
            WrapHorizontal(MakeSwitch(Mc.DefenseForSolo, v =>
            {
                Mc.DefenseForSolo = v;
                if (v) Mc.DefenseFirstRound = false;
                RefreshTab();
            }), FieldLabel("小指良单通杀家人机制")),
            Mc.DefenseForSolo
                ? MakeCombo(new[] { "防御回合数: 1 回合", "防御回合数: 2 回合", "防御回合数: 3 回合", "防御回合数: 4 回合", "防御回合数: 5 回合" },
                    Mc.DefenseForSoloTurns - 1, i => Mc.DefenseForSoloTurns = i + 1, 190)
                : new Control()));
        soloRow.Children.Add(Muted("连续指定回合全员防御触发良秀单通献祭"));
        defense.Children.Add(soloRow);
        root.Children.Add(Section(defense.Children.ToArray()));

        // 技能替换
        var skillPanel = WrapVertical(null,
            Row(FieldLabel("技能替换"), MakeSwitch(Mc.SkillReplacement, v => { Mc.SkillReplacement = v; RefreshTab(); })));
        if (Mc.SkillReplacement)
            skillPanel.Children.Add(WrapVertical(FieldLabel("替换偏好"),
                MakeCombo(new[] { "1技能替换为2技能", "1技能替换为3技能" }, Mc.SkillReplacementMode, i => Mc.SkillReplacementMode = i, 190)));
        root.Children.Add(Section(skillPanel.Children.ToArray()));
        return root;
    }

    private Control BuildStarlight()
    {
        var root = new StackPanel();

        root.Children.Add(Section(Row(
            WrapVertical("开局星光换钱", Muted("开局消耗星光兑换镜牢初始经费")),
            MakeSwitch(Mc.UseStarlight, v => Mc.UseStarlight = v))));

        // 一键全选 + 总消耗
        var quickBar = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6 };
        quickBar.Children.Add(FieldLabel("一键全选:"));
        string[] lvlNames = { "0 关闭", "1 基础", "2 增益+", "3 增益++" };
        for (var lvl = 0; lvl < 4; lvl++)
        {
            var l = lvl;
            var btn = new Button { Classes = { "app-btn", "outline" }, Height = 24, Padding = new Thickness(8, 0), Content = lvlNames[l], FontSize = 11 };
            btn.Click += (_, _) =>
            {
                Mc.OpeningBonus = Enumerable.Repeat(l, 10).ToList();
                RefreshTab();
            };
            quickBar.Children.Add(btn);
        }
        var totalCost = StarlightItems.Select((it, i) => it.Cost * Math.Max(0, Mc.OpeningBonus.ElementAtOrDefault(i))).Sum();
        var costContent = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 4 };
        costContent.Children.Add(new AppIcon
        {
            Icon = "sparkles",
            Width = 13,
            Height = 13,
            Stroke = (IBrush)global::Avalonia.Application.Current!.Resources["WarningBrush"]!,
            VerticalAlignment = VerticalAlignment.Center,
        });
        costContent.Children.Add(new TextBlock
        {
            Text = $"{Localization.T("总计消耗星光:")} {totalCost}",
            FontFamily = (global::Avalonia.Application.Current!.Resources["MonoFont"] as FontFamily)!,
            FontSize = 11,
            Foreground = (IBrush)global::Avalonia.Application.Current!.Resources["WarningBrush"]!,
            VerticalAlignment = VerticalAlignment.Center,
        });
        var costBadge = new Border
        {
            Classes = { "badge" },
            Background = (IBrush)global::Avalonia.Application.Current!.Resources["WarningLightBrush"]!,
            Child = costContent,
        };
        var bar = Row(quickBar, costBadge);
        root.Children.Add(Section(bar));

        // 10 项星光加成
        var itemsWrap = new WrapPanel { Orientation = Orientation.Horizontal };
        for (var i = 0; i < StarlightItems.Length; i++)
        {
            var idx = i;
            var item = StarlightItems[i];
            var currentLvl = Math.Max(0, Mc.OpeningBonus.ElementAtOrDefault(idx));
            var active = currentLvl > 0;

            var titleStack = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 5 };
            titleStack.Children.Add(Label(item.Zh, 12, weight: FontWeight.SemiBold));
            var costText = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 3 };
            costText.Children.Add(new AppIcon
            {
                Icon = "star",
                Width = 11,
                Height = 11,
                Stroke = (IBrush)global::Avalonia.Application.Current!.Resources["WarningBrush"]!,
                VerticalAlignment = VerticalAlignment.Center,
            });
            costText.Children.Add(new TextBlock
            {
                Text = item.Cost.ToString(),
                FontSize = 11,
                FontFamily = (global::Avalonia.Application.Current!.Resources["MonoFont"] as FontFamily)!,
                Foreground = (IBrush)global::Avalonia.Application.Current!.Resources["MutedFgBrush"]!,
                VerticalAlignment = VerticalAlignment.Center,
            });
            titleStack.Children.Add(costText);

            var seg = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 1 };
            string[] segNames = { "0", "1", "2+", "3++" };
            for (var lvl = 0; lvl < 4; lvl++)
            {
                var l = lvl;
                var segBtn = new Button
                {
                    Classes = { "app-btn" },
                    Content = segNames[l],
                    FontSize = 11,
                    FontFamily = (global::Avalonia.Application.Current!.Resources["MonoFont"] as FontFamily)!,
                    Padding = new Thickness(6, 1),
                    CornerRadius = new CornerRadius(2),
                };
                segBtn.Background = currentLvl == l ? (IBrush)global::Avalonia.Application.Current!.Resources["WarningBrush"]! : Brushes.Transparent;
                segBtn.Foreground = currentLvl == l ? Brushes.White : (IBrush)global::Avalonia.Application.Current!.Resources["MutedFgBrush"]!;
                segBtn.Click += (_, _) =>
                {
                    while (Mc.OpeningBonus.Count <= idx) Mc.OpeningBonus.Add(0);
                    Mc.OpeningBonus[idx] = l;
                    RefreshTab();
                };
                seg.Children.Add(segBtn);
            }
            var headGrid = Row(titleStack, seg);

            var card = new Border
            {
                Padding = new Thickness(10),
                CornerRadius = new CornerRadius(8),
                Margin = new Thickness(0, 0, 10, 10),
                Width = 370,
                BorderThickness = new Thickness(1),
                BorderBrush = active ? (IBrush)global::Avalonia.Application.Current!.Resources["WarningBrush"]! : (IBrush)global::Avalonia.Application.Current!.Resources["InputBorderBrush"]!,
                Background = active ? (IBrush)global::Avalonia.Application.Current!.Resources["WarningLightBrush"]! : (IBrush)global::Avalonia.Application.Current!.Resources["CardBrush"]!,
                Child = new StackPanel { Spacing = 5, Children = { headGrid, Muted(item.Desc, 11) } },
            };
            itemsWrap.Children.Add(card);
        }
        root.Children.Add(itemsWrap);
        return root;
    }

    private TextBox? _observeInput;
    private WrapPanel? _observeTags;

    private Control BuildAdvanced()
    {
        var root = new StackPanel();

        // 观测 E.G.O 饰品
        var observe = WrapVertical(null,
            Row(WrapVertical("观测 E.G.O 饰品", Muted("进入镜牢时优先抓取的特定 E.G.O 饰品名称")),
                MakeSwitch(Mc.ObserveEgoGift, v => { Mc.ObserveEgoGift = v; RefreshTab(); })));
        if (Mc.ObserveEgoGift)
        {
            var inputRow = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6 };
            _observeInput = new TextBox
            {
                Classes = { "app-input" },
                PlaceholderText = "输入饰品名称并按回车添加",
                MinWidth = 260,
                MinHeight = 30,
            };
            _observeInput.KeyDown += (_, e) =>
            {
                if (e.Key == Key.Enter) AddObserveGift();
            };
            inputRow.Children.Add(_observeInput);
            var addBtn = new Button { Classes = { "app-btn", "brand" }, Height = 30, Padding = new Thickness(12, 0) };
            var addContent = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 5 };
            addContent.Children.Add(new AppIcon
            {
                Icon = "plus",
                Width = 13,
                Height = 13,
                Stroke = (IBrush)global::Avalonia.Application.Current!.Resources["BrandFgBrush"]!,
            });
            addContent.Children.Add(new TextBlock { Text = Localization.T("添加") });
            addBtn.Content = addContent;
            addBtn.Click += (_, _) => AddObserveGift();
            inputRow.Children.Add(addBtn);
            observe.Children.Add(inputRow);
            _observeTags = new WrapPanel();
            RebuildObserveTags();
            observe.Children.Add(_observeTags);
        }
        root.Children.Add(Section(observe.Children.ToArray()));

        // 队伍专属主题包权重
        root.Children.Add(Section(Row(
            WrapVertical("队伍专属主题包权重", Muted("为此编队单独启用独立的主题卡包出现权重配置")),
            MakeSwitch(Mc.UseCustomThemePackWeight, v => Mc.UseCustomThemePackWeight = v))));

        // 导入 / 导出
        var pasteBtn = new Button { Classes = { "app-btn", "outline" }, Height = 26, Padding = new Thickness(10, 0) };
        var pasteContent = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 5 };
        pasteContent.Children.Add(new AppIcon
        {
            Icon = "clipboard-paste",
            Width = 13,
            Height = 13,
        });
        pasteContent.Children.Add(new TextBlock { Text = Localization.T("粘贴配置覆盖") });
        pasteBtn.Content = pasteContent;
        pasteBtn.Click += OnPasteJson;
        var ioPanel = WrapVertical(null, Row(FieldLabel("队伍配置导入 / 导出"), pasteBtn));
        root.Children.Add(Section(ioPanel.Children.ToArray()));

        return root;
    }

    private void AddObserveGift()
    {
        var text = _observeInput?.Text?.Trim();
        if (string.IsNullOrEmpty(text) || Mc.ObserveEgoGiftSelected.Contains(text)) return;
        Mc.ObserveEgoGiftSelected.Add(text);
        if (_observeInput != null) _observeInput.Text = "";
        RebuildObserveTags();
    }

    private void RebuildObserveTags()
    {
        if (_observeTags == null) return;
        _observeTags.Children.Clear();
        foreach (var gift in Mc.ObserveEgoGiftSelected.ToList())
        {
            var g = gift;
            var chip = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 5 };
            chip.Children.Add(Label(gift, 11));
            var closeBtn = new Button
            {
                Classes = { "app-btn" },
                Width = 18,
                Height = 18,
                MinHeight = 18,
                Padding = new Thickness(0),
                CornerRadius = new CornerRadius(4),
                Background = Brushes.Transparent,
                BorderBrush = Brushes.Transparent,
                Content = new AppIcon
                {
                    Icon = "x",
                    Width = 12,
                    Height = 12,
                    Stroke = (IBrush)global::Avalonia.Application.Current!.Resources["MutedFgBrush"]!,
                },
            };
            closeBtn.Click += (_, _) =>
            {
                Mc.ObserveEgoGiftSelected.Remove(g);
                RebuildObserveTags();
            };
            chip.Children.Add(closeBtn);
            _observeTags.Children.Add(new Border
            {
                Classes = { "badge" },
                Child = chip,
                Padding = new Thickness(8, 3),
                Margin = new Thickness(0, 0, 6, 6),
            });
        }
    }

    /* ==================== 小工具 ==================== */

    private static StackPanel WrapVertical(params object?[] children)
    {
        var sp = new StackPanel { Spacing = 5 };
        foreach (var c in children)
        {
            if (c is string text && !string.IsNullOrWhiteSpace(text))
                sp.Children.Add(SubsectionTitle(text));
            else if (c is Control ctl)
                sp.Children.Add(ctl);
        }
        return sp;
    }

    private static StackPanel WrapHorizontal(params object?[] children)
    {
        var sp = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        foreach (var c in children)
            if (c is Control ctl) sp.Children.Add(ctl);
        return sp;
    }

    private static StackPanel SwitchGroup((string Label, bool Initial, Action<bool> OnChange)[] items)
    {
        var sp = new StackPanel { Spacing = 9 };
        foreach (var (label, initial, onChange) in items)
        {
            var row = Row(MakeSwitch(initial, onChange), FieldLabel(label));
            sp.Children.Add(row);
        }
        return sp;
    }

    private static Separator Separator(double topMargin = 2, double bottomMargin = 2)
        => new() { Margin = new Thickness(0, topMargin, 0, bottomMargin) };

    /// <summary>重建当前 Tab（受条件显隐影响的控件）。</summary>
    private void RefreshTab()
    {
        var current = new[] { TabBasic, TabShop, TabCombat, TabStarlight, TabAdvanced }.First(b => b.Classes.Contains("active"));
        SelectTab(current, animate: false);
    }

    /* ==================== 导入 / 导出 / 保存 ==================== */

    private async void OnCopyJson(object? sender, RoutedEventArgs e)
    {
        try
        {
            var clipboard = TopLevel.GetTopLevel(this)?.Clipboard;
            if (clipboard != null)
                await clipboard.SetTextAsync(JsonSerializer.Serialize(_team, Models.AalcJsonContext.Default.TeamDetail));
            MainWindow.Toast("队伍配置已复制到剪贴板");
        }
        catch
        {
            MainWindow.Toast("复制失败", "error");
        }
    }

    private async void OnPasteJson(object? sender, RoutedEventArgs e)
    {
        try
        {
            var clipboard = TopLevel.GetTopLevel(this)?.Clipboard;
            var text = clipboard != null ? await clipboard.TryGetTextAsync() : null;
            if (string.IsNullOrWhiteSpace(text))
            {
                MainWindow.Toast("剪贴板为空，导入失败", "warning");
                return;
            }
            var parsed = JsonSerializer.Deserialize(text, Models.AalcJsonContext.Default.TeamDetail);
            if (parsed == null || string.IsNullOrEmpty(parsed.Name))
            {
                MainWindow.Toast("配置格式无效，导入失败", "error");
                return;
            }
            parsed.Id = _team.Id;
            _team.Name = parsed.Name;
            _team.Purpose = parsed.Purpose;
            _team.Sinners = parsed.Sinners;
            _team.AccessoryScheme = parsed.AccessoryScheme;
            _team.Enabled = parsed.Enabled;
            _team.MirrorConfig = parsed.MirrorConfig != null
                ? MergeDefault(parsed.MirrorConfig)
                : TeamMirrorConfig.CreateDefault();
            MainWindow.Toast("队伍配置导入成功");
            RefreshTab();
        }
        catch
        {
            MainWindow.Toast("配置格式无效，导入失败", "error");
        }
    }

    private static TeamMirrorConfig MergeDefault(TeamMirrorConfig parsed)
    {
        var def = TeamMirrorConfig.CreateDefault();
        def.TeamSystem = parsed.TeamSystem;
        def.ShopStrategy = parsed.ShopStrategy;
        def.DiscardSystems = parsed.DiscardSystems;
        def.DoNotHeal = parsed.DoNotHeal;
        def.DoNotBuy = parsed.DoNotBuy;
        def.DoNotFuse = parsed.DoNotFuse;
        def.DoNotSell = parsed.DoNotSell;
        def.DoNotEnhance = parsed.DoNotEnhance;
        def.OnlyAggressiveFuse = parsed.OnlyAggressiveFuse;
        def.DoNotSystemFuse = parsed.DoNotSystemFuse;
        def.OnlySystemFuse = parsed.OnlySystemFuse;
        def.AggressiveAlsoEnhance = parsed.AggressiveAlsoEnhance;
        def.AggressiveSaveSystems = parsed.AggressiveSaveSystems;
        def.AfterLevelIv = parsed.AfterLevelIv;
        def.AfterLevelIvSelect = parsed.AfterLevelIvSelect;
        def.IgnoreShop = parsed.IgnoreShop;
        def.MaxKeywordRefresh = parsed.MaxKeywordRefresh;
        def.MaxNormalRefresh = parsed.MaxNormalRefresh;
        def.SecondSystem = parsed.SecondSystem;
        def.SecondSystemSelect = parsed.SecondSystemSelect;
        def.SecondSystemSetting = parsed.SecondSystemSetting;
        def.SecondSystemFuseIv = parsed.SecondSystemFuseIv;
        def.SecondSystemBuy = parsed.SecondSystemBuy;
        def.SecondSystemSelectReward = parsed.SecondSystemSelectReward;
        def.SecondSystemPowerUp = parsed.SecondSystemPowerUp;
        def.AvoidSkill3 = parsed.AvoidSkill3;
        def.PrioritizeSkill3 = parsed.PrioritizeSkill3;
        def.ReFormationEachFloor = parsed.ReFormationEachFloor;
        def.DefenseFirstRound = parsed.DefenseFirstRound;
        def.DefenseForSolo = parsed.DefenseForSolo;
        def.DefenseForSoloTurns = parsed.DefenseForSoloTurns;
        def.SkillReplacement = parsed.SkillReplacement;
        def.SkillReplacementSelect = parsed.SkillReplacementSelect;
        def.SkillReplacementMode = parsed.SkillReplacementMode;
        def.UseStarlight = parsed.UseStarlight;
        def.OpeningBonus = parsed.OpeningBonus;
        def.FixedTeamUse = parsed.FixedTeamUse;
        def.FixedTeamUseSelect = parsed.FixedTeamUseSelect;
        def.UseTeamCode = parsed.UseTeamCode;
        def.TeamCode = parsed.TeamCode;
        def.UseCustomThemePackWeight = parsed.UseCustomThemePackWeight;
        def.ObserveEgoGift = parsed.ObserveEgoGift;
        def.ObserveEgoGiftSelected = parsed.ObserveEgoGiftSelected;
        return def;
    }

    private async void OnCancel(object? sender, RoutedEventArgs e)
        => await CloseWithAnimationAsync(false);

    private async void OnKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key == Key.Escape)
        {
            e.Handled = true;
            await CloseWithAnimationAsync(false);
        }
    }

    private async void OnSave(object? sender, RoutedEventArgs e)
    {
        if (string.IsNullOrWhiteSpace(_team.Name))
        {
            MainWindow.Toast("请输入队伍名称", "warning");
            return;
        }
        // 返回工作副本，由 TeamsViewModel 统一负责新增/更新和持久化。
        Result = Clone(_team);
        Saved = true;
        await CloseWithAnimationAsync(true);
    }

    private async Task CloseWithAnimationAsync(bool dialogResult)
    {
        if (_closeRequested) return;
        _closeRequested = true;
        await RequestCloseAsync(dialogResult);
    }
}
