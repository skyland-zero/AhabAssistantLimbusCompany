import urllib.request
from PIL import Image
import io

steam_urls = [
    "https://cdn.akamai.steamstatic.com/steam/apps/1973530/logo.png",
    "https://cdn.cloudflare.steamstatic.com/steam/apps/1973530/logo.png",
    "https://cdn.akamai.steamstatic.com/steam/apps/1973530/header.jpg",
    "https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/1973530/690847be8641973c09b69b6574f802ea4ba66ee5.ico",
    "https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/1973530/c5bece46e59aa386927a7c73ff055fa242b5883d.jpg"
]

for url in steam_urls:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = resp.read()
            im = Image.open(io.BytesIO(data))
            print(f"Steam URL: {url} => Size: {im.size}, Format: {im.format}, Mode: {im.mode}")
            fname = url.split('/')[-1]
            if not fname.endswith(('.png', '.ico', '.jpg')):
                fname += '.png'
            im.save(f"scripts/{fname}")
    except Exception as e:
        print(f"Error {url}: {e}")
