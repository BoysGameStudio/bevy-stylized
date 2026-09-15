"""Resolve explicitly named material textures from verified original JSON records."""
import json
from pathlib import Path

def ramp_bindings(source,manifest,recipe):
    source=Path(source).resolve();files={f['path']:f for f in manifest['files']};result={}
    expected={name for name,role in recipe['material_roles'].items() if role in {'body','hair'}}
    if set(recipe['material_sources'])!=expected:raise ValueError('ramp source material coverage mismatch')
    for material,relative in sorted(recipe['material_sources'].items()):
        path=(source/relative).resolve()
        if relative not in files or not path.is_relative_to(source):raise ValueError('material source outside verified inventory')
        data=json.loads(path.read_text());texture=data['m_SavedProperties']['m_TexEnvs']['_PackedShadowRampTex']['m_Texture']['Name']
        matches=[name for name in files if Path(name).suffix.lower()=='.png' and Path(name).stem==texture]
        if len(matches)!=1:raise ValueError(f'ambiguous/missing named ramp: {texture}')
        result[material]={'material_json':relative,'material_json_sha256':files[relative]['sha256'],'texture':matches[0],'texture_sha256':files[matches[0]]['sha256'],'name':texture}
    return result
