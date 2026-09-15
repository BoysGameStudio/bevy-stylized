"""Project path/configuration resolution shared by developer tools (stdlib only)."""
from pathlib import Path
import json
import os
import tomllib


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

    def resolve(self, value):
        path = Path(value).expanduser()
        return (path if path.is_absolute() else self.root / path).resolve()

    def config_relative(self, value):
        path = Path(value).expanduser()
        return (path if path.is_absolute() else self.config_path.parent / path).resolve()

    def report(self, name, data):
        self.output.mkdir(parents=True, exist_ok=True)
        path = self.output / (name + '.json')
        temporary = path.with_suffix('.json.tmp')
        temporary.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')
        os.replace(temporary, path)
        return path
