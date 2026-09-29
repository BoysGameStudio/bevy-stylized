"""Android catalog lane: each courtyard capture spec runs in a fresh process
on the emulator (R4 isolation), captured through the shared capture contract
(apps/android-smoke catalog mode), judged against the showcase golden baselines
(canonical Linux/Vulkan; the android-vulkan overlay where accepted) at the
golden tolerance. The emulator forwards to the host GPU (MoltenVK on Apple
Silicon); evidence is emulator evidence, never a physical-device claim.
Canonical 庭院基线缺席时本 lane 显式 BLOCKED（不是 FAIL）。
"""
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import time

from task_runner import CheckFailed, ResourceBlocked, invoke

TOLERANCE = dict(max_channel_delta=4, mean_absolute_delta=0.5, min_psnr_db=40.0)
"""Mirrors golden_harness Tolerance::default() (labs/validation diff)."""
PACKAGE = 'org.bevystylized.smoke'
ACTIVITY = f'{PACKAGE}/.MainActivity'
FILES_DIR = f'/sdcard/Android/data/{PACKAGE}/files'
CONTROL = f'{FILES_DIR}/catalog-scene.txt'
OVERLAY_PLATFORM = 'android-vulkan'


def adb(serial, *args, timeout=120):
    result = subprocess.run(['adb', '-s', serial, *args], capture_output=True, text=True, timeout=timeout)
    return result


def wait_file(serial, path, timeout_s=120):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if adb(serial, 'shell', 'test', '-f', path, '&&', 'echo', 'ok').stdout.strip() == 'ok':
            return True
        time.sleep(2)
    return False


def _frame_stats(data):
    """luma 均值/标准差（Rec.601；与 labs/validation frame.rs 同一公式）。"""
    n = len(data) // 4
    luma = [0.299 * data[i] + 0.587 * data[i + 1] + 0.114 * data[i + 2] for i in range(0, len(data), 4)]
    mean = sum(luma) / n
    var = sum((v - mean) ** 2 for v in luma) / n
    return mean, var ** 0.5


def judge(capture, reference, sidecar=None):
    """捕获判定（与 labs/validation 同一语义）：

    - sidecar 无 regions：strict 整图 golden 容差（max/mean/PSNR）；
    - sidecar 带 regions（05 §9 additive 扩展）：整图宽松（non-black +
      luma/chroma 包络）+ 每区域 per-pixel 每通道阈值与失败像素比双层
      门限（courtyard 混合判定模式——hybrid 等捕获的 accepted 语义）。
    """
    if not sidecar or not sidecar.get('regions'):
        import math
        diffs = [abs(x - y) for x, y in zip(capture, reference)]
        mean = sum(diffs) / len(diffs)
        mse = sum(d * d for d in diffs) / len(diffs)
        psnr = 99.0 if mse == 0 else 20 * math.log10(255.0 / math.sqrt(mse))
        return {'pass': max(diffs, default=0) <= TOLERANCE['max_channel_delta']
                and mean <= TOLERANCE['mean_absolute_delta'] and psnr >= TOLERANCE['min_psnr_db'],
                'max_channel_delta': max(diffs, default=0), 'mean_channel_delta': mean, 'psnr_db': psnr,
                'mode': 'strict-whole-frame'}
    base_mean, base_std = _frame_stats(reference)
    cand_mean, cand_std = _frame_stats(capture)
    violations = []
    # 整图宽松包络（capture_spec.rs loose_envelope 同一规则）。
    if not (cand_mean > 1.0 and cand_mean >= max(1.0, base_mean * 0.6)
            and cand_mean <= base_mean * 1.4 + 10.0 and cand_std >= max(5.0, base_std * 0.6)):
        violations.append('whole-frame loose envelope exceeded')
    for region in sidecar['regions']:
        rect = region['rect']
        x0, y0, w, h = rect['x'], rect['y'], rect['width'], rect['height']
        failing, total = 0, max(1, w * h)
        max_delta = 0
        for y in range(y0, y0 + h):
            for x in range(x0, x0 + w):
                i = (y * 512 + x) * 4
                delta = max(abs(capture[i + k] - reference[i + k]) for k in range(3))
                max_delta = max(max_delta, delta)
                failing += delta > region['max_channel_delta']
        ratio = failing / total
        if ratio > region['max_fail_ratio']:
            violations.append(
                f"region {region['id']}: fail_ratio {ratio:.5f} > {region['max_fail_ratio']}")
    return {'pass': not violations, 'mode': 'loose+rois', 'violations': violations,
            'whole_frame': {'mean_luma': cand_mean, 'stddev_luma': cand_std}}


def read_sidecar(baseline_path):
    """读取基线 sidecar（<case>.golden.json；缺席 = None = strict 整图判定）。"""
    sidecar_path = baseline_path.with_name(baseline_path.stem + '.golden.json')
    if not sidecar_path.is_file():
        return None
    return json.loads(sidecar_path.read_text())


def run(project, out):
    out = project.resolve(out)
    if out.exists():
        raise CheckFailed('android-catalog output exists; use a new candidate directory')
    for exe in ['adb']:
        if not shutil.which(exe):
            raise ResourceBlocked(f'android-catalog needs {exe} in PATH')
    for var in ['ANDROID_HOME', 'ANDROID_NDK_HOME', 'JAVA_HOME']:
        if not os.environ.get(var):
            raise ResourceBlocked(f'android-catalog needs {var}')
    serial = os.environ.get('STYLIZED_ANDROID_SERIAL', 'emulator-5554')
    devices = adb(serial, 'get-state', timeout=30)
    if devices.returncode:
        raise ResourceBlocked(f'no emulator device at {serial}; boot Tenacity_API35 with -gpu host (CONTRIBUTING)')
    suites = json.loads((project.root/'tests/suites.json').read_text())
    captures = [s.removeprefix('showcase_golden_').replace('_', '-') for s in suites['gpu']['showcase_golden']]
    baselines = project.root/'fixtures/golden/showcase'
    overlay = baselines/OVERLAY_PLATFORM
    # 基线解析与 golden.rs 的 resolve_baseline_paths 一致：overlay 优先、
    # canonical 兜底。必需基线缺席是 BLOCKED 不是 FAIL（05 §9：canonical
    # 庭院基线由参考宿主产出；缺席且无 overlay 时本 lane 显式 BLOCKED）。
    def resolved_baseline(capture):
        if (overlay/f'{capture}.png').is_file():
            return overlay/f'{capture}.png'
        return baselines/f'{capture}.png'
    missing = [s for s in captures if not resolved_baseline(s).is_file()]
    rebase = os.environ.get('GOLDEN_REBASELINE') == '1'
    if missing and not rebase:
        # 必需基线缺席是 BLOCKED 不是 FAIL（05 §9：canonical 庭院基线由
        # 参考宿主产出；缺席且无 overlay 时本 lane 显式 BLOCKED）。
        # 显式 GOLDEN_REBASELINE=1 走播种流程：设备捕获直接写 overlay
        # 候选（评审 + 提交后被接受，fixtures/README.md）。
        raise ResourceBlocked(f'accepted showcase baselines missing for: {missing}')
    out.mkdir(parents=True)
    staged = project.root/'target/android-smoke/catalog-assets'
    staged.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(project.root/'fixtures/synthetic/reference-scene/reference_scene.glb', staged/'reference_scene.glb')
    env = dict(os.environ, CARGO_INCREMENTAL='0', STYLIZED_ANDROID_CATALOG_DIR=str(staged))
    command = ['cargo', 'ndk', '-t', 'arm64-v8a', '-P', '31', '-o', str(project.root/'target/android-smoke/jniLibs'),
               'build', '--release', '--locked', '-p', 'stylized-android-smoke']
    with (out/'cargo-ndk.log').open('w') as stream:
        if invoke(command, cwd=project.root, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=1800).returncode:
            raise CheckFailed(f'cargo-ndk build failed; inspect {out}/cargo-ndk.log')
    with (out/'gradle.log').open('w') as stream:
        if invoke(['./gradlew', '--no-daemon', ':app:assembleDebug'], cwd=project.root/'apps/android-smoke/android',
                  env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=1800).returncode:
            raise CheckFailed(f'gradle build failed; inspect {out}/gradle.log')
    apk = project.root/'apps/android-smoke/android/app/build/outputs/apk/debug/app-debug.apk'
    if adb(serial, 'install', '-r', str(apk), timeout=600).returncode:
        raise CheckFailed('apk install failed')
    reports = {}
    for capture in captures:
        entry = {'capture': capture}
        try:
            # 每次启动前清掉该场景的既有产物：等待绝不对陈旧文件放行，
            # 一次进程只产出本场景的当次报告（R4 进程隔离）。
            adb(serial, 'shell', 'am', 'force-stop', PACKAGE, timeout=30)
            adb(serial, 'shell', 'rm', '-f', f'{FILES_DIR}/catalog-{capture}.json',
                f'{FILES_DIR}/catalog-{capture}.png', timeout=30)
            result = adb(serial, 'shell', f'echo {capture} > {CONTROL}', timeout=30)
            if result.returncode:
                raise RuntimeError(f'control write failed: {result.stderr.strip()}')
            start = adb(serial, 'shell', 'am', 'start', '-W', '-n', ACTIVITY, timeout=60)
            if start.returncode:
                raise RuntimeError(f'am start failed: {start.stderr.strip()}')
            report_path = f'{FILES_DIR}/catalog-{capture}.json'
            if not wait_file(serial, report_path, 240):
                raise RuntimeError('report timeout (no catalog report file appeared)')
            local_dir = out/'device'
            local_dir.mkdir(exist_ok=True)
            for remote in [report_path, f'{FILES_DIR}/catalog-{capture}.png']:
                pull = adb(serial, 'pull', remote, str(local_dir), timeout=120)
                if pull.returncode:
                    raise RuntimeError(f'adb pull {remote} failed')
            report = json.loads((local_dir/f'catalog-{capture}.json').read_text())
            entry['device_report'] = report
            if report.get('status') != 'PASS':
                entry['status'] = report.get('status', 'FAIL')
                entry['reason'] = report.get('reason')
                reports[capture] = entry
                continue
            from PIL import Image
            data = Image.open(local_dir/f'catalog-{capture}.png').convert('RGBA').tobytes()
            baseline_path = resolved_baseline(capture)
            if not baseline_path.is_file():
                # 播种流程（GOLDEN_REBASELINE=1）：设备捕获直接写 overlay
                # 候选并据此判定（评审 + 提交后才被接受；canonical 绝不被
                # 本 lane 覆写）。
                if not rebase:
                    raise RuntimeError('accepted baseline missing (BLOCKED)')
                overlay.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(local_dir/f'catalog-{capture}.png', overlay/f'{capture}.png')
                # regions 是规格数据（分区 AABB × 捕获机位投影，与平台无关）：
                # 从既有 overlay sidecar 携带；缺席则 strict 整图（如实记录）。
                regions = None
                pc_sidecar = baselines/'macos-metal'/f'{capture}.golden.json'
                if pc_sidecar.is_file():
                    regions = json.loads(pc_sidecar.read_text()).get('regions')
                sidecar = {
                    'reference_toolchain': 'engine-golden-v1', 'blender': None,
                    'gacha_setup': 'synthetic-fixtures', 'gacha_setup_commit': None,
                    'export_contract': 'blender-gltf-npr-v1', 'profile_schema': 'bevy_stylized.material/v1',
                    'fixture': f'showcase/{capture}', 'renderer': 'stylized-android-smoke',
                    'engine_git_commit': _git_head(),
                    'engine_source_revision': None,
                    'gpu': report.get('device', 'unknown'),
                    'driver': report.get('driver', 'unknown'),
                    'captured_unix_seconds': int(time.time()),
                    'width': report.get('width', 512), 'height': report.get('height', 512),
                    'png_fnv1a64': _fnv1a64((overlay/f'{capture}.png').read_bytes()),
                }
                if regions:
                    sidecar['regions'] = regions
                (overlay/f'{capture}.golden.json').write_text(json.dumps(sidecar, indent=2)+'\n')
                baseline_path = overlay/f'{capture}.png'
                entry['overlay_candidate'] = str(baseline_path.relative_to(project.root))
            reference = Image.open(baseline_path).convert('RGBA').tobytes()
            verdict = judge(data, reference, read_sidecar(baseline_path))
            entry['status'] = 'PASS' if verdict['pass'] else 'FAIL'
            entry['judgment'] = verdict
            entry['baseline'] = str(baseline_path.relative_to(project.root))
            entry['capture_sha256'] = hashlib.sha256(data).hexdigest()
            entry['baseline_sha256'] = hashlib.sha256(baseline_path.read_bytes()).hexdigest()
            if not verdict['pass'] and os.environ.get('GOLDEN_REBASELINE') == '1':
                # 显式 overlay 候选捕获：canonical 基线绝不被本 lane 覆写；
                # 候选经评审 + 提交才被接受（fixtures/README.md）。
                overlay.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(local_dir/f'catalog-{capture}.png', overlay/f'{capture}.png')
                # regions 是规格数据（分区 AABB × 捕获机位投影，与平台无关）：
                # 从既有 overlay sidecar 携带；缺席则 strict 整图（如实记录）。
                regions = None
                pc_sidecar = baselines/'macos-metal'/f'{capture}.golden.json'
                if pc_sidecar.is_file():
                    regions = json.loads(pc_sidecar.read_text()).get('regions')
                sidecar = {
                    'reference_toolchain': 'engine-golden-v1', 'blender': None,
                    'gacha_setup': 'synthetic-fixtures', 'gacha_setup_commit': None,
                    'export_contract': 'blender-gltf-npr-v1', 'profile_schema': 'bevy_stylized.material/v1',
                    'fixture': f'showcase/{capture}', 'renderer': 'stylized-android-smoke',
                    'engine_git_commit': _git_head(),
                    'engine_source_revision': None,
                    'gpu': report.get('device', 'unknown'),
                    'driver': report.get('driver', 'unknown'),
                    'captured_unix_seconds': int(time.time()),
                    'width': report.get('width', 512), 'height': report.get('height', 512),
                    'png_fnv1a64': _fnv1a64((overlay/f'{capture}.png').read_bytes()),
                }
                if regions:
                    sidecar['regions'] = regions
                (overlay/f'{capture}.golden.json').write_text(json.dumps(sidecar, indent=2)+'\n')
                entry['overlay_candidate'] = str(overlay/f'{capture}.png')
        except (RuntimeError, subprocess.TimeoutExpired, json.JSONDecodeError) as error:
            entry['status'] = 'FAIL'
            entry['reason'] = str(error)
        finally:
            adb(serial, 'shell', 'am', 'force-stop', PACKAGE, timeout=30)
        reports[capture] = entry
    failed = [s for s, r in reports.items() if r['status'] != 'PASS']
    result = dict(schema='bevy_stylized.android-catalog/v1',
                  status='PASS' if not failed else 'FAIL',
                  captures=reports, total=len(captures), failed=failed,
                  serial=serial, package=PACKAGE,
                  scope='emulator evidence (Tenacity_API35, -gpu host, MoltenVK -> host GPU); not a physical-device claim; per-capture fresh process; golden tolerance vs canonical/overlay showcase baselines',
                  source=project.source_identity())
    (out/'report.json').write_text(json.dumps(result, indent=2)+'\n')
    project.report('android-catalog', result)
    if result['status'] != 'PASS':
        raise CheckFailed(f'android catalog captures failed: {failed}; inspect {out}/report.json')
    return result


def _fnv1a64(data):
    """FNV-1a-64, byte-identical to golden_harness::fnv1a64_hex."""
    hash_value = 0xcbf29ce484222325
    for byte in data:
        hash_value = ((hash_value ^ byte) * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return f'{hash_value:016x}'

def _git_head():
    """本机构建源码身份（设备端无 git；runner 以本地 HEAD 为准）。"""
    result = subprocess.run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True)
    return result.stdout.strip() if result.returncode == 0 else 'unknown'
