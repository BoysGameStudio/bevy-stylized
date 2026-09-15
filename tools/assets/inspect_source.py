#!/usr/bin/env python3
"""Read-only source inventory. Emits JSON to stdout; never downloads or rewrites assets."""
import argparse
import hashlib
import json
from pathlib import Path


def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def inventory(root, compare=None):
    root = root.resolve(strict=True)
    if not root.is_dir():
        raise ValueError('source must be a directory')
    files, problems = [], []
    for path in sorted(root.rglob('*')):
        if path.is_symlink():
            problems.append(f'symlink requires explicit review: {path.relative_to(root)}')
            continue
        if not path.is_file():
            continue
        rel = path.relative_to(root)
        item = {'path': rel.as_posix(), 'bytes': path.stat().st_size, 'sha256': sha(path)}
        if compare is not None:
            other = compare / rel
            item['comparison'] = 'missing'
            if other.is_file():
                item['comparison'] = 'identical' if sha(other) == item['sha256'] else 'different'
                if item['comparison'] == 'different' and path.suffix.lower() == '.json':
                    try:
                        if json.loads(path.read_text()) == json.loads(other.read_text()):
                            item['comparison'] = 'json-equivalent'
                    except (ValueError, UnicodeError):
                        pass
        files.append(item)
    fbx = [f['path'] for f in files if f['path'].lower().endswith('.fbx')]
    if len(fbx) != 1:
        problems.append(f'expected one character FBX in isolated input, found {len(fbx)}')
    return {'source_root': str(root), 'fbx': fbx, 'files': files,
            'review_items': problems, 'scope': 'file inventory only; not import validation'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('--compare', type=Path)
    args = parser.parse_args()
    try:
        report = inventory(args.source, args.compare.resolve(strict=True) if args.compare else None)
    except (OSError, ValueError) as exc:
        parser.exit(2, f'{exc}\n')
    print(json.dumps(report, indent=2, ensure_ascii=False))
    return 1 if report['review_items'] else 0


if __name__ == '__main__':
    raise SystemExit(main())
