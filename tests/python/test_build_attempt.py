"""Recovery preserves failed work and validates successful cache entries."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools'))
from assets.attempt import completed_or_new
from assets.build import finish, verify_run
from project import Project


class Attempts(unittest.TestCase):
    def test_failed_attempt_preserved_and_retry_reuses_success(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / 'content-key'
            path, attempt = completed_or_new(root, {'source': 'hash'}, verify_run)
            with self.assertRaises(RuntimeError):
                with attempt:
                    (path / 'author.blend').write_bytes(b'failed evidence')
                    raise RuntimeError('step failed')
            fresh, retry = completed_or_new(root, {'source': 'hash'}, verify_run)
            self.assertNotEqual(fresh, path)
            with retry:
                (fresh / 'output.glb').write_bytes(b'good')
                finish(fresh, {'source': 'hash'})
            cached, runner = completed_or_new(root, {'source': 'hash'}, verify_run)
            self.assertIsNone(runner)
            self.assertEqual(cached, fresh)
            self.assertEqual((path / 'author.blend').read_bytes(), b'failed evidence')
            self.assertEqual(json.loads((path / 'attempt.json').read_text())['status'], 'FAIL')
            (fresh / 'output.glb').write_bytes(b'changed')
            with self.assertRaises(ValueError):completed_or_new(root, {'source': 'hash'}, verify_run)

    def test_incomplete_legacy_root_and_killed_attempt_do_not_block_retry(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'character.blend').write_bytes(b'author')
            first, _ = completed_or_new(root, {}, verify_run)
            second, _ = completed_or_new(root, {}, verify_run)
            self.assertNotEqual(first, second)
            self.assertEqual(json.loads((first / 'attempt.json').read_text())['status'], 'RUNNING')
            self.assertEqual((root / 'character.blend').read_bytes(), b'author')

    def test_stage_failure_records_partial_outputs(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'Cargo.toml').write_text('[workspace]\nmembers=[]\n')
            project = Project(root)
            path, attempt = completed_or_new(root / 'cache', {}, verify_run)
            def failed(command, **kwargs):
                (path / 'partial.glb').write_bytes(b'partial')
                return subprocess.CompletedProcess(command, 9)
            with patch('assets.attempt.invoke', side_effect=failed):
                with self.assertRaises(subprocess.CalledProcessError):
                    with attempt:attempt.run(project, ['tool'], path / 'export.log')
            stage = json.loads((path / 'attempt.json').read_text())['stages'][0]
            self.assertEqual(stage['exit_code'], 9)
            self.assertEqual(stage['status'], 'FAIL')
            self.assertIn('partial.glb', stage['outputs'])
            self.assertFalse((path / 'build-manifest.json').exists())

    def test_legacy_success_still_validates_and_reuses(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'output.glb').write_bytes(b'old success')
            finish(root, {})
            self.assertEqual(completed_or_new(root, {}, verify_run), (root, None))

    def test_uncommitted_manifest_never_blocks_new_attempt(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            path, _ = completed_or_new(root, {}, verify_run)
            (path / 'build-manifest.json.interrupted.tmp').write_bytes(b'{')
            fresh, attempt = completed_or_new(root, {}, verify_run)
            self.assertNotEqual(path, fresh)
            self.assertIsNotNone(attempt)
            self.assertEqual((path / 'build-manifest.json.interrupted.tmp').read_bytes(), b'{')

    def test_mismatched_completed_inputs_are_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'artifact.glb').write_bytes(b'good')
            finish(root, {'source': 'original'})
            with self.assertRaisesRegex(ValueError, 'inputs differ'):
                completed_or_new(root, {'source': 'changed'}, verify_run)
