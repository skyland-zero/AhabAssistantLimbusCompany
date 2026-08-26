import urllib.request
import re

url = "https://limbuscompany.com/"
req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"})
with urllib.request.urlopen(req, timeout=10) as resp:
    html = resp.read().decode('utf-8', errors='ignore')
    img_tags = re.findall(r'<img[^>]+src=[\'"]([^\'"]+)[\'"][^>]*>', html)
    print("Found img tags:")
    for src in img_tags:
        print(src)
