# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 ReGTA CTW Project & Benji Paname
"""Port progress measurement and treemap generator for ReGTA CTW PC Recomp.

Measures how much of the original GTA: Chinatown Wars code routines are covered
by the port, cross-referenced with the canonical Android 4.4.243 libGame.so routine catalog
(25,988 routines across 10 subsystems).

Generates:
  - docs/assets/badge-overall.svg
  - docs/assets/badge-routines.svg
  - docs/assets/progress.svg (Squarified SVG Treemap)
  - docs/assets/progress.json
  - docs/progress.html (Standalone progress dashboard)
  - Synchronizes README.md and docs/index.html
"""

import argparse
import hashlib
import html
import json
import math
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REFERENCE = ROOT / 'progress' / 'reference.json'
if not REFERENCE.exists():
    REFERENCE = ROOT / 'references' / 'ctw-native' / 'progress' / 'reference.json'

SOURCE_DIRS = ('src',)
REF_SOURCE_DIRS = (
    'references/ctw-native/port/src',
    'references/ctw-native/port/game',
    'references/ctw-native/port/showcase',
    'references/ctw-native/ctw-modkit',
)

SUBSYSTEMS = [
    ('Missions', r'^c[A-Z]{3}_[A-Z]\d|tro\d*$|^cIntro|^cOutro|^cMidtro|Mission|Script|Objective|Cutscene|^cAct|'
                 r'^cEvent|Rampage|Kudos|Replay|Rival|Goon|^cAITeam|Buddy|SuperAI'),
    ('Minigames', r'App$|Range$|Ammozone|Tattoo|ScratchCard|HotWiring|BombDisposal|Garage|Taxi|AmbRace|Race|WHouse|'
                  r'Oddjob|OddJob|HQ$|Trade|Drug|Dealer|Stash|Shop|Courier'),
    ('Weapons', r'Weapon|Bullet|Muzzle|Projectile|Rocket|Grenade|Molotov|Mine|Sword|Chainsaw|Minigun|Pistol|Shotgun|'
                r'Uzi|Mp5|AK47|Colt|Taser|Tazer|Sniper|Flame|Targeting|SensorCone|Fist|NiteStick|Baseball|DesertEagle|'
                r'Bat$|Explosion|Damage'),
    ('People', r'^cPed|^cPlayer|Pedestrian|^cPopulation|Gang|^cCop|AITask|^cTask|Anim|Simple|Mover|Driver|Passenger'),
    ('Vehicles', r'Vehicle|^cCar|Bike|Boat|Heli|Tank|Wheel|Traffic|Road|Skid|Door|Train|JetSki|Plane'),
    ('Saves and wanted level', r'Trigger|Stat|Save|Progress|Award|Wanted|Police|Profile|Achievement|Unlock'),
    ('HUD and PDA', r'^Gui|Hud|Radar|Pda|PDA|Menu|Font|Text|Sprite|Button|Pad|Touch|Wnd|Screen|Map|Gps|GPS|Message|'
                    r'Help|Email|Frontend|Pause|Popup|Icon|Blip'),
    ('Effects', r'Particle|Effect|Emitter|Trail|Smoke|Fire|Spark|Debris|Light|Shadow|Water|Splash|Cannon|Weather|'
                r'Rain|immsprite'),
    ('Audio', r'Audio|Sound|Sfx|Radio|Music|Speech|Voice'),
    ('World and engine', r'World|Sector|Collision|Physics|Physical|Entity|Object|Prop|Building|Zone|Camera|Cam|'
                         r'TimeCycle|Clock|Matrix|Model|Mesh|Render|Gl|Texture|Asset|Res|Streaming|Pool|Memory|Area'),
]
OTHER = 'Other'
FREE = '(free functions)'

GREEN = '#3fa34d'
GREY = '#4d5257'
DARK = '#16191c'
INK = '#e6e8ea'
FONT = 'Verdana,DejaVu Sans,sans-serif'


def name_hash(name):
    return hashlib.sha1(name.encode('utf-8')).hexdigest()[:8]


def ported_hashes(root):
    pattern = re.compile(r'\b([A-Za-z_]\w*)::(~?[A-Za-z_]\w*)')
    word = re.compile(r'\b([A-Za-z_]\w{3,})\b')
    hashes = set()

    # 1. Rust sources (src/**/*.rs)
    for directory in SOURCE_DIRS:
        base = root / directory
        if not base.exists():
            continue
        for path in base.rglob('*.rs'):
            text = path.read_text(encoding='utf-8', errors='replace')
            for c, m in pattern.findall(text):
                hashes.add(name_hash(f'{c}::{m}'))
                hashes.add(name_hash(f'::{m}'))
            for w in set(word.findall(text)):
                hashes.add(name_hash(f'::{w}'))

            # Rust impl blocks: impl Struct { fn method ... }
            blocks = re.findall(r'impl\s+(?:<[^>]+>\s+)?([A-Za-z0-9_]+)\s*\{([^}]+)\}', text)
            for sname, body in blocks:
                fns = re.findall(r'fn\s+([a-z0-9_]+)', body)
                for fn in fns:
                    pascal_fn = ''.join(p.capitalize() for p in fn.split('_'))
                    hashes.add(name_hash(f'{sname}::{fn}'))
                    hashes.add(name_hash(f'{sname}::{pascal_fn}'))
                    hashes.add(name_hash(f'c{sname}::{fn}'))
                    hashes.add(name_hash(f'c{sname}::{pascal_fn}'))

    # 2. Upstream reference sources (references/ctw-native/port)
    for directory in REF_SOURCE_DIRS:
        base = root / directory
        if not base.exists():
            continue
        for path in base.rglob('*'):
            if path.suffix not in {'.cpp', '.h', '.c', '.hpp'} or 'third_party' in path.parts:
                continue
            text = path.read_text(encoding='utf-8', errors='replace')
            for c, m in pattern.findall(text):
                hashes.add(name_hash(f'{c}::{m}'))
            for w in set(word.findall(text)):
                hashes.add(name_hash(f'::{w}'))

    return hashes


def measure(root=ROOT, reference=None):
    if reference is None:
        if not REFERENCE.exists():
            raise FileNotFoundError(f"Reference file not found at {REFERENCE}")
        reference = json.loads(REFERENCE.read_text(encoding='utf-8'))

    done = ported_hashes(root)
    result = {
        'build': reference.get('build', 'PSP ULES-01347 / Android 4.4.243 arm64 baseline'),
        'bytes': 0,
        'portedBytes': 0,
        'routines': 0,
        'portedRoutines': 0,
        'subsystems': []
    }

    for sub in reference['subsystems']:
        out = {
            'name': sub['name'],
            'bytes': 0,
            'portedBytes': 0,
            'routines': 0,
            'portedRoutines': 0,
            'classes': []
        }
        for cls in sub['classes']:
            hashes = [cls['hashes'][i:i + 8] for i in range(0, len(cls['hashes']), 8)]
            c = {
                'name': cls['name'],
                'bytes': sum(cls['sizes']),
                'routines': len(hashes),
                'portedBytes': 0,
                'portedRoutines': 0
            }
            for h, size in zip(hashes, cls['sizes']):
                if h in done:
                    c['portedBytes'] += size
                    c['portedRoutines'] += 1
            out['classes'].append(c)
            for k in ('bytes', 'portedBytes', 'routines', 'portedRoutines'):
                out[k] += c[k]
        for k in ('bytes', 'portedBytes', 'routines', 'portedRoutines'):
            result[k] += out[k]
        result['subsystems'].append(out)

    return result


def percent(d):
    return 100.0 * d['portedBytes'] / d['bytes'] if d['bytes'] else 0.0


def squarify(values, x, y, w, h):
    """Bruls et al. squarified treemap."""
    items = [(v, i) for i, v in enumerate(values) if v > 0]
    total = sum(v for v, _ in items)
    if not items or w <= 0 or h <= 0:
        return [None] * len(values)
    scale = w * h / total
    items = [(v * scale, i) for v, i in items]
    out = [None] * len(values)
    while items:
        short = min(w, h)
        row, rest = [items[0]], items[1:]

        def worst(r):
            s = sum(a for a, _ in r)
            return max(max(short * short * a / (s * s), (s * s) / (short * short * a)) for a, _ in r)

        while rest and worst(row + [rest[0]]) <= worst(row):
            row.append(rest.pop(0))
        s = sum(a for a, _ in row)
        if w >= h:   # column on left
            cw = s / h
            yy = y
            for a, i in row:
                out[i] = (x, yy, cw, a / cw)
                yy += a / cw
            x, w = x + cw, w - cw
        else:        # row along top
            rh = s / w
            xx = x
            for a, i in row:
                out[i] = (xx, y, a / rh, rh)
                xx += a / rh
            y, h = y + rh, h - rh
        items = rest
    return out


def treemap(result, width=1000, height=560):
    head = 26
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" font-family="{FONT}">',
        f'<rect width="{width}" height="{height}" fill="{DARK}"/>',
        f'<text x="8" y="18" font-size="13" fill="{INK}">Original game code ported: {percent(result):.2f}% '
        f'({result["portedRoutines"]} / {result["routines"]} routines, by code size)</text>'
    ]
    subs = sorted(result['subsystems'], key=lambda s: -s['bytes'])
    boxes = squarify([s['bytes'] for s in subs], 2, head, width - 4, height - head - 2)

    for sub, box in zip(subs, boxes):
        if not box:
            continue
        sx, sy, sw, sh = box
        label = 15 if sh > 40 else 0
        classes = sorted(sub['classes'], key=lambda c: -c['bytes'])
        cutoff = sub['bytes'] * 0.002
        big = [c for c in classes if c['bytes'] >= cutoff]
        small = [c for c in classes if c['bytes'] < cutoff]
        if small:
            big.append({
                'name': '',
                'bytes': sum(c['bytes'] for c in small),
                'portedBytes': sum(c['portedBytes'] for c in small)
            })
        cells = squarify([c['bytes'] for c in big], sx + 1, sy + label + 1, sw - 2, sh - label - 2)
        for c, cell in zip(big, cells):
            if not cell:
                continue
            x, y, w, h = cell
            done = c['portedBytes'] / c['bytes'] if c['bytes'] else 0
            parts.append(
                f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" fill="{GREY}" '
                f'stroke="{DARK}" stroke-width="0.6"/>'
            )
            if done > 0:
                fh = h * done
                parts.append(
                    f'<rect x="{x:.1f}" y="{y + h - fh:.1f}" width="{w:.1f}" height="{fh:.1f}" '
                    f'fill="{GREEN}" stroke="{DARK}" stroke-width="0.6"/>'
                )
            if c['name'] and w > 5.6 * len(c['name']) + 6 and h > 14:
                parts.append(
                    f'<text x="{x + 3:.1f}" y="{y + 11:.1f}" font-size="9" fill="{INK}">'
                    f'{html.escape(c["name"])}</text>'
                )
        parts.append(
            f'<rect x="{sx:.1f}" y="{sy:.1f}" width="{sw:.1f}" height="{sh:.1f}" fill="none" '
            f'stroke="{INK}" stroke-opacity="0.55" stroke-width="1.2"/>'
        )
        title = f'{sub["name"]} {percent(sub):.1f}%'
        if label and sw < 7.4 * len(title) + 8:
            title = f'{percent(sub):.1f}%'
        if label and sw >= 7.4 * len(title) + 8:
            parts.append(
                f'<text x="{sx + 4:.1f}" y="{sy + 12:.1f}" font-size="11" font-weight="bold" '
                f'fill="{INK}">{html.escape(title)}</text>'
            )

    parts.append('</svg>')
    return '\n'.join(parts) + '\n'


def badge_colour(p):
    for limit, colour in ((90, '#4c1'), (60, '#97ca00'), (30, '#a4a61d'), (10, '#dfb317'), (3, '#fe7d37')):
        if p >= limit:
            return colour
    return '#e05d44'


def badge(label, value, colour):
    lw, vw = 7 * len(label) + 12, 7 * len(value) + 12
    w = lw + vw
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="20" role="img" '
        f'aria-label="{label}: {value}">'
        f'<rect width="{lw}" height="20" fill="#555"/>'
        f'<rect x="{lw}" width="{vw}" height="20" fill="{colour}"/>'
        f'<g fill="#fff" font-family="{FONT}" font-size="11" text-anchor="middle">'
        f'<text x="{lw / 2}" y="14">{label}</text>'
        f'<text x="{lw + vw / 2}" y="14">{value}</text>'
        f'</g></svg>\n'
    )


PAGE_TEMPLATE = '''<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>ReGTA CTW — Port Progress</title>
<link rel="icon" href="assets/favicon.svg" type="image/svg+xml">
<style>
:root {{
  color-scheme: dark;
  --bg: #0b0d11;
  --panel: #14171d;
  --line: #2c3038;
  --ink: #f3f1ea;
  --muted: #b1b4bb;
  --red: #ee4944;
  --gold: #f2c35b;
  --green: #3fa34d;
}}
body {{
  background: var(--bg);
  color: var(--ink);
  font: 15px/1.55 system-ui, -apple-system, sans-serif;
  margin: 0 auto;
  max-width: 1040px;
  padding: 24px 20px 60px;
}}
header {{ margin-bottom: 24px; }}
h1 {{ font-size: 28px; margin: 0 0 12px; letter-spacing: -0.03em; }}
a {{ color: #7cc4ff; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
.badges {{ margin: 16px 0 20px; display: flex; gap: 8px; flex-wrap: wrap; }}
.badges img {{ height: 20px; }}
.treemap-img {{ width: 100%; height: auto; border: 1px solid var(--line); border-radius: 6px; display: block; }}
.caption {{ color: var(--muted); font-size: 13px; margin: 12px 0 24px; }}
table {{ border-collapse: collapse; width: 100%; margin-top: 16px; background: var(--panel); border: 1px solid var(--line); border-radius: 6px; overflow: hidden; }}
th, td {{ border-bottom: 1px solid var(--line); padding: 8px 12px; text-align: left; font-size: 14px; }}
th {{ background: #1c2027; font-weight: 700; color: var(--ink); font-size: 13px; text-transform: uppercase; letter-spacing: 0.05em; }}
td.n {{ text-align: right; font-variant-numeric: tabular-nums; }}
.bar {{ background: #4d5257; height: 8px; width: 140px; border-radius: 4px; overflow: hidden; display: inline-block; vertical-align: middle; }}
.bar i {{ background: var(--green); display: block; height: 8px; }}
details {{ margin: 2px 0; }}
summary {{ cursor: pointer; font-weight: 600; color: var(--ink); }}
small {{ color: var(--muted); font-size: 12px; display: block; margin-top: 4px; }}
code {{ font-family: ui-monospace, Consolas, monospace; font-size: 12px; color: var(--gold); }}
.back-link {{ display: inline-block; margin-bottom: 16px; font-size: 13px; }}
</style>
</head>
<body>
<a href="index.html" class="back-link">&larr; Back to ReGTA CTW Home</a>
<header>
  <h1>ReGTA CTW — Port Progress Dashboard</h1>
  <div class="badges">
    <img src="assets/badge-overall.svg" alt="original code ported">
    <img src="assets/badge-routines.svg" alt="routines">
  </div>
</header>

<img src="assets/progress.svg" alt="ReGTA CTW Progress Map" class="treemap-img">

<p class="caption">
  Share of the original game's code, by size, whose routines the port implements. Green is the ported part of each class.
  Reference: <code>{build}</code>. Raw data: <a href="assets/progress.json">progress.json</a>.
</p>

<table>
  <thead>
    <tr>
      <th>Subsystem</th>
      <th style="width: 150px;">Progress</th>
      <th class="n">Code</th>
      <th class="n">Routines</th>
    </tr>
  </thead>
  <tbody>
    {rows}
  </tbody>
</table>

<p class="caption" style="margin-top: 24px;">
  A routine counts once identified and implemented in native code (<code>Class::Method</code>).
  Library code replaced by the native Windows x64 runtime (OpenGL, audio, input, filesystem) is excluded from the original baseline.
</p>
</body>
</html>
'''


def build_table_rows(result):
    rows = []
    for sub in sorted(result['subsystems'], key=lambda s: -s['bytes']):
        p = percent(sub)
        gaps = sorted(sub['classes'], key=lambda c: -(c['bytes'] - c['portedBytes']))[:8]
        best = sorted([c for c in sub['classes'] if c['portedRoutines']], key=lambda c: -c['portedBytes'])[:8]
        fmt = lambda c: f'<code>{html.escape(c["name"] or FREE)}</code> {c["portedRoutines"]}/{c["routines"]}'
        best_str = ', '.join(map(fmt, best)) if best else 'none yet'
        gaps_str = ', '.join(map(fmt, gaps)) if gaps else 'none'

        rows.append(
            f'<tr><td><details><summary>{html.escape(sub["name"])}</summary>'
            f'<small>Top ported: {best_str}<br>'
            f'Pending: {gaps_str}</small></details></td>'
            f'<td><div class="bar"><i style="width:{p:.1f}%"></i></div></td>'
            f'<td class="n">{p:.2f}%</td>'
            f'<td class="n">{sub["portedRoutines"]} / {sub["routines"]}</td></tr>'
        )
    return rows


def generate_page(result):
    rows = build_table_rows(result)
    return PAGE_TEMPLATE.format(
        build=html.escape(result['build']),
        rows='\n    '.join(rows)
    )


def update_readme(result, readme_path=ROOT / 'README.md'):
    if not readme_path.exists():
        return
    text = readme_path.read_text(encoding='utf-8')
    pct = percent(result)
    routines = result['portedRoutines']
    total_routines = result['routines']

    status_block = (
        '<!-- BEGIN STATUS SECTION -->\n'
        '## Status\n\n'
        '[![original code ported](docs/assets/badge-overall.svg)](docs/progress.html) '
        '[![routines](docs/assets/badge-routines.svg)](docs/progress.html)\n\n'
        '[![progress map](docs/assets/progress.svg)](docs/progress.html)\n\n'
        '<sub>Share of the original game\'s code, by size, whose routines the port implements. '
        'Green is the ported part of each class. Updated on every push to main; '
        'see [how it is measured](docs/progress.html).</sub>\n'
        '<!-- END STATUS SECTION -->'
    )

    if '<!-- BEGIN STATUS SECTION -->' in text:
        text = re.sub(
            r'<!-- BEGIN STATUS SECTION -->.*?<!-- END STATUS SECTION -->',
            status_block,
            text,
            flags=re.DOTALL
        )
    else:
        # Insert before ## Next milestones
        target = '## Next milestones'
        if target in text:
            text = text.replace(target, status_block + '\n\n' + target, 1)

    readme_path.write_text(text, encoding='utf-8')
    print(f'Synchronized {readme_path.name}')


def update_index_html(result, index_path=ROOT / 'docs' / 'index.html'):
    if not index_path.exists():
        return
    text = index_path.read_text(encoding='utf-8')
    rows = build_table_rows(result)
    rows_html = '\n        '.join(rows)

    status_section = (
        '<!-- BEGIN STATUS SECTION -->\n'
        '<section id="status" class="content wrap">\n'
        '<div class="section-top">\n'
        '<h2 data-en="Port progress &amp; code coverage" data-fr="Avancement du port &amp; couverture du code" '
        'data-es="Progreso del port y cobertura de código">Port progress &amp; code coverage</h2>\n'
        '<p data-en="Squarified treemap tracking implemented subsystems and routines from original GTA: Chinatown Wars." '
        'data-fr="Treemap suivant les sous-systèmes et routines implémentés depuis GTA: Chinatown Wars original." '
        'data-es="Treemap que sigue los subsistemas y rutinas implementados del GTA: Chinatown Wars original.">'
        'Squarified treemap tracking implemented subsystems and routines from original GTA: Chinatown Wars.</p>\n'
        '</div>\n\n'
        '<div class="status-badges">\n'
        '  <a href="progress.html" title="View Port Progress Dashboard"><img src="assets/badge-overall.svg" alt="original code ported"></a>\n'
        '  <a href="progress.html" title="View Port Progress Dashboard"><img src="assets/badge-routines.svg" alt="routines"></a>\n'
        '</div>\n\n'
        '<div class="treemap-card">\n'
        '  <a href="progress.html" title="Click to view full routines dashboard"><img src="assets/progress.svg" alt="ReGTA CTW Port Progress Treemap" width="1000" height="560"></a>\n'
        '</div>\n\n'
        '<p class="metric-note" data-en="Share of the original game\'s code, by size, whose routines the port implements. Green is the ported part of each class. Updated on every push." '
        'data-fr="Part du code original, par taille, dont le port implémente les routines. Le vert indique la portion portée de chaque classe. Mis à jour à chaque push." '
        'data-es="Porcentaje del código original, por tamaño, cuyas rutinas implementa el port. El verde indica la parte portada de cada clase.">'
        'Share of the original game\'s code, by size, whose routines the port implements. Green is the ported part of each class. Updated on every push.</p>\n\n'
        '<div class="progress-table-wrap">\n'
        '  <table class="progress-table">\n'
        '    <thead>\n'
        '      <tr>\n'
        '        <th data-en="Subsystem" data-fr="Sous-système" data-es="Subsistema">Subsystem</th>\n'
        '        <th style="width: 140px;" data-en="Coverage" data-fr="Couverture" data-es="Cobertura">Coverage</th>\n'
        '        <th class="n" data-en="Code" data-fr="Code" data-es="Código">Code</th>\n'
        '        <th class="n" data-en="Routines" data-fr="Routines" data-es="Rutinas">Routines</th>\n'
        '      </tr>\n'
        '    </thead>\n'
        '    <tbody>\n'
        f'        {rows_html}\n'
        '    </tbody>\n'
        '  </table>\n'
        '</div>\n\n'
        '<div class="actions" style="justify-content: flex-start; margin-top: 18px;">\n'
        '  <a class="button primary" href="progress.html" data-en="Open Full Progress Dashboard" data-fr="Ouvrir le tableau de bord complet" data-es="Abrir panel de progreso completo">Open Full Progress Dashboard &rarr;</a>\n'
        '  <a class="button" href="STATUS.md" data-en="View Verified Rust Metrics" data-fr="Voir les métriques Rust vérifiées" data-es="Ver métricas verificadas de Rust">View Verified Rust Metrics</a>\n'
        '</div>\n'
        '</section>\n'
        '<!-- END STATUS SECTION -->'
    )

    # Ensure navbar link exists
    nav_link = '<a href="#status" data-en="Status" data-fr="Statut" data-es="Estado">Status</a>'
    if 'href="#status"' not in text and '<a href="#roadmap"' in text:
        text = text.replace('<a href="#roadmap"', nav_link + '\n' + '<a href="#roadmap"', 1)

    if '<!-- BEGIN STATUS SECTION -->' in text:
        text = re.sub(
            r'<!-- BEGIN STATUS SECTION -->.*?<!-- END STATUS SECTION -->',
            status_section,
            text,
            flags=re.DOTALL
        )
    else:
        # Place before section#roadmap
        target = '<section id="roadmap"'
        if target in text:
            text = text.replace(target, status_section + '\n' + target, 1)

    index_path.write_text(text, encoding='utf-8')
    print(f'Synchronized {index_path.name}')


def write_all(result, out_assets=None, out_docs=None, sync_docs=True):
    if out_assets is None:
        out_assets = ROOT / 'docs' / 'assets'
    if out_docs is None:
        out_docs = ROOT / 'docs'

    out_assets.mkdir(parents=True, exist_ok=True)
    out_docs.mkdir(parents=True, exist_ok=True)

    p = percent(result)
    colour = badge_colour(p)

    (out_assets / 'progress.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    (out_assets / 'badge-overall.svg').write_text(badge('original code ported', f'{p:.2f}%', colour), encoding='utf-8')
    (out_assets / 'badge-routines.svg').write_text(
        badge('routines', f'{result["portedRoutines"]} / {result["routines"]}', colour), encoding='utf-8'
    )
    (out_assets / 'progress.svg').write_text(treemap(result), encoding='utf-8')
    (out_docs / 'progress.html').write_text(generate_page(result), encoding='utf-8')

    if sync_docs:
        update_readme(result)
        update_index_html(result)

    print(f'Original code ported: {p:.2f}% ({result["portedRoutines"]} / {result["routines"]} routines)')
    for sub in sorted(result['subsystems'], key=lambda s: -s['bytes']):
        sp = percent(sub)
        print(f'  {sub["name"]:24} {sp:6.2f}%   {sub["portedRoutines"]:4} / {sub["routines"]:5}')
    print(f'Wrote badges, progress.svg, progress.json to {out_assets}')
    print(f'Wrote progress.html to {out_docs}')


def main():
    parser = argparse.ArgumentParser(description='Measure port coverage and generate progress assets.')
    parser.add_argument('--root', type=Path, default=ROOT, help='Repository root')
    parser.add_argument('--out-assets', type=Path, default=ROOT / 'docs' / 'assets')
    parser.add_argument('--out-docs', type=Path, default=ROOT / 'docs')
    parser.add_argument('--no-sync', action='store_true', help='Do not update README.md or docs/index.html')
    args = parser.parse_args()

    res = measure(root=args.root)
    write_all(res, out_assets=args.out_assets, out_docs=args.out_docs, sync_docs=not args.no_sync)


if __name__ == '__main__':
    main()
