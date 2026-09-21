"""Reject incomplete GPU collectors even when their subprocess exits successfully."""
import copy
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2]/'tools'))
from checks.browser import validate_capture
from checks.workloads import validate
from task_runner import CheckFailed


class MeasurementLanes(unittest.TestCase):
    def test_workloads_reject_missing_duplicate_and_inactive_features(self):
        # Construct the independent expected matrix; it must reject loss of any
        # cell even when all remaining rows and process statuses say PASS.
        cases = ['hero-1', 'hero-3', 'character-32', 'portable-100', 'portable-500',
                 'portable-1000', 'hero-skin-morph', 'hero-outline', 'hero-post']
        stats = dict(min=1., max=1., median=1., mean=1., p95=1., count=600)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out/'capture.png').write_bytes(b'capture presence')
            record = dict(schema='bevy_stylized.workloads/v1', status='PASS', optimized=True,
                          device='GPU / driver', repeats=3, expected_cases=cases, cells=[])
            for name in cases:
                for repeat in range(3):
                    record['cells'].append(dict(case=name, repeat=repeat, status='PASS', resolution=[512,512],
                        views=1, msaa=4, requested_tier='Portable', resolved='Portable', warmup=60, requested_samples=600,
                        gpu_render_graph_ms=stats, cpu_update_ms=stats, raw_gpu_ms=[1.]*600, raw_cpu_ms=[1.]*600,
                        active_features=dict(post=name=='hero-post', skin_morph=name=='hero-skin-morph',
                                             outline_replicas=int(name=='hero-outline')), capture='capture.png'))
            validate(record, out)
            for mutate in [lambda r:r['cells'].pop(), lambda r:r['cells'].append(r['cells'][0]),
                           lambda r:r['cells'][-1]['active_features'].update(post=False),
                           lambda r:r['cells'][0].update(raw_gpu_ms=[1.]*539),
                           lambda r:r['cells'][0].update(capture='../outside.png')]:
                candidate=copy.deepcopy(record);mutate(candidate)
                with self.assertRaises(CheckFailed):validate(candidate,out)

    def test_capture_requires_backend_pixels_and_adoption(self):
        value=dict(schema='bevy_stylized.browser-smoke/v1',status='PASS',backend='BrowserWebGpu',
                   width=512,height=512,adopted_primitives=10,outline_primitives=8,
                   stable_render_frames=32,classes=['Cel','Character','Hero'],rgba=list(range(256))*4096)
        self.assertEqual(len(validate_capture(value,'BrowserWebGpu',10)),512*512*4)
        for fields in [dict(backend='Gl'),dict(adopted_primitives=9),dict(stable_render_frames=31),
                       dict(rgba=[0]*1048576),dict(rgba=[300]*1048576),dict(outline_primitives=0)]:
            with self.assertRaises(CheckFailed):validate_capture(dict(value,**fields),'BrowserWebGpu',10)


if __name__ == '__main__':unittest.main()
