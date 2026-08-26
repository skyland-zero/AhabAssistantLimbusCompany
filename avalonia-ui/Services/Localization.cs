using System;
using System.Collections.Generic;
using Avalonia.Controls;
using Avalonia.Controls.Documents;
using Avalonia.Data;
using Avalonia.Controls.Primitives;
using Avalonia.VisualTree;

namespace AhabAssistant.Avalonia.Services;

/// <summary>
/// 轻量级 UI 本地化。
/// Avalonia 页面使用固定的中文文案来保持 XAML 可读性，这里在控件创建时把固定文案
/// 映射到英文；绑定到数据的 TextBlock 会被跳过，动态文案由各 ViewModel 使用 T() 生成。
/// </summary>
public static class Localization
{
    private static readonly Dictionary<string, string> ZhToEn = new(StringComparer.Ordinal)
    {
        ["主控台"] = "Console",
        ["队伍管理"] = "Teams",
        ["主题包"] = "Themes",
        ["工具箱"] = "Toolbox",
        ["资源中心"] = "Resources",
        ["帮助"] = "Help",
        ["设置"] = "Settings",
        ["最小化"] = "Minimize",
        ["最大化"] = "Maximize",
        ["恢复"] = "Restore",
        ["关闭"] = "Close",

        ["自动化任务"] = "Workflow Tasks",
        ["窗口设置"] = "Window Settings",
        ["日常任务"] = "Daily Tasks",
        ["领取奖励"] = "Claim Rewards",
        ["狂气换体"] = "Refill Enkephalin",
        ["坐牢设置 (镜牢)"] = "Mirror Dungeon",
        ["亚哈共鸣"] = "Ahab Resonance",
        ["常规设置"] = "General",
        ["高级设置"] = "Advanced",
        ["窗口分辨率"] = "Window Resolution",
        ["设置游戏运行窗口的目标分辨率"] = "Set target resolution for the game window",
        ["窗口位置"] = "Window Position",
        ["设置自动调整窗口在屏幕上的摆放位置"] = "Set screen alignment for the game window",
        ["结束后恢复窗口"] = "Restore Window on Finish",
        ["任务执行完毕后将窗口恢复为初始状态，防误触"] = "Reset window state when automation completes",
        ["截图间隔"] = "Screenshot Interval",
        ["两次画面捕获之间的冷却时间"] = "Cooldown between screen captures",
        ["鼠标操作间隔"] = "Mouse Action Interval",
        ["模拟鼠标移动与点击的操作间隔"] = "Delay between simulated mouse clicks and moves",
        ["异步 PostMessage 输入"] = "Async PostMessage Input",
        ["提高键鼠输入效率，低延迟模式"] = "Faster background input with low latency",
        ["经验本次数"] = "EXP Dungeon Runs",
        ["每日采光·经验采集副本执行次数"] = "Number of Luxcavation EXP runs per day",
        ["纽本次数"] = "Thread Dungeon Runs",
        ["每日采光·纽带采集副本执行次数"] = "Number of Luxcavation Thread runs per day",
        ["默认日常编队"] = "Default Daily Team",
        ["执行日常经验本与纽本时优先使用的编队"] = "Team lineup used for daily stages",
        ["连续作战模式"] = "Continuous Combat Mode",
        ["自动连战，减少进出副本加载时间"] = "Continuous battles to avoid repeated loading",
        ["经验本针对性配队"] = "Targeted EXP Lineups",
        ["根据每周不同属性弱点自动轮换对应克制队伍"] = "Auto switch teams targeting weekly weaknesses",
        ["纽本针对性配队"] = "Targeted Thread Lineups",
        ["根据周一到周日不同罪孽属性自动切换编队"] = "Auto switch teams targeting daily sin weaknesses",
        ["领取奖励模式"] = "Claim Mode",
        ["选择需要自动领取的奖励范围"] = "Choose reward scope to claim automatically",
        ["狂气换体次数"] = "Refill Times",
        ["消耗狂气兑换脑啡肽的次数 (0-10)"] = "Number of Lunacy enkephalin refills (0-10)",
        ["葛朗台模式"] = "Dr. Grandet Mode",
        ["仅在体力不足时才进行兑换，避免浪费"] = "Only refill when enkephalin is insufficient",
        ["跳过模块合成"] = "Skip Module Crafting",
        ["除狂气换体外，不自动将多余体力合成为脑啡肽模块"] = "Do not convert surplus enkephalin into modules",
        ["镜牢探索进度 ("] = "Mirror Dungeon Progress (",
        ["坐牢次数"] = "Run Count",
        ["镜牢探索计划执行轮数"] = "Number of Mirror Dungeon exploration runs",
        ["无限坐牢"] = "Infinite Mirror Runs",
        ["不设上限持续刷取镜牢，直至体力耗尽或手动停止"] = "Run continuously until out of enkephalin or stopped",
        ["困难镜牢"] = "Hard Mirror Dungeon",
        ["启用困难镜牢模式 (周四自动消耗奖励次数)"] = "Enable Hard mode (auto consumes weekly bonus on Thu)",
        ["不使用每周加成"] = "Do Not Use Weekly Bonuses",
        ["保留每周加成次数，仅刷取基础点数"] = "Keep weekly bonus claims for later",
        ["只打三层"] = "Exit at Floor 3",
        ["探索至第 3 层后主动结算退出"] = "Settle and claim rewards after finishing Floor 3",
        ["保存困牢奖励"] = "Save Hard Rewards",
        ["通关困难镜牢后不立即领取奖励"] = "Do not claim Hard rewards immediately",
        ["困牢单次加成"] = "Single Bonus per Run",
        ["困难镜牢每次结算仅消耗 1 次每周加成"] = "Consume only 1 weekly bonus per hard run",
        ["第5层选活动包"] = "Pick Event Pack on F5",
        ["第 5 层优先进入最左侧活动卡包"] = "Prioritize leftmost event pack on Floor 5",
        ["第5层跳过活动包"] = "Skip Event Pack on F5",
        ["第 5 层不选择活动卡包"] = "Do not select event packs on Floor 5",
        ["再次领取奖励"] = "Re-claim Rewards",
        ["首次镜牢完成后再次触发一次领奖任务"] = "Run reward claim again after the first mirror run",
        ["不跳过白棉花"] = "Do Not Skip Gossypium",
        ["遇到白棉花相关事件时不跳过"] = "Do not skip White Gossypium encounters",
        ["战斗直到全灭"] = "Fight to the Last Sinner",
        ["不主动逃跑，作战至最后一员倒下"] = "Fight until wiped out without retreating",
        ["键盘寻路导航"] = "Keyboard Pathfinding",
        ["使用键盘模拟按键进行镜牢节点选择与移动"] = "Use keyboard simulated keys for pathfinding and nodes",
        ["全选"] = "Select All",
        ["清空"] = "Clear",
        ["结束后操作"] = "After Completion Actions",
        ["前置动作 (可多选)"] = "Exit Actions (Multi-select)",
        ["退出游戏"] = "Exit Game",
        ["退出模拟器"] = "Exit Emulator",
        ["退出 AALC"] = "Exit AALC",
        ["最终动作 (单选)"] = "Power Action (Single-select)",
        ["仅本次生效"] = "Apply Once",
        ["保存为默认"] = "Save as Default",
        ["设备连接"] = "Device Connection",
        ["实时画面"] = "Live Screen",
        ["等待游戏窗口画面接入"] = "Waiting for game screen connection",
        ["运行日志"] = "Execution Logs",
        ["停止"] = "Stop",
        ["运行"] = "Run",
        ["运行中"] = "Running",
        ["工具均为 Mock 模拟，后端接入后生效"] = "Tools are mocked until the backend lands",

        ["外观"] = "Appearance",
        ["主题模式"] = "Theme Mode",
        ["浅色"] = "Light",
        ["深色"] = "Dark",
        ["跟随系统"] = "System",
        ["强调色"] = "Accent Color",
        ["语言 / Language"] = "Language",
        ["简体中文"] = "Simplified Chinese",
        ["全局热键"] = "Global Hotkeys",
        ["启用全局热键"] = "Enable Global Hotkeys",
        ["启动 / 停止热键"] = "Start / Stop Hotkey",
        ["暂停 / 继续热键"] = "Pause / Resume Hotkey",
        ["清除"] = "Clear",
        ["模拟器设置"] = "Simulator Settings",
        ["使用模拟器模式"] = "Use Simulator Mode",
        ["启用模拟器 ADB 自动化控制连接"] = "Enable ADB automation connection to Android emulator",
        ["模拟器类型"] = "Simulator Type",
        ["选择当前运行的安卓模拟器"] = "Select active Android emulator",
        ["ADB 端口号"] = "ADB Port",
        ["模拟器连接端口 (MuMu 默认 16384)"] = "Port used for connection (MuMu default 16384)",
        ["启动模拟器超时 (秒)"] = "Launch Timeout (seconds)",
        ["仅限 MuMu 模拟器拉起等待时间"] = "Wait duration when launching MuMu Player",
        ["系统与防护"] = "System & Protection",
        ["内存占用保护"] = "Memory Protection",
        ["电脑总内存占用超过 90% 时自动清理内存防崩溃"] = "Clean memory automatically if overall RAM usage exceeds 90%",
        ["最小化到托盘"] = "Minimize to Tray",
        ["窗口最小化时隐藏到系统托盘区"] = "Hide window to system tray when minimized",
        ["开机自动启动"] = "Start on Boot",
        ["跟随 Windows 系统开机自启 AALC"] = "Launch AALC automatically when Windows boots",
        ["实验性功能"] = "Experimental Features",
        ["运行时阻止休眠"] = "Prevent System Sleep",
        ["任务执行期间阻止系统与显示器进入休眠，任务结束后自动恢复"] = "Prevent sleep and display turn-off during task execution",
        ["显示器 HDR 检测提示"] = "Display HDR Detection",
        ["检测到游戏处于 HDR 显示器时提示潜在图像识别问题"] = "Warn about potential visual recognition issues on HDR monitors",
        ["更新与源配置"] = "Updates & Sources",
        ["参与预览版渠道"] = "Pre-release Channel",
        ["接收测试版与预发布版更新推送"] = "Receive beta and preview update notifications",
        ["更新源选择"] = "Update Mirror",
        ["选择检查与下载更新使用的镜像服务"] = "Select mirror service for downloads",
        ["Mirror 酱 CDK"] = "Mirror-Chyan CDK",
        ["关于"] = "About",
        ["版本"] = "Version",
        ["开源地址"] = "Repository",
        ["检查更新"] = "Check for Updates",

        ["全部"] = "All",
        ["镜牢"] = "Mirror",
        ["经验本"] = "EXP Dungeon",
        ["通用"] = "General",
        ["新建队伍"] = "New Team",
        ["星光已配"] = "Starlight ready",
        ["第二体系"] = "2nd system",
        ["良秀单通"] = "Solo pass",
        ["编队码"] = "Team code",
        ["已停用"] = "Disabled",
        ["删除队伍"] = "Delete Team",
        ["确定删除队伍“"] = "Delete team \"",
        ["”吗？此操作不可撤销。"] = "\"? This action cannot be undone.",
        ["取消"] = "Cancel",
        ["删除"] = "Delete",
        ["编辑队伍"] = "Edit Team",
        ["复制配置 (JSON)"] = "Copy JSON Config",
        ["基础编成"] = "Basic & Formation",
        ["商店与合成"] = "Shop & Fusion",
        ["二体系与战斗"] = "Second System & Combat",
        ["开局星光"] = "Starlight Bonus",
        ["观测与高级"] = "Observe & Advanced",
        ["保存"] = "Save",
        ["总权重："] = "Total Weight: ",
        ["按权重排序"] = "Sort by Weight",
        ["全部启用"] = "Enable All",
        ["全部停用"] = "Disable All",
        ["恢复默认权重"] = "Reset Weights",
        ["权重"] = "Weight",
        ["立即同步"] = "Sync Now",
        ["目录"] = "Contents",
        ["正在执行任务："] = "Executing task: ",
        ["正在执行任务…"] = "Executing…",
        ["执行已暂停"] = "Paused",
        ["暂停"] = "Pause",
        ["继续"] = "Resume",
        ["待机中"] = "Idle",
        ["分辨率"] = "Res",
        ["异步输入"] = "Async Input",
        ["开"] = "On",
        ["关"] = "Off",
        ["经验本标签"] = "EXP",
        ["纽本"] = "Thread",
        ["连战"] = "Chain",
        ["模式"] = "Mode",
        ["狂气/通行证"] = "Lunacy/Pass",
        ["邮件"] = "Mail",
        ["换体"] = "Swaps",
        ["次"] = "x",
        ["葛朗台"] = "Grandet",
        ["坐牢"] = "Runs",
        ["难度"] = "Diff",
        ["困难"] = "Hard",
        ["普通"] = "Normal",
        ["语录"] = "Quote",
        ["开启"] = "Enabled",
        ["关闭状态"] = "Disabled",
        ["游戏"] = "Game",
        ["模拟器"] = "Emulator",
        ["退出"] = "Exit ",
        ["与"] = " & ",
        ["后"] = " then ",
        ["睡眠"] = "Sleep",
        ["休眠"] = "Hibernate",
        ["锁屏"] = "Lock Screen",
        ["关机"] = "Shut Down",
        ["默认"] = "Default",
        ["本次"] = "This run",
        ["什么也不干"] = "Do nothing",
        ["暂无队伍"] = "No teams yet",
        ["创建一个队伍用于镜牢、经验本等任务"] = "Create a team for Mirror, EXP dungeons and more",
        ["当前分类暂无队伍"] = "No teams in this category",
        ["点击上方“新建队伍”创建一个队伍"] = "Click New Team above to create one",
        ["已是最新"] = "Up to date",
        ["可更新到"] = "Update available:",
        ["从未同步"] = "Never",

        // 队伍编辑器
        ["烧伤"] = "Burn",
        ["流血"] = "Bleed",
        ["震颤"] = "Tremor",
        ["破裂"] = "Rupture",
        ["沉沦"] = "Sinking",
        ["呼吸"] = "Poise",
        ["充能"] = "Charge",
        ["斩击"] = "Slash",
        ["突刺"] = "Pierce",
        ["打击"] = "Blunt",
        ["初始之星"] = "Star of the Beginning",
        ["积聚的星云"] = "Cumulating Starcloud",
        ["星际漫游"] = "Interstellar Travel",
        ["流星"] = "Star-shower",
        ["双星商店"] = "Binary Star-shop",
        ["卫星商店"] = "Moon Star-shop",
        ["星云的宠爱"] = "Favor of the Starcloud",
        ["星芒的引导"] = "Guidance of the Starlight",
        ["偶然的彗星"] = "Accidental Comet",
        ["全面的可能性"] = "All-round Possibility",
        ["初始经费增加，卡包/饰品展出+1，免费普通刷新"] = "More starting cost, +1 pack/gift display, free normal refresh",
        ["进阶经费利息+10%~30%，售卖饰品经费加成"] = "+10%~30% advanced cost interest and gift sale bonus",
        ["卡包出现+1，卡包刷新+2~4，未记录卡包等级提升"] = "+1 pack, +2~4 pack refreshes, unrecorded pack level up",
        ["初始经费+400~700，初始饰品可选择数+1"] = "+400~700 starting cost and +1 starting gift choice",
        ["展出饰品+1，战斗经费+20%~40%，高阶饰品概率提升"] = "+1 gift display, +20%~40% battle cost, higher-tier gift chance",
        ["免费关键词刷新，进入第1层送1~3件1级饰品"] = "Free keyword refreshes and 1~3 level-1 gifts on floor 1",
        ["进入第1层人格等级+3，通关阶段人格等级提升"] = "+3 sinner level on floor 1 and level ups after stages",
        ["最大速度+2~3，拼点威力/伤害强化/守护提升"] = "+2~3 max speed and stronger clashes, damage and guard",
        ["进商店赠送合成/售卖专用饰品，赠送对应关键词3/4级饰品"] = "Shop grants fusion/sale gifts and matching level 3/4 gifts",
        ["开局自选3级饰品，获得残影饰品"] = "Choose a level-3 gift at start and gain an afterimage gift",
        ["输入队伍名称"] = "Enter team name",
        ["队伍名称"] = "Team name",
        ["用途"] = "Purpose",
        ["主体系"] = "Main Scheme",
        ["人格编成"] = "Sinners",
        ["已选"] = "selected",
        ["点击选择/取消，按点击顺序分配出战位次 #1~#12"] = "Click to select/order deployment slots #1~#12",
        ["使用编队码"] = "Use Team Code",
        ["直接填入游戏内编队码自动配队"] = "Use an in-game team export code to deploy automatically",
        ["输入或粘贴游戏内编队码"] = "Enter or paste team code",
        ["固定队伍用途"] = "Fixed Team Purpose",
        ["限制此队伍仅在特定镜牢难度下使用"] = "Restrict this team to a specific Mirror Dungeon difficulty",
        ["困难镜牢专用"] = "Hard Mirror Only",
        ["普通镜牢专用"] = "Normal Mirror Only",
        ["全部通用"] = "All (Normal & Hard)",
        ["启用"] = "Enabled",
        ["商店策略"] = "Shop Strategy",
        ["默认策略"] = "Default Strategy",
        ["保守策略"] = "Conservative Strategy",
        ["激进策略"] = "Aggressive Strategy",
        ["舍弃的饰品体系 (多选)"] = "Discard Systems (Multi-select)",
        ["在商店与战斗掉落中避开或售卖选中的体系饰品"] = "Avoid or sell selected system gifts in shops and rewards",
        ["不治疗罪人"] = "Do Not Heal Sinners",
        ["不购买饰品"] = "Do Not Buy Gifts",
        ["不合成饰品"] = "Do Not Fuse Gifts",
        ["不出售饰品"] = "Do Not Sell Gifts",
        ["不升级饰品"] = "Do Not Enhance Gifts",
        ["基础操作限制"] = "Shop Action Restrictions",
        ["只激进合成"] = "Only Aggressive Fusion",
        ["不使用公式合成"] = "Do Not Use Formula Fusion",
        ["只使用公式合成"] = "Only Use Formula Fusion",
        ["激进合成期间也升级饰品"] = "Enhance Gifts During Aggressive Fusion",
        ["激进合成保留体系饰品"] = "Keep System Gifts During Aggressive Fusion",
        ["进阶合成策略"] = "Advanced Fusion Settings",
        ["合成四级饰品后行为"] = "Behavior After Tier 4 Fusion",
        ["停止合成"] = "Stop Fusing",
        ["继续合成其他"] = "Continue Fusing Others",
        ["转为升级已有饰品"] = "Switch to Enhancing Existing Gifts",
        ["商店刷新与忽略"] = "Shop Refresh & Floors",
        ["定向刷新上限"] = "Max Keyword Refreshes",
        ["普通刷新上限"] = "Max Normal Refreshes",
        ["忽略指定楼层商店"] = "Ignore Shop on Floors",
        ["第二体系设置"] = "Second System Settings",
        ["在指定楼层后引入次要饰品体系与合成策略"] = "Introduce a secondary gift system after a selected floor",
        ["次要体系"] = "Secondary System",
        ["起始启用楼层"] = "Start Floor",
        ["第2层"] = "Floor 2",
        ["第3层"] = "Floor 3",
        ["第4层"] = "Floor 4",
        ["第5层"] = "Floor 5",
        ["第二体系联动动作"] = "Second System Linked Actions",
        ["合成四级"] = "Fuse Tier 4",
        ["购买饰品"] = "Buy Gifts",
        ["选取胜利奖励"] = "Select in Rewards",
        ["升级四级"] = "Upgrade Tier 4",
        ["战斗与技能策略"] = "Combat & Skill Preferences",
        ["链接战避免使用三技能"] = "Avoid Skill 3 in Chained Battles",
        ["链接战优先使用三技能"] = "Prioritize Skill 3 in Chained Battles",
        ["每楼层重新编队"] = "Re-form Team Each Floor",
        ["防御与特殊机制"] = "Defense Strategies",
        ["链接战首回合全员防御"] = "All Defend in Round 1",
        ["小指良单通杀家人机制"] = "Solo Ryoshu Sacrificial Defense",
        ["防御回合数: 1 回合"] = "Defend Turns: 1",
        ["防御回合数: 2 回合"] = "Defend Turns: 2",
        ["防御回合数: 3 回合"] = "Defend Turns: 3",
        ["防御回合数: 4 回合"] = "Defend Turns: 4",
        ["防御回合数: 5 回合"] = "Defend Turns: 5",
        ["连续指定回合全员防御触发良秀单通献祭"] = "Continuously defend to trigger solo sacrifice",
        ["技能替换"] = "Skill Replacement",
        ["替换偏好"] = "Replacement Mode",
        ["1技能替换为2技能"] = "Replace Skill 1 with Skill 2",
        ["1技能替换为3技能"] = "Replace Skill 1 with Skill 3",
        ["开局星光换钱"] = "Convert Starlight to Cost",
        ["开局消耗星光兑换镜牢初始经费"] = "Consume starlight for starting cost",
        ["一键全选:"] = "Set All:",
        ["总计消耗星光:"] = "Total Starlight Cost:",
        ["0 关闭"] = "0 Off",
        ["1 基础"] = "1 Base",
        ["2 增益+"] = "2 Buff+",
        ["3 增益++"] = "3 Buff++",
        ["观测 E.G.O 饰品"] = "Observe E.G.O Gifts",
        ["进入镜牢时优先抓取的特定 E.G.O 饰品名称"] = "Prioritize specific E.G.O gifts when encountered",
        ["输入饰品名称并按回车添加"] = "Enter gift name and press Enter",
        ["添加"] = "Add",
        ["队伍专属主题包权重"] = "Custom Theme Pack Weight",
        ["为此编队单独启用独立的主题卡包出现权重配置"] = "Use independent theme pack weights for this team",
        ["粘贴配置覆盖"] = "Paste Config Override",
        ["队伍配置导入 / 导出"] = "Import / Export Configuration",
        ["队伍配置已复制到剪贴板"] = "Team config copied to clipboard",
        ["复制失败"] = "Copy failed",
        ["剪贴板为空，导入失败"] = "Clipboard is empty; import failed",
        ["配置格式无效，导入失败"] = "Invalid configuration; import failed",
        ["队伍配置导入成功"] = "Team config imported successfully",
        ["请输入队伍名称"] = "Please enter a team name",
        ["自动战斗"] = "Auto Battle",
        ["循环执行战斗直至手动停止"] = "Loop battles until stopped manually",
        ["体力换饼"] = "Enkephalin Module",
        ["自动将狂气转换为体力并合成脑啡肽模块，防止体力溢出"] = "Convert Lunacy to Enkephalin modules automatically to prevent overflow",
        ["辅助截图"] = "Screenshot Tool",
        ["截取当前游戏窗口画面并保存到 AALC 目录"] = "Capture the game window and save it to the AALC folder",
        ["截图完成，已保存到 AALC 目录"] = "Screenshot saved to the AALC folder",
        ["待机"] = "Idle",
        ["当前已是最新版本（"] = "You are on the latest version (",
        ["资源同步完成"] = "Resource sync completed",
        ["同步中…"] = "Syncing…",
        ["点击输入框后按下快捷键"] = "Click input and press key",
        ["编队 1"] = "Team 1",
    };

    private static readonly Dictionary<string, string> EnToZh = BuildReverseMap();

    public static bool IsEnglish => App.Language == "en-US";

    public static string T(string text)
    {
        if (string.IsNullOrEmpty(text)) return text;
        if (IsEnglish)
            return ZhToEn.TryGetValue(text, out var en) ? en : text;
        return EnToZh.TryGetValue(text, out var zh) ? zh : text;
    }

    private static Dictionary<string, string> BuildReverseMap()
    {
        var result = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var pair in ZhToEn)
            if (!result.ContainsKey(pair.Value)) result[pair.Value] = pair.Key;
        return result;
    }

    /// <summary>翻译页面中不带 Binding 的固定文案。</summary>
    public static void ApplyStatic(Control root)
    {
        if (root is Window window) window.Title = T(window.Title ?? "");

        foreach (var visual in root.GetVisualDescendants())
        {
            if (visual is not Control control) continue;
            switch (control)
            {
                case ItemsControl itemsControl:
                    for (var i = 0; i < itemsControl.Items.Count; i++)
                    {
                        var item = itemsControl.Items[i];
                        if (item is ComboBoxItem comboItem && comboItem.Content is string itemText)
                            comboItem.Content = T(itemText);
                    }
                    break;
                case TextBlock textBlock:
                    if (BindingOperations.GetBindingExpressionBase(textBlock, TextBlock.TextProperty) == null)
                    {
                        if (!string.IsNullOrEmpty(textBlock.Text)) textBlock.Text = T(textBlock.Text);
                        if (textBlock.Inlines != null)
                            foreach (var inline in textBlock.Inlines)
                                if (inline is Run run
                                    && BindingOperations.GetBindingExpressionBase(run, Run.TextProperty) == null
                                    && !string.IsNullOrEmpty(run.Text))
                                    run.Text = T(run.Text);
                    }
                    break;
                case Button button:
                    if (BindingOperations.GetBindingExpressionBase(button, Button.ContentProperty) == null && button.Content is string content)
                        button.Content = T(content);
                    break;
                case ComboBoxItem comboItem:
                    if (BindingOperations.GetBindingExpressionBase(comboItem, ContentControl.ContentProperty) == null && comboItem.Content is string comboText)
                        comboItem.Content = T(comboText);
                    break;
                case TextBox textBox when !string.IsNullOrEmpty(textBox.PlaceholderText):
                    textBox.PlaceholderText = T(textBox.PlaceholderText!);
                    break;
            }

            var tip = ToolTip.GetTip(control);
            if (tip is string tipText) ToolTip.SetTip(control, T(tipText));
        }
    }
}
