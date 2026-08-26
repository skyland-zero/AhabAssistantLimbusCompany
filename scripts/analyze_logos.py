from PIL import Image

def analyze(path):
    im = Image.open(path)
    bbox = im.getbbox()
    print(f"{path}: Size={im.size}, Mode={im.mode}, BoundingBox={bbox}")
    if bbox:
        cropped = im.crop(bbox)
        print(f"  Cropped Size: {cropped.size}")
        cropped.save(f"{path}_cropped.png")

analyze("scripts/logo.png")
analyze("scripts/eng.png")
analyze("scripts/eng_m.png")
analyze("assets/logo/my_icon.png")
