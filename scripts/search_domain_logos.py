import urllib.request
import re
import sys

headers = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"}

for domain in ["https://limbuscompany.com/", "https://limbuscompany.kr/"]:
    try:
        req = urllib.request.Request(domain, headers=headers)
        with urllib.request.urlopen(req, timeout=10) as resp:
            content = resp.read().decode('utf-8', errors='ignore')
            urls = re.findall(r'https?://[^\s\"\'<>]+\.(?:png|svg|ico|webp)', content)
            print(f"--- Domain {domain} ---")
            for u in sorted(set(urls)):
                if "logo" in u.lower() or "icon" in u.lower() or "favicon" in u.lower() or "title" in u.lower() or "bus" in u.lower() or "symbol" in u.lower():
                    print(u)
    except Exception as e:
        print(f"Error {domain}: {e}")
