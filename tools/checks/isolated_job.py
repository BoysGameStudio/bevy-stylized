"""Inner independence job. This file runs only in the restricted source copy."""
from pathlib import Path
import json
import os
import subprocess
import shutil
import sys

root=Path.cwd();sys.path.insert(0,str(root/'tools'))
from project import Project
from assets.build import build
project=Project(root)
# Check the mounted namespace, not merely an unset environment variable.
if (Path.home()/'Projects').exists():raise RuntimeError('sibling project directory visible')
if (root/'.git').exists():raise RuntimeError('gitless source test accidentally received .git')
if (root/'assets/local').exists():raise RuntimeError('asset cache was not empty')
steps=[]

def run(name,args,expected=0,env=None):
    print('ISOLATED_STEP',name,flush=True)
    result=subprocess.run(args,cwd=root,env=env)
    steps.append({'name':name,'exit_code':result.returncode,'expected':expected})
    if result.returncode!=expected:raise RuntimeError(f'{name}: got {result.returncode}, expected {expected}')

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
character=build(project,'genshin-odette','/opt/blender/blender')
run('asset-integration',['python3','tools/dev.py','check','--lane','asset-integration','--required','--asset',str(character/'compiled/character.glb')])
run('inspector-unit',['cargo','test','--locked','-p','showcase','--features','inspector','--lib','--','--test-threads=1'])
run('inspector',['python3','tools/dev.py','check','--lane','inspector','--required','--asset',str(character/'compiled/character.glb'),'--out',str(project.output/'inspector')])
for mode in ['scan','measure']:
    run('realtime-'+mode,['cargo','run','--locked','-p','showcase','--features','inspector','--','inspect','--asset',str(character/'compiled/character.glb'),'--'+mode,'--frames','60' if mode=='measure' else '24','--resolution','256','--out',str(project.output/('realtime-'+mode))])
character_case=character/'character-matched/case.json'
character_output=project.output/'character-reference'
run('character-reference-diagnostic',['python3','tools/dev.py','check','--lane','character-reference','--case',str(character_case),'--out',str(character_output)])
character_report=json.loads((character_output/'report.json').read_text())
if len(character_report['reports'])!=6:raise RuntimeError('character case coverage incomplete')
run('character-reference-strict',['python3','tools/dev.py','check','--lane','character-reference','--required','--case',str(character_case),'--out',str(project.output/'character-reference-strict')])
release=json.loads((root/'assets/sources/gacha-authoring.json').read_text())['release']
(project.output/'isolated-result.json').write_text(json.dumps({'status':'PASS','steps':steps,'gacha_release':release,'character_parity_status':character_report['parity_status'],'parity_status':character_report['parity_status'],'scope':'independence and current supported static character parity; archived upstream versions are outside acceptance'},indent=2)+'\n')
