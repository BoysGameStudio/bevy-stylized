#!/usr/bin/env python3
"""Project tasks. Paths use --root/config; no implicit game checkout or asset download."""
import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from project import Project

class CheckFailed(RuntimeError):
    pass


GPU_TESTS=['golden_cel','golden_character','outline_advanced','instance_batching','morph_skinning','instance_table','interim_budget']


def run(project, command, env=None):
    subprocess.run(list(map(str,command)),cwd=project.root,env=env,check=True)


def run_tests(project, name, command, expected, env):
    env=dict(env);env.pop('GOLDEN_REBASELINE',None)
    result=subprocess.run(command,cwd=project.root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    project.output.mkdir(parents=True,exist_ok=True)
    log=project.output/(name+'.log');log.write_text(result.stdout)
    summaries=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',result.stdout)
    passed=sum(int(row[0]) for row in summaries)
    failed=sum(int(row[1]) for row in summaries)
    blocked=any(message in result.stdout for message in ['required GPU unavailable','accepted baseline missing','accepted regression baseline missing'])
    state={'suite':name,'expected':expected,'executed':passed+failed,'passed':passed,
           'failed':0 if blocked else failed,'blocked':failed if blocked else 0,
           'not_executed':max(0,expected-passed-failed),'exit_code':result.returncode,'log':str(log),
           'status':'PASS' if result.returncode==0 and passed==expected else ('BLOCKED' if blocked else 'FAIL')}
    project.report(name,state);print(json.dumps(state,indent=2))
    if state['status']=='BLOCKED':raise RuntimeError(f'{name} missing required resources; inspect {log}')
    if state['status']!='PASS':raise CheckFailed(f'{name} did not execute/pass the required coverage; inspect {log}')


def report_check(project, result):
    result['status']='FAIL' if result.get('errors') else 'PASS'
    print(json.dumps(result,indent=2));project.report(result['check'],result)
    if result.get('errors'):raise CheckFailed('validation failed: '+ '; '.join(result['errors']))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root');parser.add_argument('--config');parser.add_argument('--cache')
    sub=parser.add_subparsers(dest='command',required=True);sub.add_parser('paths')
    check=sub.add_parser('check');check.add_argument('--lane',choices=['docs','architecture','schema','cpu','gpu','asset-integration','reference','character-reference','inspector','realtime','independence','consumers'],required=True)
    check.add_argument('--required',action='store_true');check.add_argument('--asset');check.add_argument('--case');check.add_argument('--pack',default='synthetic');check.add_argument('--out')
    assets=sub.add_parser('assets');asset_sub=assets.add_subparsers(dest='action',required=True)
    for action in ['prepare','build']:
        p=asset_sub.add_parser(action);p.add_argument('--pack',required=True);p.add_argument('--manifest');p.add_argument('--mirror');p.add_argument('--blender',default='blender')
    golden=sub.add_parser('golden');g=golden.add_subparsers(dest='action',required=True)
    for action in ['compare','accept']:
        p=g.add_parser(action);p.add_argument('--candidate',required=True);p.add_argument('--baseline',required=True);p.add_argument('--blender',default='blender')
    capture=g.add_parser('capture');capture.add_argument('--asset',required=True);capture.add_argument('--out',required=True)
    args=parser.parse_args();project=Project(args.root,args.config,args.cache)
    if args.command=='paths':
        print(json.dumps({'root':str(project.root),'cache':str(project.cache),'output':str(project.output)},indent=2));return 0
    if args.command=='assets':
        if args.action=='build':
            from assets.build import build
            output=build(project,args.pack,args.blender)
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
            env=dict(os.environ,STYLIZED_REQUIRE_GPU='1',STYLIZED_TEST_ASSET=str(project.resolve(args.asset)),STYLIZED_CAPTURE_OUT=str(project.resolve(args.out)))
            run_tests(project,'asset-integration-required',['cargo','test','--locked','-p','stylized-integration-tests','--test','asset_adoption','declared_asset','--','--ignored','--nocapture'],1,env);return 0
        candidate=project.resolve(args.candidate);baseline=project.resolve(args.baseline)
        if not candidate.is_file():raise ValueError('candidate missing')
        if args.action=='compare':
            if not baseline.is_file():raise ValueError('accepted baseline missing')
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
    if lane in {'architecture','cpu','independence'}:
        from checks.dependencies import check as graph
        from checks.independence import check as independence
        report_check(project,graph(project.root));report_check(project,independence(project.root))
    if lane in {'schema','cpu'}:
        run(project,[sys.executable,'tools/checks/generate_attributes.py','--check'])
        run(project,['cargo','run','--locked','-p','bevy-stylized-tools','--features','schema','--bin','stylized-schema','--','schemas/runtime','--check'])
    if lane=='cpu':
        run(project,[sys.executable,'-m','unittest','discover','-s','tests/python'])
        run(project,['cargo','test','--locked','--workspace','--','--test-threads=1'])
    elif lane=='gpu':
        env=dict(os.environ,STYLIZED_REQUIRE_GPU='1')
        command=['cargo','test','--locked','-p','stylized-integration-tests']
        for name in GPU_TESTS:command+=['--test',name]
        suites=json.loads((project.root/'tests/suites.json').read_text())
        run_tests(project,'gpu-required',command+['--','--ignored','--nocapture','--test-threads=1'],sum(map(len,suites['gpu'].values())),env)
        run_tests(project,'synthetic-import',['cargo','test','--locked','-p','stylized-integration-tests','--test','asset_adoption','synthetic_character','--','--ignored','--nocapture'],1,env)
        project.report('gpu',{'status':'PASS','binaries':GPU_TESTS+['asset_adoption:synthetic_character'],'required':True})
    elif lane=='asset-integration':
        if not args.asset:raise ValueError('--asset must identify the explicitly prepared GLB')
        env=dict(os.environ,STYLIZED_REQUIRE_GPU='1',STYLIZED_TEST_ASSET=str(project.resolve(args.asset)),STYLIZED_CAPTURE_OUT=str(project.output/'asset-integration.png'))
        run_tests(project,'asset-integration-required',['cargo','test','--locked','-p','stylized-integration-tests','--test','asset_adoption','declared_asset','--','--ignored','--nocapture'],1,env)
        project.report('asset-integration',{'status':'PASS','asset':str(project.resolve(args.asset)),'scope':'adoption/shared materials/reload; not upstream parity'})
    elif lane=='realtime':
        if not args.asset or not args.out or args.case:raise CheckFailed('realtime requires --asset and a new --out, without --case')
        from checks.realtime import run as realtime
        print(json.dumps(realtime(project,args.asset,args.out),indent=2))
    elif lane=='inspector':
        if not args.asset or not args.out:raise CheckFailed('--asset and a new --out directory are required')
        output=project.resolve(args.out)
        if output.exists():raise CheckFailed('inspector output already exists')
        command=['cargo','run','--locked','-p','showcase','--features','inspector','--','inspect','--asset',str(project.resolve(args.asset)),'--headless','--exercise-reload','--out',str(output)]
        if args.case:command+=['--config',str(project.resolve(args.case))]
        result=subprocess.run(command,cwd=project.root,env=dict(os.environ,STYLIZED_REQUIRE_GPU='1'),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        project.output.mkdir(parents=True,exist_ok=True)
        log=project.output/'inspector.log';log.write_text(result.stdout)
        if 'required GPU unavailable' in result.stdout:raise RuntimeError(f'required GPU unavailable; inspect {log}')
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
        try:validate(case)
        except (ValueError,KeyError,TypeError) as error:raise CheckFailed(str(error)) from error
        if project.resolve(args.out).exists():raise CheckFailed('candidate output already exists')
        result=subprocess.run(['cargo','run','--locked','-p','reference-comparison','--','--character-case',str(case),'--out',str(project.resolve(args.out))]+(['--required'] if args.required else []),cwd=project.root,env=dict(os.environ,STYLIZED_REQUIRE_GPU='1'))
        if (project.resolve(args.out)/'report.json').exists():
            from assets.character_report import generate
            generate(project.resolve(args.out))
        if result.returncode:raise subprocess.CalledProcessError(result.returncode,result.args)
    elif lane=='reference':
        if not args.case:raise ValueError('--case must identify the prepared reference case JSON')
        env=dict(os.environ,STYLIZED_REQUIRE_GPU='1',STYLIZED_REFERENCE_CASE=str(project.resolve(args.case)))
        run(project,['cargo','run','--locked','-p','reference-comparison','--']+(['--required'] if args.required else []),env)
    elif lane=='consumers':
        from checks.consumers import validate
        print(json.dumps(validate(project),indent=2))
    elif lane=='independence':
        from checks.isolated import validate
        validate(project,asset=project.resolve(args.asset) if args.asset else None,case=project.resolve(args.case) if args.case else None)
    return 0


if __name__=='__main__':
    try:raise SystemExit(main())
    except CheckFailed as error:
        print(f'FAIL: {error}',file=sys.stderr);raise SystemExit(1)
    except subprocess.CalledProcessError as error:raise SystemExit(error.returncode)
    except (OSError,ValueError,RuntimeError,KeyError) as error:
        print(f'BLOCKED: {error}',file=sys.stderr);raise SystemExit(2)
