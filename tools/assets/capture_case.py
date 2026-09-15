"""Versioned, portable static reference inputs. No Blender/GPU dependency."""
import hashlib
import json
import math
from pathlib import Path

SCHEMA = 'bevy_stylized.character-capture/v1'
IDS = [f'{view}-{light}' for light in ['front','left','right'] for view in ['full_body','face_closeup']]

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def vector(value, size=3):
    if not isinstance(value,list) or len(value)!=size or any(type(v) not in (int,float) or not math.isfinite(v) for v in value):
        raise ValueError('finite vector required')
    return value

def local(root, name):
    if not isinstance(name,str) or Path(name).is_absolute():raise ValueError('relative input path required')
    path=(root/name).resolve()
    if not path.is_relative_to(root.resolve()):raise ValueError('input escapes capture package')
    return path

def validate(path):
    path=Path(path).resolve();data=json.loads(path.read_text());root=path.parent
    if data.get('schema') not in {SCHEMA,'bevy_stylized.character-capture/v2','bevy_stylized.character-capture/v3'}:raise ValueError('unknown capture schema')
    if data['schema'].endswith('/v3') and data.get('supersample') not in [2,4,8]:raise ValueError('matched sampling grid required')
    if data['schema'].endswith('/v3'):
        if data.get('diagnostics',{}).get('policy')!='lighting-factors/v1':raise ValueError('matched qualification requires actual lighting factors')
        matched=data.get('sampling',{}).get('matched',{})
        if matched.get('policy')!='matched-box/v1' or matched.get('grid')!=data['supersample']:raise ValueError('matched sampling policy mismatch')
    if data.get('resolution')!=[512,512] or data.get('coordinates')!='gltf-y-up-metres':raise ValueError('unsupported capture domain')
    if data.get('required_cases')!=IDS or [c['id'] for c in data.get('cases',[])]!=IDS:raise ValueError('required cases missing, duplicated or reordered')
    if not data.get('files'):raise ValueError('input inventory required')
    for name,digest in data['files'].items():
        if sha(local(root,name))!=digest:raise ValueError(f'input identity mismatch: {name}')
    for required in ['runtime/character.glb','runtime/character.npr.json','runtime/character.view.json','authoring.json','audit.json','calibration.json','calibration-blender.png','runtime/calibration.glb']:
        if required not in data['files']:raise ValueError(f'input not inventoried: {required}')
    vector(data['background_linear'],4)
    for case in data['cases']:
        camera=case['camera'];position=vector(camera['position']);target=vector(camera['target']);up=vector(camera['up'])
        if camera['projection']!='orthographic' or not all(math.isfinite(camera[k]) for k in ['near','far']) or not 0<camera['near']<camera['far'] or not math.isfinite(camera['scale']) or camera['scale']<=0:raise ValueError('invalid camera')
        delta=[b-a for a,b in zip(position,target)]
        cross=[delta[1]*up[2]-delta[2]*up[1],delta[2]*up[0]-delta[0]*up[2],delta[0]*up[1]-delta[1]*up[0]]
        if sum(x*x for x in cross)<1e-12:raise ValueError('degenerate camera basis')
        direction=vector(case['direction_to_light'])
        if abs(sum(x*x for x in direction)-1)>1e-5:raise ValueError('unit light direction required')
        for file in case['reference']+[case['actual'],case['silhouette']]:
            if file not in data['files']:raise ValueError('reference not inventoried')
        if data['schema'].endswith(('/v2','/v3')):
            for key in ['factor_reference','center_reference']:
                if case.get(key) not in data['files']:raise ValueError('missing diagnostic reference')
        verify_actual(root,case)
    sidecar=json.loads(local(root,'runtime/character.npr.json').read_text())
    for mat in sidecar['materials'].values():
        for texture in mat.get('textures',[]):
            uri='runtime/'+texture['uri']
            if uri not in data['files']:raise ValueError('descriptor texture not inventoried')
            local(root,uri)
    return data

def verify_actual(root,case):
    """Check the upstream camera and light evaluation, not a copied request."""
    root=Path(root);actual=json.loads(local(root,case['actual']).read_text());camera=case['camera'];m=actual['camera']['matrix_world']
    convert=lambda v:[v[0],v[2],-v[1]]
    position=convert([m[i][3] for i in range(3)])
    forward=convert([-m[i][2] for i in range(3)])
    up=convert([m[i][1] for i in range(3)])
    target=[a-b for a,b in zip(camera['target'],camera['position'])];length=math.sqrt(sum(v*v for v in target));target=[v/length for v in target]
    for a,b in [(position,camera['position']),(forward,target),(up,camera['up'])]:
        if max(abs(x-y) for x,y in zip(a,b))>1e-5:raise ValueError('actual camera transform mismatch')
    if abs(actual['camera']['ortho_scale']-camera['scale'])>1e-5:raise ValueError('actual camera scale mismatch')
    if actual['exposure_ev']!=0 or actual['gamma']!=1 or actual['view_transform']!='Standard' or actual['resolution']!=[512,512]:raise ValueError('actual output domain mismatch')
    direction=convert(actual['body_direction_blender']);length=math.sqrt(sum(v*v for v in direction))
    if max(abs(a/length-b) for a,b in zip(direction,case['direction_to_light']))>1e-5:raise ValueError('actual light direction mismatch')
    return actual
