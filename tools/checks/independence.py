"""Check active source boundaries and recorded coverage replacements."""
from pathlib import Path
import json
import re


def check(root):
    root=Path(root).resolve();errors=[]
    for old in ['crates/legacy','crates/dev','fixtures/golden-local']:
        if (root/old).exists():errors.append(f'retired active directory remains: {old}')
    forbidden=['TENACITY_ROOT','/Projects/Tenacity','crates/client-bevy/assets']
    for folder in ['crates','apps','labs','tests','tools']:
        for path in (root/folder).rglob('*'):
            if path.is_symlink():
                if not path.resolve().is_relative_to(root):errors.append(f'external symlink: {path.relative_to(root)}')
                continue
            if not path.is_file() or path.suffix not in {'.rs','.toml','.py','.sh'} or path.name==Path(__file__).name:continue
            source=path.read_text()
            for term in forbidden:
                if term in source:errors.append(f'{path.relative_to(root)} contains an external-project dependency marker')
            if path.suffix=='.rs':
                for rel in re.findall(r'include_(?:str|bytes)!\("([^"]+)"\)',source):
                    target=(path.parent/rel).resolve()
                    if not target.is_relative_to(root) or not target.is_file():errors.append(f'bad include: {path.relative_to(root)} -> {rel}')
    contract=json.loads((root/'tests/current-contracts.json').read_text())
    for test in contract['required_tests']:
        if not any((root/p).is_file() and re.search(r'fn\s+'+re.escape(test['name'])+r'\b',(root/p).read_text()) for p in test['sources']):
            errors.append(f'required current test disappeared: {test["name"]}')
    for path in contract['required_sources']:
        if not (root/path).is_file(): errors.append(f'missing current test owner: {path}')
    return {'check':'independence-static','status':'FAIL' if errors else 'PASS',
            'required_tests':len(contract['required_tests']),'errors':sorted(set(errors)),
            'scope':'static current-source boundaries; test execution is verified separately'}
