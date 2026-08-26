import urllib.request
from PIL import Image
import io

official_urls = [
    "https://limbuscompany.com/wp-content/uploads/2021/10/cropped-limbus_logo_feather.png",
    "https://limbuscompany.kr/wp-content/uploads/2021/08/cropped-limbus_logo_normal.png",
    "https://limbuscompany.com/wp-content/uploads/2021/10/limbus_logo_text_02-1.png",
    "https://limbuscompany.com/wp-content/uploads/slider30/00-%EB%A1%9C%EA%B3%A0-%EC%B4%88%EC%83%81%ED%99%94-1.png",
]

for url in official_urls:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = resp.read()
            im = Image.open(io.BytesIO(data))
            name = url.split('/')[-1]
            print(f"Downloaded {name}: size={im.size}, mode={im.mode}")
            im.save(f"scripts/{name}")
    except Exception as e:
        print(f"Error {url}: {e}")
