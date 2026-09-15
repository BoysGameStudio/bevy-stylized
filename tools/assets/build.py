"""Build independent asset packs using prepared source manifests and explicit tools."""
from pathlib import Path
from task_runner import ResourceBlocked, invoke
import hashlib
import json
import os
import shutil
import subprocess
import sys
from .prepare import verify, unpack_archive, digest


def prepared(project, name):
    manifest=json.loads((project.root/'assets/sources'/f'{name}.json').read_text())
    revision=hashlib.sha256(json.dumps(manifest,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    source=project.cache/name/'sources'/revision
    if not source.exists():raise ResourceBlocked(f'prepare {name} first: assets prepare --pack {name}')
    verify(source,manifest)
    return unpack_archive(source,manifest)


def run_job(project, command, log, env=None):
    log.parent.mkdir(parents=True,exist_ok=True)
    with log.open('w') as output:
        result=invoke(list(map(str,command)),cwd=project.root,env=env,stdout=output,stderr=subprocess.STDOUT)
    if result.returncode:raise subprocess.CalledProcessError(result.returncode, command)


def verify_run(path):
    record=json.loads((path/'build-manifest.json').read_text())
    for name,expected in record['files'].items():
        if digest(path/name)!=expected:raise ValueError(f'generated artifact changed: {path/name}')
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
    files={str(p.relative_to(path)):digest(p) for p in sorted(path.rglob('*')) if p.is_file() and p.name not in {'build-manifest.json'} and not p.name.endswith('.log')}
    (path/'build-manifest.json').write_text(json.dumps({'schema':'bevy_stylized.build/v1','inputs':inputs,'files':files},indent=2)+'\n')
    return verify_run(path)


def build(project, pack, blender='blender'):
    if pack not in {'genshin-odette','reference'}:raise ValueError('supported build packs: genshin-odette, reference')
    raw=prepared(project,'genshin-odette')
    scripts=sorted((project.root/'tools/blender').glob('*.py'))
    inputs={'orchestrator':digest(__file__),'source_preparation':digest(project.root/'tools/assets/prepare.py'),
            'capture_validator':digest(project.root/'tools/assets/capture_case.py'),
            'material_sources':digest(project.root/'tools/assets/material_sources.py'),
            'rust_lock':digest(project.root/'Cargo.lock'),'rust_toolchain':digest(project.root/'rust-toolchain.toml'),
            'source_manifest':digest(project.root/'assets/sources/genshin-odette.json'),
            'scripts':{p.name:digest(p) for p in scripts}}
    inputs['authoring_schemas']={p.name:digest(p) for p in sorted((project.root/'schemas/authoring').glob('*.json'))}
    recipe=project.root/'assets/recipes/genshin-static-v4.json'
    if pack=='genshin-odette':
        addon=prepared(project,'gacha-authoring')
        inputs['recipe']=digest(recipe)
        inputs['addon_manifest']=digest(project.root/'assets/sources/gacha-authoring.json')
        # Compiler source changes invalidate the runtime package as well.
        inputs['compiler'] = compiler_inputs(project.root)
    else:
        addon=prepared(project,'gacha-reference')
        inputs['addon_manifest']=digest(project.root/'assets/sources/gacha-reference.json')
        inputs['lock']=digest(project.root/'docs/reference/reference-toolchain.toml')
        inputs['fixture_generator']=digest(project.root/'tools/blender/make_reference_scene.py')
    key=hashlib.sha256(json.dumps(inputs,sort_keys=True).encode()).hexdigest()
    out=project.cache/pack/'builds'/key
    if (out/'build-manifest.json').exists():return verify_run(out)
    if out.exists():raise ValueError(f'incomplete build at {out}; inspect it before retrying with a new recipe')
    out.mkdir(parents=True)
    manifest=project.root/'assets/sources/genshin-odette.json'
    tools=project.root/'tools/blender'
    if pack=='genshin-odette':
        # An isolated configuration avoids personal add-on dependencies and never
        # saves/replaces the user's preferences or current authoring scene.
        env=dict(os.environ)
        config=out/'blender-config';config.mkdir()
        runtime_addon=out/'tooling/gacha'
        shutil.copytree(addon,runtime_addon,ignore=shutil.ignore_patterns('__pycache__','*.pyc','unpacked-manifest.json'))
        env.update(BLENDER_USER_CONFIG=str(config),STYLIZED_ADDON_ROOT=str(runtime_addon))
        prefix=[blender,'--background','--python-exit-code','1','--python',tools/'bootstrap.py']
        run_job(project,prefix+['--python',tools/'import_character.py','--','--source',raw,'--manifest',manifest,'--recipe',recipe,'--out',out/'authoring'],out/'import.log',env)
        for script,arguments in [
            ('inspect_character.py',['--out',out/'authoring/audit.json']),
            ('capture_character.py',['--authoring',out/'authoring/authoring.json','--out',out/'reference']),
            ('export_character.py',['--authoring',out/'authoring/authoring.json','--out',out/'runtime'])]:
            run_job(project,[blender,'--background',out/'authoring/character.blend','--python-exit-code','1','--python',tools/'bootstrap.py','--python',tools/script,'--']+arguments,out/(script+'.log'),env)
        audit=json.loads((out/'authoring/audit.json').read_text())
        if audit['invalid_constraints'] or audit['invalid_drivers'] or any(not i['exists'] for i in audit['images']):
            raise ValueError('static authoring audit failed; inspect authoring/audit.json')
        run_job(project,['cargo','run','--locked','-p','bevy-stylized-tools','--features','profile-genshin','--','compile','--profile','genshin','--strict','--json','--out',out/'compiled',out/'runtime/character.glb'],out/'compile.log')
        run_job(project,[blender,'--background',out/'authoring/character.blend','--python-exit-code','1','--python',tools/'bootstrap.py','--python',tools/'character_cases.py','--','--build',out,'--out',out/'character-cases'],out/'character-cases.log',env)
        for script,source,target in [
            ('lighting_probe.py','character-cases','character-diagnostics'),
            ('matched_capture.py','character-diagnostics','character-matched')]:
            run_job(project,[blender,'--background',out/'authoring/character.blend','--python-exit-code','1','--python',tools/'bootstrap.py','--python',tools/script,'--','--case',out/source/'case.json','--out',out/target],out/(target+'.log'),env)
        shutil.rmtree(out/'tooling')
    else:
        vendor=addon/'setup_wizard/shaders/gi/HoYoverse - Genshin Impact v3.4.blend'
        run_job(project,[blender,'--background','--python-exit-code','1','--python',tools/'prepare_reference_textures.py','--','--source',raw,'--manifest',manifest,'--vendor-blend',vendor,'--vendor-sha256',digest(vendor),'--out',out/'textures'],out/'textures.log')
        (out/'fixture').mkdir()
        run_job(project,[blender,'--background','--python-exit-code','1','--python',tools/'make_reference_scene.py','--','--out',out/'fixture/reference_scene.blend'],out/'fixture.log')
        run_job(project,[blender,'--background',out/'fixture/reference_scene.blend','--python-exit-code','1','--python',tools/'export_reference.py','--','--signature','expected-signature.fixture-v1.json','--role','fixture','--out',out/'fixture-export'],out/'fixture-export.log')
        run_job(project,[blender,'--background','--python-exit-code','1','--python',tools/'render_golden.py','--','--blend',out/'fixture/reference_scene.blend','--out-dir',out/'upstream','--vendor-blend',vendor,'--texture-dir',out/'textures'],out/'capture.log')
        run_job(project,[blender,'--background',vendor,'--python-exit-code','1','--python',tools/'export_reference.py','--','--role','main','--out',out/'export-main'],out/'export-main.log')
        record={'schema':'bevy_stylized.reference-case/v1','fixture':'fixture-export/reference_scene.glb','upstream':'upstream/gacha_upstream.png','sidecar':'export-main/export-sidecar.json','textures':'textures','output':os.path.relpath(project.output/'reference',out)}
        (out/'case.json').write_text(json.dumps(record,indent=2)+'\n')
    return finish(out,inputs)
