"""Compile minimal consumers outside the workspace to expose feature-unification leaks."""
from pathlib import Path
from task_runner import ResourceBlocked, invoke
import json
import shutil
import subprocess
import tempfile


def validate(project, fetch=False):
    cases=[('core-min','bevy-stylized-core',False,[],'let _ = bevy_stylized_core::StylizedSurfaceClass::Cel;'),
           ('gltf-pure','bevy-stylized-gltf',False,[],'let _ = bevy_stylized_gltf::NPR_ATTRIBUTE_SPECS.len();'),
           ('facade-min','bevy-stylized',False,[],'let _ = bevy_stylized::StylizedPlugin::default();'),
           ('facade-default','bevy-stylized',True,[],'let _ = bevy_stylized::StylizedPlugin::default();'),
           ('facade-profile','bevy-stylized',False,['render','gltf','profile-genshin'],'let _ = bevy_stylized::StylizedAssetIntegrationPlugin;')]
    outcomes=[]
    for name,package,default,features,code in cases:
        with tempfile.TemporaryDirectory(prefix='stylized-consumer-') as tmp:
            root=Path(tmp);(root/'src').mkdir()
            dependency=project.root/'crates'/package
            manifest='[package]\nname="independent-consumer"\nversion="0.0.0"\nedition="2024"\n[workspace]\nresolver="3"\n[dependencies]\n'
            manifest+=f'{package} = {{ path = {json.dumps(str(dependency))}, default-features = {str(default).lower()}, features = {json.dumps(features)} }}\n'
            (root/'Cargo.toml').write_text(manifest);(root/'src/main.rs').write_text('fn main() { '+code+' }\n')
            shutil.copyfile(project.root/'Cargo.lock',root/'Cargo.lock')
            if fetch:
                invoke(['cargo','fetch','--manifest-path',str(root/'Cargo.toml')],check=True,cwd=root)
            command=['cargo','check','--offline','--manifest-path',str(root/'Cargo.toml'),'--target-dir',str(project.root/'target')]
            result=invoke(command,cwd=root,capture_output=True,text=True)
            log=project.run_output/(name+'.log');log.parent.mkdir(parents=True,exist_ok=True)
            log.write_text(result.stdout+result.stderr)
            if result.returncode:
                if any(marker in result.stderr for marker in ['attempting to make an HTTP request', 'no matching package named', 'failed to download']):
                    raise ResourceBlocked(f'{name}: offline dependencies unavailable; run consumers --fetch; inspect {log}')
                raise subprocess.CalledProcessError(result.returncode,command)

            raw=subprocess.check_output(['cargo','metadata','--offline','--format-version','1','--manifest-path',str(root/'Cargo.toml')],cwd=root)
            meta=json.loads(raw)
            if name in {'core-min','gltf-pure'}:
                names={p['name'] for p in meta['packages']}
                forbidden=names&{'bevy','bevy_render','bevy_pbr','bevy_mesh','bevy_asset'}
                if forbidden:raise ValueError(f'{name}: unexpected engine dependencies {forbidden}')
            outcomes.append({'case':name,'status':'PASS'})
    project.report('isolated-consumers',{'status':'PASS','cases':outcomes})
    return outcomes
