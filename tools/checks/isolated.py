"""Execute the project from a gitless, space-containing source copy in Bubblewrap.

Only system/toolchain files, Cargo caches, GPU devices and verified raw inputs
are mounted. No sibling project or previously generated character is visible.
"""
from pathlib import Path
from task_runner import ResourceBlocked, CheckFailed
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from assets.build import prepared


def source_snapshot(project, destination):
    if (project.root/'.git').exists():
        names=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'],cwd=project.root).split(b'\0')
        files=sorted({project.root/os.fsdecode(n) for n in names if n})
    else:
        files=[]
        for directory,dirs,names in os.walk(project.root):
            dirs[:]=[d for d in dirs if d not in {'target','.git','local','__pycache__'}]
            files.extend(Path(directory)/n for n in names)
    revision=hashlib.sha256()
    for path in files:
        if not path.is_file():continue
        if not path.resolve().is_relative_to(project.root):raise ValueError(f'external source symlink: {path}')
        rel=path.relative_to(project.root)
        if any(part in {'target','.git','__pycache__'} for part in rel.parts) or rel.parts[:2]==('assets','local'):continue
        data=path.read_bytes();revision.update(str(rel).encode()+b'\0'+data)
        out=destination/rel;out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(data)
    (destination/'SOURCE_REVISION').write_text('worktree-sha256:'+revision.hexdigest()+'\n')
    return revision.hexdigest()


def validate(project, asset=None, case=None):
    if asset or case:raise ValueError('independence rebuilds inputs; do not provide generated --asset/--case')
    if not shutil.which('bwrap'):raise ResourceBlocked('Bubblewrap required for isolated execution on this qualified Linux backend')
    raw={}
    for name in ['genshin-odette','gacha-authoring']:
        manifest=json.loads((project.root/'assets/sources'/f'{name}.json').read_text())
        key=hashlib.sha256(json.dumps(manifest,sort_keys=True,separators=(',',':')).encode()).hexdigest()
        folder=project.cache/name/'sources'/key
        from assets.prepare import verify
        if not folder.is_dir():raise ResourceBlocked(f'required source pack missing: {name}')
        verify(folder,manifest);raw[name]=folder
    blender=Path('/snap/blender/current')
    if not (blender/'blender').is_file():raise ResourceBlocked('this isolation recipe requires the qualified Blender installation')
    work=Path(tempfile.mkdtemp(prefix='stylized-independent-'))
    source=work/'source with spaces';source.mkdir()
    source_hash=source_snapshot(project,source)
    inside='/work/project with spaces'
    home=Path.home()
    # Cargo may reuse workspace binaries containing compile-time manifest paths.
    # Never let /work paths leak back into the host's ordinary target directory.
    cargo_target=project.root/'target/isolated-cargo'
    cargo_target.mkdir(parents=True,exist_ok=True)
    command=['bwrap','--die-with-parent','--unshare-net','--unshare-pid',
             '--ro-bind','/usr','/usr','--ro-bind','/etc','/etc','--ro-bind','/lib','/lib','--ro-bind','/lib64','/lib64',
             '--symlink','usr/bin','/bin','--symlink','usr/sbin','/sbin','--proc','/proc','--dev','/dev',
             '--ro-bind','/sys','/sys','--tmpfs','/tmp','--dir',str(home),
             '--ro-bind',str(home/'.rustup'),str(home/'.rustup'),
             '--bind',str(home/'.cargo'),str(home/'.cargo'),
             '--bind',str(cargo_target),'/cargo-target',
             '--ro-bind',str(blender),'/opt/blender',
             '--bind',str(source),inside,'--chdir',inside,
             '--setenv','CARGO_TARGET_DIR','/cargo-target','--setenv','STYLIZED_SOURCE_REVISION','worktree-sha256:'+source_hash,
             '--unsetenv','DISPLAY','--unsetenv','WAYLAND_DISPLAY']
    for device in sorted(Path('/dev').glob('nvidia*')):
        command+=['--dev-bind',str(device),str(device)]
    if Path('/dev/dri').exists():command+=['--dev-bind','/dev/dri','/dev/dri']
    for name,folder in raw.items():command+=['--ro-bind',str(folder),'/inputs/'+name]
    command+=['python3','tools/checks/isolated_job.py']
    log=project.run_output/'isolated.log';log.parent.mkdir(parents=True,exist_ok=True)
    with log.open('w') as output:result=subprocess.run(command,stdout=output,stderr=subprocess.STDOUT)
    summary=source/'target/stylized/isolated-result.json'
    report=json.loads(summary.read_text()) if summary.exists() else {'status':'FAIL','reason':'isolated job did not finish'}
    report.update(snapshot_sha256=source_hash,source_copy=str(source),log=str(log),exit_code=result.returncode,
                  network='disabled during all build/test jobs',input_scope='verified raw sources and toolchains only')
    project.report('independence-execution',report)
    print(json.dumps(report,indent=2))
    if result.returncode:
        if report.get('status') == 'BLOCKED':raise ResourceBlocked(f'isolated resources missing; inspect {log}')
        raise CheckFailed(f'isolated execution failed; inspect {log}')
    if report.get('status') != 'PASS':raise CheckFailed('isolated result missing or not passed')
