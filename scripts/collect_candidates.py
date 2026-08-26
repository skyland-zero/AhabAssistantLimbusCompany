import urllib.request
import os
import io
from PIL import Image
import shutil

out_dir = "assets/icon_candidates"
os.makedirs(out_dir, exist_ok=True)

# 1. 复制已在 scripts/ 下载好的精品官方图标
copies = [
    ("scripts/cropped-limbus_logo_normal.png", "01_Official_Limbus_Normal_Logo.png"),
    ("scripts/cropped-limbus_logo_feather.png", "02_Official_Limbus_Feather_Golden.png"),
    ("scripts/limbus_logo_text_02-1.png", "03_Official_Limbus_Text_Title_Logo.png"),
    ("scripts/logo.png", "04_Official_Steam_Game_Logo.png"),
    ("scripts/00-%EB%A1%9C%EA%B3%A0-%EC%B4%88%EC%83%81%ED%99%94-1.png", "05_Dante_Clock_Face_Emblem.png"),
    ("assets/logo/my_icon.png", "06_Ahab_Assistant_Original_Emblem.png"),
]

for src, dst_name in copies:
    if os.path.exists(src):
        dst = os.path.join(out_dir, dst_name)
        shutil.copy2(src, dst)
        print(f"[COPY] {dst_name}")

# 2. 从 limbuscompany.com / limbuscompany.kr 下载更多直接 URL
headers = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"}

direct_urls = [
    ("07_Sinners_Group_Intro_BG.png", "https://limbuscompany.com/wp-content/uploads/slider30/00-%ED%83%80%EC%9D%B4%ED%8B%80-character-intro-bg.png"),
    ("08_Mephistopheles_Inferno_Bus.png", "https://limbuscompany.com/wp-content/uploads/slider39/00-system-%EC%A7%80%EC%98%A5%EB%B2%84%EC%8A%A4.png"),
    ("09_World_EGO_Symbol.png", "https://limbuscompany.com/wp-content/uploads/slider29/world-e.g.o.png"),
    ("10_World_Abnormality_Symbol.png", "https://limbuscompany.com/wp-content/uploads/slider29/world-%ED%99%98%EC%83%81%EC%B2%B4.png"),
    ("11_World_Golden_Bough_Symbol.png", "https://limbuscompany.com/wp-content/uploads/slider29/world-%ED%99%A9%EA%B8%88%EA%B0%80%EC%A7%80.png"),
]

for fname, url in direct_urls:
    try:
        req = urllib.request.Request(url, headers=headers)
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = resp.read()
            im = Image.open(io.BytesIO(data))
            out_file = os.path.join(out_dir, fname)
            im.save(out_file, "PNG")
            print(f"[DOWNLOAD] {fname}: size={im.size}")
    except Exception as e:
        print(f"[FAIL] {fname}: {e}")

print("Done! Check assets/icon_candidates/ for candidate images.")
