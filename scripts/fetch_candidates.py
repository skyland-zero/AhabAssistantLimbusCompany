import requests
import re
import os
from PIL import Image
import io
import sys

headers = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
}

out_dir = "assets/icon_candidates"
os.makedirs(out_dir, exist_ok=True)

# 目标高价值图标列表
targets = [
    # 但丁官方头像
    ("01_Dante_Clock_Head_Avatar.png", "https://static.wikia.nocookie.net/limbuscompany/images/7/7b/Dante_Profile.png"),
    ("02_Dante_Icon_Square.png", "https://static.wikia.nocookie.net/limbuscompany/images/8/8c/Danteicon.jpg"),
    ("03_Dante_Post_Canto_IV.png", "https://static.wikia.nocookie.net/limbuscompany/images/f/f6/Dante_Portrait_Post_Canto_IV.png"),
    
    # 亚哈 (Ahab) 专属头像 - 契合项目名 Ahab Assistant
    ("04_Ahab_Captain_Avatar.png", "https://static.wikia.nocookie.net/limbuscompany/images/7/77/Ahab_Portrait.png"),
    ("05_Ahab_Announcer_Icon.png", "https://static.wikia.nocookie.net/limbuscompany/images/b/b3/Announcer_Ahab.png"),
    ("06_GasHarpoon_Ahab_EGO.png", "https://static.wikia.nocookie.net/limbuscompany/images/4/4e/GasHarpoonAhab_Portrait.png"),
    
    # 边狱公司官方徽章与标志
    ("07_Limbus_Company_Feather_Emblem.png", "https://limbuscompany.com/wp-content/uploads/2021/10/cropped-limbus_logo_feather.png"),
    ("08_Limbus_Company_Normal_Emblem.png", "https://limbuscompany.kr/wp-content/uploads/2021/08/cropped-limbus_logo_normal.png"),
    ("09_Limbus_Company_Full_Title_Logo.png", "https://limbuscompany.com/wp-content/uploads/2021/10/limbus_logo_text_02-1.png"),
    ("10_LCB_Drive_Bus_Emblem.png", "https://static.wikia.nocookie.net/limbuscompany/images/0/05/Mephistopheles_Drive.png"),
    ("11_Dante_Notebook_Icon.png", "https://static.wikia.nocookie.net/limbuscompany/images/c/c2/Dante%27s_Notes_Icon.png"),
]

for filename, url in targets:
    try:
        resp = requests.get(url, headers=headers, timeout=15)
        if resp.status_code == 200:
            im = Image.open(io.BytesIO(resp.content)).convert("RGBA")
            save_path = os.path.join(out_dir, filename)
            im.save(save_path, "PNG")
            print(f"[OK] Downloaded {filename} ({im.size[0]}x{im.size[1]})")
        else:
            print(f"[FAIL] Failed {filename} (HTTP {resp.status_code}): {url}")
    except Exception as e:
        print(f"[ERROR] Error {filename}: {e}")

# 也去寻找一些 Fandom / Gallery 里的头像补充
gallery_url = "https://limbuscompany.fandom.com/wiki/Dante/Gallery"
try:
    resp = requests.get(gallery_url, headers=headers, timeout=15)
    if resp.status_code == 200:
        matches = re.findall(r'https://static\.wikia\.nocookie\.net/limbuscompany/images/[^\s\"\'<>]+\.(?:png|jpg|webp)', resp.text)
        count = 0
        for m in set(matches):
            if any(k in m.lower() for k in ["portrait", "profile", "icon", "sprite"]):
                clean_url = re.sub(r'/revision/.*', '', m)
                raw_name = clean_url.split('/')[-1]
                target_file = os.path.join(out_dir, f"extra_{raw_name}")
                if not os.path.exists(target_file) and count < 8:
                    try:
                        r = requests.get(clean_url, headers=headers, timeout=10)
                        if r.status_code == 200:
                            im = Image.open(io.BytesIO(r.content)).convert("RGBA")
                            im.save(target_file, "PNG")
                            print(f"[OK] Downloaded extra candidate {raw_name} ({im.size[0]}x{im.size[1]})")
                            count += 1
                    except Exception:
                        pass
except Exception as e:
    print(f"Extra search error: {e}")

print("\nFinished downloading candidates to assets/icon_candidates/")
