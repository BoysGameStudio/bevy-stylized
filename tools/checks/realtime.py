"""Repeatable scan and instrumented optimized measurement; no baseline acceptance."""
from pathlib import Path
from task_runner import ResourceBlocked
import json,subprocess,statistics,os

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
    if not review['nonempty_geometry']:raise RuntimeError('scan contains empty geometry frames')
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
    result={'schema':'bevy_stylized.realtime-validation/v1','collection_status':'PASS','replay_status':review['replay_status'],'temporal_visual_status':'REQUIRES_REVIEW','draw_calls':'UNAVAILABLE; no inferred mesh-count replacement','scope':'instrumented two-view inspector render graph, not presented FPS or an accepted shipping budget','cells':cells}
    (out/'summary.json').write_text(json.dumps(result,indent=2)+'\n');project.report('realtime',result)
    return result
