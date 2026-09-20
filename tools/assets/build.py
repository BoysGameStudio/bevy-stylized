"""Build independent asset packs using prepared source manifests and explicit tools."""
from pathlib import Path
from task_runner import ResourceBlocked
from project import Project
import hashlib
import json
import os
import re
import shutil
from .prepare import verify, unpack_archive, digest
from .attempt import completed_or_new


def prepared(project, name):
    manifest=json.loads((project.root/'assets/sources'/f'{name}.json').read_text())
    revision=hashlib.sha256(json.dumps(manifest,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    source=project.cache/name/'sources'/revision
    if not source.exists():
        raise ResourceBlocked(f'prepare {name} first: assets prepare --pack {name}')
    verify(source,manifest)
    return unpack_archive(source,manifest)


def verify_run(path):
    path = Path(path).resolve(strict=True)
    record = json.loads((path / 'build-manifest.json').read_text())
    if (not isinstance(record, dict)
            or record.get('schema') != 'bevy_stylized.build/v1'
            or not isinstance(record.get('inputs'), dict)
            or not isinstance(record.get('files'), dict)
            or not record['files']):
        raise ValueError('invalid or empty build completion manifest')
    for name, expected in record['files'].items():
        if (not isinstance(name, str) or not Path(name).parts
                or Path(name).is_absolute() or '..' in Path(name).parts
                or not isinstance(expected, str)
                or not re.fullmatch(r'[0-9a-f]{64}', expected)):
            raise ValueError('invalid build artifact path/hash')
        artifact = path / name
        if (artifact.is_symlink() or not artifact.resolve().is_relative_to(path)
                or not artifact.is_file()):
            raise ValueError(f'missing/unsafe build artifact: {name}')
        if digest(artifact) != expected:
            raise ValueError(f'generated artifact changed: {artifact}')
    return path


def compiler_inputs(root):
    """Hash the complete local compiler source/configuration closure."""
    paths = [root/'Cargo.toml']
    for name in ['tools', 'profile-genshin', 'core', 'gltf']:
        crate = root/'crates'/f'bevy-stylized-{name}'
        paths.append(crate/'Cargo.toml')
        paths.extend((crate/'src').rglob('*.rs'))
    return {str(p.relative_to(root)): digest(p) for p in sorted(paths)}


def finish(path, inputs):
    files={str(p.relative_to(path)):digest(p) for p in sorted(path.rglob('*')) if p.is_file() and p.name not in {'build-manifest.json', 'attempt.json'} and not p.name.endswith('.log')}
    Project.atomic_json(path/'build-manifest.json', {'schema':'bevy_stylized.build/v1','inputs':inputs,'files':files})
    return verify_run(path)


def build(project, pack, blender='blender'):
    if pack != 'genshin-odette':
        raise ValueError('supported build pack: genshin-odette')
    raw=prepared(project,'genshin-odette')
    scripts=sorted((project.root/'tools/blender').glob('*.py'))
    inputs={'orchestrator':digest(__file__),'attempt_runner':digest(project.root/'tools/assets/attempt.py'),'source_preparation':digest(project.root/'tools/assets/prepare.py'),
            'capture_validator':digest(project.root/'tools/assets/capture_case.py'),
            'material_sources':digest(project.root/'tools/assets/material_sources.py'),
            'rust_lock':digest(project.root/'Cargo.lock'),'rust_toolchain':digest(project.root/'rust-toolchain.toml'),
            'source_manifest':digest(project.root/'assets/sources/genshin-odette.json'),
            'scripts':{p.name:digest(p) for p in scripts}}
    inputs['authoring_schemas']={p.name:digest(p) for p in sorted((project.root/'schemas/authoring').glob('*.json'))}
    recipe=project.root/'assets/recipes/genshin-static-v4.json'
    addon=prepared(project,'gacha-authoring')
    inputs['recipe']=digest(recipe)
    inputs['addon_manifest']=digest(project.root/'assets/sources/gacha-authoring.json')
    # Compiler source changes invalidate the runtime package as well.
    inputs['compiler'] = compiler_inputs(project.root)
    key=hashlib.sha256(json.dumps(inputs,sort_keys=True).encode()).hexdigest()
    out=project.cache/pack/'builds'/key
    out, attempt = completed_or_new(out, inputs, verify_run)
    if attempt is None:
        return out
    with attempt:
        return execute_build(project, pack, blender, raw, addon, recipe, out, inputs, attempt)


def execute_build(project, pack, blender, raw, addon, recipe, out, inputs, attempt):
    manifest=project.root/'assets/sources/genshin-odette.json'
    tools=project.root/'tools/blender'
    # An isolated configuration avoids personal add-on dependencies and never
    # saves/replaces the user's preferences or current authoring scene.
    env=dict(os.environ)
    config=out/'blender-config'
    config.mkdir()
    runtime_addon=out/'tooling/gacha'
    shutil.copytree(addon,runtime_addon,ignore=shutil.ignore_patterns('__pycache__','*.pyc','unpacked-manifest.json'))
    env.update(BLENDER_USER_CONFIG=str(config),STYLIZED_ADDON_ROOT=str(runtime_addon))
    prefix=[blender,'--background','--python-exit-code','1','--python',tools/'bootstrap.py']
    attempt.run(project,prefix+['--python',tools/'import_character.py','--','--source',raw,'--manifest',manifest,'--recipe',recipe,'--out',out/'authoring'],out/'import.log',env)
    for script,arguments in [
        ('inspect_character.py',['--out',out/'authoring/audit.json']),
        ('capture_character.py',['--authoring',out/'authoring/authoring.json','--out',out/'reference']),
        ('export_character.py',['--authoring',out/'authoring/authoring.json','--out',out/'runtime'])]:
        attempt.run(project,[blender,'--background',out/'authoring/character.blend','--python-exit-code','1','--python',tools/'bootstrap.py','--python',tools/script,'--']+arguments,out/(script+'.log'),env)
    audit=json.loads((out/'authoring/audit.json').read_text())
    if audit['invalid_constraints'] or audit['invalid_drivers'] or any(not i['exists'] for i in audit['images']):
        raise ValueError('static authoring audit failed; inspect authoring/audit.json')
    attempt.run(project,['cargo','run','--locked','-p','bevy-stylized-tools','--features','profile-genshin','--','compile','--profile','genshin','--strict','--json','--out',out/'compiled',out/'runtime/character.glb'],out/'compile.log')
    attempt.run(project,[blender,'--background',out/'authoring/character.blend','--python-exit-code','1','--python',tools/'bootstrap.py','--python',tools/'character_cases.py','--','--build',out,'--out',out/'character-cases'],out/'character-cases.log',env)
    for script,source,target in [
        ('lighting_probe.py','character-cases','character-diagnostics'),
        ('matched_capture.py','character-diagnostics','character-matched')]:
        attempt.run(project,[blender,'--background',out/'authoring/character.blend','--python-exit-code','1','--python',tools/'bootstrap.py','--python',tools/script,'--','--case',out/source/'case.json','--out',out/target],out/(target+'.log'),env)
    shutil.rmtree(out/'tooling')
    return finish(out,inputs)
