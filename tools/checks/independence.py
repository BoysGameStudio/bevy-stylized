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
    ledger=json.loads((root/'docs/plans/independence-coverage.json').read_text())
    count=0
    for file in ledger['files']:
        for test in file['tests']:
            count+=1
            if test.get('migration_status')=='pending' or not test.get('reason') or not test.get('replacement'):
                errors.append(f'unresolved coverage: {file["path"]}:{test["name"]}')
            if test.get('migration_status') == 'preserved':
                replacements = [root/r for r in test.get('replacement') or []]
                if not any(p.is_file() and re.search(r'fn\s+'+re.escape(test['name'])+r'\b',p.read_text()) for p in replacements):
                    errors.append(f'preserved test disappeared: {test["name"]}')
            for replacement in test.get('replacement') or []:
                if not (root/replacement).is_file():errors.append(f'missing replacement: {replacement}')
    return {'check':'independence-static','status':'FAIL' if errors else 'PASS','coverage_records':count,'errors':sorted(set(errors)),
            'scope':'static boundaries only; isolated execution is a separate required validation'}
