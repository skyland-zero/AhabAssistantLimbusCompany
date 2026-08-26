import urllib.request
import urllib.parse
import os
from PIL import Image
import io

out_dir = "assets/icon_candidates"
os.makedirs(out_dir, exist_ok=True)

headers = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"}

files = [
    # 0. 官方标志与徽章
    ("01_Official_Limbus_Normal_Logo.png", "https://i0.wp.com/limbuscompany.kr/wp-content/uploads/2021/08/cropped-limbus_logo_normal.png"),
    ("02_Official_Limbus_Feather_Golden.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/2021/10/cropped-limbus_logo_feather.png"),
    ("03_Official_Limbus_Text_Title_Logo.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/2021/10/limbus_logo_text_02-1.png"),
    ("04_Official_Steam_Game_Logo.png", "https://cdn.akamai.steamstatic.com/steam/apps/1973530/logo.png"),
    ("05_Dante_Clock_Face_Emblem.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/00-%EB%A1%9C%EA%B3%A0-%EC%B4%88%EC%83%81%ED%99%94-1.png"),
    ("06_Mephistopheles_Inferno_Bus.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider39/00-system-%EC%A7%80%EC%98%A5%EB%xb2%84%EC%8A%A4.png"),
    
    # 12 罪人官方超清肖像标志 (Slider 30)
    ("07_Sinner_01_YiSang.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/01-%EC%9D%B4%EC%83%81.png"),
    ("08_Sinner_02_Faust.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/02-%ED%8C%8C%EC%9A%B0%EC%8A%A4%ED%8A%B8.png"),
    ("09_Sinner_03_DonQuixote.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/03-%EB%8F%88%ED%82%A4%ED%98%B8%ED%85%8C.png"),
    ("10_Sinner_04_Ryoshu.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/04-%EB%A3%8C%EC%8A%88.png"),
    ("11_Sinner_05_Meursault.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/05-%EB%AB%BC%EB%A5%B4%EC%86%8C.png"),
    ("12_Sinner_06_HongLu.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/06-%ED%99%8D%EB%A3%A8.png"),
    ("13_Sinner_07_Heathcliff.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/07-%ED%9E%88%EC%8A%A4%ED%81%B4%EB%A6%AC%ED%94%84.png"),
    ("14_Sinner_08_Ishmael.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/08-%EC%9D%B4%EC%8A%A4%EB%A7%88%EC%97%98.png"),
    ("15_Sinner_09_Rodion.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/09-%EB%A1%9C%EC%9F%88.png"),
    ("16_Sinner_11_Sinclair.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/11-%EC%97%90%EB%B0%80-%EC%8B%B1%ED%81%B4%EB%A0%88%EC%96%B4.png"),
    ("17_Sinner_12_Outis.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/12-%EC%98%A4%ED%8B%B0%EC%8A%A4.png"),
    ("18_Sinner_13_Gregor.png", "https://i0.wp.com/limbuscompany.com/wp-content/uploads/slider30/13-%EA%B7%B8%EB%A0%88%EA%B3%A0%EB%A5%B4.png"),
]

for filename, url in files:
    try:
        req = urllib.request.Request(url, headers=headers)
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = resp.read()
            im = Image.open(io.BytesIO(data))
            save_path = os.path.join(out_dir, filename)
            im.save(save_path, "PNG")
            print(f"[OK] Downloaded {filename}: size={im.size}")
    except Exception as e:
        print(f"[FAIL] {filename} ({url}): {e}")

print("Candidate icons saved to assets/icon_candidates/")
