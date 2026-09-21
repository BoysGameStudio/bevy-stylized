"""Measurement acceptance must reject stale, incomplete and incomparable runs."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2]/'tools'))
from checks.realtime import validate_files, validate_measurement
from task_runner import CheckFailed


def stats(value, count):
    return dict(min=value, max=value, median=value, mean=value, p95=value, count=count)


def report():
    return dict(schema='bevy_stylized.realtime-performance/v1',status='PASS',optimized=True,
                resolution_per_view=[512,512],view_count=2,msaa=4,capture_readbacks=False,
                diagnostic_readbacks=True,pipelined_rendering=False,device='Vulkan GPU A driver A',
                files={'character.glb':'abc'}, reports=[dict(mode=mode,warmup_render_frames=60,
                    requested_samples=600,app_update_wall_ms=stats(2.,600),
                    diagnostics={'render/inspector_frame/elapsed_gpu':stats(1.,597)},
                    raw_diagnostics={'render/inspector_frame/elapsed_gpu':[1.]*597},
                    memory=dict(allocated_bytes=1024,reserved_bytes=2048)) for mode in ['beauty','lighting']])


class RealtimeReports(unittest.TestCase):
    def test_successful_processes_cannot_hide_failed_replay(self):
        from project import Project
        from checks.realtime import run
        import subprocess
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'Cargo.toml').write_text('[workspace]\nmembers=[]\n')
            asset=root/'character.glb';asset.write_bytes(b'asset')
            with patch('checks.realtime.subprocess.run',return_value=subprocess.CompletedProcess([],0)), patch('checks.realtime.subprocess.check_output',return_value=json.dumps({'target_directory':str(root/'target')})), patch('assets.realtime_report.generate',return_value={'replay_status':'FAIL','nonempty_geometry':True}):
                with self.assertRaisesRegex(CheckFailed,'scan replay'):
                    run(Project(root),asset,root/'new-report')

    def test_valid_repetitions_preserve_identity(self):
        r=report()
        self.assertEqual(validate_measurement(r,512,r['files']),r['device'])
        self.assertEqual(validate_measurement(r,512,r['files'],r['device']),r['device'])
        r['reports'][0]['memory']=None
        validate_measurement(r,512,r['files'])  # Optional allocator query is not total VRAM.

    def test_configuration_and_device_changes_fail(self):
        original=report()
        for key,value in [('optimized',False),('status','FAIL'),('device',''),('files',{}),
                          ('view_count',1),('resolution_per_view',[1024,1024]),('msaa',1),
                          ('capture_readbacks',True),('diagnostic_readbacks',False),('pipelined_rendering',True)]:
            with self.subTest(key=key):
                r=copy.deepcopy(original);r[key]=value
                with self.assertRaises(CheckFailed):validate_measurement(r,512,original['files'])
        with self.assertRaises(CheckFailed):validate_measurement(original,512,original['files'],'other GPU')

    def test_missing_duplicate_or_short_cells_fail(self):
        for modes in [[],['beauty'],['beauty','beauty'],['beauty','lighting','lighting']]:
            r=report();r['reports']=[dict(copy.deepcopy(r['reports'][0]),mode=m) for m in modes]
            with self.assertRaises(CheckFailed):validate_measurement(r,512,r['files'])
        for count in [0,539,601,True]:
            r=report();r['reports'][0]['diagnostics']['render/inspector_frame/elapsed_gpu']['count']=count
            with self.assertRaises(CheckFailed):validate_measurement(r,512,r['files'])

    def test_nonfinite_or_fabricated_timing_fails(self):
        for value in [float('nan'),float('inf'),0,-1]:
            r=report();r['reports'][0]['raw_diagnostics']['render/inspector_frame/elapsed_gpu'][0]=value
            with self.assertRaises(CheckFailed):validate_measurement(r,512,r['files'])
        r=report();r['reports'][0]['diagnostics']['render/inspector_frame/elapsed_gpu']=stats(2.,597)
        with self.assertRaises(CheckFailed):validate_measurement(r,512,r['files'])
        r=report();r['reports'][0]['app_update_wall_ms']['count']=599
        with self.assertRaises(CheckFailed):validate_measurement(r,512,r['files'])
        r=report();r['reports'][0]['memory']['reserved_bytes']=1
        with self.assertRaises(CheckFailed):validate_measurement(r,512,r['files'])

    def test_bundle_inventory_cannot_escape_or_accept_changed_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);asset=root/'character.glb';asset.write_bytes(b'valid')
            files={asset.name:hashlib.sha256(asset.read_bytes()).hexdigest()}
            validate_files(asset,files)
            for invalid in [{},{**files,'../outside':'x'},{asset.name:'wrong'}]:
                with self.assertRaises(CheckFailed):validate_files(asset,invalid)
            asset.write_bytes(b'changed')
            with self.assertRaises(CheckFailed):validate_files(asset,files)


if __name__=='__main__':unittest.main()
