"""Project path/configuration resolution shared by developer tools (stdlib only)."""
from pathlib import Path
import json
import os
import tomllib
import hashlib
import re
import uuid
from datetime import datetime, timezone


class Project:
    def __init__(self, root=None, config=None, cache=None):
        self.root = Path(root or Path(__file__).resolve().parents[1]).resolve(strict=True)
        manifest = self.root / 'Cargo.toml'
        with manifest.open('rb') as stream:
            if 'workspace' not in tomllib.load(stream):
                raise ValueError(f'not a workspace root: {self.root}')
        self.config_path = Path(config).resolve(strict=True) if config else self.root / 'stylized.toml'
        values = {}
        if self.config_path.exists():
            with self.config_path.open('rb') as stream:
                values = tomllib.load(stream).get('paths', {})
        self.cache = self.resolve(cache) if cache else self.config_relative(values.get('cache', 'assets/local'))
        self.output = self.config_relative(values.get('output', 'target/stylized'))
        self.run_id = None
        self.run_output = self.output
        self.records = {}

    def resolve(self, value):
        path = Path(value).expanduser()
        return (path if path.is_absolute() else self.root / path).resolve()

    def config_relative(self, value):
        path = Path(value).expanduser()
        return (path if path.is_absolute() else self.config_path.parent / path).resolve()

    @staticmethod
    def atomic_json(path, data):
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_name(path.name + '.' + uuid.uuid4().hex + '.tmp')
        try:
            temporary.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')
            os.replace(temporary, path)
        finally:
            temporary.unlink(missing_ok=True)

    def source_identity(self):
        # Same bounded set in Git and Git-free copies; excludes generated output.
        digest = hashlib.sha256()
        roots = ['crates', 'apps', 'labs', 'tests', 'tools', 'schemas', 'fixtures',
                 'assets/sources', 'assets/recipes', '.github', '.cargo']
        files = [p for p in self.root.iterdir() if p.is_file()
                 and p.suffix in {'.toml', '.lock'}]
        for name in roots:
            folder = self.root / name
            if not folder.exists():
                continue
            for directory, dirs, names in os.walk(folder):
                dirs[:] = sorted(d for d in dirs if d not in {'__pycache__', 'target', '.git'}
                                 and not (Path(directory) / d).is_symlink())
                files.extend(Path(directory) / n for n in names if not n.endswith('.pyc'))
        for path in sorted(files):
            if path.is_symlink():
                raise ValueError(f'source symlink is not an identity input: {path}')
            digest.update(str(path.relative_to(self.root)).encode() + b'\0')
            digest.update(hashlib.sha256(path.read_bytes()).digest())
        return digest.hexdigest()

    def begin_run(self, lane, inputs, run_id=None):
        run_id = run_id or uuid.uuid4().hex
        if not re.fullmatch(r'[a-zA-Z0-9_-]{1,80}', run_id):
            raise ValueError('invalid run ID')
        self.run_id = run_id
        self.lane = lane
        self.run_output = self.output / 'runs' / run_id
        self.run_output.mkdir(parents=True, exist_ok=False)
        self.records = {}
        self.identity = {'schema': 'bevy_stylized.task/v1', 'run_id': run_id,
                         'lane': lane, 'started_at': datetime.now(timezone.utc).isoformat()}
        # Publish before hashing/validating inputs; errors cannot expose old PASS.
        self.report(lane, {'status': 'RUNNING', 'reason_code': 'started'})
        self.identity['source_sha256'] = self.source_identity()
        self.identity['inputs'] = {
            key: {'path': str(self.resolve(value)),
                  'sha256': hashlib.sha256(self.resolve(value).read_bytes()).hexdigest()
                  if self.resolve(value).is_file() else None}
            for key, value in inputs.items() if value
        }
        self.report(lane, {'status': 'RUNNING', 'reason_code': 'started'})

    def report(self, name, data):
        record = dict(data)
        if self.run_id:
            record.update(self.identity)
        self.records[name] = record
        path = self.run_output / (name + '.json')
        self.atomic_json(path, record)
        if self.run_id:
            self.atomic_json(self.output / (name + '.json'), record)
        return path

    def finish_run(self, status, reason, **details):
        previous = self.records.get(self.lane, {})
        report = dict(previous, status=status, reason_code=reason, **details)
        report['finished_at'] = datetime.now(timezone.utc).isoformat()
        report['reports'] = [name + '.json' for name in self.records if name != self.lane]
        self.report(self.lane, report)

    def read_report(self, name, run_id, source_sha256=None):
        if not re.fullmatch(r'[a-zA-Z0-9_-]{1,80}', run_id):
            raise ValueError('invalid run ID')
        data = json.loads((self.output / 'runs' / run_id / (name + '.json')).read_text())
        if data.get('run_id') != run_id or data.get('status') not in {'PASS', 'FAIL', 'BLOCKED'}:
            raise ValueError('missing current terminal report')
        if data['status'] == 'PASS' and not data.get('source_sha256'):
            raise ValueError('successful report lacks source identity')
        if source_sha256 is not None and data.get('source_sha256') != source_sha256:
            raise ValueError('report source identity differs')
        return data
