import os
from PIL import Image

# 1. 准备顶栏横版透明 Logo (Banner Logo)
# 来源: scripts/logo.png_cropped.png (Steam 官方透明版 640x349) 或 scripts/limbus_logo_text_02-1.png
banner_src = "scripts/logo.png_cropped.png"
if not os.path.exists(banner_src):
    banner_src = "scripts/logo.png"

banner_im = Image.open(banner_src).convert("RGBA")
# 自动裁剪透明边缘
bbox = banner_im.getbbox()
if bbox:
    banner_im = banner_im.crop(bbox)

banner_im.save("ui/src/assets/limbus_title_banner.png", "PNG")
banner_im.save("ui/public/limbus_title_banner.png", "PNG")
print(f"Generated title banner logo: {banner_im.size}")

# 2. 准备正方形 1:1 徽章 (Square Emblem for Tray, Favicon & App Icon)
# 选用 512x512 官方罗马数字时钟徽章
emblem_src = "scripts/cropped-limbus_logo_normal.png"
emblem_im = Image.open(emblem_src).convert("RGBA")

emblem_im.save("ui/src/assets/logo.png", "PNG")
emblem_im.save("ui/public/logo.png", "PNG")
print(f"Generated square master emblem: {emblem_im.size}")

# 生成 multi-res .ico (支持 16, 24, 32, 48, 64, 128, 256)
ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
emblem_im.save("ui/public/favicon.ico", format="ICO", sizes=ico_sizes)
emblem_im.save("ui/src-tauri/icons/icon.ico", format="ICO", sizes=ico_sizes)

# 生成 Tauri 桌面与托盘各个尺寸
tauri_icons = {
    "32x32.png": (32, 32),
    "128x128.png": (128, 128),
    "128x128@2x.png": (256, 256),
    "icon.png": (512, 512),
    "Square30x30Logo.png": (30, 30),
    "Square44x44Logo.png": (44, 44),
    "Square71x71Logo.png": (71, 71),
    "Square89x89Logo.png": (89, 89),
    "Square107x107Logo.png": (107, 107),
    "Square142x142Logo.png": (142, 142),
    "Square150x150Logo.png": (150, 150),
    "Square284x284Logo.png": (284, 284),
    "Square310x310Logo.png": (310, 310),
    "StoreLogo.png": (50, 50),
}

for fname, size in tauri_icons.items():
    resized = emblem_im.resize(size, Image.Resampling.LANCZOS)
    out_path = os.path.join("ui/src-tauri/icons", fname)
    resized.save(out_path, "PNG")

print("Scheme B assets successfully built!")
