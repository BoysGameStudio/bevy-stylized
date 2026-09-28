#!/usr/bin/env python3
"""Project tasks. Paths use --root/config; no implicit game checkout or asset download."""
import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from project import Project
from assets.build import DEFAULT_RECIPE

from task_runner import CheckFailed, ResourceBlocked, run_tests, require_suites, invoke, validate_gpu_suites, select_gpu_suites


def run(project, command, env=None):
    logs = project.run_output / 'logs'
    logs.mkdir(parents=True, exist_ok=True)
    index = len(list(logs.glob('command-*.log')))
    log = logs / f'command-{index}.log'
    print('COMMAND ' + ' '.join(map(str, command)), flush=True)
    with log.open('w') as stream:
        result = invoke(list(map(str, command)), cwd=project.root, env=env,
                        stdout=stream, stderr=subprocess.STDOUT)
    blocked = result.returncode != 0 and 'BLOCKED: required GPU unavailable' in log.read_text()
    status = 'BLOCKED' if blocked else 'FAIL' if result.returncode else 'PASS'
    project.report(f'command-{index}', {'status':status,
                                      'command':list(map(str, command)),
                                      'exit_code':result.returncode, 'log':str(log)})
    if result.returncode:
        if blocked:
            raise ResourceBlocked(f'required GPU unavailable; inspect {log}')
        raise subprocess.CalledProcessError(result.returncode, command)



def report_check(project, result):
    result['status']='FAIL' if result.get('errors') else 'PASS'
    print(json.dumps(result,indent=2));project.report(result['check'],result)
    if result.get('errors'):raise CheckFailed('validation failed: '+ '; '.join(result['errors']))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run-id');parser.add_argument('--root');parser.add_argument('--config');parser.add_argument('--cache')
    sub=parser.add_subparsers(dest='command',required=True);sub.add_parser('paths')
    check=sub.add_parser('check');check.add_argument('--lane',choices=['docs','architecture','schema','cpu','gpu','asset-integration','character-reference','inspector','realtime','performance','workloads','webgpu','browser-showcase','android-catalog','independence','consumers','inspector-cpu'],required=True)
    check.add_argument('--fetch',action='store_true',help='prepare independent consumer dependencies online');check.add_argument('--required',action='store_true');check.add_argument('--asset');check.add_argument('--case');check.add_argument('--pack',default='synthetic');check.add_argument('--out')
    check.add_argument('--suite', action='append', help='GPU lane only: select an exact registered test binary; reports partial coverage')
    check.add_argument('--recipe', help='independence lane only: rebuild this repository authoring recipe')
    check.add_argument('--baseline', help='performance lane only: compatible accepted relative-performance baseline')
    assets=sub.add_parser('assets');asset_sub=assets.add_subparsers(dest='action',required=True)
    for action in ['prepare','build']:
        p=asset_sub.add_parser(action);p.add_argument('--pack',required=True);p.add_argument('--manifest');p.add_argument('--mirror');p.add_argument('--blender',default='blender')
        if action == 'build':p.add_argument('--recipe', default=str(DEFAULT_RECIPE), help=f'authoring recipe (default: {DEFAULT_RECIPE})')
    golden=sub.add_parser('golden');g=golden.add_subparsers(dest='action',required=True)
    for action in ['compare','accept']:
        p=g.add_parser(action);p.add_argument('--candidate',required=True);p.add_argument('--baseline',required=True);p.add_argument('--blender',default='blender')
    capture=g.add_parser('capture');capture.add_argument('--asset',required=True);capture.add_argument('--out',required=True)
    args = parser.parse_args()
    if args.command == 'check' and args.recipe and args.lane != 'independence':
        parser.error('check --recipe is supported only by the independence lane')
    if args.command == 'check' and args.baseline and args.lane != 'performance':
        parser.error('check --baseline is supported only by the performance lane')
    if args.command == 'check' and args.lane == 'independence' and not args.recipe:
        args.recipe = str(DEFAULT_RECIPE)
    project = Project(args.root, args.config, args.cache)
    if args.command == 'paths':
        return execute(project, args)
    lane = args.lane if args.command == 'check' else args.command + '-' + args.action
    if lane == 'gpu' and getattr(args, 'suite', None):
        lane = 'gpu-selected'
    try:
        project.begin_run(lane, {key: getattr(args, key, None) for key in
                                ['asset', 'case', 'manifest', 'candidate', 'baseline', 'recipe']}, args.run_id)
        if getattr(args, 'suite', None):
            project.identity['options'] = {'gpu_suites': args.suite}
        print(f'RUN {project.run_id}', flush=True)
        execute(project, args)
        project.finish_run('PASS', 'completed')
        return 0
    except BaseException as error:
        if isinstance(error, ResourceBlocked):
            status, reason, code = 'BLOCKED', 'required-resource-missing', 2
        elif isinstance(error, KeyboardInterrupt):
            status, reason, code = 'FAIL', 'interrupted', 1
        else:
            reasons = [(CheckFailed, 'validation-failed'),
                       (subprocess.CalledProcessError, 'process-failed'),
                       (ValueError, 'input-contract-invalid'),
                       (OSError, 'io-failed')]
            reason = next((reason for kind, reason in reasons if isinstance(error, kind)), 'internal-error')
            status, code = 'FAIL', 1
        if project.run_id and hasattr(project, 'identity'):
            project.finish_run(status, reason, error=str(error),
                               exit_code=getattr(error, 'returncode', code))
        print(f'{status}: {error}', file=sys.stderr)
        return code


def execute(project, args):
    if getattr(args, 'suite', None) and (args.command != 'check' or args.lane != 'gpu'):
        raise CheckFailed('--suite is only valid for the GPU lane')
    if args.command == 'check' and args.fetch and args.lane != 'consumers':
        raise CheckFailed('--fetch is only valid for consumers')
    if args.command=='paths':
        print(json.dumps({'root':str(project.root),'cache':str(project.cache),'output':str(project.output)},indent=2));return 0
    if args.command=='assets':
        if args.action=='build':
            from assets.build import build
            output=build(project,args.pack,args.blender,args.recipe)
            print(json.dumps({'status':'PASS','build':str(output)},indent=2));return 0
        if args.pack=='synthetic':
            from assets.synthetic import generate
            output=generate(project.root/'fixtures/synthetic/character')
            print(json.dumps({'status':'PASS','asset':str(output)},indent=2));return 0
        from assets.prepare import prepare,unpack_archive,validate_manifest
        path=project.resolve(args.manifest or f'assets/sources/{args.pack}.json');manifest=json.loads(path.read_text());validate_manifest(manifest)
        if manifest['id']!=args.pack:raise ValueError('pack differs from manifest')
        source=prepare(path,project.cache,project.resolve(args.mirror) if args.mirror else None)
        print(json.dumps({'status':'PASS','source':str(source),'unpacked':str(unpack_archive(source,manifest))},indent=2));return 0
    if args.command=='golden':
        if args.action=='capture':
            if not project.resolve(args.asset).is_file():raise ResourceBlocked('required compiled asset missing')
            env=dict(os.environ,STYLIZED_REQUIRE_GPU='1',STYLIZED_TEST_ASSET=str(project.resolve(args.asset)),STYLIZED_CAPTURE_OUT=str(project.resolve(args.out)))
            require_suites([run_tests(project,'asset-integration-required',['cargo','test','--locked','-p','stylized-integration-tests','--test','asset_adoption','declared_asset'],json.loads((project.root/'tests/suites.json').read_text())['asset-integration'],env)]);return 0
        candidate=project.resolve(args.candidate);baseline=project.resolve(args.baseline)
        if not candidate.is_file():raise ResourceBlocked('candidate missing')
        if args.action=='compare':
            if not baseline.is_file():raise ResourceBlocked('accepted baseline missing')
            run(project,[args.blender,'--background','--python-exit-code','1','--python',project.root/'tools/blender/compare_golden.py','--',candidate,baseline]);return 0
        # Explicit acceptance preserves provenance and records the previous bytes.
        import shutil,hashlib,tempfile
        provenance=candidate.with_suffix('.golden.json')
        if not provenance.is_file():raise ValueError('candidate provenance required for acceptance')
        data=json.loads(provenance.read_text())
        expected=data.get('image_sha256')
        if not expected or hashlib.sha256(candidate.read_bytes()).hexdigest()!=expected:raise ValueError('candidate provenance/image hash mismatch')
        if baseline.is_relative_to(project.root/'fixtures/golden') and data.get('asset_ownership')!='first-party':
            raise ValueError('public baseline requires first-party ownership provenance')
        baseline.parent.mkdir(parents=True,exist_ok=True)
        if baseline.exists():
            history=project.output/'baseline-history';history.mkdir(parents=True,exist_ok=True)
            previous=hashlib.sha256(baseline.read_bytes()).hexdigest();shutil.copy2(baseline,history/(previous+'.png'))
            old=baseline.with_suffix('.golden.json')
            if old.exists():shutil.copy2(old,history/(previous+'.golden.json'))
        # Stage both files before acceptance; only this explicit task may replace them.
        with tempfile.TemporaryDirectory(dir=baseline.parent,prefix='.accept-') as tmp:
            staged=Path(tmp);shutil.copy2(candidate,staged/'image.png');shutil.copy2(provenance,staged/'provenance.json')
            os.replace(staged/'image.png',baseline);os.replace(staged/'provenance.json',baseline.with_suffix('.golden.json'))
        return 0
    lane=args.lane
    if lane in {'docs','cpu'}:run(project,[sys.executable,'tools/checks/check_docs.py'])
    if lane in {'architecture','cpu'}:
        from checks.dependencies import independent_check as graph
        from checks.independence import check as independence
        report_check(project,graph(project));report_check(project,independence(project.root))
    if lane in {'schema','cpu'}:
        run(project,[sys.executable,'tools/checks/generate_attributes.py','--check'])
        run(project,['cargo','run','--locked','-p','bevy-stylized-tools','--features','schema','--bin','stylized-schema','--','schemas/runtime','--check'])
    if lane=='cpu':
        run(project,[sys.executable,'-m','unittest','discover','-s','tests/python'])
        run(project,['cargo','test','--locked','--workspace','--','--test-threads=1'])
    elif lane=='gpu':
        env = dict(os.environ, STYLIZED_REQUIRE_GPU='1')
        suites = json.loads((project.root/'tests/suites.json').read_text())
        states = []
        selected = select_gpu_suites(suites['gpu'], getattr(args, 'suite', None))
        partial = bool(getattr(args, 'suite', None))
        for binary, identities in selected.items():
            states.append(run_tests(project, binary,
                ['cargo','test','--locked','-p','stylized-integration-tests','--test',binary], identities, env))
        from task_runner import aggregate
        totals = {key: sum(s[key] for s in states) for key in
                  ['expected','executed','passed','failed','blocked','not_executed']}
        project.report('gpu-selected-required' if partial else 'gpu-required', dict(totals, status=aggregate(states)['status'], suites=states,
                                          coverage='selected-suites' if partial else 'full-registry',
                                          full_gpu_suite=not partial))
        if not partial:
            states.append(run_tests(project,'synthetic-import',
                ['cargo','test','--locked','-p','stylized-integration-tests','--test','asset_adoption','synthetic_character'],
                suites['synthetic-import'],env))
        require_suites(states)
        project.report('gpu-selected' if partial else 'gpu', {'status':'PASS','binaries':list(selected), 'required':True,
                               'coverage':'selected-suites' if partial else 'full-registry',
                               'full_gpu_suite':not partial})
    elif lane=='asset-integration':
        if not args.asset:raise ValueError('--asset must identify the explicitly prepared GLB')
        if not project.resolve(args.asset).is_file():raise ResourceBlocked('required compiled asset missing')
        env=dict(os.environ,STYLIZED_REQUIRE_GPU='1',STYLIZED_TEST_ASSET=str(project.resolve(args.asset)),STYLIZED_CAPTURE_OUT=str(project.run_output/'asset-integration.png'))
        require_suites([run_tests(project,'asset-integration-required',['cargo','test','--locked','-p','stylized-integration-tests','--test','asset_adoption','declared_asset'],json.loads((project.root/'tests/suites.json').read_text())['asset-integration'],env)])
        project.report('asset-integration',{'status':'PASS','asset':str(project.resolve(args.asset)),'scope':'adoption/shared materials/reload; not upstream parity'})
    elif lane=='performance':
        if not args.out or args.asset or args.case:raise CheckFailed('performance requires a new --out directory, without --asset/--case')
        from checks.performance import run as performance
        print(json.dumps(performance(project,args.out,args.baseline),indent=2))
    elif lane=='workloads':
        if not args.out or args.asset or args.case:raise CheckFailed('workloads requires a new --out, without --asset/--case')
        from checks.workloads import run as workloads
        print(json.dumps(workloads(project,args.out),indent=2))
    elif lane=='webgpu':
        if not args.asset or not args.out or args.case:raise CheckFailed('webgpu requires --asset and a new --out, without --case')
        from checks.browser import run as browser
        print(json.dumps(browser(project,args.asset,args.out),indent=2))
    elif lane=='browser-showcase':
        if not args.out or args.asset or args.case:raise CheckFailed('browser-showcase requires a new --out, without --asset/--case')
        from checks.browser import run_showcase
        print(json.dumps(run_showcase(project,args.out),indent=2))
    elif lane=='android-catalog':
        if not args.out or args.asset or args.case:raise CheckFailed('android-catalog requires a new --out, without --asset/--case')
        from checks.android_catalog import run as android_catalog
        print(json.dumps(android_catalog(project,args.out),indent=2))
    elif lane=='realtime':
        if not args.asset or not args.out or args.case:raise CheckFailed('realtime requires --asset and a new --out, without --case')
        from checks.realtime import run as realtime
        print(json.dumps(realtime(project,args.asset,args.out),indent=2))
    elif lane=='inspector':
        if not args.asset or not args.out:raise CheckFailed('--asset and a new --out directory are required')
        if not project.resolve(args.asset).is_file():raise ResourceBlocked('required compiled asset missing')
        output=project.resolve(args.out)
        if output.exists():raise CheckFailed('inspector output already exists')
        command=['cargo','run','--locked','-p','showcase','--features','inspector','--','inspect','--asset',str(project.resolve(args.asset)),'--headless','--exercise-reload','--out',str(output)]
        if args.case:command+=['--config',str(project.resolve(args.case))]
        result=invoke(command,cwd=project.root,env=dict(os.environ,STYLIZED_REQUIRE_GPU='1'),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        project.output.mkdir(parents=True,exist_ok=True)
        log=project.run_output/'inspector.log';log.write_text(result.stdout)
        if 'required GPU unavailable' in result.stdout:raise ResourceBlocked(f'required GPU unavailable; inspect {log}')
        if result.returncode:raise CheckFailed(f'inspector failed ({result.returncode}); inspect {log}')
        report=json.loads((output/'capture.json').read_text())
        import hashlib
        if report.get('status')!='PASS' or report.get('generation',0)<2 or report.get('retired_captures',0)<1 or report.get('ready_render_frames',0)<24:raise CheckFailed('inspector readiness/reload coverage incomplete')
        for name in ['a.png','b.png']:
            if hashlib.sha256((output/name).read_bytes()).hexdigest()!=report['images'][name]:raise CheckFailed('inspector capture identity mismatch')
        project.report('inspector',{'status':'PASS','capture':str(output),'generation':report['generation'],'retired_captures':report['retired_captures'],'scope':'live capture and stale-request rejection, not upstream parity'})
    elif lane=='character-reference':
        if not args.case or not args.out:raise ValueError('--case and new --out required')
        from assets.capture_case import validate
        case=project.resolve(args.case)
        if not case.is_file():raise ResourceBlocked('required reference case missing')
        try:validate(case)
        except (ValueError,KeyError,TypeError) as error:raise CheckFailed(str(error)) from error
        if project.resolve(args.out).exists():raise CheckFailed('candidate output already exists')
        command=['cargo','run','--locked','-p','reference-comparison','--','--character-case',str(case),'--out',str(project.resolve(args.out))]+(['--required'] if args.required else [])
        result=invoke(command,cwd=project.root,env=dict(os.environ,STYLIZED_REQUIRE_GPU='1'),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        log=project.run_output/'character-reference.log';log.write_text(result.stdout)
        if (project.resolve(args.out)/'report.json').exists():
            from assets.character_report import generate
            generate(project.resolve(args.out))
        if result.returncode:
            if 'BLOCKED: required GPU unavailable' in result.stdout:raise ResourceBlocked(f'required GPU unavailable; inspect {log}')
            raise subprocess.CalledProcessError(result.returncode,result.args)
    elif lane=='inspector-cpu':
        run(project,['cargo','test','--locked','-p','showcase','--features','inspector','--lib','--','--test-threads=1'])
        run(project,['cargo','clippy','--locked','-p','showcase','--features','inspector','--all-targets','--','-D','warnings'])
    elif lane=='consumers':
        from checks.consumers import validate
        print(json.dumps(validate(project, fetch=args.fetch),indent=2))
    elif lane=='independence':
        from checks.isolated import validate
        validate(project,asset=project.resolve(args.asset) if args.asset else None,case=project.resolve(args.case) if args.case else None,recipe=args.recipe)
    return 0


if __name__=='__main__':
    try:raise SystemExit(main())
    except CheckFailed as error:
        print(f'FAIL: {error}',file=sys.stderr);raise SystemExit(1)
    except (OSError,ValueError,RuntimeError,KeyError) as error:
        print(f'FAIL: {error}',file=sys.stderr);raise SystemExit(1)
