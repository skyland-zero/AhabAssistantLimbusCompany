import math
import os
from PIL import Image, ImageDraw, ImageFont, ImageFilter

def create_ahab_limbus_icon(size=1024):
    scale = 4  # 4x 超采样绘制以获得顶级抗锯齿平滑边缘
    w = size * scale
    h = size * scale
    cx, cy = w // 2, h // 2
    r_outer = int(w * 0.47)

    img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # 1. 外部暗影光晕 (Shadow / Outer Glow)
    glow_img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    glow_draw = ImageDraw.Draw(glow_img)
    glow_draw.ellipse([cx - r_outer, cy - r_outer, cx + r_outer, cy + r_outer], fill=(156, 8, 11, 140))
    glow_img = glow_img.filter(ImageFilter.GaussianBlur(radius=int(30 * scale)))
    img.alpha_composite(glow_img)

    # 2. 基础暗夜金属底盘 (Dark Obsidian Base)
    draw.ellipse([cx - r_outer, cy - r_outer, cx + r_outer, cy + r_outer], fill=(18, 12, 14, 255), outline=(156, 8, 11, 255), width=int(14 * scale))

    # 3. 外层 12 刻度与罗盘齿轮刻线 (12 Sinners / Hours Notches)
    for i in range(12):
        angle_deg = i * 30 - 90
        rad = math.radians(angle_deg)
        r1 = r_outer - int(14 * scale)
        r2 = r_outer - int(32 * scale)
        x1 = cx + r1 * math.cos(rad)
        y1 = cy + r1 * math.sin(rad)
        x2 = cx + r2 * math.cos(rad)
        y2 = cy + r2 * math.sin(rad)
        # 罗马刻度大刻线
        line_w = int(10 * scale) if i % 3 == 0 else int(6 * scale)
        color = (235, 180, 50, 255) if i % 3 == 0 else (180, 50, 50, 220)
        draw.line([(x1, y1), (x2, y2)], fill=color, width=line_w)

    # 4. 次级金色圆环 (Golden Bough Ring)
    r_gold_ring = r_outer - int(42 * scale)
    draw.ellipse([cx - r_gold_ring, cy - r_gold_ring, cx + r_gold_ring, cy + r_gold_ring], outline=(212, 160, 40, 240), width=int(6 * scale))

    # 5. 赤红边狱内盘 (Crimson Limbus Inner Dial)
    r_crimson = r_outer - int(55 * scale)
    draw.ellipse([cx - r_crimson, cy - r_crimson, cx + r_crimson, cy + r_crimson], fill=(36, 8, 12, 255), outline=(156, 8, 11, 220), width=int(10 * scale))

    # 6. 但丁火焰与罗马钟表中心刻度 (Dante Clock Ticks)
    r_inner_ticks = r_crimson - int(35 * scale)
    for i in range(60):
        if i % 5 != 0:
            rad = math.radians(i * 6 - 90)
            x1 = cx + (r_inner_ticks - int(8 * scale)) * math.cos(rad)
            y1 = cy + (r_inner_ticks - int(8 * scale)) * math.sin(rad)
            x2 = cx + r_inner_ticks * math.cos(rad)
            y2 = cy + r_inner_ticks * math.sin(rad)
            draw.line([(x1, y1), (x2, y2)], fill=(120, 80, 40, 150), width=int(2 * scale))

    # 7. 绘制 12 罪人罗马数字 (Roman Numerals)
    roman_numerals = ["XII", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI"]
    try:
        font_size = int(28 * scale)
        font = ImageFont.truetype("arial.ttf", font_size)
    except Exception:
        font = ImageFont.load_default()

    r_num = r_crimson - int(24 * scale)
    for i, num in enumerate(roman_numerals):
        angle_deg = i * 30 - 90
        rad = math.radians(angle_deg)
        nx = cx + r_num * math.cos(rad)
        ny = cy + r_num * math.sin(rad)
        # 计算文本尺寸居中
        bbox = draw.textbbox((0, 0), num, font=font)
        tw = bbox[2] - bbox[0]
        th = bbox[3] - bbox[1]
        col = (255, 215, 80, 255) if i == 0 else (220, 190, 140, 220)
        draw.text((nx - tw / 2, ny - th / 2), num, font=font, fill=col)

    # 8. 绘制亚哈之矛与但丁时钟指针 (Ahab Harpoon & Dante Clock Hand)
    # 8.1 背后罗盘十字星芒 (Compass Star)
    r_star = int(r_crimson * 0.7)
    star_points_v = [(cx, cy - r_star), (cx + int(14 * scale), cy), (cx, cy + r_star), (cx - int(14 * scale), cy)]
    star_points_h = [(cx - r_star, cy), (cx, cy + int(14 * scale)), (cx + r_star, cy), (cx, cy - int(14 * scale))]
    draw.polygon(star_points_v, fill=(180, 40, 40, 140))
    draw.polygon(star_points_h, fill=(180, 40, 40, 140))

    # 8.2 亚哈传奇鱼叉主针 (Ahab Harpoon Main Needle - 指向上方)
    harpoon_len = int(r_crimson * 0.85)
    harpoon_w = int(22 * scale)
    
    # 鱼叉尖矛头 (Harpoon Spearhead)
    spear_tip = (cx, cy - harpoon_len)
    spear_left = (cx - int(38 * scale), cy - harpoon_len + int(70 * scale))
    spear_barb_left = (cx - int(20 * scale), cy - harpoon_len + int(50 * scale))
    spear_right = (cx + int(38 * scale), cy - harpoon_len + int(70 * scale))
    spear_barb_right = (cx + int(20 * scale), cy - harpoon_len + int(50 * scale))
    spear_base = (cx, cy - harpoon_len + int(100 * scale))

    draw.polygon([spear_tip, spear_left, spear_barb_left, spear_base, spear_barb_right, spear_right], fill=(245, 190, 50, 255), outline=(120, 20, 20, 255))

    # 鱼叉柄身 (Harpoon Shaft)
    draw.polygon([
        (cx - harpoon_w // 2, cy - harpoon_len + int(90 * scale)),
        (cx + harpoon_w // 2, cy - harpoon_len + int(90 * scale)),
        (cx + int(8 * scale), cy + int(60 * scale)),
        (cx - int(8 * scale), cy + int(60 * scale))
    ], fill=(210, 160, 40, 255), outline=(100, 15, 20, 255))

    # 8.3 红色分针 (Crimson Minute Needle - 指向右上方 40度)
    m_rad = math.radians(42 - 90)
    m_len = int(r_crimson * 0.6)
    m_px = cx + m_len * math.cos(m_rad)
    m_py = cy + m_len * math.sin(m_rad)
    m_ortho_x = math.cos(m_rad + math.pi / 2) * int(12 * scale)
    m_ortho_y = math.sin(m_rad + math.pi / 2) * int(12 * scale)
    draw.polygon([
        (m_px, m_py),
        (cx + m_ortho_x, cy + m_ortho_y),
        (cx - m_len * 0.2 * math.cos(m_rad), cy - m_len * 0.2 * math.sin(m_rad)),
        (cx - m_ortho_x, cy - m_ortho_y)
    ], fill=(220, 40, 45, 240), outline=(255, 215, 0, 255), width=int(2 * scale))

    # 9. 核心炽热齿轮与锁链中轴 (Core Burning Core & Dante Hub)
    r_core = int(50 * scale)
    # 外层金色锯齿环
    draw.ellipse([cx - r_core - int(8 * scale), cy - r_core - int(8 * scale), cx + r_core + int(8 * scale), cy + r_core + int(8 * scale)], fill=(180, 130, 20, 255))
    # 核心深红/烈焰渐变圆球
    draw.ellipse([cx - r_core, cy - r_core, cx + r_core, cy + r_core], fill=(160, 10, 15, 255), outline=(255, 220, 100, 255), width=int(5 * scale))
    # 中心高光聚焦点
    r_dot = int(22 * scale)
    draw.ellipse([cx - r_dot, cy - r_dot, cx + r_dot, cy + r_dot], fill=(255, 235, 120, 255))

    # 10. 高质量下采样到目标尺寸 (Lanczos Antialiasing)
    final_img = img.resize((size, size), Image.Resampling.LANCZOS)
    return final_img

# 生成 1024x1024 预览与存储
os.makedirs("assets/generated_icons", exist_ok=True)
icon = create_ahab_limbus_icon(1024)
icon.save("assets/generated_icons/Ahab_Limbus_Emblem_1024.png", "PNG")
icon.save("assets/icon_candidates/00_Custom_Ahab_Limbus_Fusion_Emblem.png", "PNG")

print("Generated custom emblem icon successfully!")
