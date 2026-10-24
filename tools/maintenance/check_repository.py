"""Fail if Git tracks assets, private research, build products or unexpected files."""
from pathlib import Path
import json
import subprocess

root = Path(__file__).resolve().parents[2]
names = subprocess.check_output(['git', 'ls-files', '-z'], cwd=root).decode('utf-8').split('\0')
suffixes = {'.rs', '.md', '.toml', '.lock', '.ps1', '.cmd', '.py', '.yml', '.yaml'}
errors = []
size = 0
for name in filter(None, names):
    p = root / name
    if not p.is_file() or p.is_symlink():
        errors.append(f'Missing file or symlink: {name}')
        continue
    if name.startswith(('work/', 'target/', 'outputs/CTW-Rust/data/')):
        errors.append(f'Private/generated directory: {name}')
    if name.startswith('data/') and p.name != 'README.md':
        errors.append(f'Private data: {name}')
    if p.suffix.lower() not in suffixes and p.name not in {'.gitignore', '.gitattributes', 'LICENSE'}:
        errors.append(f'Unexpected file type: {name}')
    data = p.read_bytes()
    size += len(data)
    try:
        data.decode('utf-8-sig')
    except UnicodeDecodeError:
        errors.append(f'Binary/non-UTF8 file: {name}')
    if b'\0' in data or len(data) > 1_000_000:
        errors.append(f'Binary or unexpectedly large file: {name}')

print(json.dumps({'tracked_files': len([n for n in names if n]), 'tracked_bytes': size, 'errors': errors}, indent=2))
raise SystemExit(bool(errors))
