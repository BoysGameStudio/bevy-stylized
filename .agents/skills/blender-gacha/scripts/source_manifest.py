"""Delegate to the maintained project helper; resolve only an explicit/current project."""
import os
from pathlib import Path
import runpy
hint=os.environ.get('STYLIZED_PROJECT_ROOT')
roots=[Path(hint)] if hint else [Path(__file__).resolve().parents[4],Path.cwd(),*Path.cwd().parents]
for root in roots:
    tool=root/'tools/assets/inspect_source.py'
    if (root/'crates/bevy-stylized/Cargo.toml').is_file() and tool.is_file():
        runpy.run_path(str(tool),run_name='__main__')
        break
else:
    raise SystemExit('Set STYLIZED_PROJECT_ROOT to the Bevy Stylized checkout, or use the installed asset CLI.')
