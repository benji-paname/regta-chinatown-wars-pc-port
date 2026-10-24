"""Read-only inventory and lossless-image cleanup plan; never deletes files.

Run with a Python installation providing Pillow for the image comparison.
All reports are private local files under work/restructure/.
"""
from pathlib import Path
import argparse
import hashlib
import json
import os

ROOT = Path(__file__).resolve().parents[2]
REPORTS = ROOT / 'work/restructure'
CACHES = {f'work/{name}' for name in ('ctw-build', 'ctw-roll-build', 'ctw-slice-build')}


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def files():
    for directory, dirs, names in os.walk(ROOT / 'work', followlinks=False):
        dirs[:] = [d for d in dirs if not (Path(directory) / d).is_symlink()
                   and (Path(directory) / d).relative_to(ROOT).as_posix() not in CACHES
                   and (Path(directory) / d) != REPORTS]
        for name in names:
            path = Path(directory) / name
            if not path.is_symlink():
                yield path


def snapshot():
    REPORTS.mkdir(exist_ok=True)
    records = {p.relative_to(ROOT).as_posix(): {'bytes': p.stat().st_size, 'sha256': digest(p)}
               for p in files()}
    (REPORTS / 'preserved-before.json').write_text(json.dumps(records, indent=2), encoding='utf-8')
    print(f'Preservation snapshot: {len(records)} files.')


def plan():
    from PIL import Image
    matched, skipped = [], []
    for base in (ROOT / 'work', ROOT / 'outputs'):
        for bmp in base.rglob('*.bmp'):
            if bmp.is_symlink() or any(bmp.is_relative_to(ROOT / c) for c in CACHES):
                continue
            png = bmp.with_suffix('.png')
            if not png.is_file() or png.is_symlink():
                continue
            try:
                with Image.open(bmp) as left, Image.open(png) as right:
                    equal = left.size == right.size and left.convert('RGBA').tobytes() == right.convert('RGBA').tobytes()
                if equal:
                    matched.append({'bmp': bmp.relative_to(ROOT).as_posix(), 'png': png.relative_to(ROOT).as_posix(),
                                    'bytes': bmp.stat().st_size, 'bmp_sha256': digest(bmp), 'png_sha256': digest(png)})
                else:
                    skipped.append(bmp.relative_to(ROOT).as_posix())
            except (OSError, ValueError) as err:
                skipped.append(f'{bmp.relative_to(ROOT)}: {err}')
    cache_records = []
    for name in sorted(CACHES):
        p = ROOT / name
        if p.is_dir():
            cache_records.append({'path': name, 'bytes': sum(f.stat().st_size for f in p.rglob('*') if f.is_file())})
    data = {'caches': cache_records, 'identical_images': matched, 'nonidentical_or_unreadable': skipped}
    (REPORTS / 'cleanup-plan.json').write_text(json.dumps(data, indent=2), encoding='utf-8')
    print(json.dumps({'cache_bytes': sum(p['bytes'] for p in cache_records), 'identical_bmps': len(matched),
                      'bmp_bytes': sum(p['bytes'] for p in matched), 'different_bmps_preserved': len(skipped)}))


def check():
    before = json.loads((REPORTS / 'preserved-before.json').read_text(encoding='utf-8'))
    plan_data = json.loads((REPORTS / 'cleanup-plan.json').read_text(encoding='utf-8'))
    allowed = {p['bmp'] for p in plan_data['identical_images']}
    errors = []
    for name, record in before.items():
        p = ROOT / name
        if name in allowed and not p.exists():
            continue
        if not p.is_file() or digest(p) != record['sha256']:
            errors.append(name)
    report = {'checked': len(before), 'allowed_bmp_removals': len(allowed), 'changed_or_missing': errors}
    (REPORTS / 'preservation-result.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps(report))
    if errors:
        raise SystemExit(1)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['snapshot', 'plan', 'check'])
    args = parser.parse_args()
    globals()[args.action]()
