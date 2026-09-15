"""Prepare immutable, hash-verified source packs. No implicit runtime downloads."""
from pathlib import Path, PurePosixPath
import hashlib
import json
import os
import re
import shutil
import tempfile
import urllib.request


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def validate_manifest(manifest):
    contract=json.loads((Path(__file__).resolve().parents[2]/'schemas/authoring/source-manifest.schema.json').read_text())
    if not isinstance(manifest,dict) or any(key not in manifest for key in contract['required']):raise ValueError('source manifest required fields missing')
    if manifest.get('schema') != contract['properties']['schema']['const']:
        raise ValueError('unsupported source manifest schema')
    identifier = manifest.get('id', '')
    if not isinstance(identifier,str) or not re.fullmatch(contract['properties']['id']['pattern'],identifier):
        raise ValueError('invalid pack id')
    if not isinstance(manifest.get('files'),list) or not manifest['files']:
        raise ValueError('empty source pack')
    seen = set()
    for item in manifest['files']:
        fields=contract['properties']['files']['items']
        if not isinstance(item,dict) or any(key not in item for key in fields['required']):raise ValueError('source file required fields missing')
        if not isinstance(item['path'],str) or not item['path'] or ':' in item['path']:raise ValueError('invalid source path')
        path = PurePosixPath(item['path'])
        if path.is_absolute() or '..' in path.parts or '\\' in item['path'] or str(path) in seen:
            raise ValueError(f'unsafe/duplicate file path: {path}')
        seen.add(str(path))
        if not isinstance(item['sha256'],str) or not re.fullmatch(fields['properties']['sha256']['pattern'],item['sha256']):
            raise ValueError(f'invalid sha256: {path}')
        if type(item.get('bytes')) is not int or item['bytes'] < fields['properties']['bytes']['minimum']:
            raise ValueError(f'invalid size: {path}')


def verify(root, manifest):
    validate_manifest(manifest)
    root = Path(root).resolve(strict=True)
    for item in manifest['files']:
        path = root / item['path']
        if not path.resolve().is_relative_to(root) or path.is_symlink() or not path.is_file():
            raise ValueError(f'missing/unsafe source file: {item["path"]}')
        if path.stat().st_size != item['bytes'] or digest(path) != item['sha256']:
            raise ValueError(f'source hash/size mismatch: {item["path"]}')


def prepare(manifest_path, cache, mirror=None):
    manifest = json.loads(Path(manifest_path).read_text())
    validate_manifest(manifest)
    revision = hashlib.sha256(json.dumps(manifest, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    parent = Path(cache) / manifest['id'] / 'sources'
    dest = parent / revision
    if dest.exists():
        verify(dest, manifest)
        return dest
    if mirror:
        verify(mirror, manifest)
    parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix='.prepare-', dir=parent))
    try:
        for item in manifest['files']:
            target = stage / item['path']
            target.parent.mkdir(parents=True, exist_ok=True)
            if mirror:
                shutil.copyfile(Path(mirror) / item['path'], target)
            else:
                url = item.get('url', '')
                if not url.startswith('https://'):
                    raise ValueError(f'expected explicit HTTPS URL: {item["path"]}')
                with urllib.request.urlopen(url, timeout=60) as response, target.open('wb') as stream:
                    shutil.copyfileobj(response, stream)
        verify(stage, manifest)
        (stage / 'source-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        os.replace(stage, dest)
        return dest
    except Exception as exc:
        (stage / 'prepare-error.txt').write_text(str(exc))
        raise


def unpack_archive(source, manifest):
    import stat
    import zipfile
    verify(source, manifest)
    archive = manifest.get('archive')
    if not archive:
        return Path(source)
    if archive not in {f['path'] for f in manifest['files']}:
        raise ValueError('archive is not a verified source input')
    target = Path(source).parent.parent / 'unpacked' / digest(Path(source) / archive)
    if target.exists():
        index = json.loads((target/'unpacked-manifest.json').read_text())
        expected_paths = {entry['path'] for entry in index}
        actual_paths = {str(p.relative_to(target)) for p in target.rglob('*') if p.is_file() and '__pycache__' not in p.parts and p.suffix != '.pyc' and p.name != 'unpacked-manifest.json'}
        if expected_paths != actual_paths:raise ValueError('unpacked source file set changed')
        for entry in index:
            if digest(target/entry['path']) != entry['sha256']:
                raise ValueError('unpacked source changed')
        return target
    target.parent.mkdir(parents=True,exist_ok=True)
    stage=Path(tempfile.mkdtemp(prefix='.unpack-',dir=target.parent))
    with zipfile.ZipFile(Path(source)/archive) as pack:
        for info in pack.infolist():
            path=PurePosixPath(info.filename)
            if path.is_absolute() or '..' in path.parts or '\\' in info.filename or stat.S_ISLNK(info.external_attr>>16):
                raise ValueError(f'unsafe archive member: {info.filename}')
        pack.extractall(stage)
    index=[{'path':str(p.relative_to(stage)),'sha256':digest(p)} for p in sorted(stage.rglob('*')) if p.is_file()]
    (stage/'unpacked-manifest.json').write_text(json.dumps(index,indent=2))
    os.replace(stage,target)
    return target
