"""Matched offline sampling: both renderers shade the same regular grid in linear HDR."""
import argparse,json,hashlib,shutil,sys
from pathlib import Path
import bpy,numpy as np
from mathutils import Vector
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--case',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--scale',type=int,default=4,choices=[2,4,8]);a=p.parse_args(sys.argv[sys.argv.index('--')+1:])
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
j=json.loads(a.case.read_text());author=json.loads((a.case.parent/'authoring.json').read_text());recipe=author['recipe']
if sha(bpy.data.filepath)!=author['blend_sha256']:raise RuntimeError('frozen authoring mismatch')
if a.out.exists():raise RuntimeError('candidate exists')
shutil.copytree(a.case.parent,a.out,ignore=shutil.ignore_patterns('case.json'))
scene=bpy.context.scene;scene.frame_set(recipe['frame'])
for obj in scene.objects:
 if obj.type=='MESH':obj.hide_render=obj.name not in set(list(recipe['meshes'])+list(recipe.get('effect_meshes',[])))
scene.render.engine='BLENDER_EEVEE';scene.eevee.taa_render_samples=1;scene.render.filter_size=.01;scene.render.dither_intensity=0;scene.render.resolution_percentage=100;scene.render.pixel_aspect_x=scene.render.pixel_aspect_y=1;scene.render.film_transparent=False;scene.render.use_compositing=False
scene.view_settings.view_transform='Standard';scene.view_settings.look='None';scene.view_settings.exposure=0;scene.view_settings.gamma=1
world=bpy.data.worlds.new('MatchedWorld');world.use_nodes=True;world.node_tree.nodes['Background'].inputs['Color'].default_value=tuple(j['background_linear']);scene.world=world
cd=bpy.data.cameras.new('MatchedCamera');cam=bpy.data.objects.new('MatchedCamera',cd);scene.collection.objects.link(cam);scene.camera=cam;cd.type='ORTHO'
convert=lambda v:Vector((v[0],-v[2],v[1]));size=512*a.scale
for case in j['cases']:
 c=case['camera'];cam.location=convert(c['position']);target=convert(c['target']);cam.rotation_euler=(target-cam.location).to_track_quat('-Z','Y').to_euler();cd.ortho_scale=c['scale'];cd.clip_start=c['near'];cd.clip_end=c['far']
 bpy.data.objects['Light Direction'].rotation_euler=Vector((0,0,-1)).rotation_difference(convert(case['direction_to_light'])).to_euler('XYZ');bpy.context.view_layer.update()
 refs=[]
 for repetition in range(2):
  scene.render.resolution_x=scene.render.resolution_y=size;scene.render.image_settings.file_format='OPEN_EXR';scene.render.image_settings.color_depth='32';scene.render.image_settings.color_mode='RGBA';scene.render.image_settings.exr_codec='ZIP'
  exr=a.out/f"matched-{case['id']}-{repetition}.exr";scene.render.filepath=str(exr);bpy.ops.render.render(write_still=True)
  image=bpy.data.images.load(str(exr),check_existing=False);values=np.empty(size*size*4,dtype=np.float32);image.pixels.foreach_get(values);bpy.data.images.remove(image)
  if not np.isfinite(values).all():raise RuntimeError('nonfinite reference capture')
  resolved=values.reshape(512,a.scale,512,a.scale,4).mean(axis=(1,3))
  image=bpy.data.images.new('Resolved',width=512,height=512,alpha=True,float_buffer=True);image.pixels.foreach_set(resolved.ravel())
  scene.render.image_settings.file_format='PNG';scene.render.image_settings.color_depth='8';name=f"matched-{case['id']}-{repetition}.png";image.save_render(str(a.out/name),scene=scene);bpy.data.images.remove(image);refs.append(name)
 case['legacy_reference']=case['reference'];case['reference']=refs
 actual=json.loads((a.case.parent/case['actual']).read_text());actual['camera']={'matrix_world':[list(row) for row in cam.matrix_world],'ortho_scale':cd.ortho_scale,'clip_start':cd.clip_start,'clip_end':cd.clip_end};actual['internal_resolution']=[size,size];actual['sampling']='single-sample linear HDR regular grid';actual['box_scale']=a.scale;actual['dither']=0
 name=f"matched-{case['id']}-actual.json";(a.out/name).write_text(json.dumps(actual,indent=2)+'\n');case['actual']=name
j['schema']='bevy_stylized.character-capture/v3';j['supersample']=a.scale;j['sampling']['matched']={'grid':a.scale,'source':'linear RGBA32F, one sample per high-resolution pixel','runtime':'linear RGBA16F, one sample per high-resolution pixel','resolve':'linear box, then Standard/sRGB exactly once','policy':'matched-box/v1','producer_sha256':sha(__file__)}
j['files']={str(p.relative_to(a.out)):sha(p) for p in a.out.rglob('*') if p.is_file()};(a.out/'case.json').write_text(json.dumps(j,indent=2)+'\n')
