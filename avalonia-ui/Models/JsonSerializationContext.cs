using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace AhabAssistant.Avalonia.Models;

/// <summary>
/// Native AOT 使用的 JSON 序列化元数据。
/// 配置文件和队伍剪贴板数据都通过这里序列化，避免运行时反射在裁剪后失效。
/// </summary>
[JsonSourceGenerationOptions(WriteIndented = true)]
[JsonSerializable(typeof(UiPreferences))]
[JsonSerializable(typeof(TasksConfig))]
[JsonSerializable(typeof(SystemSettingsConfig))]
[JsonSerializable(typeof(HotkeyConfig))]
[JsonSerializable(typeof(ThemePackState))]
[JsonSerializable(typeof(List<TeamDetail>))]
[JsonSerializable(typeof(TeamDetail))]
internal partial class AalcJsonContext : JsonSerializerContext
{
}

public class UiPreferences
{
    [JsonPropertyName("themeMode")]
    public string ThemeMode { get; set; } = "light";

    [JsonPropertyName("accentId")]
    public string AccentId { get; set; } = "crimson";

    [JsonPropertyName("rightPanelWidth")]
    public double RightPanelWidth { get; set; } = 280;

    [JsonPropertyName("rightPanelCollapsed")]
    public bool RightPanelCollapsed { get; set; }

    [JsonPropertyName("language")]
    public string Language { get; set; } = "zh-CN";
}
