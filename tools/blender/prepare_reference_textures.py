"""Create canonical reference-map names from verified raw sources and shader assets."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sys
import tempfile
import bpy
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from assets.prepare import verify

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--source',required=True,type=Path);p.add_argument('--manifest',required=True,type=Path)
p.add_argument('--vendor-blend',required=True,type=Path);p.add_argument('--vendor-sha256',required=True)
p.add_argument('--out',required=True,type=Path)
a=p.parse_args(sys.argv[sys.argv.index('--')+1:])
manifest=json.loads(a.manifest.read_text());verify(a.source,manifest)
if hashlib.sha256(a.vendor_blend.read_bytes()).hexdigest()!=a.vendor_sha256:raise RuntimeError('shader digest mismatch')
if a.out.exists():raise RuntimeError('output exists; use a new candidate')
a.out.parent.mkdir(parents=True,exist_ok=True);stage=Path(tempfile.mkdtemp(prefix='.reference-',dir=a.out.parent))
with bpy.data.libraries.load(str(a.vendor_blend),link=False) as (available,requested):
 requested.node_groups=['HoYoverse - GI Body','HoYoverse - GI Face']
files={'Avatar_Girl_Sword_Odette_Tex_Hair_Shadow_Ramp.png':'shadow_ramp.png','Avatar_Girl_Sword_Odette_Tex_Hair_Lightmap.png':'hair_lightmap.png',
       'Avatar_Girl_Sword_Odette_Tex_Body_Lightmap.png':'body_lightmap.png','Avatar_Girl_Sword_Odette_Tex_Body01_Lightmap.png':'body01_lightmap.png',
       'Avatar_Tex_MetalMap.png':'metal_map.png','Avatar_Tex_Specular_Ramp.png':'specular_ramp.png','Avatar_Tex_Face02_Shadow.png':'face02_shadow.png'}
for original,output in files.items():shutil.copyfile(a.source/'Textures'/original,stage/output)
for original,output in [('Avatar_Girl_Tex_FaceLightmap.png','face_lightmap.png'),('Avatar_Tex_Face_Shadow.png','face_shadow.png')]:
 image=bpy.data.images.get(original)
 if not image:raise RuntimeError(f'packed upstream map missing: {original}')
 image.save(filepath=str(stage/output))
record={'schema':'bevy_stylized.reference-textures/v1','source_manifest':manifest,'shader_sha256':a.vendor_sha256,
        'generator_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'files':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in stage.iterdir() if p.is_file()}}
(stage/'provenance.json').write_text(json.dumps(record,indent=2)+'\n');shutil.move(str(stage),str(a.out))
print('REFERENCE_TEXTURES',a.out)
