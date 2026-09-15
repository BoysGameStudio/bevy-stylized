"""Review frame-indexed scans without conflating replay equality with temporal quality."""
import argparse,html,json
from pathlib import Path
from PIL import Image,ImageChops,ImageStat

def generate(folder):
    folder=Path(folder);report=json.loads((folder/'report.json').read_text())
    if report['schema']!='bevy_stylized.realtime-scan/v1':raise ValueError('scan report required')
    groups={};analysis=[]
    for phase in report['phases']:
        rows=sorted((r for r in report['rows'] if r['phase']==phase and r['repeat']==0),key=lambda r:r['frame'])
        entries=[];previous=None
        for row in rows:
            names=[image['file'] for image in row['images']]
            for name in names:
                if Path(name).name!=name:raise ValueError('unsafe image path')
            with Image.open(folder/names[0]) as image:current=image.convert('RGB')
            delta=sum(ImageStat.Stat(ImageChops.difference(previous,current)).mean)/3 if previous else 0
            with Image.open(folder/names[1]) as image:hist=image.convert('RGB').getchannel('B').histogram()
            geometry=sum(hist[201:])
            entries.append({'frame':row['frame'],'images':names,'mean_adjacent_delta_255':delta,'diagnostic_coverage_pixels':geometry})
            previous=current
        groups[phase]=entries
        analysis.append({'phase':phase,'frames':len(entries),'minimum_diagnostic_coverage':min(e['diagnostic_coverage_pixels'] for e in entries),'largest_changes':sorted(entries,key=lambda e:e['mean_adjacent_delta_255'],reverse=True)[:5]})
    result={'schema':'bevy_stylized.scan-review/v1','replay_status':report['status'],'nonempty_geometry':all(r['minimum_diagnostic_coverage']>=100 for r in analysis),'temporal_visual_status':'REQUIRES_REVIEW','interpretation':'adjacent-frame differences include expected camera motion and toon transitions; no error-derived masks or automatic temporal PASS','phases':analysis}
    (folder/'review.json').write_text(json.dumps(result,indent=2)+'\n')
    data=json.dumps(groups).replace('<','\\u003c')
    page='''<!doctype html><meta charset="utf-8"><title>Real-time scan review</title><style>body{font:16px system-ui;background:#17191e;color:#eee;margin:24px}header{display:flex;gap:20px;align-items:center}button,select{font:inherit;padding:7px}input{width:60%}.pair{display:flex;gap:16px;margin-top:16px}.pair figure{margin:0;width:48%}img{width:100%;max-width:800px}pre{white-space:pre-wrap}</style><h1>Static rendering motion review</h1><p>Live rendering at MSAA 4x. Repeatability is distinct from temporal visual quality. Capture readbacks are excluded from performance measurements.</p><header><select id="phase"></select><button id="play">Play / pause</button><input id="frame" type="range" min="0" value="0"><span id="label"></span></header><div class="pair"><figure><img id="a"><figcaption>Beauty</figcaption></figure><figure><img id="b"><figcaption>Lighting factors — blue marks geometry</figcaption></figure></div><details><summary>Measured changes and coverage</summary><pre>'''+html.escape(json.dumps(result,indent=2))+'''</pre></details><script>
const groups='''+data+''';const phase=document.getElementById('phase'),frame=document.getElementById('frame');for(const key of Object.keys(groups)){const o=document.createElement('option');o.text=key;phase.add(o)}
let revision=0;async function show(){const ticket=++revision;const rows=groups[phase.value];frame.max=rows.length-1;frame.value=Math.min(+frame.value,rows.length-1);const r=rows[frame.value];await Promise.all(r.images.map(src=>{const image=new Image();image.src=src;return image.decode()}));if(ticket!==revision)return;document.getElementById('a').src=r.images[0];document.getElementById('b').src=r.images[1];document.getElementById('label').textContent=`${r.frame}/${rows.length-1} · Δ ${r.mean_adjacent_delta_255.toFixed(2)}/255`}
phase.onchange=()=>{frame.value=0;show()};frame.oninput=show;let playing=false;document.getElementById('play').onclick=()=>playing=!playing;setInterval(()=>{if(playing){frame.value=(+frame.value+1)%groups[phase.value].length;show()}},1000/30);show();</script>'''
    (folder/'index.html').write_text(page)
    # Contact sheet highlights evenly spaced frames without selecting by error.
    sheet=Image.new('RGB',(8*200,3*220),(23,25,30))
    from PIL import ImageDraw,ImageFont
    draw=ImageDraw.Draw(sheet);font=ImageFont.load_default(size=14)
    for y,(phase,entries) in enumerate(groups.items()):
        for x in range(8):
            row=entries[round(x*(len(entries)-1)/7)]
            with Image.open(folder/row['images'][0]) as image:
                image=image.convert('RGB');image.thumbnail((196,196));sheet.paste(image,(x*200,y*220+20))
            draw.text((x*200+3,y*220+3),f"{phase} / {row['frame']}",fill='white',font=font)
    sheet.save(folder/'contact-sheet.png')
    return result

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('folder',type=Path);a=p.parse_args();print(json.dumps(generate(a.folder),indent=2))
