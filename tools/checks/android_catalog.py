"""Android catalog lane: each registered catalog scene runs in a fresh process
on the emulator (R4 isolation), captured through the shared capture contract
(apps/android-smoke catalog mode), judged against the showcase golden baselines
(canonical Linux/Vulkan; the android-vulkan overlay where accepted) at the
golden tolerance. The emulator forwards to the host GPU (MoltenVK on Apple
Silicon); evidence is emulator evidence, never a physical-device claim.
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


def judge(capture, reference):
    """Golden tolerance per scene (max/mean/PSNR), mirroring Tolerance::default()."""
    import math
    diffs = [abs(x - y) for x, y in zip(capture, reference)]
    mean = sum(diffs) / len(diffs)
    mse = sum(d * d for d in diffs) / len(diffs)
    psnr = 99.0 if mse == 0 else 20 * math.log10(255.0 / math.sqrt(mse))
    return {'pass': max(diffs, default=0) <= TOLERANCE['max_channel_delta']
            and mean <= TOLERANCE['mean_absolute_delta'] and psnr >= TOLERANCE['min_psnr_db'],
            'max_channel_delta': max(diffs, default=0), 'mean_channel_delta': mean, 'psnr_db': psnr}


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
    scenes = [s.removeprefix('showcase_golden_').replace('_', '-') for s in suites['gpu']['showcase_golden']]
    baselines = project.root/'fixtures/golden/showcase'
    missing = [s for s in scenes if not (baselines/f'{s}.png').is_file()]
    if missing:
        raise CheckFailed(f'canonical showcase baselines missing for: {missing}')
    overlay = baselines/OVERLAY_PLATFORM
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
    for scene in scenes:
        entry = {'scene': scene}
        try:
            # 每次启动前清掉该场景的既有产物：等待绝不对陈旧文件放行，
            # 一次进程只产出本场景的当次报告（R4 进程隔离）。
            adb(serial, 'shell', 'am', 'force-stop', PACKAGE, timeout=30)
            adb(serial, 'shell', 'rm', '-f', f'{FILES_DIR}/catalog-{scene}.json',
                f'{FILES_DIR}/catalog-{scene}.png', timeout=30)
            result = adb(serial, 'shell', f'echo {scene} > {CONTROL}', timeout=30)
            if result.returncode:
                raise RuntimeError(f'control write failed: {result.stderr.strip()}')
            start = adb(serial, 'shell', 'am', 'start', '-W', '-n', ACTIVITY, timeout=60)
            if start.returncode:
                raise RuntimeError(f'am start failed: {start.stderr.strip()}')
            report_path = f'{FILES_DIR}/catalog-{scene}.json'
            if not wait_file(serial, report_path, 240):
                raise RuntimeError('report timeout (no catalog report file appeared)')
            local_dir = out/'device'
            local_dir.mkdir(exist_ok=True)
            for remote in [report_path, f'{FILES_DIR}/catalog-{scene}.png']:
                pull = adb(serial, 'pull', remote, str(local_dir), timeout=120)
                if pull.returncode:
                    raise RuntimeError(f'adb pull {remote} failed')
            report = json.loads((local_dir/f'catalog-{scene}.json').read_text())
            entry['device_report'] = report
            if report.get('status') != 'PASS':
                entry['status'] = report.get('status', 'FAIL')
                entry['reason'] = report.get('reason')
                reports[scene] = entry
                continue
            from PIL import Image
            data = Image.open(local_dir/f'catalog-{scene}.png').convert('RGBA').tobytes()
            baseline_path = overlay/f'{scene}.png' if (overlay/f'{scene}.png').is_file() else baselines/f'{scene}.png'
            reference = Image.open(baseline_path).convert('RGBA').tobytes()
            verdict = judge(data, reference)
            entry['status'] = 'PASS' if verdict['pass'] else 'FAIL'
            entry['judgment'] = verdict
            entry['baseline'] = str(baseline_path.relative_to(project.root))
            entry['capture_sha256'] = hashlib.sha256(data).hexdigest()
            entry['baseline_sha256'] = hashlib.sha256(baseline_path.read_bytes()).hexdigest()
            if not verdict['pass'] and os.environ.get('GOLDEN_REBASELINE') == '1':
                # 显式 overlay 候选捕获：canonical 基线绝不被本 lane 覆写；
                # 候选经评审 + 提交才被接受（fixtures/README.md）。
                overlay.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(local_dir/f'catalog-{scene}.png', overlay/f'{scene}.png')
                sidecar = {
                    'reference_toolchain': 'engine-golden-v1', 'blender': None,
                    'gacha_setup': 'synthetic-fixtures', 'gacha_setup_commit': None,
                    'export_contract': 'blender-gltf-npr-v1', 'profile_schema': 'bevy_stylized.material/v1',
                    'fixture': f'showcase/{scene}', 'renderer': 'stylized-android-smoke',
                    'engine_git_commit': _git_head(),
                    'engine_source_revision': None,
                    'gpu': report.get('device', 'unknown'),
                    'driver': report.get('driver', 'unknown'),
                    'captured_unix_seconds': int(time.time()),
                    'width': report.get('width', 512), 'height': report.get('height', 512),
                    'png_fnv1a64': _fnv1a64((overlay/f'{scene}.png').read_bytes()),
                }
                (overlay/f'{scene}.golden.json').write_text(json.dumps(sidecar, indent=2)+'\n')
                entry['overlay_candidate'] = str(overlay/f'{scene}.png')
        except (RuntimeError, subprocess.TimeoutExpired, json.JSONDecodeError) as error:
            entry['status'] = 'FAIL'
            entry['reason'] = str(error)
        finally:
            adb(serial, 'shell', 'am', 'force-stop', PACKAGE, timeout=30)
        reports[scene] = entry
    failed = [s for s, r in reports.items() if r['status'] != 'PASS']
    result = dict(schema='bevy_stylized.android-catalog/v1',
                  status='PASS' if not failed else 'FAIL',
                  scenes=reports, total=len(scenes), failed=failed,
                  serial=serial, package=PACKAGE,
                  scope='emulator evidence (Tenacity_API35, -gpu host, MoltenVK -> host GPU); not a physical-device claim; per-scene fresh process; golden tolerance vs canonical/overlay showcase baselines',
                  source=project.source_identity())
    (out/'report.json').write_text(json.dumps(result, indent=2)+'\n')
    project.report('android-catalog', result)
    if result['status'] != 'PASS':
        raise CheckFailed(f'android catalog scenes failed: {failed}; inspect {out}/report.json')
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
