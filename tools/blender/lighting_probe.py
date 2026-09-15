"""Append upstream lighting-factor and center-coverage diagnostics to a frozen case."""
import argparse,json,hashlib,shutil,sys
from pathlib import Path
import bpy
from mathutils import Vector,Euler
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--case',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--base-color',action='store_true');p.add_argument('--lightmap',action='store_true');p.add_argument('--pre-palette',action='store_true');p.add_argument('--highlight-alpha',action='store_true');p.add_argument('--ndoth',action='store_true');p.add_argument('--beauty',action='store_true')
a=p.parse_args(sys.argv[sys.argv.index('--')+1:]);original=json.loads(a.case.read_text());author=json.loads((a.case.parent/'authoring.json').read_text());recipe=author['recipe']
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
if sha(bpy.data.filepath)!=author['blend_sha256']:raise RuntimeError('frozen scene identity mismatch')
if a.out.exists():raise RuntimeError('candidate exists')
shutil.copytree(a.case.parent,a.out,ignore=shutil.ignore_patterns('case.json'))
scene=bpy.context.scene;scene.frame_set(recipe['frame']);objects=[bpy.data.objects[n] for n in recipe['meshes']]
for obj in scene.objects:
 if obj.type=='MESH':obj.hide_render=obj not in objects
saved=[(o,list(o.data.materials)) for o in objects];factor_materials={};bindings={}
for material in {m for _,mats in saved for m in mats}:
 copy=material.copy();root=next(n for n in copy.node_tree.nodes if n.type=='GROUP' and n.node_tree and n.node_tree.name.split('.')[0] in {'HoYoverse - GI Body','HoYoverse - GI Face'})
 if 'Lit Factor' not in root.outputs or 'Lit Sharp Factor' not in root.outputs:raise RuntimeError('shader does not expose lighting factors')
 nt=copy.node_tree;output=next(n for n in nt.nodes if n.type=='OUTPUT_MATERIAL');combine=nt.nodes.new('ShaderNodeCombineXYZ');combine.inputs['Z'].default_value=1
 nt.links.new(root.outputs['Lit Factor'],combine.inputs['X']);nt.links.new(root.outputs['Lit Sharp Factor'],combine.inputs['Y']);em=nt.nodes.new('ShaderNodeEmission');nt.links.new(combine.outputs[0],em.inputs['Color']);nt.links.new(em.outputs[0],output.inputs['Surface'])
 if a.base_color or a.lightmap:
  socket=root.inputs['Lightmap Color' if a.lightmap else 'Diffuse Color']
  if socket.is_linked:nt.links.new(socket.links[0].from_socket,em.inputs['Color'])
  else:
   for link in list(em.inputs['Color'].links):nt.links.remove(link)
   em.inputs['Color'].default_value=socket.default_value
 if a.pre_palette:
  root.node_tree=root.node_tree.copy();g=root.node_tree
  output_group=next(n for n in g.nodes if n.type=='GROUP_OUTPUT')
  source=g.nodes['Specular'].outputs['Color'] if g.name.split('.')[0]=='HoYoverse - GI Body' else g.nodes['Group'].inputs['Color'].links[0].from_socket
  g.links.new(source,output_group.inputs['BSDF'])
  nt.links.new(root.outputs['BSDF'],output.inputs['Surface'])
 if a.highlight_alpha:
  for link in list(em.inputs['Color'].links):nt.links.remove(link)
  if 'Shader to RGB' in nt.nodes:nt.links.new(nt.nodes['Shader to RGB'].outputs['Alpha'],em.inputs['Color'])
  else:em.inputs['Color'].default_value=(0,0,0,1)
 if a.ndoth:
  root.node_tree=root.node_tree.copy();g=root.node_tree;go=next(n for n in g.nodes if n.type=='GROUP_OUTPUT')
  if 'Shading NdotL / NdotH' in g.nodes:g.links.new(g.nodes['Shading NdotL / NdotH'].outputs['NdotH'],go.inputs['BSDF'])
  else:
   for link in list(go.inputs['BSDF'].links):g.links.remove(link)
  nt.links.new(root.outputs['BSDF'],output.inputs['Surface'])
 factor_materials[material]=material if a.beauty else copy;bindings[material.name]={'group':root.node_tree.name,'red':'Lit Factor','green':'Lit Sharp Factor','blue':'geometry coverage'}
white=bpy.data.materials.new('CenterCoverage');white.use_nodes=True;nt=white.node_tree;nt.nodes.clear();em=nt.nodes.new('ShaderNodeEmission');em.inputs['Color'].default_value=(1,1,1,1);nt.links.new(em.outputs[0],nt.nodes.new('ShaderNodeOutputMaterial').inputs['Surface'])
scene.render.engine='BLENDER_EEVEE';scene.eevee.taa_render_samples=1;scene.render.filter_size=.01;scene.render.dither_intensity=0
scene.render.resolution_x=scene.render.resolution_y=512;scene.render.resolution_percentage=100;scene.render.pixel_aspect_x=scene.render.pixel_aspect_y=1
scene.render.image_settings.file_format='PNG';scene.render.image_settings.color_mode='RGBA';scene.render.image_settings.color_depth='8';scene.render.use_compositing=False;scene.render.film_transparent=False
scene.view_settings.view_transform='Standard';scene.view_settings.look='None';scene.view_settings.exposure=0;scene.view_settings.gamma=1
world=bpy.data.worlds.new('ProbeBackground');world.use_nodes=True;world.node_tree.nodes['Background'].inputs['Color'].default_value=(0,0,0,1);scene.world=world
camdata=bpy.data.cameras.new('ProbeCamera');cam=bpy.data.objects.new('ProbeCamera',camdata);scene.collection.objects.link(cam);scene.camera=cam;camdata.type='ORTHO'
convert=lambda v:Vector((v[0],-v[2],v[1]))
for item in original['cases']:
 data=item['camera'];cam.location=convert(data['position']);target=convert(data['target']);cam.rotation_euler=(target-cam.location).to_track_quat('-Z','Y').to_euler();camdata.ortho_scale=data['scale'];camdata.clip_start=data['near'];camdata.clip_end=data['far']
 light=convert(item['direction_to_light']);bpy.data.objects['Light Direction'].rotation_euler=Vector((0,0,-1)).rotation_difference(light).to_euler('XYZ');bpy.context.view_layer.update()
 for obj,mats in saved:
  for i,m in enumerate(mats):obj.data.materials[i]=factor_materials[m]
 name=item['id']+'-lighting-factors.png';scene.render.filepath=str(a.out/name);bpy.ops.render.render(write_still=True);item['factor_reference']=name
 view=item['id'].rsplit('-',1)[0];mask=view+'-center-coverage.png'
 if not (a.out/mask).exists():
  for obj,mats in saved:
   for i in range(len(mats)):obj.data.materials[i]=white
  scene.render.filepath=str(a.out/mask);bpy.ops.render.render(write_still=True)
 item['center_reference']=mask
 for obj,mats in saved:
  for i,m in enumerate(mats):obj.data.materials[i]=m
original['schema']='bevy_stylized.character-capture/v2';original['diagnostics']={'policy':'beauty-center/v1' if a.beauty else ('ndoth/v1' if a.ndoth else ('pre-palette/v1' if a.pre_palette else ('lightmap/v1' if a.lightmap else ('base-color/v1' if a.base_color else 'lighting-factors/v1')))),'source_case_sha256':sha(a.case),'bindings':bindings,'blender_samples':scene.eevee.taa_render_samples,'filter_size':scene.render.filter_size,'dither':scene.render.dither_intensity,'bevy_msaa':'off','background_linear':[0,0,0,1],'interpretation':'linear factors encoded through Standard/sRGB; geometry and factor masks are separate from AA beauty comparisons'}
original['files']={str(p.relative_to(a.out)):sha(p) for p in sorted(a.out.rglob('*')) if p.is_file()}
(a.out/'case.json').write_text(json.dumps(original,indent=2)+'\n')
