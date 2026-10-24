"""Create fixed SVG logo artwork from user-supplied fonts (requires fontTools).

Font software is private and is never embedded in the generated artwork.
See docs/BRANDING.md for provenance and publication considerations.
"""

import argparse
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen


def lettering(font, text, cx, top, max_width, max_height, fill):
    glyphs = font.getGlyphSet()
    cmap = font.getBestCmap()
    placed = []
    advance = 0
    bounds = BoundsPen(glyphs)
    for character in text:
        glyph = glyphs[cmap[ord(character)]]
        transform = (1, 0, 0, 1, advance, 0)
        glyph.draw(TransformPen(bounds, transform))
        placed.append((glyph, advance))
        advance += glyph.width
    x0, y0, x1, y1 = bounds.bounds
    scale = min(max_width / (x1 - x0), max_height / (y1 - y0))
    left = cx - (x1 - x0) * scale / 2
    pen = SVGPathPen(glyphs)
    for glyph, offset in placed:
        glyph.draw(TransformPen(pen, (scale, 0, 0, -scale,
                                     left + (offset - x0) * scale,
                                     top + y1 * scale)))
    # One fixed wordmark path, no reusable addressable glyph library.
    path = pen.getCommands()
    return (f'<path d="{path}" fill="#000" stroke="#000" stroke-width="13" '
            'stroke-linejoin="round" transform="translate(4 6)"/>'
            f'<path d="{path}" fill="{fill}" stroke="#050608" '
            'stroke-width="8" stroke-linejoin="round" paint-order="stroke fill"/>')


def svg(width, height, content, title):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" '
            f'width="{width}" height="{height}" role="img">\n'
            f'<title>{title}</title>\n'
            '<desc>Independent fan project. Fixed lettering from user-supplied '
            'Pricedown Black and Bruce Mikita fonts. Not an official Rockstar logo.</desc>\n'
            + '\n'.join(content) + '\n</svg>\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pricedown', type=Path, required=True)
    parser.add_argument('--bruce-mikita', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=Path('docs/assets'))
    args = parser.parse_args()
    gta = TTFont(args.pricedown)
    ctw = TTFont(args.bruce_mikita)
    args.output.mkdir(parents=True, exist_ok=True)
    main_logo = [
        '<text x="157" y="112" fill="#ee4944" font-family="Arial,sans-serif" '
        'font-size="64" font-weight="900" transform="rotate(-8 157 112)">Re</text>',
        lettering(gta, 'GTA', 490, 25, 440, 175, '#fffdf6'),
        lettering(ctw, 'CHINATOWN', 450, 230, 730, 86, '#f2c35b'),
        lettering(ctw, 'WARS', 450, 335, 310, 78, '#f2c35b'),
        '<text x="450" y="455" text-anchor="middle" fill="#d8d9dd" '
        'font-family="Arial,sans-serif" font-size="17" letter-spacing="6">PSP RECOMP</text>',
    ]
    compact = [
        '<text x="40" y="106" fill="#ee4944" font-family="Arial,sans-serif" '
        'font-size="56" font-weight="900" transform="rotate(-8 40 106)">Re</text>',
        lettering(gta, 'GTA', 294, 35, 300, 130, '#fffdf6'),
        lettering(ctw, 'CTW', 650, 48, 350, 120, '#f2c35b'),
        '<text x="450" y="229" text-anchor="middle" fill="#d8d9dd" '
        'font-family="Arial,sans-serif" font-size="17" letter-spacing="6">PSP RECOMP</text>',
    ]
    (args.output / 'logo.svg').write_text(svg(900, 480, main_logo, 'ReGTA Chinatown Wars — PSP Recomp'), encoding='utf-8')
    (args.output / 'logo-ctw.svg').write_text(svg(900, 260, compact, 'ReGTA CTW — PSP Recomp'), encoding='utf-8')


if __name__ == '__main__':
    main()
