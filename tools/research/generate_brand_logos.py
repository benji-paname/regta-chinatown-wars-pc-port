"""Fixed ReGTA CTW wordmarks from private fonts. No browser or font embedding."""
from pathlib import Path
import argparse
from fontTools.ttLib import TTFont
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen

ROOT=Path(__file__).resolve().parents[2]
RED='#d90000'
YELLOW='#ffda22'
WHITE='#fffdf5'

def outline(font,text,cx,top,width,height,lower_e=False):
    glyphs=font.getGlyphSet();cmap=font.getBestCmap();placed=[];advance=0
    bounds=BoundsPen(glyphs)
    for ch in text:
        g=glyphs[cmap[ord(ch)]];k=.78 if lower_e and ch=='e' else 1
        g.draw(TransformPen(bounds,(k,0,0,k,advance,0)))
        placed.append((g,k,advance));advance+=g.width*k
    x0,y0,x1,y1=bounds.bounds;s=min(width/(x1-x0),height/(y1-y0));left=cx-(x1-x0)*s/2
    pen=SVGPathPen(glyphs)
    for g,k,advance in placed:g.draw(TransformPen(pen,(s*k,0,0,-s*k,left+(advance-x0)*s,top+y1*s)))
    return pen.getCommands()

def paint(path,ctw=False):
    # Flat ink and a single yellow keyline, as on the supplied cover reference.
    s=f'<path d="{path}" fill="#080808" stroke="#080808" stroke-width="20" stroke-linejoin="round" transform="translate(3 5)"/>'
    if ctw:s+=f'<path d="{path}" fill="{YELLOW}" stroke="{YELLOW}" stroke-width="8" stroke-linejoin="round"/>'
    s+=f'<path d="{path}" fill="{RED if ctw else WHITE}"'+('' if ctw else ' stroke="#080808" stroke-width="7" paint-order="stroke fill" stroke-linejoin="round"')+'/>'
    return s

def make(gta,ctw,stacked):
    w,h=(900,540) if stacked else (1200,280)
    if stacked:
        paths=[paint(outline(gta,'ReGTA',450,26,780,210,True)),paint(outline(ctw,'CTW',450,265,530,180),True)]
        baseline=510
    else:
        paths=[paint(outline(gta,'ReGTA',373,26,690,174,True)),paint(outline(ctw,'CTW',978,33,365,166),True)]
        baseline=257
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img">
<title>ReGTA CTW — PC Recomp</title><desc>Independent project wordmark. Pricedown Black with a reduced lowercase e; Bruce Mikita Two in red with a yellow outline. Not an official Rockstar logo.</desc>
{''.join(paths)}
<text x="{w/2}" y="{baseline}" text-anchor="middle" fill="{WHITE}" font-family="Arial,sans-serif" font-size="22" letter-spacing="6">PC RECOMP</text>
</svg>'''

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--pricedown',type=Path,default=ROOT/'target/private-fonts/pricedown/Pricedown Bl.otf')
    ap.add_argument('--bruce-mikita',type=Path,default=ROOT/'target/private-fonts/bruce-mikita/Bruce Mikita Two.ttf')
    ap.add_argument('--output',type=Path,default=ROOT/'docs/assets');a=ap.parse_args()
    gta=TTFont(a.pricedown);ctw=TTFont(a.bruce_mikita);a.output.mkdir(parents=True,exist_ok=True)
    for stem,stacked in [('logo',True),('logo-ctw',False)]:
        content=make(gta,ctw,stacked)
        for suffix in ['','-transparent']:(a.output/f'{stem}{suffix}.svg').write_text(content,encoding='utf-8')
        black=content.replace('<title>', '<rect width="100%" height="100%" fill="#000000"/><title>',1)
        (a.output/f'{stem}-black.svg').write_text(black,encoding='utf-8')
if __name__=='__main__':main()
