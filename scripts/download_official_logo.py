import urllib.request
from PIL import Image
import io

urls = [
    "https://limbuscompany.com/wp-content/uploads/2021/10/limbus_logo_text_02-1.png",
    "https://limbuscompany.com/wp-content/uploads/2021/10/limbus_logo_text_02.png",
    "https://limbuscompany.com/wp-content/uploads/2021/10/limbus_logo_01.png",
    "https://limbuscompany.com/wp-content/uploads/2021/10/logo.png",
    "https://limbuscompany.com/wp-content/uploads/2021/11/logo.png",
    "https://limbuscompany.com/wp-content/uploads/2021/11/icon.png",
    "https://limbuscompany.com/favicon.ico",
]

for url in urls:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = resp.read()
            im = Image.open(io.BytesIO(data))
            name = url.split('/')[-1]
            print(f"Success {name}: size={im.size}, mode={im.mode}")
            im.save(f"scripts/limbus_{name}")
    except Exception as e:
        print(f"Failed {url}: {e}")
