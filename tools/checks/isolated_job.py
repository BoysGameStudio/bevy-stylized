"""Inner independence job. This file runs only in the restricted source copy."""
from pathlib import Path
import json
import os
import subprocess
import shutil
import sys
import uuid

root=Path.cwd();sys.path.insert(0,str(root/'tools'))
from project import Project
from task_runner import ResourceBlocked
from assets.build import build
project=Project(root)
# Check the mounted namespace, not merely an unset environment variable.
if (Path.home()/'Projects').exists():raise RuntimeError('sibling project directory visible')
if (root/'.git').exists():raise RuntimeError('gitless source test accidentally received .git')
if (root/'assets/local').exists():raise RuntimeError('asset cache was not empty')
steps=[]
qualification={'engineering_status':'INCOMPLETE','image_regression_status':'NOT_EXECUTED',
               'parity_status':'NOT_EXECUTED','performance_status':'NOT_EXECUTED'}

def run(name,args,expected=0,env=None):
    print('ISOLATED_STEP',name,flush=True)
    run_id = None
    if len(args)>2 and args[1]=='tools/dev.py':
        run_id=uuid.uuid4().hex
        args=args[:2]+['--run-id',run_id]+args[2:]
    source_sha256=project.source_identity() if run_id else None
    result=subprocess.run(args,cwd=root,env=env)
    step={'name':name,'run_id':run_id,'exit_code':result.returncode,'expected':expected}
    steps.append(step)
    if run_id:
        index=args.index('--run-id')+2
        lane=args[args.index('--lane')+1] if args[index]=='check' else args[index]+'-'+args[index+1]
        report=project.read_report(lane,run_id,source_sha256)
        step['report_status']=report['status']
        step['source_identity_verified']=True
        expected_status={0:'PASS',1:'FAIL',2:'BLOCKED'}[expected]
        if expected == 0 and report['status'] == 'BLOCKED':raise ResourceBlocked(f'{name}: required resources unavailable')
        if report['status'] != expected_status:raise RuntimeError(f'{name}: report state differs')
    if result.returncode!=expected:raise RuntimeError(f'{name}: got {result.returncode}, expected {expected}')

def main():
    for pack in ['genshin-odette','gacha-authoring']:
        run('prepare-'+pack,['python3','tools/dev.py','assets','prepare','--pack',pack,'--mirror','/inputs/'+pack])
    run('synthetic-rebuild',['python3','tools/dev.py','assets','prepare','--pack','synthetic'])
    run('cpu',['python3','tools/dev.py','check','--lane','cpu','--required'])
    run('missing-gpu-is-blocked',['python3','tools/dev.py','check','--lane','gpu','--required'],2,dict(os.environ,STYLIZED_GPU='disabled'))
    baseline=root/'fixtures/golden/cel-portable';backup=root/'target/negative-test-baseline'
    backup.parent.mkdir(parents=True,exist_ok=True);shutil.move(str(baseline),str(backup));baseline.mkdir()
    try:
        run('missing-baseline-is-blocked',['python3','tools/dev.py','check','--lane','gpu','--required'],2)
        if list(baseline.iterdir()):raise RuntimeError('a required check implicitly created an accepted baseline')
    finally:
        baseline.rmdir();shutil.move(str(backup),str(baseline))
    run('gpu',['python3','tools/dev.py','check','--lane','gpu','--required'])
    qualification['image_regression_status']='PASS'
    character=build(project,'genshin-odette','/opt/blender/blender')
    run('asset-integration',['python3','tools/dev.py','check','--lane','asset-integration','--required','--asset',str(character/'compiled/character.glb')])
    run('inspector-unit',['python3','tools/dev.py','check','--lane','inspector-cpu'])
    run('inspector',['python3','tools/dev.py','check','--lane','inspector','--required','--asset',str(character/'compiled/character.glb'),'--out',str(project.output/'inspector')])
    for mode in ['scan','measure']:
        run('realtime-'+mode,['cargo','run','--locked','-p','showcase','--features','inspector','--','inspect','--asset',str(character/'compiled/character.glb'),'--'+mode,'--frames','60' if mode=='measure' else '24','--resolution','256','--out',str(project.output/('realtime-'+mode))])
    qualification['engineering_status']='PASS'
    qualification['performance_status']='SMOKE_ONLY'
    character_case=character/'character-matched/case.json'
    character_output=project.output/'character-reference'
    run('character-reference-diagnostic',['python3','tools/dev.py','check','--lane','character-reference','--case',str(character_case),'--out',str(character_output)])
    character_report=json.loads((character_output/'report.json').read_text())
    expected=json.loads((root/'tests/suites.json').read_text())['character-reference']
    if [row['case'] for row in character_report['reports']]!=expected:raise RuntimeError('character case identity coverage differs')
    qualification['parity_status']=character_report['parity_status']
    run('character-reference-strict',['python3','tools/dev.py','check','--lane','character-reference','--required','--case',str(character_case),'--out',str(project.output/'character-reference-strict')])
    release=json.loads((root/'assets/sources/gacha-authoring.json').read_text())['release']
    (project.output/'isolated-result.json').write_text(json.dumps({'status':'PASS',**qualification,'steps':steps,'gacha_release':release,'character_parity_status':character_report['parity_status'],'parity_status':character_report['parity_status'],'scope':'independence and current supported static character parity; archived upstream versions are outside acceptance'},indent=2)+'\n')


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        status = 'BLOCKED' if isinstance(error, ResourceBlocked) else 'FAIL'
        project.output.mkdir(parents=True, exist_ok=True)
        (project.output/'isolated-result.json').write_text(json.dumps({
            'status': status, **qualification, 'steps': steps, 'error': str(error),
        }, indent=2)+'\n')
        raise SystemExit(2 if status == 'BLOCKED' else 1)
