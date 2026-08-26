import math
import os
from PIL import Image, ImageDraw, ImageFont, ImageFilter

os.makedirs("assets/generated_icons", exist_ok=True)
os.makedirs("assets/icon_candidates", exist_ok=True)

# ----------------------------------------------------
# 设计 2：但丁烈焰时钟头像徽章 (Dante Flaming Clock Emblem)
# ----------------------------------------------------
def create_dante_flame_clock(size=1024):
    scale = 4
    w, h = size * scale, size * scale
    cx, cy = w // 2, h // 2
    r_outer = int(w * 0.46)

    img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    
    # 烈焰外部发光
    glow = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glow)
    gdraw.ellipse([cx - r_outer - int(20 * scale), cy - r_outer - int(20 * scale), cx + r_outer + int(20 * scale), cy + r_outer + int(20 * scale)], fill=(255, 60, 20, 180))
    glow = glow.filter(ImageFilter.GaussianBlur(radius=int(35 * scale)))
    img.alpha_composite(glow)

    draw = ImageDraw.Draw(img)

    # 黑色轮廓表壳
    draw.ellipse([cx - r_outer, cy - r_outer, cx + r_outer, cy + r_outer], fill=(12, 8, 10, 255), outline=(220, 30, 20, 255), width=int(16 * scale))

    # 火焰内圈渐变盘
    r_inner = r_outer - int(24 * scale)
    draw.ellipse([cx - r_inner, cy - r_inner, cx + r_inner, cy + r_inner], fill=(45, 10, 12, 255), outline=(245, 170, 30, 255), width=int(8 * scale))

    # 绘制火焰纹理光芒
    for i in range(24):
        ang = math.radians(i * 15)
        flen = int(r_inner * (0.85 if i % 2 == 0 else 0.7))
        fx = cx + flen * math.cos(ang)
        fy = cy + flen * math.sin(ang)
        draw.line([(cx, cy), (fx, fy)], fill=(255, 80, 20, 90), width=int(6 * scale))

    # 罗马数字
    roman_numerals = ["XII", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI"]
    try:
        font = ImageFont.truetype("arial.ttf", int(32 * scale))
    except Exception:
        font = ImageFont.load_default()

    r_num = r_inner - int(38 * scale)
    for i, num in enumerate(roman_numerals):
        rad = math.radians(i * 30 - 90)
        nx = cx + r_num * math.cos(rad)
        ny = cy + r_num * math.sin(rad)
        bbox = draw.textbbox((0, 0), num, font=font)
        tw = bbox[2] - bbox[0]
        th = bbox[3] - bbox[1]
        draw.text((nx - tw / 2, ny - th / 2), num, font=font, fill=(255, 225, 110, 255))

    # 但丁尖锐时钟指针 (10 点 10 分经典主角姿态)
    # 时针 (指向 X 点 = -150度)
    h_rad = math.radians(-150)
    h_len = int(r_inner * 0.55)
    hx = cx + h_len * math.cos(h_rad)
    hy = cy + h_len * math.sin(h_rad)
    draw.polygon([(hx, hy), (cx + int(12 * scale), cy), (cx - int(12 * scale), cy)], fill=(255, 240, 180, 255), outline=(180, 20, 20, 255), width=int(2 * scale))

    # 分针 (指向 II 点 = -30度)
    m_rad = math.radians(-30)
    m_len = int(r_inner * 0.78)
    mx = cx + m_len * math.cos(m_rad)
    my = cy + m_len * math.sin(m_rad)
    draw.polygon([(mx, my), (cx + int(10 * scale), cy), (cx - int(10 * scale), cy)], fill=(255, 200, 50, 255), outline=(180, 20, 20, 255), width=int(2 * scale))

    # 中心炽热表盘核心
    draw.ellipse([cx - int(35 * scale), cy - int(35 * scale), cx + int(35 * scale), cy + int(35 * scale)], fill=(255, 60, 20, 255), outline=(255, 255, 200, 255), width=int(6 * scale))
    draw.ellipse([cx - int(16 * scale), cy - int(16 * scale), cx + int(16 * scale), cy + int(16 * scale)], fill=(255, 255, 255, 255))

    return img.resize((size, size), Image.Resampling.LANCZOS)


# ----------------------------------------------------
# 设计 3：边狱公司六边形机甲徽章 (Limbus Hexagon Mech Shield)
# ----------------------------------------------------
def create_limbus_hex_shield(size=1024):
    scale = 4
    w, h = size * scale, size * scale
    cx, cy = w // 2, h // 2
    r_hex = int(w * 0.46)

    img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # 计算正六边形顶点
    hex_pts = []
    for i in range(6):
        ang = math.radians(i * 60 - 30)
        hex_pts.append((cx + r_hex * math.cos(ang), cy + r_hex * math.sin(ang)))

    # 外层深红六边形外壳
    draw.polygon(hex_pts, fill=(156, 8, 11, 255), outline=(235, 180, 50, 255), width=int(12 * scale))

    # 内层暗夜六边形
    hex_inner_pts = []
    r_inner = r_hex - int(28 * scale)
    for i in range(6):
        ang = math.radians(i * 60 - 30)
        hex_inner_pts.append((cx + r_inner * math.cos(ang), cy + r_inner * math.sin(ang)))
    draw.polygon(hex_inner_pts, fill=(16, 12, 14, 255), outline=(100, 15, 20, 255), width=int(6 * scale))

    # 绘制双翼与黄金罗盘
    r_core = int(r_inner * 0.65)
    draw.ellipse([cx - r_core, cy - r_core, cx + r_core, cy + r_core], fill=(30, 8, 10, 255), outline=(212, 160, 40, 255), width=int(8 * scale))

    # 12 刻度
    for i in range(12):
        ang = math.radians(i * 30)
        x1 = cx + (r_core - int(16 * scale)) * math.cos(ang)
        y1 = cy + (r_core - int(16 * scale)) * math.sin(ang)
        x2 = cx + (r_core - int(4 * scale)) * math.cos(ang)
        y2 = cy + (r_core - int(4 * scale)) * math.sin(ang)
        draw.line([(x1, y1), (x2, y2)], fill=(235, 190, 60, 255), width=int(5 * scale))

    # 居中 LCB 标识
    try:
        font = ImageFont.truetype("arialbd.ttf", int(64 * scale))
    except Exception:
        font = ImageFont.load_default()

    txt = "LCB"
    bbox = draw.textbbox((0, 0), txt, font=font)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    draw.text((cx - tw / 2, cy - th / 2), txt, font=font, fill=(255, 220, 90, 255))

    return img.resize((size, size), Image.Resampling.LANCZOS)

# 生成全部 3 款设计并保存
dante_icon = create_dante_flame_clock(1024)
dante_icon.save("assets/generated_icons/Design_02_Dante_Flame_Clock.png", "PNG")
dante_icon.save("assets/icon_candidates/Design_02_Dante_Flame_Clock.png", "PNG")

hex_icon = create_limbus_hex_shield(1024)
hex_icon.save("assets/generated_icons/Design_03_Limbus_Hex_Shield.png", "PNG")
hex_icon.save("assets/icon_candidates/Design_03_Limbus_Hex_Shield.png", "PNG")

print("Generated all 3 custom designs successfully!")
