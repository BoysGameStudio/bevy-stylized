"""Required local sweep collection; CPU update timings are not GPU timestamps."""
import json
import os
import subprocess
from task_runner import CheckFailed, ResourceBlocked, invoke


def run(project, out, baseline=None):
    out=project.resolve(out)
    if out.exists():raise CheckFailed('performance output exists; use a new candidate')
    baseline=project.resolve(baseline) if baseline else None
    if baseline and not baseline.is_file():raise ResourceBlocked('accepted performance baseline missing')
    out.mkdir(parents=True)
    command=['cargo','run','--locked','--release','-p','regression-lab','--','--required','--out',str(out/'report.json')]
    if baseline:command+=['--gate',str(baseline)]
    env=dict(os.environ,CARGO_INCREMENTAL='0')
    # This CLI owns resource classification; don't replace it with the harness's
    # test-only resource panic or permit automatic Golden acceptance.
    env.pop('STYLIZED_REQUIRE_GPU',None)
    env.pop('GOLDEN_REBASELINE',None)
    log=out/'sweep.log'
    with log.open('w') as f:result=invoke(command,cwd=project.root,env=env,stdout=f,stderr=subprocess.STDOUT)
    if result.returncode:
        message=log.read_text()
        if 'BLOCKED: required GPU unavailable' in message or 'accepted regression baseline missing' in message:
            raise ResourceBlocked(f'performance resources missing; inspect {log}')
        raise CheckFailed(f'performance collection/gate failed; inspect {log}')
    path=out/'report.json'
    if not path.is_file():raise CheckFailed('performance report missing despite process success')
    record=json.loads(path.read_text())
    if record.get('schema')!='bevy_stylized.sweep/v2' or record.get('status')!='PASS' or record.get('collection_status')!='PASS':
        raise CheckFailed('performance report identity/status invalid')
    if baseline and record.get('regression_status')!='PASS':raise CheckFailed('required regression gate did not pass')
    report={'status':'PASS','collection_status':'PASS','regression_status':record['regression_status'],
            'absolute_budget_status':'NOT_DEFINED','metric':record['metric'],'report':str(path),
            'scope':'required fixed sweep collection and optional relative CPU update-time regression; no GPU-time or absolute-budget claim'}
    project.report('performance',report)
    return report
