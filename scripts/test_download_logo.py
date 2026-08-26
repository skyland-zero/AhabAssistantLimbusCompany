import urllib.request
from PIL import Image
import io

urls = [
    "https://limbuscompany.com/wp-content/uploads/2021/12/eng.png",
    "https://limbuscompany.com/wp-content/uploads/2021/12/eng_m.png",
]

for url in urls:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = resp.read()
            im = Image.open(io.BytesIO(data))
            print(f"URL: {url} => Size: {im.size}, Format: {im.format}, Mode: {im.mode}")
            im.save(f"scripts/{url.split('/')[-1]}")
    except Exception as e:
        print(f"Error {url}: {e}")
