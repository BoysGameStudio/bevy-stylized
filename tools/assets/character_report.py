"""Create a local, portable inspection page from existing comparison artifacts."""
import html,json
from pathlib import Path

def generate(folder):
    folder=Path(folder);report=json.loads((folder/'report.json').read_text())
    sections=[]
    for row in report['reports']:
        name=row['case'];metric=row['legacy_tier2'];safe=html.escape(name)
        sections.append(f'''<section><h2>{safe} · {html.escape(row['parity_status'])}</h2><p>PSNR {metric['psnr_db']:.2f} dB · mean {metric['mean_abs_delta']:.5f} · 最大非边缘色差 {metric['max_abs_delta_nonedge']*255:.0f}/255</p>
<div class="pair"><figure><img src="{safe}-blender.png"><figcaption>Blender / Gacha</figcaption></figure><figure><img src="{safe}-bevy.png"><figcaption>Bevy Stylized</figcaption></figure><figure><img src="{safe}-diff.png"><figcaption>Absolute RGB difference × 4 (diagnostic)</figcaption></figure></div>
<details><summary>完整判定与限制</summary><pre>{html.escape(json.dumps(row,ensure_ascii=False,indent=2))}</pre></details></section>''')
    text='''<!doctype html><meta charset="utf-8"><title>Static character rendering comparison</title><style>body{font:16px system-ui;background:#17191e;color:#ddd;max-width:1700px;margin:2em auto;padding:1em}.pair{display:flex;flex-wrap:wrap}figure{margin:8px;flex:1;min-width:280px}img{width:100%}pre{white-space:pre-wrap}section{margin:3em 0}button{padding:.5em}</style><h1>静态角色渲染对比</h1><p>诊断产物，不是已接受的 Golden。几何/色彩校准、重复性、数值误差和阴影资格分别报告。现有中位亮度阴影指标只是代理。</p><button onclick="document.querySelectorAll('.pair').forEach(e=>e.classList.toggle('blink'))">切换闪烁比较</button><style>.blink figure:nth-child(3){display:none}.blink{position:relative;max-width:512px;height:540px}.blink figure{position:absolute;inset:0}.blink figure:nth-child(2){animation:blink 1.6s steps(1) infinite}@keyframes blink{50%{opacity:0}}</style>'''+f"<p><strong>当前资格：{html.escape(report['parity_status'])}</strong> · {html.escape(report['threshold_policy'])}</p>"+''.join(sections)
    (folder/'index.html').write_text(text)

    # Bind runtime evidence to the source actually used for this invocation.
    import hashlib,os
    root=Path(__file__).resolve().parents[2]
    paths=[root/'Cargo.toml',root/'Cargo.lock',root/'rust-toolchain.toml']
    for directory in ['crates','labs/reference/src','tests/harness/src']:
        paths.extend(p for p in (root/directory).rglob('*') if p.is_file() and p.suffix in {'.rs','.wgsl','.toml'})
    source={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(set(paths))}
    (folder/'runtime-source.json').write_text(json.dumps({'files':source,'isolation_revision':os.environ.get('STYLIZED_SOURCE_REVISION'),'source_digest':hashlib.sha256(json.dumps(source,sort_keys=True).encode()).hexdigest()},indent=2)+'\n')
