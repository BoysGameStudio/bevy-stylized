"""Required single-view GPU workload collection, without implicit Golden acceptance."""
import json
import os
import subprocess
from pathlib import Path
from checks.realtime import validate_stats
from task_runner import CheckFailed, ResourceBlocked, invoke

CASES = {'hero-1', 'hero-3', 'character-32', 'portable-100', 'portable-500',
         'portable-1000', 'hero-skin-morph', 'hero-outline', 'hero-post'}


def validate(record, out):
    if (record.get('schema') != 'bevy_stylized.workloads/v1'
            or record.get('status') != 'PASS' or record.get('optimized') is not True
            or not record.get('device') or record.get('repeats') != 3
            or set(record.get('expected_cases', [])) != CASES):
        raise CheckFailed('workload report identity/configuration invalid')
    rows = record.get('cells', [])
    expected = {(name, repeat) for name in CASES for repeat in range(3)}
    if len(rows) != len(expected) or {(r.get('case'), r.get('repeat')) for r in rows} != expected:
        raise CheckFailed('workload cells missing or duplicated')
    for row in rows:
        required = dict(status='PASS', resolution=[512, 512], views=1, msaa=4,
                        requested_tier='Portable', warmup=60, requested_samples=600)
        if any(row.get(k) != v for k, v in required.items()) or not row.get('resolved'):
            raise CheckFailed('workload cell configuration differs')
        count = row.get('gpu_render_graph_ms', {}).get('count')
        if type(count) is not int or not 540 <= count <= 600:
            raise CheckFailed('insufficient GPU samples')
        validate_stats(row['gpu_render_graph_ms'], count, row.get('raw_gpu_ms', []))
        validate_stats(row.get('cpu_update_ms'), 600, row.get('raw_cpu_ms', []))
        features = row.get('active_features', {})
        if (features.get('post') != (row['case'] == 'hero-post')
                or features.get('skin_morph') != (row['case'] == 'hero-skin-morph')
                or features.get('outline_replicas') != (1 if row['case'] == 'hero-outline' else 0)):
            raise CheckFailed('named workload feature was inactive')
        path = Path(row.get('capture', ''))
        if (path.is_absolute() or '..' in path.parts
                or not (out/path).resolve().is_relative_to(out.resolve())
                or not (out/path).is_file()):
            raise CheckFailed('workload capture missing or outside output')
    return record


def run(project, out):
    out = project.resolve(out)
    if out.exists():
        raise CheckFailed('workloads requires a new output directory')
    if os.environ.get('STYLIZED_GPU') == 'disabled':
        raise ResourceBlocked('required GPU unavailable')
    log = project.run_output/'workloads.log'
    command = ['cargo', 'run', '--locked', '--release', '-p', 'showcase',
               '--features', 'inspector', '--', 'workloads', '--out', str(out)]
    with log.open('w') as stream:
        result = invoke(command, cwd=project.root, env=dict(os.environ, CARGO_INCREMENTAL='0'),
                        stdout=stream, stderr=subprocess.STDOUT)
    if result.returncode:
        if any(s in log.read_text() for s in ['required GPU unavailable', 'required GPU timestamps unavailable']):
            raise ResourceBlocked(f'workload GPU resources missing; inspect {log}')
        raise CheckFailed(f'workload collection failed; inspect {log}')
    path = out/'report.json'
    if not path.is_file():
        raise CheckFailed('workload report missing')
    record = validate(json.loads(path.read_text()), out)
    project.report('workloads', {'status': 'PASS', 'collection_status': 'PASS',
                                'regression_status': 'NOT_EXECUTED', 'absolute_budget_status': 'NOT_DEFINED',
                                'report': str(path), 'cells': len(record['cells'])})
    return {'status': 'PASS', 'report': str(path)}
