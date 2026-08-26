import os
from PIL import Image

# 选用官方高清标志 cropped-limbus_logo_normal.png / cropped-limbus_logo_feather.png
src_img_path = "scripts/cropped-limbus_logo_normal.png"
if not os.path.exists(src_img_path):
    src_img_path = "scripts/cropped-limbus_logo_feather.png"

im = Image.open(src_img_path).convert("RGBA")
print("Selected master icon source:", src_img_path, "Size:", im.size)

# 保存标准 logo.png (512x512)
im.save("ui/src/assets/logo.png", "PNG")
im.save("ui/public/logo.png", "PNG")

# 生成高质量 multi-res .ico
ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
im.save("ui/public/favicon.ico", format="ICO", sizes=ico_sizes)
im.save("ui/src-tauri/icons/icon.ico", format="ICO", sizes=ico_sizes)

# 生成 Tauri 所需的各个尺寸 PNG
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
    resized = im.resize(size, Image.Resampling.LANCZOS)
    out_path = os.path.join("ui/src-tauri/icons", fname)
    resized.save(out_path, "PNG")
    print(f"Generated {out_path} ({size[0]}x{size[1]})")

print("All icons successfully generated from official Limbus Company logo!")
