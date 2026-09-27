"""Local standalone WebGPU/native consumer and strict same-fixture comparison.

Requires Node 22+, a Chromium browser, the pinned Rust wasm target, matching
wasm-bindgen-cli and a live native display. No personal browser profile is used.
"""
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import hashlib
import json
import os
import shutil
import struct
import subprocess
import tempfile
import threading
import tomllib
from task_runner import CheckFailed, ResourceBlocked, invoke


def validate_capture(value, backend, primitives):
    required = dict(schema='bevy_stylized.browser-smoke/v1', status='PASS',
                    backend=backend, width=512, height=512, adopted_primitives=primitives,
                    classes=['Cel', 'Character', 'Hero'])
    if any(value.get(k) != v for k, v in required.items()):
        raise CheckFailed('browser/native capture identity, backend or coverage differs')
    if value.get('stable_render_frames', 0) < 32 or value.get('outline_primitives', 0) < 1:
        raise CheckFailed('browser/native readiness or outline coverage missing')
    rgba = value.get('rgba', [])
    if len(rgba) != 512*512*4 or any(type(x) is not int or not 0 <= x <= 255 for x in rgba):
        raise CheckFailed('invalid browser/native readback')
    data = bytes(rgba)
    if len(set(zip(data[0::4], data[1::4], data[2::4]))) < 16:
        raise CheckFailed('browser/native capture is empty or unshaded')
    return data


def expected_native_backend():
    """本机侧的期望 backend（严格身份，无静默回落）：Linux/其他 = Vulkan；
    macOS = Metal（MoltenVK 上游不可用，见 ADR-0021 证据）。"""
    import sys
    return 'Metal' if sys.platform == 'darwin' else 'Vulkan'


# ---------------------------------------------------------------------------
# 浏览器比较 envelope（docs/reference/08 §5；ADR-0021）。
# 只对 "WebGPU 浏览器（Dawn/Tint）vs 本机 Vulkan（wgpu/naga）" 域生效；
# 边缘判定规则与 labs/validation 的 tier2 `is_edge_pixel` 逐位一致。

TIER2_EDGE_GRADIENT = 32.0 / 255.0
"""tier2 3×3 亮度梯度阈值（与 labs/validation/src/tier2.rs 相同）。"""
ENVELOPE_EDGE_RADIUS = 3
"""差异像素 7×7 窗口内（任一图像）必须存在 tier2 边缘像素。"""
ENVELOPE_MAX_DIFFERING_PIXELS = 13
"""差异像素上限（512² 的 0.005%）。"""
ENVELOPE_MAX_CHANNEL_DELTA = 32
"""最大通道差上限（32/255）。"""


def _f32(value):
    """舍入到 IEEE 单精度（与 tier2 的 Rust f32 算术逐位一致）。"""
    return struct.unpack('f', struct.pack('f', value))[0]


def _luminance(data, offset):
    """BT.709 亮度（tier2 同一权重；逐步 f32 舍入保持与 Rust 逐位一致）。"""
    r = _f32(data[offset] / 255.0)
    g = _f32(data[offset + 1] / 255.0)
    b = _f32(data[offset + 2] / 255.0)
    return _f32(_f32(_f32(_f32(0.2126) * r) + _f32(_f32(0.7152) * g))
                + _f32(_f32(0.0722) * b))


def _is_edge_pixel(data, width, height, x, y, threshold=TIER2_EDGE_GRADIENT):
    """labs/validation/src/tier2.rs `is_edge_pixel` 的逐位 Python 移植。"""
    center = _luminance(data, (y * width + x) * 4)
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            if dx == 0 and dy == 0:
                continue
            nx = max(0, min(x + dx, width - 1))
            ny = max(0, min(y + dy, height - 1))
            if abs(_luminance(data, (ny * width + nx) * 4) - center) > threshold:
                return True
    return False


def _edge_within(data, width, height, x, y, radius=ENVELOPE_EDGE_RADIUS):
    """(x, y) 的 (2r+1)² 窗口内是否存在 tier2 边缘像素。"""
    for dy in range(-radius, radius + 1):
        for dx in range(-radius, radius + 1):
            nx = max(0, min(x + dx, width - 1))
            ny = max(0, min(y + dy, height - 1))
            if _is_edge_pixel(data, width, height, nx, ny):
                return True
    return False


def evaluate_browser_envelope(a, b, width=512, height=512):
    """评估编译器分歧 envelope（仅浏览器 vs 本机域；reference/08 §5）。

    返回结构化结果：差异像素数、最大通道差、是否全部差异都落在 tier2
    边缘窗口内（edge_confined）及 envelope 是否成立（pass）。
    """
    differing = [i for i in range(width * height)
                 if a[i * 4:i * 4 + 4] != b[i * 4:i * 4 + 4]]
    max_delta = max((abs(x - y) for x, y in zip(a, b)), default=0)
    edge_confined = all(
        _edge_within(a, width, height, i % width, i // width)
        or _edge_within(b, width, height, i % width, i // width)
        for i in differing)
    passed = (len(differing) <= ENVELOPE_MAX_DIFFERING_PIXELS
              and max_delta <= ENVELOPE_MAX_CHANNEL_DELTA and edge_confined)
    return {
        'policy': 'docs/reference/08#5-browser-webgpu-dawn-vs-native-vulkanmetal',
        'pass': passed,
        'differing_pixels': len(differing),
        'max_channel_delta': max_delta,
        'edge_confined': edge_confined,
        'limits': {
            'differing_pixels': ENVELOPE_MAX_DIFFERING_PIXELS,
            'max_channel_delta': ENVELOPE_MAX_CHANNEL_DELTA,
            'edge_window_radius': ENVELOPE_EDGE_RADIUS,
            'edge_gradient': TIER2_EDGE_GRADIENT,
        },
    }


def primitive_count(asset):
    data = asset.read_bytes()
    if data[:4] != b'glTF' or data[16:20] != b'JSON':
        raise CheckFailed('WebGPU fixture requires a compiled GLB')
    size = struct.unpack_from('<I', data, 12)[0]
    doc = json.loads(data[20:20+size])

    def count(index, ancestors):
        if index in ancestors:
            raise CheckFailed('cyclic GLB scene')
        node = doc['nodes'][index]
        own = len(doc['meshes'][node['mesh']]['primitives']) if 'mesh' in node else 0
        return own + sum(count(i, ancestors | {index}) for i in node.get('children', []))

    return sum(count(i, set()) for i in doc['scenes'][doc.get('scene', 0)]['nodes'])


def run(project, asset, out):
    asset, out = project.resolve(asset), project.resolve(out)
    if out.exists():
        raise CheckFailed('WebGPU output exists; use a new candidate directory')
    if not asset.is_file() or any(not asset.with_suffix(ext).is_file() for ext in ['.npr.json', '.view.json']):
        raise ResourceBlocked('compiled character GLB, NPR sidecar and view required')
    chrome = shutil.which(os.environ.get('STYLIZED_CHROME', 'google-chrome'))
    node = shutil.which('node')
    bindgen = shutil.which(os.environ.get('STYLIZED_WASM_BINDGEN', 'wasm-bindgen'))
    if not all([chrome, node, bindgen]):
        raise ResourceBlocked('WebGPU needs Chromium, Node 22+ and matching wasm-bindgen-cli')
    if not (os.environ.get('DISPLAY') or os.environ.get('WAYLAND_DISPLAY')):
        raise ResourceBlocked('WebGPU/native comparison requires a local display')
    lock = tomllib.loads((project.root/'Cargo.lock').read_text())
    versions = {p['version'] for p in lock['package'] if p['name'] == 'wasm-bindgen'}
    if len(versions) != 1:
        raise CheckFailed('ambiguous wasm-bindgen lock version')
    wasm_version = versions.pop()
    if subprocess.check_output([bindgen, '--version'], text=True).strip() != f'wasm-bindgen {wasm_version}':
        raise ResourceBlocked(f'wasm-bindgen-cli {wasm_version} required')
    if int(subprocess.check_output([node, '--version'], text=True).strip().lstrip('v').split('.')[0]) < 22:
        raise ResourceBlocked('Node 22+ required for built-in WebSocket')
    flags = json.loads(os.environ.get('STYLIZED_CHROME_FLAGS', '[]'))
    if not isinstance(flags, list) or any(not isinstance(s, str) for s in flags):
        raise CheckFailed('STYLIZED_CHROME_FLAGS must be a JSON string array')
    out.mkdir(parents=True)
    site = out/'site'
    bundle = site/'assets/character'
    shutil.copytree(asset.parent, bundle)
    shutil.copyfile(asset, bundle/'character.glb')
    shutil.copyfile(project.root/'tests/browser/index.html', site/'index.html')
    files = {str(p.relative_to(bundle)): hashlib.sha256(p.read_bytes()).hexdigest()
             for p in sorted(bundle.rglob('*')) if p.is_file()}
    primitives = primitive_count(asset)
    version = tomllib.loads((project.root/'Cargo.toml').read_text())['workspace']['dependencies']['bevy']['version']
    manifest = '[workspace]\n[package]\nname="stylized-browser-smoke"\nversion="0.0.0"\nedition="2024"\n[dependencies]\n'
    manifest += 'bevy={version='+json.dumps(version)+',default-features=false,features=["bevy_winit","x11","webgpu","bevy_gltf","bevy_scene","bevy_animation","png","reflect_auto_register"]}\n'
    # Apple 上 wgpu 仅在 vulkan-portability feature 下编译 Vulkan 后端（MoltenVK）；
    # 该 feature 在非 Apple 目标为空转依赖，不改变 Linux 合格路径。
    manifest += 'bevy_render={version='+json.dumps(version)+',default-features=false,features=["vulkan-portability"]}\n'
    for name, path in [('bevy-stylized', 'crates/bevy-stylized'), ('golden-harness', 'tests/harness')]:
        manifest += name+'={path='+json.dumps(str(project.root/path))+'}\n'
    manifest += 'serde_json="1"\nwasm-bindgen='+json.dumps('='+wasm_version)+'\n'
    env = dict(os.environ, CARGO_INCREMENTAL='0', CARGO_TARGET_DIR=str(project.root/'target/browser-cargo'),
               STYLIZED_BROWSER_ASSETS='assets', STYLIZED_BROWSER_SIDECAR=str(asset.with_suffix('.npr.json')),
               STYLIZED_BROWSER_VIEW=str(asset.with_suffix('.view.json')), STYLIZED_BROWSER_PRIMITIVES=str(primitives))

    def command(args, cwd, name, timeout=None):
        with (out/(name+'.log')).open('w') as stream:
            result = invoke(args, cwd=cwd, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=timeout)
        if result.returncode:
            text = (out/(name+'.log')).read_text()
            if name == 'browser-run' and (out/'browser-capture.json').is_file():
                captured = json.loads((out/'browser-capture.json').read_text())
                if captured.get('result', {}).get('status') == 'BLOCKED':
                    raise ResourceBlocked('required hardware WebGPU adapter unavailable; inspect browser capture')
            if any(s in text for s in ["target may not be installed", 'failed to download', 'attempting to make an HTTP request', 'no matching package named']):
                raise ResourceBlocked(f'WebGPU toolchain/cache missing; inspect {out/(name+".log")}')
            raise CheckFailed(f'WebGPU {name} failed; inspect {out/(name+".log")}')

    with tempfile.TemporaryDirectory(prefix='stylized-browser-') as temporary:
        consumer = Path(temporary)
        (consumer/'src').mkdir()
        (consumer/'Cargo.toml').write_text(manifest)
        shutil.copyfile(project.root/'tests/browser/smoke.rs', consumer/'src/main.rs')
        for name in ['rust-toolchain.toml', 'Cargo.lock']:
            shutil.copyfile(project.root/name, consumer/name)
        # Resolving outside ancestor Cargo configuration is deliberate. Only the
        # disposable lock changes, and only cached dependencies may be used.
        command(['cargo', 'build', '--offline', '--release', '--target', 'wasm32-unknown-unknown'], consumer, 'wasm-build')
        command([bindgen, '--target', 'web', '--out-dir', str(site/'pkg'),
                 str(project.root/'target/browser-cargo/wasm32-unknown-unknown/release/stylized-browser-smoke.wasm')], consumer, 'wasm-bindgen')
        standalone_lock = hashlib.sha256((consumer/'Cargo.lock').read_bytes()).hexdigest()
        env['STYLIZED_BROWSER_ASSETS'] = str(site/'assets')
        env['STYLIZED_CAPTURE_OUT'] = str(out/'native-capture.json')
        command(['cargo', 'build', '--offline', '--locked', '--release'], consumer, 'native-build')
        command([str(project.root/'target/browser-cargo/release/stylized-browser-smoke')], consumer, 'native-run', 120)
    handler = partial(SimpleHTTPRequestHandler, directory=str(site))
    server = ThreadingHTTPServer(('127.0.0.1', 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        command([node, str(project.root/'tools/checks/browser_capture.mjs'), chrome,
                 f'http://127.0.0.1:{server.server_port}/', str(out), json.dumps(flags)], project.root, 'browser-run', 240)
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
    browser = json.loads((out/'browser-capture.json').read_text())
    native = json.loads((out/'native-capture.json').read_text())
    if browser.get('errors'):
        raise CheckFailed('browser reported asynchronous errors')
    a = validate_capture(browser['result'], 'BrowserWebGpu', primitives)
    b = validate_capture(native, expected_native_backend(), primitives)
    from PIL import Image
    Image.frombytes('RGBA', (512, 512), a).save(out/'browser.png')
    Image.frombytes('RGBA', (512, 512), b).save(out/'native.png')
    differences = [abs(x-y) for x, y in zip(a, b)]
    exact = a == b
    # exact 优先；不一致时评估 reference/08 §5 的编译器分歧 envelope（仅此域）。
    envelope = None if exact else evaluate_browser_envelope(a, b)
    passed = exact or (envelope is not None and envelope['pass'])
    result = dict(schema='bevy_stylized.webgpu-validation/v1',
                  status='PASS' if passed else 'FAIL',
                  exact_match=exact, envelope=envelope,
                  browser=browser['browser'], browser_adapter=browser['result'].get('adapter'),
                  native_device=native['device'], flags=flags, files=files, standalone_lock_sha256=standalone_lock,
                  max_channel_difference=max(differences), differing_channels=sum(x != 0 for x in differences),
                  source=project.source_identity(),
                  scope='512x512 same-fixture browser-WebGPU vs native (Vulkan on Linux, Metal on macOS) reproduction (exact pixel equality preferred; approved compiler-divergence envelope per docs/reference/08 §5 as fallback for this domain only): compiled static asset, Cel/Character/Hero and native hull outlines; no WebGL2, shadow/prepass/table/deformation or general browser support claim')
    (out/'report.json').write_text(json.dumps(result, indent=2)+'\n')
    project.report('webgpu', result)
    if result['status'] != 'PASS':
        raise CheckFailed('same-fixture browser/native pixels differ beyond the approved envelope; inspect captures without rebaselining')
    return result

def run_showcase(project, out):
    """Browser showcase lane: every catalog scene executes in the disposable
    browser through the CaseDriver host (apps/browser-showcase) and is judged
    against the canonical showcase golden (fixtures/golden/showcase/) — exact
    match preferred, reference/08 §5 envelope as the only fallback. Browser
    baselines are never created; a scene failing both is a FAIL with evidence.
    """
    out = project.resolve(out)
    if out.exists():
        raise CheckFailed('browser-showcase output exists; use a new candidate directory')
    chrome = shutil.which(os.environ.get('STYLIZED_CHROME', 'google-chrome'))
    node = shutil.which('node')
    bindgen = shutil.which(os.environ.get('STYLIZED_WASM_BINDGEN', 'wasm-bindgen'))
    if not all([chrome, node, bindgen]):
        raise ResourceBlocked('browser showcase needs Chromium, Node 22+ and matching wasm-bindgen-cli')
    lock = tomllib.loads((project.root/'Cargo.lock').read_text())
    versions = {p['version'] for p in lock['package'] if p['name'] == 'wasm-bindgen'}
    if len(versions) != 1:
        raise CheckFailed('ambiguous wasm-bindgen lock version')
    wasm_version = versions.pop()
    if subprocess.check_output([bindgen, '--version'], text=True).strip() != f'wasm-bindgen {wasm_version}':
        raise ResourceBlocked(f'wasm-bindgen-cli {wasm_version} required')
    if int(subprocess.check_output([node, '--version'], text=True).strip().lstrip('v').split('.')[0]) < 22:
        raise ResourceBlocked('Node 22+ required for built-in WebSocket')
    flags = json.loads(os.environ.get('STYLIZED_CHROME_FLAGS', '[]'))
    if not isinstance(flags, list) or any(not isinstance(s, str) for s in flags):
        raise CheckFailed('STYLIZED_CHROME_FLAGS must be a JSON string array')
    suites = json.loads((project.root/'tests/suites.json').read_text())
    identities = suites['gpu']['showcase_golden']
    scenes = [s.removeprefix('showcase_golden_').replace('_', '-') for s in identities]
    baselines = project.root/'fixtures/golden/showcase'
    missing = [s for s in scenes if not (baselines/f'{s}.png').is_file()]
    if missing:
        raise CheckFailed(f'canonical showcase baselines missing for: {missing}')
    out.mkdir(parents=True)
    site = out/'site'
    site.mkdir()
    shutil.copyfile(project.root/'apps/browser-showcase/index.html', site/'index.html')
    shutil.copytree(project.root/'fixtures/synthetic/reference-scene', site/'assets/reference-scene')

    def command(args, cwd, name, timeout=None):
        with (out/(name+'.log')).open('w') as stream:
            result = invoke(args, cwd=cwd, env=dict(os.environ, CARGO_INCREMENTAL='0'), stdout=stream, stderr=subprocess.STDOUT, timeout=timeout)
        if result.returncode:
            text = (out/(name+'.log')).read_text()
            if any(s in text for s in ["target may not be installed", 'failed to download', 'attempting to make an HTTP request', 'no matching package named']):
                raise ResourceBlocked(f'browser-showcase toolchain/cache missing; inspect {out/(name+".log")}')
            raise CheckFailed(f'browser-showcase {name} failed; inspect {out/(name+".log")}')

    command(['cargo', 'build', '--offline', '--release', '--target', 'wasm32-unknown-unknown', '-p', 'browser-showcase'], project.root, 'wasm-build', 600)
    command([bindgen, '--target', 'web', '--out-dir', str(site/'pkg'),
             str(project.root/'target/wasm32-unknown-unknown/release/browser-showcase.wasm')], project.root, 'wasm-bindgen')
    command(['cargo', 'build', '--offline', '--release', '-p', 'browser-showcase'], project.root, 'native-build', 600)
    # Same-machine native captures: the reference/08 §5 domain is browser
    # (Dawn/Tint) vs native (wgpu/naga) on one machine — never cross-platform.
    native_dir = out/'native'
    native_dir.mkdir()
    for scene in scenes:
        log = (out/(f'native-{scene}.log')).open('w')
        result = invoke([str(project.root/'target/release/browser-showcase')], cwd=project.root,
                        env=dict(os.environ, CARGO_INCREMENTAL='0',
                                 STYLIZED_BROWSER_SCENE=scene,
                                 STYLIZED_CAPTURE_OUT=str(native_dir/f'{scene}.json')),
                        stdout=log, stderr=subprocess.STDOUT, timeout=120)
        log.close()
        if result.returncode or not (native_dir/f'{scene}.json').is_file():
            raise CheckFailed(f'native reference capture failed for {scene}; inspect native-{scene}.log')
    handler = partial(SimpleHTTPRequestHandler, directory=str(site))
    server = ThreadingHTTPServer(('127.0.0.1', 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        command([node, str(project.root/'tools/checks/browser_showcase.mjs'), chrome,
                 f'http://127.0.0.1:{server.server_port}/', str(out), json.dumps(flags), json.dumps(scenes)],
                project.root, 'browser-run', max(240, 60*len(scenes)))
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
    captured = json.loads((out/'browser-showcase-capture.json').read_text())
    from PIL import Image
    captures = out/'captures'
    captures.mkdir()
    # Native-leg baseline = the current platform's accepted baseline: on macOS
    # the macos-metal overlay holds the native-wgpu platform truth; Linux has
    # no overlay and resolves to canonical. No browser-specific baselines.
    import sys
    overlay = baselines/'macos-metal' if sys.platform == 'darwin' else None
    reports = {}
    for scene in scenes:
        entry = captured['scenes'].get(scene, {'status': 'FAIL', 'reason': 'scene absent from browser capture'})
        if entry.get('status') != 'PASS':
            reports[scene] = {'status': entry.get('status', 'FAIL'), 'reason': entry.get('reason'), 'errors': entry.get('errors', [])}
            continue
        rgba = entry.get('rgba', [])
        if len(rgba) != 512*512*4 or any(type(x) is not int or not 0 <= x <= 255 for x in rgba):
            reports[scene] = {'status': 'FAIL', 'reason': 'invalid browser readback'}
            continue
        browser_data = bytes(rgba)
        Image.frombytes('RGBA', (512, 512), browser_data).save(captures/f'{scene}.png')
        native = json.loads((native_dir/f'{scene}.json').read_text())
        native_data = bytes(native['rgba'])
        # Chain of judgments, both per scene and never hidden:
        # (1) browser vs same-machine native — reference/08 §5 (exact preferred,
        #     approved envelope fallback for the Dawn/Tint vs naga domain only);
        # (2) native vs the platform's accepted golden (canonical on Linux;
        #     macos-metal overlay where accepted) at the golden tolerance
        #     (the same judgment the showcase_golden lane applies).
        exact = browser_data == native_data
        envelope = None if exact else evaluate_browser_envelope(browser_data, native_data)
        browser_passed = exact or (envelope is not None and envelope['pass'])
        baseline_path = overlay/f'{scene}.png' if overlay and (overlay/f'{scene}.png').is_file() else baselines/f'{scene}.png'
        canonical = baseline_path.read_bytes()
        reference = Image.open(baseline_path).convert('RGBA').tobytes()
        diffs = [abs(x-y) for x, y in zip(native_data, reference)]
        mean = sum(diffs)/len(diffs)
        import math
        mse = sum(d*d for d in diffs)/len(diffs)
        psnr = 99.0 if mse == 0 else 20*math.log10(255.0/math.sqrt(mse))
        native_passed = max(diffs, default=0) <= 4 and mean <= 0.5 and psnr >= 40.0
        identity = {key: entry.get(key) for key in
                    ['backend', 'device', 'resolved_tier', 'generation', 'experiment_tick', 'stamp', 'engine_git_commit']}
        identity['native_backend'] = native.get('backend')
        identity['native_device'] = native.get('device')
        passed = browser_passed and native_passed
        reports[scene] = {'status': 'PASS' if passed else 'FAIL',
                          'browser_vs_native': 'exact' if exact else ('envelope' if browser_passed else 'fail'),
                          'envelope': envelope,
                          'native_vs_canonical': {'pass': native_passed, 'max_channel_delta': max(diffs, default=0),
                                                  'mean_channel_delta': mean, 'psnr_db': psnr},
                          'identity': identity, 'errors': entry.get('errors', []),
                          'capture_sha256': hashlib.sha256(browser_data).hexdigest(),
                          'native_sha256': hashlib.sha256(native_data).hexdigest(),
                          'baseline': str(baseline_path.relative_to(project.root)),
                          'baseline_sha256': hashlib.sha256(canonical).hexdigest()}
    exact_count = sum(1 for r in reports.values() if r.get('browser_vs_native') == 'exact')
    envelope_count = sum(1 for r in reports.values() if r.get('browser_vs_native') == 'envelope')
    failed = [s for s, r in reports.items() if r['status'] != 'PASS']
    result = dict(schema='bevy_stylized.browser-showcase/v1',
                  status='PASS' if not failed else 'FAIL',
                  scenes=reports, total=len(scenes), exact=exact_count, envelope=envelope_count, failed=failed,
                  browser=captured.get('browser'), browser_adapter=captured.get('adapter'),
                  flags=flags, source=project.source_identity(),
                  scope='per-scene browser-WebGPU execution of the full catalog; browser vs same-machine native (exact preferred, reference/08 §5 envelope is the only fallback); native vs canonical Linux/Vulkan golden at golden tolerance; no browser baselines are created')
    (out/'report.json').write_text(json.dumps(result, indent=2)+'\n')
    project.report('browser-showcase', result)
    if result['status'] != 'PASS':
        raise CheckFailed(f'browser showcase scenes failed: {failed}; inspect {out}/report.json')
    return result
