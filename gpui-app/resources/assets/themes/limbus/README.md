# Limbus 主题占位图（原创自绘 v2）

本目录图片由 `scripts/generate_limbus_theme.py` 程序化生成，
铆钉标签框 / 暗红金属条 / 警示纹 / 印章均为原创绘制，
**不含 ProjectMoon 官方素材**，可随仓库按 AGPLv3 分发。
配色数值（金 `#D8A800`、暗红 `#4A1010`、冷黑底）为从官方美术采样
的色值灵感，不构成复制。

| 文件 | 用途 | 尺寸 |
|---|---|---|
| `bg.png` | 主背景（冷黑+暗蓝星云+裂纹，低不透明度垫底） | 1600x900 |
| `tagband.png` | 暗红金属条（标签孔+铆钉）：卡片头/主按钮底 | 800x72 |
| `frame.png` | 铆钉边框（透明中部）：重点卡外框 | 256x256 |
| `divider.png` | 芥末金分隔线（透明底） | 1200x26 |
| `seal-red.png` | 血红印章点缀（透明底） | 256x256 |

重新生成：`python3 scripts/generate_limbus_theme.py`
