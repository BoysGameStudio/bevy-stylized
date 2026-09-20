"""Validate all local Cargo dependency edges and normal/build closures."""
from pathlib import Path
import json
import subprocess

PRODUCTS = {
 'bevy-stylized': {'bevy-stylized-core', 'bevy-stylized-render', 'bevy-stylized-gltf', 'bevy-stylized-profile-genshin'},
 'bevy-stylized-core': set(),
 'bevy-stylized-render': {'bevy-stylized-core'},
 'bevy-stylized-gltf': {'bevy-stylized-core'},
 'bevy-stylized-profile-genshin': {'bevy-stylized-core'},
 'bevy-stylized-tools': {'bevy-stylized-core', 'bevy-stylized-gltf', 'bevy-stylized-profile-genshin'},
}


def violations(metadata, root):
    root = Path(root).resolve()
    packages = {p['id']: p for p in metadata['packages']}
    nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    local = {p['id']: p for p in packages.values() if p.get('source') is None}
    errors = []
    for package in packages.values():
        if "tenacity" in (package.get("source") or "").lower():
            errors.append(f"external game source: {package["name"]}")
    for package in local.values():
        if not Path(package['manifest_path']).resolve().is_relative_to(root):
            errors.append(f"external local package: {package['name']}")
        for dep in package['dependencies']:
            if dep.get('path') and not Path(dep['path']).resolve().is_relative_to(root):
                errors.append(f"external path: {package['name']} -> {dep['name']}")
            if package['name'] in PRODUCTS and dep.get('kind') != 'dev' and dep.get('path'):
                if dep['name'] not in PRODUCTS[package['name']]:
                    errors.append(f"forbidden product edge: {package['name']} -> {dep['name']}")
    for package in local.values():
        if package['name'] not in PRODUCTS:
            continue
        queue, seen = [package['id']], set()
        while queue:
            ident = queue.pop()
            if ident in seen:
                continue
            seen.add(ident)
            for dep in nodes.get(ident, {}).get('deps', []):
                if not any(k['kind'] in (None, 'build') for k in dep['dep_kinds']):
                    continue
                target = packages[dep['pkg']]
                if dep['pkg'] in local and target['name'] not in PRODUCTS:
                    errors.append(f"non-product in {package['name']} closure: {target['name']}")
                queue.append(dep['pkg'])
    return sorted(set(errors))


def check(root, *, locked=True):
    result = subprocess.run(['cargo', 'metadata', *(['--locked'] if locked else []), '--offline', '--format-version', '1'],
                            cwd=root, capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(result.stderr)
    metadata = json.loads(result.stdout)
    return {'check': 'dependency-graph', 'errors': violations(metadata, root),
            'workspace_members': len(metadata['workspace_members']),
            'default_members': len(metadata['workspace_default_members']),
            'scope': 'complete resolved normal/build graph; optional feature combinations additionally require isolated consumers'}


def independent_check(project):
    """Check a source snapshot outside ancestor Cargo config discovery.

    This is dependency-graph evidence, not Bubblewrap/runtime qualification.
    Only the disposable lock may change to resolve registry replacements for
    enclosing-workspace patches. The original source and lock stay untouched.
    """
    from checks.isolated import source_snapshot
    from task_runner import ResourceBlocked
    import hashlib
    import tempfile
    source = Path(tempfile.mkdtemp(prefix='stylized graph '))
    digest = source_snapshot(project, source)
    try:
        result = check(source, locked=False)
    except RuntimeError as error:
        if any(marker in str(error) for marker in
               ['attempting to make an HTTP request', 'no matching package named', 'failed to download']):
            raise ResourceBlocked(f'independent graph dependencies unavailable: {error}') from error
        raise
    lock = source / 'Cargo.lock'
    result.update(mode='independent-source-snapshot', snapshot_sha256=digest,
                  source_copy=str(source),
                  resolved_lock_sha256=hashlib.sha256(lock.read_bytes()).hexdigest(),
                  source_lock_mutated=False,
                  scope='strict standalone dependency graph; not isolated runtime, image or parity qualification')
    return result
