import os
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
        t = (1, 0, 1, 0)
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

def generate_logos():
    # -------------------------------------------------------------
    # 1. HORIZONTAL LOGO (docs/assets/logo-ctw.svg)
    #    "ReGTA : CTW" on a single horizontal row
    # -------------------------------------------------------------
    width_h, height_h = 960, 260
    scale_regta_h = 0.185
    y_regta_h = 152
    x_regta_h = 65
    tracking_p = -20

    # Pricedown "Re"
    p_re_h, adv_re_h = text_to_svg_path(pricedown, "Re", x_regta_h, y_regta_h, scale_regta_h, tracking_p)
    # Pricedown "GTA" immediately following "Re"
    x_gta_h = x_regta_h + adv_re_h + (tracking_p * scale_regta_h)
    p_gta_h, adv_gta_h = text_to_svg_path(pricedown, "GTA", x_gta_h, y_regta_h, scale_regta_h, tracking_p)
    # Combined "ReGTA" for unified shadow
    p_regta_all_h, _ = text_to_svg_path(pricedown, "ReGTA", x_regta_h, y_regta_h, scale_regta_h, tracking_p)

    # Bruce Mikita "CTW" on right
    scale_ctw_h = 0.185
    y_ctw_h = 150
    x_ctw_h = 585
    tracking_b = 0

    p_ctw_solid_h, adv_ctw_h = text_to_svg_path(bruce_solid, "CTW", x_ctw_h, y_ctw_h, scale_ctw_h, tracking_b)
    p_ctw_reg_h, _ = text_to_svg_path(bruce_reg, "CTW", x_ctw_h, y_ctw_h, scale_ctw_h, tracking_b)

    svg_horizontal = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width_h} {height_h}" width="{width_h}" height="{height_h}" role="img">
  <title>ReGTA : CTW — PSP Recomp</title>
  <desc>Official Project Horizontal Logo — ReGTA CTW PSP Recompilation by Benji Paname</desc>
  <rect width="100%" height="100%" fill="#08090d" rx="16" />
  {DEFS}

  <!-- Ambient Chinatown Glow -->
  <circle cx="280" cy="125" r="190" fill="radial-gradient(circle, rgba(229,37,33,0.12) 0%, transparent 70%)" />
  <circle cx="730" cy="125" r="190" fill="radial-gradient(circle, rgba(255,200,0,0.08) 0%, transparent 70%)" />

  <!-- ========================================================= -->
  <!-- 1. LEFT: ReGTA (Pricedown Black, integrated, lowercase e)  -->
  <!-- ========================================================= -->
  <!-- ReGTA Unified Drop Shadow -->
  <path d="{p_regta_all_h}" fill="#000000" stroke="#000000" stroke-width="16" stroke-linejoin="round" transform="translate(6 8)" />

  <!-- "Re" (Large, Recomp Red, bold black contour) -->
  <path d="{p_re_h}" fill="url(#regRedGrad)" stroke="#050608" stroke-width="10" stroke-linejoin="round" paint-order="stroke fill" />

  <!-- "GTA" (Classic White, bold black contour) -->
  <path d="{p_gta_h}" fill="url(#gtaWhiteGrad)" stroke="#050608" stroke-width="10" stroke-linejoin="round" paint-order="stroke fill" />

  <!-- Stylized Chinatown Gold Diamond Separator -->
  <g transform="translate(535, 115)">
    <polygon points="0,-16 6,-22 0,-28 -6,-22" fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="2" />
    <polygon points="0,22 6,16 0,10 -6,16" fill="url(#ctwGoldGrad)" stroke="#000000" stroke-width="2" />
  </g>

  <!-- ========================================================= -->
  <!-- 2. RIGHT: CTW (Authentic Bruce Mikita, Cover Red & Gold)  -->
  <!-- ========================================================= -->
  <!-- CTW Drop Shadow -->
  <path d="{p_ctw_solid_h}" fill="#000000" stroke="#000000" stroke-width="18" stroke-linejoin="round" transform="translate(6 8)" />

  <!-- CTW Layer 1: Golden Yellow Outline & Fill (shows through cutouts) -->
  <path d="{p_ctw_solid_h}" fill="url(#ctwGoldGrad)" stroke="url(#ctwGoldGrad)" stroke-width="10" stroke-linejoin="round" />

  <!-- CTW Layer 2: Authentic Lacquer Red Face (Regular variant with cutouts) -->
  <path d="{p_ctw_reg_h}" fill="url(#ctwRedGrad)" stroke="#660505" stroke-width="1.5" stroke-linejoin="round" />

  <!-- ========================================================= -->
  <!-- 3. SUBTITLE: PSP RECOMP                                   -->
  <!-- ========================================================= -->
  <text x="480" y="222" text-anchor="middle" fill="#94a3b8" font-family="'Segoe UI',-apple-system,sans-serif" font-size="13" font-weight="700" letter-spacing="8">PSP RECOMPILATION</text>
</svg>
'''
    Path('docs/assets/logo-ctw.svg').write_text(svg_horizontal, encoding='utf-8')
    print("Generated docs/assets/logo-ctw.svg (Horizontal)")

    # -------------------------------------------------------------
    # 2. VERTICAL / STACKED LOGO (docs/assets/logo.svg)
    #    "ReGTA" on top, "CTW" underneath ("en bas du ReGTA")
    # -------------------------------------------------------------
    width_v, height_v = 900, 480
    scale_regta_v = 0.25
    y_regta_v = 168

    # Center ReGTA
    metrics_regta = get_text_metrics(pricedown, "ReGTA", scale_regta_v, tracking_p)
    x_regta_v = (width_v - metrics_regta['advance']) / 2

    p_re_v, adv_re_v = text_to_svg_path(pricedown, "Re", x_regta_v, y_regta_v, scale_regta_v, tracking_p)
    x_gta_v = x_regta_v + adv_re_v + (tracking_p * scale_regta_v)
    p_gta_v, adv_gta_v = text_to_svg_path(pricedown, "GTA", x_gta_v, y_regta_v, scale_regta_v, tracking_p)
    p_regta_all_v, _ = text_to_svg_path(pricedown, "ReGTA", x_regta_v, y_regta_v, scale_regta_v, tracking_p)

    # CTW underneath ReGTA (spaced cleanly below ReGTA)
    scale_ctw_v = 0.23
    y_ctw_v = 345
    metrics_ctw = get_text_metrics(bruce_solid, "CTW", scale_ctw_v, tracking_b)
    x_ctw_v = (width_v - metrics_ctw['advance']) / 2

    p_ctw_solid_v, _ = text_to_svg_path(bruce_solid, "CTW", x_ctw_v, y_ctw_v, scale_ctw_v, tracking_b)
    p_ctw_reg_v, _ = text_to_svg_path(bruce_reg, "CTW", x_ctw_v, y_ctw_v, scale_ctw_v, tracking_b)

    svg_vertical = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width_v} {height_v}" width="{width_v}" height="{height_v}" role="img">
  <title>ReGTA CTW — PSP Recomp</title>
  <desc>Official Vertical Stacked Project Logo — ReGTA CTW PSP Recompilation by Benji Paname</desc>
  <rect width="100%" height="100%" fill="#08090d" rx="20" />
  {DEFS}

  <!-- Ambient Chinatown Lacquer Red & Imperial Gold Glow -->
  <circle cx="450" cy="240" r="280" fill="radial-gradient(circle, rgba(220,20,20,0.14) 0%, rgba(255,200,0,0.05) 50%, transparent 70%)" />

  <!-- ========================================================= -->
  <!-- 1. TOP: ReGTA (Pricedown Black, integrated, lowercase e)  -->
  <!-- ========================================================= -->
  <!-- ReGTA Unified Drop Shadow -->
  <path d="{p_regta_all_v}" fill="#000000" stroke="#000000" stroke-width="18" stroke-linejoin="round" transform="translate(6 9)" />

  <!-- "Re" (Large, Recomp Red, bold black contour) -->
  <path d="{p_re_v}" fill="url(#regRedGrad)" stroke="#050608" stroke-width="11" stroke-linejoin="round" paint-order="stroke fill" />

  <!-- "GTA" (Classic White, bold black contour) -->
  <path d="{p_gta_v}" fill="url(#gtaWhiteGrad)" stroke="#050608" stroke-width="11" stroke-linejoin="round" paint-order="stroke fill" />

  <!-- ========================================================= -->
  <!-- 2. BOTTOM: CTW (en bas du ReGTA, authentic Red & Gold)    -->
  <!-- ========================================================= -->
  <!-- CTW Drop Shadow -->
  <path d="{p_ctw_solid_v}" fill="#000000" stroke="#000000" stroke-width="20" stroke-linejoin="round" transform="translate(6 9)" />

  <!-- CTW Layer 1: Golden Yellow Outline & Fill -->
  <path d="{p_ctw_solid_v}" fill="url(#ctwGoldGrad)" stroke="url(#ctwGoldGrad)" stroke-width="12" stroke-linejoin="round" />

  <!-- CTW Layer 2: Authentic Lacquer Red Face (Regular variant with cutouts) -->
  <path d="{p_ctw_reg_v}" fill="url(#ctwRedGrad)" stroke="#660505" stroke-width="1.8" stroke-linejoin="round" />

  <!-- ========================================================= -->
  <!-- 3. SUBTITLE: PSP RECOMP                                   -->
  <!-- ========================================================= -->
  <text x="450" y="425" text-anchor="middle" fill="#94a3b8" font-family="'Segoe UI',-apple-system,sans-serif" font-size="15" font-weight="700" letter-spacing="10">PSP RECOMPILATION</text>
</svg>
'''
    Path('docs/assets/logo.svg').write_text(svg_vertical, encoding='utf-8')
    print("Generated docs/assets/logo.svg (Vertical Stacked)")

if __name__ == '__main__':
    generate_logos()
