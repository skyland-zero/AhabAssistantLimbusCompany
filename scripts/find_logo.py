import urllib.request
import re
import sys

try:
    url = "https://limbuscompany.com/"
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"})
    with urllib.request.urlopen(req, timeout=10) as response:
        content = response.read().decode('utf-8', errors='ignore')
        matches = re.findall(r'[\'"]([^\'"]*?\.(?:png|svg|ico|webp|jpg))[\'"]', content, re.I)
        for m in sorted(set(matches)):
            try:
                print(m)
            except Exception:
                print(m.encode('utf-8'))
except Exception as e:
    print("Error:", e)
