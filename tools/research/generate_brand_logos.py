import os
import shutil
import subprocess
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen

pricedown = TTFont('target/private-fonts/pricedown/Pricedown Bl.otf')
bruce_reg = TTFont('target/private-fonts/bruce-mikita/BRUCM___.TTF')
bruce_solid = TTFont('target/private-fonts/bruce-mikita/Bruce Mikita Two.ttf')

def text_to_svg_path(font, text, x, y, scale, tracking=0):
    glyphs = font.getGlyphSet()
    cmap = font.getBestCmap()
    pen = SVGPathPen(glyphs)
    adv = 0
    for i, ch in enumerate(text):
        g = glyphs[cmap[ord(ch)]]
        t = (scale, 0, 0, -scale, x + adv * scale, y)
        g.draw(TransformPen(pen, t))
        adv += g.width
        if i < len(text) - 1:
            adv += tracking
    return pen.getCommands(), adv * scale

def get_text_metrics(font, text, scale, tracking=0):
    glyphs = font.getGlyphSet()
    cmap = font.getBestCmap()
    bounds = BoundsPen(glyphs)
    adv = 0
    for i, ch in enumerate(text):
        g = glyphs[cmap[ord(ch)]]
        t = (1, 0, 0, 1, adv, 0)
        g.draw(TransformPen(bounds, t))
        adv += g.width
        if i < len(text) - 1:
            adv += tracking
    x0, y0, x1, y1 = bounds.bounds
    return {
        'x0': x0 * scale,
        'y0': y0 * scale,
        'x1': x1 * scale,
        'y1': y1 * scale,
        'width': (x1 - x0) * scale,
        'height': (y1 - y0) * scale,
        'advance': adv * scale,
    }

DEFS = '''
    <defs>
      <!-- Recomp Red Gradient for "Re" -->
      <linearGradient id="regRedGrad" x1="0%" y1="0%" x2="0%" y2="100%">
        <stop offset="0%" stop-color="#FF3B30" />
        <stop offset="35%" stop-color="#E52521" />
        <stop offset="75%" stop-color="#B91C1C" />
        <stop offset="100%" stop-color="#7F1D1D" />
      </linearGradient>

      <!-- Classic GTA Pure White / Platinum Gradient -->
      <linearGradient id="gtaWhiteGrad" x1="0%" y1="0%" x2="0%" y2="100%">
        <stop offset="0%" stop-color="#FFFFFF" />
        <stop offset="25%" stop-color="#FFFFFF" />
        <stop offset="70%" stop-color="#F1F5F9" />
        <stop offset="100%" stop-color="#CBD5E1" />
      </linearGradient>

      <!-- Chinatown Wars Authentic Lacquer Red Gradient -->
      <linearGradient id="ctwRedGrad" x1="0%" y1="0%" x2="0%" y2="100%">
        <stop offset="0%" stop-color="#F82B2B" />
        <stop offset="30%" stop-color="#DC1414" />
        <stop offset="75%" stop-color="#B91010" />
        <stop offset="100%" stop-color="#7A0808" />
      </linearGradient>

      <!-- Chinatown Wars Authentic Imperial Gold / Yellow Gradient -->
      <linearGradient id="ctwGoldGrad" x1="0%" y1="0%" x2="0%" y2="100%">
        <stop offset="0%" stop-color="#FFF875" />
        <stop offset="25%" stop-color="#FFD700" />
        <stop offset="70%" stop-color="#FFA800" />
        <stop offset="100%" stop-color="#D97706" />
      </linearGradient>
    </defs>
'''

def build_horizontal_svg(transparent=False):
    width, height = 960, 260
    scale_regta = 0.175
    y_regta = 158
    x_regta = 75
    tracking_p = -20

    p_re, adv_re = text_to_svg_path(pricedown, "Re", x_regta, y_regta, scale_regta, tracking_p)
    x_gta = x_regta + adv_re + (tracking_p * scale_regta)
    p_gta, adv_gta = text_to_svg_path(pricedown, "GTA", x_gta, y_regta, scale_regta, tracking_p)
    p_regta_all, _ = text_to_svg_path(pricedown, "ReGTA", x_regta, y_regta, scale_regta, tracking_p)

    scale_ctw = 0.175
    y_ctw = 156
    x_ctw = 580
    tracking_b = 0

    p_ctw_solid, _ = text_to_svg_path(bruce_solid, "CTW", x_ctw, y_ctw, scale_ctw, tracking_b)
    p_ctw_reg, _ = text_to_svg_path(bruce_reg, "CTW", x_ctw, y_ctw, scale_ctw, tracking_b)

    bg_elements = ''
    if not transparent:
        bg_elements = '''
  <rect width="100%" height="100%" fill="#08090d" rx="16" />
  <circle cx="280" cy="125" r="190" fill="radial-gradient(circle, rgba(229,37,33,0.12) 0%, transparent 70%)" />
  <circle cx="730" cy="125" r="190" fill="radial-gradient(circle, rgba(255,200,0,0.08) 0%, transparent 70%)" />
'''

    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="100%" height="100%" role="img">
  <title>ReGTA : CTW — PSP Recomp</title>
  <desc>Official Project Horizontal Logo — ReGTA CTW PSP Recompilation by Benji Paname</desc>
  {bg_elements}
  {DEFS}

  <!-- ========================================================= -->
  <!-- 1. LEFT: ReGTA (Pricedown Black, integrated, lowercase e)  -->
  <!-- ========================================================= -->
  <path d="{p_regta_all}" fill="#000000" stroke="#000000" stroke-width="16" stroke-linejoin="round" transform="translate(6 8)" />
  <path d="{p_re}" fill="url(#regRedGrad)" stroke="#050608" stroke-width="10" stroke-linejoin="round" paint-order="stroke fill" />
  <path d="{p_gta}" fill="url(#gtaWhiteGrad)" stroke="#050608" stroke-width="10" stroke-linejoin="round" paint-order="stroke fill" />

  <!-- Stylized Chinatown Gold Diamond Separator -->
  <g transform="translate(535, 118)">
    <polygon points="0,-16 6,-22 0,-28 -6,-22" fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="2" />
    <polygon points="0,22 6,16 0,10 -6,16" fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="2" />
  </g>

  <!-- ========================================================= -->
  <!-- 2. RIGHT: CTW (Authentic Bruce Mikita, Cover Red & Gold)  -->
  <!-- ========================================================= -->
  <path d="{p_ctw_solid}" fill="#000000" stroke="#000000" stroke-width="18" stroke-linejoin="round" transform="translate(6 8)" />
  <path d="{p_ctw_solid}" fill="url(#ctwGoldGrad)" stroke="url(#ctwGoldGrad)" stroke-width="10" stroke-linejoin="round" />
  <path d="{p_ctw_reg}" fill="url(#ctwRedGrad)" stroke="#660505" stroke-width="1.5" stroke-linejoin="round" />

  <!-- ========================================================= -->
  <!-- 3. SUBTITLE: PSP RECOMP                                   -->
  <!-- ========================================================= -->
  <text x="480" y="224" text-anchor="middle" fill="#94a3b8" font-family="'Segoe UI',-apple-system,sans-serif" font-size="13" font-weight="700" letter-spacing="8">PSP RECOMPILATION</text>
</svg>
'''

def build_vertical_svg(transparent=False):
    width, height = 900, 500
    scale_regta = 0.24
    y_regta = 185
    tracking_p = -20

    metrics_regta = get_text_metrics(pricedown, "ReGTA", scale_regta, tracking_p)
    x_regta = (width - metrics_regta['advance']) / 2

    p_re, adv_re = text_to_svg_path(pricedown, "Re", x_regta, y_regta, scale_regta, tracking_p)
    x_gta = x_regta + adv_re + (tracking_p * scale_regta)
    p_gta, adv_gta = text_to_svg_path(pricedown, "GTA", x_gta, y_regta, scale_regta, tracking_p)
    p_regta_all, _ = text_to_svg_path(pricedown, "ReGTA", x_regta, y_regta, scale_regta, tracking_p)

    scale_ctw = 0.22
    y_ctw = 358
    tracking_b = 0

    metrics_ctw = get_text_metrics(bruce_solid, "CTW", scale_ctw, tracking_b)
    x_ctw = (width - metrics_ctw['advance']) / 2

    p_ctw_solid, _ = text_to_svg_path(bruce_solid, "CTW", x_ctw, y_ctw, scale_ctw, tracking_b)
    p_ctw_reg, _ = text_to_svg_path(bruce_reg, "CTW", x_ctw, y_ctw, scale_ctw, tracking_b)

    bg_elements = ''
    if not transparent:
        bg_elements = '''
  <rect width="100%" height="100%" fill="#08090d" rx="20" />
  <circle cx="450" cy="250" r="280" fill="radial-gradient(circle, rgba(220,20,20,0.14) 0%, rgba(255,200,0,0.05) 50%, transparent 70%)" />
'''

    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="100%" height="100%" role="img">
  <title>ReGTA CTW — PSP Recomp</title>
  <desc>Official Vertical Stacked Project Logo — ReGTA CTW PSP Recompilation by Benji Paname</desc>
  {bg_elements}
  {DEFS}

  <!-- ========================================================= -->
  <!-- 1. TOP: ReGTA (Pricedown Black, integrated, lowercase e)  -->
  <!-- ========================================================= -->
  <path d="{p_regta_all}" fill="#000000" stroke="#000000" stroke-width="18" stroke-linejoin="round" transform="translate(6 9)" />
  <path d="{p_re}" fill="url(#regRedGrad)" stroke="#050608" stroke-width="11" stroke-linejoin="round" paint-order="stroke fill" />
  <path d="{p_gta}" fill="url(#gtaWhiteGrad)" stroke="#050608" stroke-width="11" stroke-linejoin="round" paint-order="stroke fill" />

  <!-- ========================================================= -->
  <!-- 2. BOTTOM: CTW (en bas du ReGTA, authentic Red & Gold)    -->
  <!-- ========================================================= -->
  <path d="{p_ctw_solid}" fill="#000000" stroke="#000000" stroke-width="20" stroke-linejoin="round" transform="translate(6 9)" />
  <path d="{p_ctw_solid}" fill="url(#ctwGoldGrad)" stroke="url(#ctwGoldGrad)" stroke-width="12" stroke-linejoin="round" />
  <path d="{p_ctw_reg}" fill="url(#ctwRedGrad)" stroke="#660505" stroke-width="1.8" stroke-linejoin="round" />

  <!-- ========================================================= -->
  <!-- 3. SUBTITLE: PSP RECOMP                                   -->
  <!-- ========================================================= -->
  <text x="450" y="440" text-anchor="middle" fill="#94a3b8" font-family="'Segoe UI',-apple-system,sans-serif" font-size="15" font-weight="700" letter-spacing="10">PSP RECOMPILATION</text>
</svg>
'''

def render_png(svg_path, png_path, width, height, transparent=False):
    edge_exe = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
    bg_color = "--default-background-color=00000000" if transparent else "--default-background-color=ff08090d"
    cmd = [
        edge_exe,
        "--headless",
        r"--user-data-dir=C:\Users\BENJAMIN\AppData\Local\Temp\edge_temp",
        bg_color,
        f"--screenshot={png_path.resolve()}",
        f"--window-size={width},{height}",
        f"file:///{svg_path.resolve().as_posix()}"
    ]
    subprocess.run(cmd, check=True)
    print(f"Rendered PNG: {png_path.name} ({width}x{height})")

def main():
    assets_dir = Path('docs/assets')
    deliverables_dir = Path('outputs/CTW-Rust/branding')
    assets_dir.mkdir(parents=True, exist_ok=True)
    deliverables_dir.mkdir(parents=True, exist_ok=True)

    # 1. Generate SVGs
    svg_h_bg = assets_dir / 'logo-ctw.svg'
    svg_h_tr = assets_dir / 'logo-ctw-transparent.svg'
    svg_v_bg = assets_dir / 'logo.svg'
    svg_v_tr = assets_dir / 'logo-transparent.svg'

    svg_h_bg.write_text(build_horizontal_svg(transparent=False), encoding='utf-8')
    svg_h_tr.write_text(build_horizontal_svg(transparent=True), encoding='utf-8')
    svg_v_bg.write_text(build_vertical_svg(transparent=False), encoding='utf-8')
    svg_v_tr.write_text(build_vertical_svg(transparent=True), encoding='utf-8')

    print("Generated 4 SVG files.")

    # 2. Render PNGs (2x High-Resolution)
    # Horizontal: 960x260 -> 1920x520
    render_png(svg_h_bg, assets_dir / 'logo-ctw.png', 1920, 520, transparent=False)
    render_png(svg_h_tr, assets_dir / 'logo-ctw-transparent.png', 1920, 520, transparent=True)

    # Vertical: 900x500 -> 1800x1000
    render_png(svg_v_bg, assets_dir / 'logo.png', 1800, 1000, transparent=False)
    render_png(svg_v_tr, assets_dir / 'logo-transparent.png', 1800, 1000, transparent=True)

    # 3. Synchronize to outputs/CTW-Rust/branding
    files_to_sync = [
        'logo.png', 'logo-transparent.png',
        'logo-ctw.png', 'logo-ctw-transparent.png',
        'logo.svg', 'logo-transparent.svg',
        'logo-ctw.svg', 'logo-ctw-transparent.svg'
    ]
    for fname in files_to_sync:
        shutil.copy2(assets_dir / fname, deliverables_dir / fname)

    print("All branding PNGs and SVGs synchronized to outputs/CTW-Rust/branding/!")

if __name__ == '__main__':
    main()
