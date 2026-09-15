#!/usr/bin/env python3
"""Read-only glTF/GLB handoff inspection; supplements, never replaces project validation."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from urllib.parse import unquote, urlparse


def inspect(path):
    path = path.resolve(strict=True)
    data = path.read_bytes()
    if path.suffix.lower() == '.glb':
        if len(data) < 20:
            raise ValueError('truncated GLB')
        magic, version, length = struct.unpack_from('<4sII', data)
        if (magic, version, length) != (b'glTF', 2, len(data)):
            raise ValueError('invalid GLB header/version/length')
        size, kind = struct.unpack_from('<II', data, 12)
        if kind != 0x4E4F534A or 20 + size > len(data):
            raise ValueError('invalid first JSON chunk')
        document = json.loads(data[20:20 + size])
    elif path.suffix.lower() == '.gltf':
        document = json.loads(data)
    else:
        raise ValueError('expected .glb or .gltf')
    if document.get('asset', {}).get('version') != '2.0':
        raise ValueError('expected glTF 2.0')
    sidecar_path = path.with_name(path.stem + '.npr.json')
    sidecar = json.loads(sidecar_path.read_text()) if sidecar_path.exists() else None
    diagnostics = []
    if sidecar is not None and sidecar.get('schema') != 'bevy-stylized.sidecar/v1':
        diagnostics.append('runtime sidecar has wrong schema; export-sidecar-v1.1 is a different contract')
    materials = []
    metadata_review = []
    for index, mat in enumerate(document.get('materials', [])):
        name = mat.get('name', f'material_{index}')
        extras = mat.get('extras', {})
        extras = extras if isinstance(extras, dict) else {}
        descriptor = extras.get('bevy_stylized.material/v1')
        from_sidecar = sidecar.get('materials', {}).get(name) if sidecar and sidecar.get('schema') == 'bevy-stylized.sidecar/v1' else None
        if descriptor is not None and from_sidecar is not None and descriptor != from_sidecar:
            metadata_review.append(f'{name}: raw extras/sidecar payloads differ; run project importer for normalized conflict validation')
        materials.append({'index': index, 'name': name, 'alpha_mode': mat.get('alphaMode', 'OPAQUE'),
                          'has_material_extras': descriptor is not None,
                          'has_runtime_sidecar_descriptor': from_sidecar is not None})
    primitives = []
    for mesh in document.get('meshes', []):
        for p in mesh.get('primitives', []):
            primitives.append({'mesh': mesh.get('name'), 'material': p.get('material'),
                               'attributes': sorted(p.get('attributes', {})), 'morph_targets': len(p.get('targets', []))})
    external = []
    for block in document.get('buffers', []) + document.get('images', []):
        uri = block.get('uri')
        if not uri or uri.startswith('data:'):
            continue
        parsed = urlparse(uri)
        if parsed.scheme:
            external.append({'uri': uri, 'status': 'remote-not-fetched'})
        else:
            exists = (path.parent / unquote(parsed.path)).is_file()
            external.append({'uri': uri, 'status': 'exists' if exists else 'missing'})
            if not exists:
                diagnostics.append(f'missing dependency: {uri}')
    count = sum(m['has_material_extras'] or m['has_runtime_sidecar_descriptor'] for m in materials)
    return {'scope': 'structural inventory; not GPU, shader, skinning or visual validation',
            'asset': str(path), 'sha256': hashlib.sha256(data).hexdigest(),
            'counts': {k: len(document.get(k, [])) for k in ['meshes', 'materials', 'nodes', 'skins', 'animations', 'images']},
            'materials': materials, 'mapped_materials': count, 'primitives': primitives,
            'runtime_sidecar': str(sidecar_path) if sidecar is not None else None,
            'bridge_sidecar_present': (path.parent / 'export-sidecar.json').is_file(),
            'external_dependencies': external, 'metadata_review_items': metadata_review, 'diagnostics': diagnostics}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('asset', type=Path)
    parser.add_argument('--require-material-metadata', action='store_true',
                        help='Require a typed descriptor per material for descriptor-driven workflows')
    args = parser.parse_args()
    try:
        report = inspect(args.asset)
    except (OSError, ValueError, TypeError, AttributeError, struct.error) as exc:
        parser.exit(2, f'{exc}\n')
    if args.require_material_metadata and report['mapped_materials'] != report['counts']['materials']:
        report['diagnostics'].append('some materials lack runtime descriptors; compiler defaults do not establish Genshin mapping')
    print(json.dumps(report, indent=2, ensure_ascii=False))
    return 1 if report['diagnostics'] else 0


if __name__ == '__main__':
    raise SystemExit(main())
