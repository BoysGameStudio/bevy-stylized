"""Generate a first-party skinned/morph character GLB without Blender or downloads."""
from pathlib import Path
import hashlib
import json
import math
import os
import struct


def generate(output):
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    binary = bytearray()
    doc = {'asset': {'version': '2.0', 'generator': 'bevy-stylized synthetic-character/v1'},
           'scene': 0, 'scenes': [{'nodes': [0, 3, 4, 5]}],
           'nodes': [{'name': 'Rig', 'children': [1]}, {'name': 'RootJoint', 'children': [2]},
                     {'name': 'HeadJoint', 'translation': [0, 1.25, 0]},
                     {'name': 'Body', 'mesh': 0, 'skin': 0},
                     {'name': 'Face', 'mesh': 1, 'skin': 0, 'weights': [0]},
                     {'name': 'Hair', 'mesh': 2, 'skin': 0}],
           'bufferViews': [], 'accessors': [], 'meshes': [],
           'materials': [{'name': name, 'pbrMetallicRoughness': {'baseColorFactor': color,
                          'metallicFactor': 0, 'roughnessFactor': 1}, 'doubleSided': True,
                          'extras': {'bevy_stylized.material/v1': {'name': name, 'surface_class': 'cel', 'static_params': dict(zip(['base_color_r','base_color_g','base_color_b','base_color_a'],color))}}}
                         for name, color in [('body', [.22,.45,.75,1]),('face',[.95,.7,.55,1]),('hair',[.15,.1,.3,1])]]}

    def acc(values, width, kind, component=5126, bounds=False):
        while len(binary) % 4:
            binary.append(0)
        offset = len(binary)
        fmt = 'f' if component == 5126 else 'H'
        for row in values:
            binary.extend(struct.pack('<'+fmt*width, *(row if width > 1 else [row])))
        view = len(doc['bufferViews'])
        doc['bufferViews'].append({'buffer': 0, 'byteOffset': offset, 'byteLength': len(binary)-offset})
        entry = {'bufferView': view, 'componentType': component, 'count': len(values), 'type': kind}
        if bounds:
            entry['min'] = [min(v[i] for v in values) for i in range(width)] if width > 1 else [min(values)]
            entry['max'] = [max(v[i] for v in values) for i in range(width)] if width > 1 else [max(values)]
        doc['accessors'].append(entry)
        return len(doc['accessors'])-1

    for mi,(x,y,z,w,h) in enumerate([(0,.7,0,.55,1.3),(0,1.5,.12,.48,.45),(0,1.77,0,.6,.18)]):
        pos = [(x-w/2,y-h/2,z),(x+w/2,y-h/2,z),(x+w/2,y+h/2,z),(x-w/2,y+h/2,z)]
        joints = [(1 if mi else 0,0,0,0)]*4
        attr = {'POSITION': acc(pos,3,'VEC3',bounds=True), 'NORMAL': acc([(0,0,1)]*4,3,'VEC3'),
                'TEXCOORD_0': acc([(0,1),(1,1),(1,0),(0,0)],2,'VEC2'),
                'COLOR_0': acc([(1,1,1,1)]*4,4,'VEC4'),
                'JOINTS_0': acc(joints,4,'VEC4',5123), 'WEIGHTS_0': acc([(1,0,0,0)]*4,4,'VEC4')}
        primitive = {'attributes': attr, 'indices': acc([0,1,2,0,2,3],1,'SCALAR',5123), 'material':mi}
        mesh={'name':['Body','Face','Hair'][mi],'primitives':[primitive]}
        if mi==1:
            primitive['targets']=[{'POSITION': acc([(0,0,0),(0,0,0),(0,.12,0),(0,.12,0)],3,'VEC3',bounds=True)}]
            mesh['weights']=[0]
        doc['meshes'].append(mesh)
    identity=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]
    head=identity.copy();head[13]=-1.25
    doc['skins']=[{'name':'CharacterSkin','joints':[1,2],'skeleton':1,
                   'inverseBindMatrices':acc([identity,head],16,'MAT4')}]
    time=acc([0,.5,1],1,'SCALAR',bounds=True)
    rot=acc([(0,0,0,1),(0,math.sin(.2),0,math.cos(.2)),(0,0,0,1)],4,'VEC4')
    weights=acc([0,1,0],1,'SCALAR')
    doc['animations']=[{'name':'head-turn-and-expression','samplers':[{'input':time,'output':rot,'interpolation':'LINEAR'},
                        {'input':time,'output':weights,'interpolation':'LINEAR'}],
                        'channels':[{'sampler':0,'target':{'node':2,'path':'rotation'}},
                                    {'sampler':1,'target':{'node':4,'path':'weights'}}]}]
    doc['buffers']=[{'byteLength':len(binary)}]
    encoded=json.dumps(doc,sort_keys=True,separators=(',',':')).encode()
    encoded+=b' '*((-len(encoded))%4);binary.extend(b'\0'*((-len(binary))%4))
    blob=struct.pack('<4sII',b'glTF',2,28+len(encoded)+len(binary))
    blob+=struct.pack('<II',len(encoded),0x4e4f534a)+encoded+struct.pack('<II',len(binary),0x004e4942)+binary
    (output/'character.npr.json').write_text(json.dumps({'schema':'bevy-stylized.sidecar/v1','materials':{m['name']:m['extras']['bevy_stylized.material/v1'] for m in doc['materials']}},indent=2)+'\n')
    path=output/'character.glb';tmp=path.with_suffix('.glb.tmp');tmp.write_bytes(blob);os.replace(tmp,path)
    provenance={'schema':'bevy_stylized.fixture/v1','id':'synthetic-character-v1','authorship':'Bevy Stylized first-party procedural geometry',
                'units':'metres','up':'+Y','forward':'+Z','head_bone':'HeadJoint',
                'sha256':hashlib.sha256(blob).hexdigest(),'generator_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (output/'fixture.json').write_text(json.dumps(provenance,indent=2)+'\n')
    return path
