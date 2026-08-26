import urllib.request
import re

url = "https://limbuscompany.fandom.com/wiki/Category:Logos"
req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"})
try:
    with urllib.request.urlopen(req, timeout=10) as resp:
        html = resp.read().decode('utf-8', errors='ignore')
        matches = re.findall(r'https://static\.wikia\.nocookie\.net/limbuscompany/images/[^\s\"\'<>]+\.(?:png|webp|svg)', html)
        print("Found fandom logos:")
        for m in sorted(set(matches)):
            print(m)
except Exception as e:
    print("Error:", e)
