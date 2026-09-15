"""Enable a prepared Gacha add-on only in this isolated background process."""
import os
import sys
from pathlib import Path
import bpy
import addon_utils
root=Path(os.environ['STYLIZED_ADDON_ROOT']).resolve(strict=True)
if not bpy.app.background:
    raise RuntimeError('bootstrap is only for isolated authoring jobs')
sys.path.insert(0,str(root))
if 'setup_wizard' not in bpy.context.preferences.addons:
    addon_utils.enable('setup_wizard',default_set=False,persistent=False)
import setup_wizard
if not Path(setup_wizard.__file__).resolve().is_relative_to(root):
    raise RuntimeError('unexpected Gacha installation loaded')
