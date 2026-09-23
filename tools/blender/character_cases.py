"""Prepare six static cases from the verified authored scene, never overwrite it."""
import argparse,hashlib,json,math,shutil,sys
from pathlib import Path
import bpy
from mathutils import Vector,Euler
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools'))
from assets.capture_case import SCHEMA,IDS,sha,validate
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--build',type=Path,required=True);p.add_argument('--out',type=Path,required=True)
a=p.parse_args(sys.argv[sys.argv.index('--')+1:]);build=a.build.resolve();out=a.out.resolve()
record=json.loads((build/'authoring/authoring.json').read_text());recipe=record['recipe']
if sha(bpy.data.filepath)!=record['blend_sha256'] or bpy.app.version_string!=recipe['blender']:raise RuntimeError('authoring identity/tool mismatch')
if out.exists():raise RuntimeError('candidate exists')
out.mkdir(parents=True);shutil.copytree(build/'compiled',out/'runtime')
for name in ['authoring.json','audit.json']:shutil.copy2(build/'authoring'/name,out/name)
scene=bpy.context.scene;scene.frame_set(recipe['frame']);bpy.context.view_layer.update()
objects=[bpy.data.objects[n] for n in list(recipe['meshes'])+list(recipe.get('effect_meshes',[]))]
# Camera fitting and shader-factor audits use the base surface. Render visibility
# stays enabled for the authored hulls in the beauty reference.
from blender.outline_handoff import modifiers
outline_modifiers=[m for _,m in modifiers(objects)]
for modifier in outline_modifiers:modifier.show_viewport=False
bpy.context.view_layer.update()
for obj in scene.objects:
    if obj.type=='MESH':obj.hide_render=obj not in objects
# Assert the locked upstream light construction, including signs and rotation mode.
for group,op in [('Dot Creation','ADD'),('Face Factor','SUBTRACT')]:
    g=bpy.data.node_groups[group]
    assert g.nodes['Vector Math.010'].operation==op
    assert tuple(g.nodes['Vector Math.010'].inputs[1].default_value)==(0,0,1)
    n=g.nodes['Vector Rotate'];assert n.rotation_type=='EULER_XYZ' and not n.invert
    assert g.nodes['Attribute'].attribute_name=='lightDir'
    assert n.inputs['Rotation'].links[0].from_node==g.nodes['Attribute']
offset_evidence={}
for name in ['AO Normal','NdotL','Ramps Normal']:
    socket=bpy.data.node_groups['HoYoverse - GI Body'].nodes[name].inputs['Offset']
    if socket.is_linked or any(abs(v)>1e-7 for v in socket.default_value):raise RuntimeError('unmapped authored light offset')
    offset_evidence[name]=list(socket.default_value)
# Compute cameras from evaluated geometry, once, then hand exact values to Bevy.
def bounds(items):
    points=[];dg=bpy.context.evaluated_depsgraph_get()
    for obj in items:
        eo=obj.evaluated_get(dg);mesh=eo.to_mesh()
        points.extend(obj.matrix_world@v.co for v in mesh.vertices);eo.to_mesh_clear()
    lo=Vector([min(v[i] for v in points) for i in range(3)]);hi=Vector([max(v[i] for v in points) for i in range(3)])
    return lo,hi
lo,hi=bounds(objects);height=hi.z-lo.z;center=(lo+hi)*.5
faces=[o for o in objects if any(s.material and recipe['material_roles'].get(s.material.name)=='face' for s in o.material_slots)]
flo,fhi=bounds(faces)
def convert(v):return [v[0],v[2],-v[1]]
camdata=bpy.data.cameras.new('SharedCapture');cam=bpy.data.objects.new('SharedCapture',camdata);scene.collection.objects.link(cam);scene.camera=cam
camdata.type='ORTHO';camdata.clip_start=.01;camdata.clip_end=100
scene.render.engine='BLENDER_EEVEE';scene.render.resolution_x=512;scene.render.resolution_y=512;scene.render.resolution_percentage=100
scene.render.pixel_aspect_x=scene.render.pixel_aspect_y=1;scene.render.image_settings.file_format='PNG';scene.render.image_settings.color_mode='RGBA';scene.render.image_settings.color_depth='8'
scene.render.film_transparent=False;scene.render.use_compositing=False
scene.view_settings.view_transform='Standard';scene.view_settings.look='None';scene.view_settings.exposure=0;scene.view_settings.gamma=1
world=bpy.data.worlds.new('SharedBackground');world.use_nodes=True;scene.world=world
world.node_tree.nodes['Background'].inputs['Color'].default_value=(.12,.12,.12,1);world.node_tree.nodes['Background'].inputs['Strength'].default_value=1
sun_data=bpy.data.lights.new('SharedSun','SUN');sun=bpy.data.objects.new('SharedSun',sun_data);scene.collection.objects.link(sun);sun.rotation_euler=(.5,-.5,-.5);sun_data.energy=3
light=bpy.data.objects['Light Direction'];cases=[]
for label,yaw in [('front',0),('left',-.65),('right',.65)]:
    light.rotation_euler=(-math.pi/2,0,yaw);bpy.context.view_layer.update();dg=bpy.context.evaluated_depsgraph_get()
    audit={}
    for obj in objects:
        eo=obj.evaluated_get(dg);mesh=eo.to_mesh();attrs={}
        for name in ['lightDir','headForward','headUp']:
            attr=mesh.attributes.get(name)
            if not attr:raise RuntimeError(f'{obj.name} missing {name}')
            values=[Vector(v.vector) for v in attr.data];first=values[0]
            if any((v-first).length>1e-5 for v in values):raise RuntimeError('nonuniform static light/basis input')
            attrs[name]=list(first)
        audit[obj.name]=attrs;eo.to_mesh_clear()
    body_object=next(o.name for o in objects if any(slot.material and recipe['material_roles'].get(slot.material.name)=='body' for slot in o.material_slots))
    angles=audit[body_object]['lightDir'];body=Euler(angles,'XYZ').to_matrix()@Vector((0,0,-1));face=-body
    for view,target,scale in [('full_body',center,height*1.2),('face_closeup',(flo+fhi)*.5,max(fhi.z-flo.z,height*.2)*1.45)]:
        identifier=f'{view}-{label}';position=target+Vector((0,-height*3,0));cam.location=position;cam.rotation_euler=(target-position).to_track_quat('-Z','Y').to_euler();camdata.ortho_scale=scale
        camera={'position':convert(position),'target':convert(target),'up':[0,1,0],'projection':'orthographic','scale':scale,'near':.01,'far':100}
        references=[]
        for repetition in range(2):
            name=f'{identifier}-blender-{repetition}.png';scene.render.filepath=str(out/name);bpy.ops.render.render(write_still=True);references.append(name)
        silhouette_name=view+'-silhouette.png'
        if label=='front':
            white=bpy.data.materials.new('DiagnosticCoverage');white.use_nodes=True;nt=white.node_tree;nt.nodes.clear()
            em=nt.nodes.new('ShaderNodeEmission');em.inputs['Color'].default_value=(1,1,1,1);nt.links.new(em.outputs[0],nt.nodes.new('ShaderNodeOutputMaterial').inputs['Surface'])
            originals=[(o,list(o.data.materials)) for o in objects]
            for obj,mats in originals:
                for index in range(len(mats)):obj.data.materials[index]=white
            outline_states=[(m,m.show_render) for m in outline_modifiers]
            for modifier,_ in outline_states:modifier.show_render=False
            scene.render.filepath=str(out/silhouette_name);bpy.ops.render.render(write_still=True)
            for modifier,state in outline_states:modifier.show_render=state
            for obj,mats in originals:
                for index,mat in enumerate(mats):obj.data.materials[index]=mat
        physical_probe = None
        if identifier == 'full_body-front':
            probe_name=identifier+'-sun-off.png';sun_data.energy=0
            scene.render.filepath=str(out/probe_name);bpy.ops.render.render(write_still=True);sun_data.energy=3
            first_image=bpy.data.images.load(str(out/references[0]),check_existing=False)
            probe_image=bpy.data.images.load(str(out/probe_name),check_existing=False)
            physical_probe={'image':probe_name,'max_linear_pixel_delta':max(abs(x-y) for x,y in zip(first_image.pixels[:],probe_image.pixels[:]))}
            bpy.data.images.remove(first_image);bpy.data.images.remove(probe_image)
        actual={'light_offset_evidence':offset_evidence,'physical_sun_probe':physical_probe,'camera' :{'matrix_world':[list(r) for r in cam.matrix_world],'ortho_scale':camdata.ortho_scale,'clip_start':camdata.clip_start,'clip_end':camdata.clip_end},
                'light_attributes':audit,'body_direction_blender':list(body),'face_internal_direction_blender':list(face),'light_mapping':'Dot Creation R(euler)*-Z; Face Factor R(euler)*+Z; face convention separate',
                'background_linear':list(world.node_tree.nodes['Background'].inputs['Color'].default_value),'exposure_ev':scene.view_settings.exposure,'gamma':scene.view_settings.gamma,'view_transform':scene.view_settings.view_transform,'look':scene.view_settings.look,
                'render_engine':scene.render.engine,'resolution':[scene.render.resolution_x,scene.render.resolution_y],'sun_energy':sun_data.energy,'blender':bpy.app.version_string,'frame':scene.frame_current}
        actual_name=identifier+'-actual.json';(out/actual_name).write_text(json.dumps(actual,indent=2)+'\n')
        cases.append({'id':identifier,'camera':camera,'direction_to_light':convert(body.normalized()),'reference':references,'actual':actual_name,'silhouette':silhouette_name})
sys.path.insert(0,str(Path(__file__).parent))
from capture_calibration import generate
generate(out)
payload={'schema':SCHEMA,'coordinates':'gltf-y-up-metres','resolution':[512,512],'background_linear':[.12,.12,.12,1],
 'required_cases':IDS,'cases':cases,'recipe_id':recipe['id'],'scope':record['scope'],'frame':recipe['frame'],
 'source_blend_sha256':record['blend_sha256'],'generator_sha256':sha(__file__),
 'sampling':{'blender':{'engine':'EEVEE','taa_render_samples':getattr(scene.eevee,'taa_render_samples',None),'resolution_percentage':scene.render.resolution_percentage,'pixel_aspect':[scene.render.pixel_aspect_x,scene.render.pixel_aspect_y],'display_device':scene.display_settings.display_device},'bevy':'MSAA 4x, no tonemapper; two captures after asset/pipeline readiness'},
 'semantic_masks':'UNQUALIFIED: legacy luminance proxy is diagnostic only',
 'files':{str(f.relative_to(out)):sha(f) for f in sorted(out.rglob('*')) if f.is_file()}}
(out/'case.json').write_text(json.dumps(payload,indent=2)+'\n');validate(out/'case.json')
print('CHARACTER_CASES',out/'case.json')
