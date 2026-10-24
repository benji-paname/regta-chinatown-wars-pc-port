"""Generates high-CTR, professional 1280x720 YouTube Thumbnails in ENGLISH,
strictly built from the 4 user-supplied assets:
1. Chinatown alley gameplay frame (media_1792884471951_37833b29.jpg / chinatown_clean.png)
2. Angled card editorial layout (media_1792884471943_bc384f1a.png)
3. Authentic vertical ReGTA CTW logo (media_1792884480055_54c1b1d6.png)
4. Authentic horizontal ReGTA CTW logo (media_1792884483430_bad07ee5.png)
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont, ImageFilter

ROOT = Path(__file__).resolve().parents[2]
USER_BASES = ROOT / "outputs" / "CTW-Rust" / "video" / "user_bases"
OUT_DIR = ROOT / "outputs" / "CTW-Rust" / "video" / "thumbnails"
OUT_DIR.mkdir(parents=True, exist_ok=True)

# Use the pixel-perfect raw 97s Chinatown frame (exact match to user Image 2, but uncompressed)
GAMEPLAY_CLEAN = USER_BASES / "chinatown_raw_97s.png"
if not GAMEPLAY_CLEAN.exists():
    GAMEPLAY_CLEAN = USER_BASES / "gameplay_chinatown.jpg"

LOGO_VERT = USER_BASES / "logo_vertical.png"
LOGO_HORIZ = USER_BASES / "logo_horizontal.png"

FONT_BOLD = r"C:\Windows\Fonts\arialbd.ttf"
FONT_REG = r"C:\Windows\Fonts\arial.ttf"
FONT_IMPACT = r"C:\Windows\Fonts\impact.ttf"

W, H = 1280, 720


def add_drop_shadow(image: Image.Image, offset=(0, 10), background_color=(0, 0, 0, 180), blur_radius=15):
    shadow = Image.new("RGBA", (image.width + blur_radius * 4, image.height + blur_radius * 4), (0, 0, 0, 0))
    alpha = image.split()[3]
    shadow_mask = Image.new("L", shadow.size, 0)
    shadow_mask.paste(alpha, (blur_radius * 2 + offset[0], blur_radius * 2 + offset[1]))
    shadow_mask = shadow_mask.filter(ImageFilter.GaussianBlur(blur_radius))
    
    shadow_layer = Image.new("RGBA", shadow.size, background_color)
    shadow.paste(shadow_layer, (0, 0), mask=shadow_mask)
    shadow.paste(image, (blur_radius * 2, blur_radius * 2), mask=image)
    return shadow, blur_radius * 2


def build_option_1_en():
    """Option 1 (English): Chinatown Action + Large Horizontal Logo + 'A NATIVE PC FUTURE'"""
    bg = Image.open(GAMEPLAY_CLEAN).convert("RGBA").resize((W, H), Image.Resampling.LANCZOS)
    
    # Gradient overlay for contrast
    overlay = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(overlay)
    
    # Top gradient to frame logo
    for y in range(250):
        alpha = int(190 * (1 - y / 250))
        d.line([(0, y), (W, y)], fill=(5, 7, 10, alpha))
        
    # Bottom gradient to anchor lower third
    for y in range(500, H):
        alpha = int(220 * ((y - 500) / (H - 500)))
        d.line([(0, y), (W, y)], fill=(5, 7, 10, alpha))
        
    bg = Image.alpha_composite(bg, overlay)
    
    # Logo Horizontal
    logo_im = Image.open(LOGO_HORIZ).convert("RGBA")
    target_w = 640
    target_h = int(logo_im.height * (target_w / logo_im.width))
    logo_im = logo_im.resize((target_w, target_h), Image.Resampling.LANCZOS)
    
    shadow_logo, pad = add_drop_shadow(logo_im, offset=(0, 12), blur_radius=18)
    bg.paste(shadow_logo, (40 - pad, 35 - pad), mask=shadow_logo)
    
    d = ImageDraw.Draw(bg)
    
    # Tag 'PRE-ALPHA / NATIVE PC' in top right (same as Image 2)
    tag_box = (880, 40, 1240, 92)
    d.rounded_rectangle(tag_box, radius=8, fill=(10, 14, 20, 235), outline=(255, 218, 34, 180), width=2)
    d.ellipse((905, 58, 921, 74), fill=(217, 0, 0, 255))
    font_tag = ImageFont.truetype(FONT_BOLD, 22)
    d.text((935, 52), "PRE-ALPHA  /  NATIVE PC", font=font_tag, fill=(255, 255, 255))
    
    # Main Lower Title Block in English
    banner_y = 525
    # Red marker
    d.rectangle((45, banner_y + 10, 53, banner_y + 135), fill="#d90000")
    
    font_label = ImageFont.truetype(FONT_BOLD, 24)
    d.text((68, banner_y + 8), "FIRST LOOK", font=font_label, fill="#d90000")
    
    font_main = ImageFont.truetype(FONT_IMPACT, 72)
    d.text((71, banner_y + 40), "A NATIVE PC FUTURE", font=font_main, fill=(0, 0, 0, 230))
    d.text((68, banner_y + 37), "A NATIVE PC FUTURE", font=font_main, fill="#ffffff")
    
    font_sub = ImageFont.truetype(FONT_BOLD, 26)
    d.text((68, banner_y + 118), "100% RUST MACHINE CODE  •  UNCAPPED HIGH-REFRESH FPS", font=font_sub, fill="#ffda22")
    
    out_path = OUT_DIR / "thumbnail_option1_chinatown_action_en.png"
    bg.save(out_path, "PNG")
    print(f"Generated {out_path.name}")
    return out_path


def build_option_2_en():
    """Option 2 (English): Angled Split Card Layout (Image 1) + Chinatown Gameplay (Image 2) + Vertical Logo"""
    bg = Image.open(GAMEPLAY_CLEAN).convert("RGBA").resize((W, H), Image.Resampling.LANCZOS)
    
    overlay = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(overlay)
    
    # Dark veil over the right side
    d.rectangle([(0, 0), (W, H)], fill=(10, 12, 16, 175))
    
    # Solid angled polygon on left exactly like Image 1
    d.polygon([(0, 0), (590, 0), (420, H), (0, H)], fill=(24, 18, 16, 250))
    # Red accent line along the diagonal
    d.line([(590, 0), (420, H)], fill="#d90000", width=4)
    
    bg = Image.alpha_composite(bg, overlay)
    d = ImageDraw.Draw(bg)
    
    # Logo Vertical inside left angled cut
    logo_im = Image.open(LOGO_VERT).convert("RGBA")
    target_w = 420
    target_h = int(logo_im.height * (target_w / logo_im.width))
    logo_im = logo_im.resize((target_w, target_h), Image.Resampling.LANCZOS)
    
    shadow_logo, pad = add_drop_shadow(logo_im, offset=(0, 10), blur_radius=15)
    bg.paste(shadow_logo, (35 - pad, 130 - pad), mask=shadow_logo)
    
    # Right side: Editorial text based on Image 1 in English
    font_h1 = ImageFont.truetype(FONT_BOLD, 68)
    font_h2 = ImageFont.truetype(FONT_BOLD, 68)
    
    rx = 540
    d.text((rx, 140), "A NATIVE", font=font_h1, fill="#ffffff")
    d.text((rx, 215), "PC FUTURE.", font=font_h2, fill="#ffda22")
    
    font_author = ImageFont.truetype(FONT_BOLD, 30)
    d.text((rx + 5, 325), "by Benji Paname", font=font_author, fill="#ffffff")
    
    # Red divider
    d.rectangle([(rx + 5, 385), (1220, 390)], fill="#d90000")
    
    # Feature points in English
    font_feat_title = ImageFont.truetype(FONT_BOLD, 26)
    font_feat_sub = ImageFont.truetype(FONT_REG, 24)
    
    d.text((rx + 5, 425), "100% CLEAN-ROOM RUST PORT", font=font_feat_title, fill="#ffda22")
    d.text((rx + 5, 460), "No PSP emulator  •  Zero CPU translation overhead", font=font_feat_sub, fill="#e2e8f0")
    
    d.text((rx + 5, 520), "REAL NATIVE GAMEPLAY", font=font_feat_title, fill="#ffda22")
    d.text((rx + 5, 555), "Uncapped high-refresh rate framerate  •  4K native", font=font_feat_sub, fill="#e2e8f0")
    
    # Bottom support pill
    support_box = (rx + 5, 620, 1220, 670)
    d.rounded_rectangle(support_box, radius=8, fill=(225, 29, 72, 220))
    font_support = ImageFont.truetype(FONT_BOLD, 22)
    d.text((rx + 30, 632), "SUPPORT THE PROJECT  •  TIPEEE & PAYPAL LINKS BELOW", font=font_support, fill="#ffffff")
    
    out_path = OUT_DIR / "thumbnail_option2_editorial_card_en.png"
    bg.save(out_path, "PNG")
    print(f"Generated {out_path.name}")
    return out_path


def build_option_3_en():
    """Option 3 (English): Chinatown Action + Logo Horizontal + Giant 'FINALLY ON PC !' Hook Banner"""
    bg = Image.open(GAMEPLAY_CLEAN).convert("RGBA").resize((W, H), Image.Resampling.LANCZOS)
    
    overlay = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(overlay)
    
    for y in range(H):
        dist_y = abs(y - H // 2) / (H // 2)
        alpha = int(140 * (dist_y ** 1.6))
        d.line([(0, y), (W, y)], fill=(5, 7, 10, alpha))
        
    bg = Image.alpha_composite(bg, overlay)
    
    # Logo Horizontal at the top
    logo_im = Image.open(LOGO_HORIZ).convert("RGBA")
    target_w = 640
    target_h = int(logo_im.height * (target_w / logo_im.width))
    logo_im = logo_im.resize((target_w, target_h), Image.Resampling.LANCZOS)
    
    shadow_logo, pad = add_drop_shadow(logo_im, offset=(0, 10), blur_radius=16)
    bg.paste(shadow_logo, ((W - target_w) // 2 - pad, 40 - pad), mask=shadow_logo)
    
    d = ImageDraw.Draw(bg)
    
    # Giant Center Hook Banner: 'FINALLY ON PC !'
    banner_w, banner_h = 780, 115
    bx = (W - banner_w) // 2
    by = 310
    
    d.rounded_rectangle([(bx + 8, by + 12), (bx + banner_w + 8, by + banner_h + 12)], radius=18, fill=(0, 0, 0, 220))
    d.rounded_rectangle([(bx, by), (bx + banner_w, by + banner_h)], radius=18, fill="#e11d48", outline="#ffffff", width=4)
    
    font_banner = ImageFont.truetype(FONT_IMPACT, 78)
    text_banner = "FINALLY ON PC !"
    tw = d.textlength(text_banner, font=font_banner)
    d.text((bx + (banner_w - tw) // 2, by + 10), text_banner, font=font_banner, fill="#ffffff")
    
    # Bottom Row of 3 Badges in English
    badge_y = 540
    bw = 360
    bh = 72
    gap = 25
    total_w = bw * 3 + gap * 2
    start_x = (W - total_w) // 2
    
    badges = [
        ("UNCAPPED FPS • 4K", "#10b981", "#ffffff"),
        ("NO EMULATOR", "#0f172a", "#ffd700"),
        ("100% RUST PORT", "#0f172a", "#ffd700"),
    ]
    
    font_badge = ImageFont.truetype(FONT_BOLD, 22)
    for idx, (label, bg_col, text_col) in enumerate(badges):
        x = start_x + idx * (bw + gap)
        d.rounded_rectangle([(x + 4, badge_y + 6), (x + bw + 4, badge_y + bh + 6)], radius=12, fill=(0, 0, 0, 180))
        d.rounded_rectangle([(x, badge_y), (x + bw, badge_y + bh)], radius=12, fill=bg_col, outline="#ffffff" if bg_col == "#10b981" else "#ffd700", width=2)
        tl = d.textlength(label, font=font_badge)
        d.text((x + (bw - tl) // 2, badge_y + 22), label, font=font_badge, fill=text_col)
        
    font_author = ImageFont.truetype(FONT_BOLD, 22)
    d.text((W // 2 - 80, 660), "by Benji Paname", font=font_author, fill="#ffffff")
    
    out_path = OUT_DIR / "thumbnail_option3_finally_on_pc_en.png"
    bg.save(out_path, "PNG")
    print(f"Generated {out_path.name}")
    return out_path


def main():
    print("--- Generating Thumbnails From User Bases (ENGLISH) ---")
    o1 = build_option_1_en()
    o2 = build_option_2_en()
    o3 = build_option_3_en()
    
    # Combined comparison board
    im1 = Image.open(o1)
    im2 = Image.open(o2)
    im3 = Image.open(o3)
    
    board = Image.new("RGB", (W, H * 3), (12, 14, 18))
    board.paste(im1, (0, 0))
    board.paste(im2, (0, H))
    board.paste(im3, (0, H * 2))
    
    board_path = OUT_DIR / "user_bases_thumbnails_comparison_en.jpg"
    board.save(board_path, quality=88)
    print(f"Comparison board saved: {board_path}")
    
    # Synchronize default thumbnail to Option 1
    (ROOT / "docs" / "assets" / "thumbnail.png").write_bytes(o1.read_bytes())
    (ROOT / "outputs" / "CTW-Rust" / "video" / "thumbnail.png").write_bytes(o1.read_bytes())
    dest_launch = ROOT / "outputs" / "CTW-Rust" / "video" / "launch-film" / "thumbnail.png"
    if dest_launch.parent.exists():
        dest_launch.write_bytes(o1.read_bytes())
    print("Synchronized Option 1 (English) to docs/assets/thumbnail.png and video paths")


if __name__ == "__main__":
    main()
