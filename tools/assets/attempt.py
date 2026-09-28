"""Immutable build attempts and stage evidence; failed work is never overwritten."""
from datetime import datetime, timezone
from pathlib import Path
import json
import subprocess
import uuid

from project import Project
from task_runner import ResourceBlocked, invoke
from .prepare import digest


def inventory(root):
    return {
        str(path.relative_to(root)): digest(path)
        for path in sorted(root.rglob('*'))
        if path.is_file()
        and path.name not in {'attempt.json', 'build-manifest.json'}
        and not path.name.endswith('.log')
        and path.relative_to(root).parts[0] not in {'tooling', 'blender-config'}
    }


def completed_or_new(root, inputs, verify):
    """Reuse verified success, or allocate a new child of the same content key.

    The legacy root-level manifest stays readable. Incomplete roots and attempts
    are preserved, including after an uncatchable termination.
    """
    # 返回路径在所有分支一致解析（verify_run 的重用分支本就 resolve；
    # 新建分支此前返回未解析路径——同一目录在 macOS 上会呈现为 /var 与
    # /private/var 两种身份）。
    root = Path(root).resolve()
    if (root / 'build-manifest.json').is_file():
        result = verify(root)
        if json.loads((root / 'build-manifest.json').read_text())['inputs'] != inputs:
            raise ValueError('completed build inputs differ from content key')
        return result, None
    manifests = sorted((root / 'attempts').glob('*/build-manifest.json'))
    if manifests:
        path = manifests[-1].parent
        result = verify(path)
        if json.loads(manifests[-1].read_text())['inputs'] != inputs:
            raise ValueError('completed build inputs differ from content key')
        return result, None
    attempt_id = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ-') + uuid.uuid4().hex
    path = root / 'attempts' / attempt_id
    path.mkdir(parents=True, exist_ok=False)
    attempt = BuildAttempt(path, inputs)
    return path, attempt


class BuildAttempt:
    def __init__(self, path, inputs):
        self.path = path
        self.record = {'schema': 'bevy_stylized.build-attempt/v1',
                       'attempt_id': path.name, 'status': 'RUNNING',
                       'inputs': inputs, 'stages': []}
        self.save()

    def save(self):
        Project.atomic_json(self.path / 'attempt.json', self.record)

    def __enter__(self):
        return self

    def __exit__(self, kind, error, traceback):
        if error is not None:
            self.record.update(status='BLOCKED' if isinstance(error, ResourceBlocked) else 'FAIL',
                               error=str(error))
        else:
            self.record['status'] = 'PASS'
        self.save()
        return False

    def run(self, project, command, log, env=None):
        before = inventory(self.path)
        stage = {'name': log.stem, 'command': list(map(str, command)),
                 'status': 'RUNNING', 'inputs': before,
                 'log': str(log.relative_to(self.path))}
        self.record['stages'].append(stage)
        self.save()
        try:
            with log.open('w') as output:
                result = invoke(stage['command'], cwd=project.root, env=env,
                                stdout=output, stderr=subprocess.STDOUT)
            stage['exit_code'] = result.returncode
            if result.returncode:
                raise subprocess.CalledProcessError(result.returncode, command)
            stage['status'] = 'PASS'
        except BaseException as error:
            stage.update(status='BLOCKED' if isinstance(error, ResourceBlocked) else 'FAIL',
                         error=str(error))
            raise
        finally:
            after = inventory(self.path)
            stage['outputs'] = {name: value for name, value in after.items()
                                if before.get(name) != value}
            stage['removed'] = sorted(before.keys() - after.keys())
            self.save()
