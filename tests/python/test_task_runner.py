"""Behavioral tests for identity, errors, and current-run report ownership."""
import contextlib
import io
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2] / 'tools'))
import dev
from project import Project
from task_runner import CheckFailed, ResourceBlocked, classify_case, run_tests, require_suites


def output(name='wanted', failed=False, message=''):
    return (f'test {name} ... {message}\n' +
            ('FAILED\ntest result: FAILED. 0 passed; 1 failed; 0 ignored;' if failed else
             'ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;'))


class RequiredCases(unittest.TestCase):
    def test_identity_and_summary_must_both_match(self):
        for text, code in [(output('wrong'), 0), ('test result: ok. 0 passed; 0 failed; 0 ignored;', 0),
                           (output() + output(), 0), (output(), 1), ('broken', 0)]:
            self.assertEqual(classify_case(text, code, 'wanted')[0], 'FAIL')
        self.assertEqual(classify_case(output(), 0, 'wanted')[0], 'PASS')

    def test_resource_phrase_in_log_cannot_hide_assertion(self):
        text = output(failed=True, message="required GPU unavailable\nthread 'wanted' panicked at file.rs:1:1:\nassertion failed")
        self.assertEqual(classify_case(text, 101, 'wanted')[0], 'FAIL')

    def test_resource_panic_is_blocked(self):
        for message in ['required GPU unavailable', 'accepted baseline missing', 'accepted regression baseline missing']:
            text = output(failed=True, message=f"\nthread 'wanted' (394622) panicked at file.rs:1:1:\n{message}: fixture")
            self.assertEqual(classify_case(text, 101, 'wanted')[0], 'BLOCKED')

    def test_inventory_and_mixed_failures(self):
        with tempfile.TemporaryDirectory() as tmp:
            project = make_project(tmp)
            answers = [subprocess.CompletedProcess([], 0, 'a: test\nb: test\n', ''),
                       subprocess.CompletedProcess([], 101, output('a', True, "\nthread 'a' panicked at a.rs:1:1:\nrequired GPU unavailable")),
                       subprocess.CompletedProcess([], 101, output('b', True, "\nthread 'b' panicked at b.rs:1:1:\nassertion failed"))]
            with patch('task_runner.subprocess.run', side_effect=answers) as call, contextlib.redirect_stdout(io.StringIO()):
                state = run_tests(project, 'suite', ['cargo'], ['a', 'b'], {'GOLDEN_REBASELINE':'1'})
            self.assertEqual((state['status'], state['failed'], state['blocked']), ('FAIL', 1, 1))
            self.assertTrue(all('GOLDEN_REBASELINE' not in c.kwargs['env'] for c in call.call_args_list))
            with self.assertRaises(CheckFailed):require_suites([state])

    def test_wrong_or_duplicate_inventory_never_executes(self):
        for inventory in ['wrong: test\n', 'wanted: test\nwanted: test\n', '']:
            with tempfile.TemporaryDirectory() as tmp:
                project = make_project(tmp)
                with patch('task_runner.subprocess.run', return_value=subprocess.CompletedProcess([], 0, inventory, '')) as call, contextlib.redirect_stdout(io.StringIO()):
                    state = run_tests(project, 'suite', ['cargo'], ['wanted'], {})
                self.assertEqual(state['status'], 'FAIL')
                self.assertEqual(state['not_executed'], 1)
                self.assertEqual(call.call_count, 1)

    def test_non_identity_expectation_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            for invalid in [1, [], ['a', 'a']]:
                with self.assertRaises(CheckFailed):run_tests(make_project(tmp), 'suite', ['cargo'], invalid, {})


def make_project(directory):
    root = pathlib.Path(directory)
    (root / 'Cargo.toml').write_text('[workspace]\nmembers=[]\n')
    return Project(root)


class RunLifecycle(unittest.TestCase):
    def test_success_followed_by_failure_block_or_interrupt_replaces_summary(self):
        for error, expected, status in [(CheckFailed('broken'), 1, 'FAIL'),
                                        (ResourceBlocked('GPU'), 2, 'BLOCKED'),
                                        (KeyboardInterrupt(), 1, 'FAIL'),
                                        (ValueError('schema'), 1, 'FAIL'),
                                        (subprocess.CalledProcessError(101, ['cargo']), 1, 'FAIL')]:
            with tempfile.TemporaryDirectory() as tmp:
                project = make_project(tmp)
                project.report('gpu', {'status':'PASS', 'marker':'old'})
                with patch.object(dev, 'Project', return_value=project), patch.object(sys, 'argv', ['dev.py','--run-id','current','check','--lane','gpu']), patch.object(dev, 'execute', side_effect=error), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(dev.main(), expected)
                report = project.read_report('gpu', 'current', project.source_identity())
                self.assertEqual(report['status'], status)
                self.assertNotIn('marker', report)
                self.assertEqual(json.loads((project.output / 'gpu.json').read_text())['run_id'], 'current')

    def test_unfinished_wrong_source_and_wrong_run_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            project = make_project(tmp)
            project.begin_run('gpu', {}, 'one')
            with self.assertRaises(ValueError):project.read_report('gpu', 'one')
            project.finish_run('PASS', 'done')
            with self.assertRaises(ValueError):project.read_report('gpu', 'one', 'wrong-source')
            data = json.loads((project.run_output / 'gpu.json').read_text())
            data['run_id'] = 'old'
            (project.run_output / 'gpu.json').write_text(json.dumps(data))
            with self.assertRaises(ValueError):project.read_report('gpu', 'one')

    def test_old_subreport_is_not_aggregated(self):
        with tempfile.TemporaryDirectory() as tmp:
            project = make_project(tmp)
            project.report('old-suite', {'status':'PASS'})
            project.begin_run('gpu', {}, 'one')
            project.finish_run('PASS', 'done')
            self.assertEqual(project.read_report('gpu','one')['reports'], [])

    def test_start_write_failure_does_not_execute(self):
        with tempfile.TemporaryDirectory() as tmp:
            project = make_project(tmp)
            with patch.object(dev, 'Project', return_value=project), patch.object(sys,'argv',['dev.py','check','--lane','gpu']), patch.object(Project, 'atomic_json', side_effect=OSError('disk full')), patch.object(dev,'execute') as execute:
                with self.assertRaises(OSError):dev.main()
                execute.assert_not_called()

    def test_run_id_cannot_reuse_or_escape(self):
        with tempfile.TemporaryDirectory() as tmp:
            project = make_project(tmp)
            project.begin_run('gpu', {}, 'same')
            with self.assertRaises(FileExistsError):make_project(tmp).begin_run('gpu',{},'same')
            with self.assertRaises(ValueError):make_project(tmp).begin_run('gpu',{},'../outside')

    def test_usage_error_does_not_claim_resource_block(self):
        with patch.object(sys,'argv',['dev.py','check','--lane','unknown']), contextlib.redirect_stderr(io.StringIO()) as stream:
            with self.assertRaises(SystemExit) as raised:dev.main()
        self.assertEqual(raised.exception.code, 2)
        self.assertIn('usage:',stream.getvalue())
        self.assertNotIn('BLOCKED',stream.getvalue())

class ProcessAndInputs(unittest.TestCase):
    def test_missing_executable_is_resource_block(self):
        from task_runner import invoke
        with patch('task_runner.subprocess.run', side_effect=FileNotFoundError('missing')):
            with self.assertRaises(ResourceBlocked):invoke(['missing-tool'])

    def test_multiple_panics_do_not_hide_assertion(self):
        text = output(failed=True, message="\nthread 'worker' panicked at a.rs:1:1:\nrequired GPU unavailable\nthread 'wanted' panicked at b.rs:1:1:\nassertion failed")
        self.assertEqual(classify_case(text, 101, 'wanted')[0], 'FAIL')

    def test_actual_cli_missing_asset_and_invalid_fetch(self):
        root = pathlib.Path(__file__).resolve().parents[2]
        for lane, tail, expected in [('asset-integration',['--asset','missing.glb'],2),
                                      ('docs',['--fetch'],1)]:
            with tempfile.TemporaryDirectory() as tmp:
                project = make_project(tmp)
                command=[sys.executable,str(root/'tools/dev.py'),'--root',str(project.root),
                         '--run-id','child','check','--lane',lane]+tail
                result=subprocess.run(command,capture_output=True,text=True)
                self.assertEqual(result.returncode,expected,result.stderr)
                report=project.read_report(lane,'child',project.source_identity())
                self.assertEqual(report['status'],'BLOCKED' if expected==2 else 'FAIL')

    def test_isolated_caller_verifies_child_run_and_status(self):
        import ast
        import uuid
        root = pathlib.Path(__file__).resolve().parents[2]
        tree=ast.parse((root/'tools/checks/isolated_job.py').read_text())
        function=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='run')
        with tempfile.TemporaryDirectory() as tmp:
            project=make_project(tmp)
            namespace=dict(root=project.root,project=project,uuid=uuid,subprocess=subprocess,
                           steps=[],ResourceBlocked=ResourceBlocked)
            exec(compile(ast.Module(body=[function],type_ignores=[]),'isolated-caller','exec'),namespace)
            def child(command,**kwargs):
                run_id=command[command.index('--run-id')+1]
                producer=Project(project.root)
                producer.begin_run('gpu',{},run_id)
                producer.finish_run('BLOCKED','required-resource-missing')
                return subprocess.CompletedProcess(command,2)
            with patch.object(subprocess,'run',side_effect=child),contextlib.redirect_stdout(io.StringIO()):
                namespace['run']('negative',[sys.executable,'tools/dev.py','check','--lane','gpu'],2)
                with self.assertRaises(ResourceBlocked):namespace['run']('positive',[sys.executable,'tools/dev.py','check','--lane','gpu'])
            self.assertEqual(len(namespace['steps']),2)
            self.assertEqual(namespace['steps'][1]['report_status'],'BLOCKED')
            self.assertIsNotNone(namespace['steps'][0]['run_id'])

    def test_reference_resource_failure_keeps_child_and_parent_classification(self):
        with tempfile.TemporaryDirectory() as tmp:
            project=make_project(tmp)
            def child(command, **kwargs):
                kwargs['stdout'].write('BLOCKED: required GPU unavailable\n')
                return subprocess.CompletedProcess(command,2)
            with patch.object(dev,'invoke',side_effect=child),contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaises(ResourceBlocked):dev.run(project,['reference'])
            self.assertEqual(project.records['command-0']['status'],'BLOCKED')
