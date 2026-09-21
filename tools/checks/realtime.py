"""Repeatable scan and instrumented optimized measurement; no baseline acceptance."""
from pathlib import Path
from task_runner import ResourceBlocked, CheckFailed
import hashlib,json,math,subprocess,statistics,os


def validate_files(asset, files):
    """Bind all measurement runs to the scanned bundle's actual bytes."""
    if not isinstance(files, dict) or not files or asset.name not in files:
        raise CheckFailed('measurement bundle inventory missing')
    root = asset.parent.resolve()
    for name, digest in files.items():
        path = Path(name)
        if path.is_absolute() or '..' in path.parts or not path.parts:
            raise CheckFailed('unsafe measurement bundle path')
        candidate = root/path
        if not candidate.resolve().is_relative_to(root) or not candidate.is_file():
            raise CheckFailed('measurement bundle dependency missing/outside root')
        if hashlib.sha256(candidate.read_bytes()).hexdigest() != digest:
            raise CheckFailed('measurement bundle changed during collection')


def validate_stats(value, count, raw=None):
    if not isinstance(value, dict) or type(value.get('count')) is not int or value['count'] != count:
        raise CheckFailed('measurement sample count differs')
    fields = ['min','median','p95','max','mean']
    if any(type(value.get(k)) not in (int,float) or not math.isfinite(value[k]) or value[k] <= 0 for k in fields):
        raise CheckFailed('measurement timing must be finite and positive')
    if not (value['min'] <= value['median'] <= value['p95'] <= value['max'] and value['min'] <= value['mean'] <= value['max']):
        raise CheckFailed('measurement statistics are inconsistent')
    if raw is not None:
        if not isinstance(raw,list) or len(raw)!=count or any(type(x) not in (int,float) or not math.isfinite(x) or x<=0 for x in raw):
            raise CheckFailed('invalid raw GPU samples')
        ordered=sorted(raw)
        expected=dict(min=ordered[0],max=ordered[-1],median=statistics.median(raw),mean=statistics.mean(raw),p95=ordered[math.ceil(count*.95)-1])
        if any(not math.isclose(value[k],v,rel_tol=1e-8,abs_tol=1e-9) for k,v in expected.items()):
            raise CheckFailed('GPU statistics disagree with raw samples')


def validate_measurement(report, size, files, device=None, samples=600):
    """A successful process is insufficient evidence of a comparable GPU run."""
    expected={'schema':'bevy_stylized.realtime-performance/v1','status':'PASS',
              'optimized':True,'resolution_per_view':[size,size],'view_count':2,
              'msaa':4,'capture_readbacks':False,'diagnostic_readbacks':True,
              'pipelined_rendering':False}
    if any(report.get(k)!=v for k,v in expected.items()):
        raise CheckFailed('measurement configuration differs from requested cell')
    if report.get('files')!=files or not isinstance(report.get('device'),str) or not report['device']:
        raise CheckFailed('measurement asset/device identity missing or changed')
    if device is not None and report['device']!=device:
        raise CheckFailed('measurement device changed between repetitions')
    rows=report.get('reports')
    if not isinstance(rows,list) or len(rows)!=2 or {r.get('mode') for r in rows}!={'beauty','lighting'}:
        raise CheckFailed('measurement requires exactly one Beauty and Lighting cell')
    for row in rows:
        if row.get('warmup_render_frames')!=60 or row.get('requested_samples')!=samples:
            raise CheckFailed('measurement warm-up/sample request differs')
        timing=row.get('diagnostics',{}).get('render/inspector_frame/elapsed_gpu',{})
        count=timing.get('count')
        if type(count) is not int or not math.ceil(samples*.9)<=count<=samples:
            raise CheckFailed('insufficient or duplicate GPU samples')
        validate_stats(timing,count,row.get('raw_diagnostics',{}).get('render/inspector_frame/elapsed_gpu',[]))
        validate_stats(row.get('app_update_wall_ms'),samples)
        memory=row.get('memory')
        if memory is not None and (any(type(memory.get(k)) is not int or memory[k]<0 for k in ['allocated_bytes','reserved_bytes']) or memory['reserved_bytes']<memory['allocated_bytes']):
            raise CheckFailed('invalid allocator accounting')
    return report['device']

def run(project,asset,out):
    asset=project.resolve(asset);out=project.resolve(out)
    if os.environ.get("STYLIZED_GPU")=="disabled":raise ResourceBlocked("required GPU unavailable for realtime validation")
    if not asset.is_file():raise ResourceBlocked("required compiled asset missing")
    if out.exists():raise ValueError('realtime output must be a new directory')
    out.mkdir(parents=True)
    command=['cargo','build','--locked','--release','-p','showcase','--features','inspector']
    with (out/'build.log').open('w') as log:subprocess.run(command,cwd=project.root,stdout=log,stderr=subprocess.STDOUT,check=True)
    # Respect Cargo's configured target directory instead of assuming a host path.
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--no-deps','--format-version','1'],cwd=project.root,text=True))
    binary=Path(metadata['target_directory'])/'release/showcase'
    jobs=[('scan',['--scan','--frames','120','--resolution','1024'])]
    for size in [512,1024]:
        for repeat in range(3):jobs.append((f'perf-{size}-{repeat}',['--measure','--frames','600','--resolution',str(size)]))
    for name,args in jobs:
        print('REALTIME',name,flush=True)
        with (out/(name+'.log')).open('w') as log:result=subprocess.run([str(binary),'inspect','--asset',str(asset),*args,'--out',str(out/name)],cwd=project.root,stdout=log,stderr=subprocess.STDOUT)
        if result.returncode:
            message=(out/(name+'.log')).read_text()
            if 'required GPU' in message:raise ResourceBlocked(f'required GPU diagnostics unavailable; inspect {out/(name+".log")}')
            raise subprocess.CalledProcessError(result.returncode,result.args)
    from assets.realtime_report import generate
    review=generate(out/'scan')
    if review['replay_status']!='PASS' or not review['nonempty_geometry']:
        raise CheckFailed('scan replay or geometry coverage failed')
    files=json.loads((out/'scan/recipe.json').read_text())['files']
    validate_files(asset,files)
    device=None
    for size in [512,1024]:
        for repeat in range(3):
            report=json.loads((out/f'perf-{size}-{repeat}'/'report.json').read_text())
            device=validate_measurement(report,size,files,device)
    cells=[]
    for size in [512,1024]:
        for mode in ['beauty','lighting']:
            runs=[]
            for repeat in range(3):
                path=out/f'perf-{size}-{repeat}'/'report.json';report=json.loads(path.read_text())
                if not report['optimized'] or report['status']!='PASS':raise RuntimeError('optimized measurements required')
                row=next(r for r in report['reports'] if r['mode']==mode);timing=row['diagnostics']['render/inspector_frame/elapsed_gpu']
                runs.append({'report':str(path.relative_to(out)),'gpu_mean_ms':timing['mean'],'gpu_p95_ms':timing['p95'],'gpu_samples':timing['count'],'app_update_wall_ms':row['app_update_wall_ms'],'memory':row['memory']})
            cells.append({'resolution_per_view':[size,size],'views':2,'mode':mode,'gpu_mean_ms':statistics.mean(r['gpu_mean_ms'] for r in runs),'runs':runs})
    result={'schema':'bevy_stylized.realtime-validation/v1','collection_status':'PASS','replay_status':review['replay_status'],'temporal_visual_status':'REQUIRES_REVIEW','regression_status':'NOT_EXECUTED','absolute_budget_status':'NOT_DEFINED','device':device,'files':files,'draw_calls':'UNAVAILABLE; no inferred mesh-count replacement','scope':'instrumented two-view inspector render graph, not presented FPS or an accepted shipping budget','cells':cells}
    (out/'summary.json').write_text(json.dumps(result,indent=2)+'\n');project.report('realtime',result)
    return result
