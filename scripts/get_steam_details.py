import urllib.request
import json

url = "https://store.steampowered.com/api/appdetails?appids=1973530"
req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
try:
    with urllib.request.urlopen(req, timeout=10) as resp:
        data = json.loads(resp.read().decode('utf-8'))
        app_data = data['1973530']['data']
        print("Header image:", app_data.get('header_image'))
        print("Capsule image:", app_data.get('capsule_image'))
        print("Capsule imagev5:", app_data.get('capsule_imagev5'))
        print("Screenshots:", len(app_data.get('screenshots', [])))
        for k, v in app_data.items():
            if isinstance(v, str) and ('.jpg' in v or '.png' in v or '.ico' in v):
                print(f"{k}: {v}")
except Exception as e:
    print("Error:", e)
