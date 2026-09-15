"""Capture the declared authoring variant without saving over its source blend."""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import bpy
from mathutils import Vector

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--authoring',required=True,type=Path);p.add_argument('--out',required=True,type=Path)
a=p.parse_args(sys.argv[sys.argv.index('--')+1:])
record=json.loads(a.authoring.read_text());recipe=record['recipe']
if hashlib.sha256(Path(bpy.data.filepath).read_bytes()).hexdigest()!=record['blend_sha256']:raise RuntimeError('source blend identity differs')
if a.out.exists():raise RuntimeError('capture candidate already exists')
a.out.mkdir(parents=True)
scene=bpy.context.scene;scene.frame_set(recipe['frame'])
selected=[bpy.data.objects[name] for name in recipe['meshes']]
for obj in scene.objects:
    if obj.type=='MESH':obj.hide_render=obj not in selected
points=[obj.matrix_world@Vector(corner) for obj in selected for corner in obj.bound_box]
lo=Vector([min(v[i] for v in points) for i in range(3)]);hi=Vector([max(v[i] for v in points) for i in range(3)])
height=hi.z-lo.z;center=(lo+hi)*.5
face_objects=[obj for obj in selected if any(slot.material and recipe['material_roles'].get(slot.material.name)=='face' for slot in obj.material_slots)]
face_points=[obj.matrix_world@Vector(c) for obj in face_objects for c in obj.bound_box]
face_lo=Vector([min(v[i] for v in face_points) for i in range(3)]);face_hi=Vector([max(v[i] for v in face_points) for i in range(3)])
cam_data=bpy.data.cameras.new('ReferenceCapture');cam=bpy.data.objects.new('ReferenceCapture',cam_data);scene.collection.objects.link(cam);scene.camera=cam;cam_data.type='ORTHO'
sun_data=bpy.data.lights.new('ReferenceCaptureSun','SUN');sun=bpy.data.objects.new('ReferenceCaptureSun',sun_data);scene.collection.objects.link(sun);sun.rotation_euler=(.5,-.5,-.5);sun_data.energy=3
scene.render.engine='BLENDER_EEVEE';scene.render.resolution_x=512;scene.render.resolution_y=512;scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG';scene.render.image_settings.color_mode='RGBA';scene.render.film_transparent=False;scene.render.use_compositing=False
scene.view_settings.view_transform='Standard';scene.view_settings.look='None';scene.view_settings.exposure=0
world=bpy.data.worlds.new('ReferenceCaptureWorld');world.use_nodes=True;scene.world=world;world.node_tree.nodes['Background'].inputs['Color'].default_value=(.12,.12,.12,1)
for name,target,scale in [('full_body',center,height*1.2),('face_closeup',(face_lo+face_hi)*.5,max(face_hi.z-face_lo.z,height*.2)*1.45)]:
    cam.location=target+Vector((0,-height*3,0));cam.rotation_euler=(target-cam.location).to_track_quat('-Z','Y').to_euler();cam_data.ortho_scale=scale
    path=a.out/(name+'.png');scene.render.filepath=str(path);bpy.ops.render.render(write_still=True)
    provenance={'schema':'bevy_stylized.capture/v1','case':name,'asset_ownership':'external','authoring_sha256':hashlib.sha256(a.authoring.read_bytes()).hexdigest(),
                'image_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'blender':bpy.app.version_string,'gacha_version':recipe['gacha_version'],
                'recipe_id':recipe['id'],'scope':record['scope'],'resolution':[512,512],'frame':recipe['frame'],
                'camera':{'position':list(cam.location),'look_at':list(target),'projection':'orthographic','scale':scale},
                'color_management':{'view_transform':'Standard','exposure':0,'compositor':False},'sun':{'rotation':[.5,-.5,-.5],'energy':3},'world':[.12,.12,.12,1]}
    path.with_suffix('.golden.json').write_text(json.dumps(provenance,indent=2)+'\n')
print('CHARACTER_REFERENCE_CAPTURE',a.out)
