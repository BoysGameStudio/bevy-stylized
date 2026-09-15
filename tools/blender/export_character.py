"""Export the explicit static-reference recipe; no game-project exporter required.

Diffuse node outputs (including pupil composition) are rasterized with upstream
EEVEE on a UV plane. Runtime meshes are evaluated after removing upstream outlines.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sys
import tempfile
import bpy

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def direct_diffuse(material, group, path):
    """Preserve a direct image/UV switch; composites retain the explicit bake path."""
    socket=group.inputs['Diffuse Color']
    if not socket.is_linked:return None
    node=socket.links[0].from_node
    if node.type=='TEX_IMAGE':nodes=[node]
    elif node.type=='GROUP' and node.node_tree.name.split('.')[0]=='UV Lerp':
        nodes=[link.from_node for item in node.inputs for link in item.links]
        if len(nodes)!=2:return None
    else:return None
    if any(n.type!='TEX_IMAGE' or not n.image or n.interpolation!='Cubic' or n.extension!='REPEAT' for n in nodes):return None
    if len({n.image.as_pointer() for n in nodes})!=1:return None
    image=nodes[0].image
    if image.colorspace_settings.name!='sRGB' or image.alpha_mode!='CHANNEL_PACKED':raise RuntimeError('unsupported direct diffuse color domain')
    image.save(filepath=str(path))
    return {'method':'authored-diffuse-cubic/v1','source_image':image.name,'sha256':sha(path)}


def bake_color(material, group, path):
    original_scene=bpy.context.scene
    bake_scene=bpy.data.scenes.new('ColorBake')
    bpy.context.window.scene=bake_scene
    mesh=bpy.data.meshes.new('ColorBakePlane')
    mesh.from_pydata([(-1,-1,0),(1,-1,0),(1,1,0),(-1,1,0)],[],[(0,1,2,3)])
    for name in ['UVMap','UV1']:
        uv=mesh.uv_layers.new(name=name)
        for i,coord in enumerate([(0,0),(1,0),(1,1),(0,1)]):uv.data[i].uv=coord
    plane=bpy.data.objects.new('ColorBakePlane',mesh);bake_scene.collection.objects.link(plane)
    copy=material.copy();nodes=copy.node_tree.nodes
    output=next(n for n in nodes if n.type=='OUTPUT_MATERIAL')
    for link in list(output.inputs['Surface'].links):copy.node_tree.links.remove(link)
    emission=nodes.new('ShaderNodeEmission');socket=nodes[group.name].inputs['Diffuse Color']
    if socket.is_linked:
        copy.node_tree.links.new(socket.links[0].from_socket,emission.inputs['Color'])
    else:emission.inputs['Color'].default_value=socket.default_value
    copy.node_tree.links.new(emission.outputs[0],output.inputs['Surface']);mesh.materials.append(copy)
    cam_data=bpy.data.cameras.new('BakeCamera');cam=bpy.data.objects.new('BakeCamera',cam_data)
    bake_scene.collection.objects.link(cam);cam.location=(0,0,3);cam_data.type='ORTHO';cam_data.ortho_scale=2;bake_scene.camera=cam
    size=max([max(n.image.size) for n in nodes if n.type=='TEX_IMAGE' and n.image]+[512])
    bake_scene.render.engine='BLENDER_EEVEE';bake_scene.render.resolution_x=size;bake_scene.render.resolution_y=size;bake_scene.render.resolution_percentage=100
    bake_scene.render.image_settings.file_format='PNG';bake_scene.render.image_settings.color_mode='RGBA';bake_scene.render.use_compositing=False
    bake_scene.view_settings.view_transform='Standard';bake_scene.view_settings.look='None';bake_scene.view_settings.exposure=0
    bake_scene.render.filepath=str(path);bpy.ops.render.render(write_still=True)
    bpy.context.window.scene=original_scene
    bpy.data.objects.remove(plane,do_unlink=True);bpy.data.objects.remove(cam,do_unlink=True)
    bpy.data.scenes.remove(bake_scene)
    return {'method':'upstream-color-socket-eevee-raster-v1','resolution':[size,size],'sha256':sha(path)}


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--authoring',required=True,type=Path);p.add_argument('--out',required=True,type=Path)
    args=p.parse_args(sys.argv[sys.argv.index('--')+1:])
    authoring=json.loads(args.authoring.read_text());recipe=authoring['recipe']
    if sha(bpy.data.filepath)!=authoring['blend_sha256']:raise RuntimeError('authoring blend hash mismatch')
    if args.out.exists():raise RuntimeError('output exists; use a new candidate')
    args.out.parent.mkdir(parents=True,exist_ok=True);stage=Path(tempfile.mkdtemp(prefix='.export-',dir=args.out.parent));textures=stage/'textures';textures.mkdir()
    source_objects=[bpy.data.objects[name] for name in recipe['meshes']]
    source_materials={s.material for o in source_objects for s in o.material_slots if s.material}
    mapped={};inputs=[];evidence={}
    for mat in sorted(source_materials,key=lambda m:m.name):
        role=recipe['material_roles'][mat.name]
        group=next(n for n in mat.node_tree.nodes if n.type=='GROUP' and n.node_tree and n.node_tree.name.split('.')[0] in {'HoYoverse - GI Body','HoYoverse - GI Face'})
        # This static profile has explicit reconstruction rules; a changed
        # upstream sampler is an unsupported input, not an implicit approximation.
        seen=set();sampling=[]
        def inspect_sampling(tree):
            if tree.as_pointer() in seen:return
            seen.add(tree.as_pointer())
            for node in tree.nodes:
                if node.type=='GROUP' and node.node_tree:inspect_sampling(node.node_tree)
                elif node.type=='TEX_IMAGE' and node.image:
                    name=node.image.name.lower()
                    expected='Closest' if 'shadow_ramp' in name else ('Cubic' if ('lightmap' in name or name=='avatar_tex_face_shadow.png') else None)
                    if expected:
                        if node.interpolation!=expected:raise RuntimeError(f'unsupported sampling: {mat.name}/{node.name}: {node.interpolation}, expected {expected}')
                        sampling.append({'image':node.image.name,'interpolation':expected})
        inspect_sampling(mat.node_tree)
        evidence[mat.name]={'control_sampling':sampling}
        safe=''.join(c if c.isalnum() else '_' for c in mat.name)
        color_path=textures/(safe+'_color.png');direct=direct_diffuse(mat,group,color_path);evidence[color_path.name]=direct or bake_color(mat,group,color_path)
        runtime=bpy.data.materials.new('runtime_'+safe);runtime.use_nodes=True
        principled=runtime.node_tree.nodes.get('Principled BSDF');principled.inputs['Base Color'].default_value=(1,1,1,1);principled.inputs['Metallic'].default_value=0;principled.inputs['Roughness'].default_value=1
        image=bpy.data.images.load(str(color_path),check_existing=True);tex=runtime.node_tree.nodes.new('ShaderNodeTexImage');tex.image=image;runtime.node_tree.links.new(tex.outputs['Color'],principled.inputs['Base Color'])
        mapped[mat]=runtime
        item={'name':runtime.name,'role':role,'base_color':'textures/'+color_path.name,'cubic_base_color':direct is not None,'ilm':None,'face_sdf':None,'face_shadow':None,'ramp':None,
              'shadow_position':float(recipe['lighting_overrides']['Shadow Position']),'ramp_width':1.0,
              'face_softness':float(group.inputs['Face Shadow Softness'].default_value) if role=='face' else .02,
              'face_forward':[0,0,1],'face_up':[0,1,0],
              'metallic':False,'specular_ramp':False,'packed_normal':False}
        if group.node_tree.name.split('.')[0]=='HoYoverse - GI Body':
            def scalar(name,default):
                socket=group.inputs.get(name)
                if socket is None:return default
                if socket.is_linked:raise RuntimeError(f'unmapped linked lighting input: {name}')
                return float(socket.default_value)
            def color(name):return list(group.inputs[name].default_value)
            item['lighting']={'use_lightmap_ao':scalar('Use Lightmap AO',1),'use_vertex_ao':scalar('Use Vertex Color AO',0),'use_shadow_ramp':scalar('Use Shadow Ramp',1),'lightmap_constant':[0,0,0,1],
                'specular_color':color('Specular Color'),'specular_rows':[],'shadow_rows':[],
                'metal_params':[scalar('Metallic Specular Shininess',10),scalar('Metallic Specular Scale',15),scalar('Metallic Specular Attenuation',0),scalar('Metallic Specular Sharp Layer Offset',.95)],
                'metal_color':color('MT Specular Color'),'metal_sharp_color':color('MT Sharp Layer Color')}
            for i in range(5):
                suffix='' if i==0 else ' '+str(i+1)
                item['lighting']['specular_rows'].append([scalar('Shininess'+suffix,10),scalar('Specular Multiplier'+suffix,.1),1 if i==0 else scalar('Use Material '+str(i+1),1),0])
                item['lighting']['shadow_rows'].append([scalar('Shadow Transition Range'+suffix,.01),scalar('Shadow Transition Softness'+suffix,.5),0,0])
            if role in {'brow','pupil'}:
                lerp=mat.node_tree.nodes.get('Lightmap Lerp');nodes=[link.from_node for link in lerp.inputs[0].links] if lerp else []
                image=nodes[0].image if nodes and nodes[0].type=='TEX_IMAGE' else None
                if image:
                    dest=textures/(safe+'_ilm.png');image.save(filepath=str(dest));item['ilm']='textures/'+dest.name
        if role in {'body','hair'}:
            ilms={n.image.name:n.image for n in mat.node_tree.nodes if n.type=='TEX_IMAGE' and n.image and n.image.name.endswith('_Lightmap.png')}
            if len(ilms)!=1:raise RuntimeError(f'expected one authored ILM image: {mat.name}: {list(ilms)}')
            image=next(iter(ilms.values()));dest=textures/(safe+'_ilm.png');image.save(filepath=str(dest));item['ilm']='textures/'+dest.name
            # Read the actual shader closure; the top-level ramp preview can be disconnected.
            imgs=[];seen=set()
            def collect_ramps(tree):
                if tree.as_pointer() in seen:return
                seen.add(tree.as_pointer())
                for node in tree.nodes:
                    if node.type=='TEX_IMAGE' and node.image and 'shadow_ramp' in node.image.name.lower():imgs.append(node.image)
                    elif node.type=='GROUP' and node.node_tree:collect_ramps(node.node_tree)
            collect_ramps(group.node_tree)
            unique={i.name:i for i in imgs}
            if len(unique)!=1:raise RuntimeError(f'expected one authored ramp for {mat.name}: {list(unique)}')
            dest=textures/(safe+'_ramp.png');next(iter(unique.values())).save(filepath=str(dest));item['ramp']='textures/'+dest.name
        if role=='face':
            item['face_tints']={key:[list(group.inputs[label+('' if i==0 else ' '+str(i+1))].default_value) for i in range(3)] for key,label in [('day','Daytime Shadow Color'),('night','Nighttime Shadow Color')]}
            face_object=next(o for o in source_objects if mat in list(o.data.materials))
            evaluated=face_object.evaluated_get(bpy.context.evaluated_depsgraph_get());face_mesh=evaluated.to_mesh()
            try:
                for attribute,key in [('headForward','face_forward'),('headUp','face_up')]:
                    values=face_mesh.attributes.get(attribute)
                    if not values or not len(values.data):raise RuntimeError(f'missing evaluated face basis: {attribute}')
                    v=values.data[0].vector;item[key]=[v.x,v.z,-v.y]
            finally:evaluated.to_mesh_clear()
            image=bpy.data.images.get('Avatar_Girl_Tex_FaceLightmap.png')
            if not image:raise RuntimeError('locked upstream packed face image missing')
            dest=textures/'face_lightmap.png';image.save(filepath=str(dest));item['face_sdf']='textures/'+dest.name;item['face_shadow']='textures/face_shadow.png'
        if role=='pupil':
            from blender.highlight_handoff import extract_highlight
            item['highlight']=extract_highlight(mat,textures)
        inputs.append(item)
    # Preserve other packed upstream inputs as traceable artifacts, not runtime slot guesses.
    for name,out in [('Avatar_Tex_Face_Shadow.png','face_shadow.png')]:
        image=bpy.data.images.get(name)
        if not image:raise RuntimeError(f'missing packed upstream image: {name}')
        image.save(filepath=str(textures/out));evidence[out]={'method':'locked-upstream-packed-image','source_image':name,'sha256':sha(textures/out)}
    runtime_scene=bpy.data.scenes.new('Runtime')
    for original in source_objects:
        for modifier in list(original.modifiers):
            if modifier.type=='NODES' and modifier.node_group and 'Outlines' in modifier.node_group.name:original.modifiers.remove(modifier)
    bpy.context.view_layer.update();depsgraph=bpy.context.evaluated_depsgraph_get()
    for original in source_objects:
        mesh=bpy.data.meshes.new_from_object(original.evaluated_get(depsgraph),preserve_all_data_layers=True,depsgraph=depsgraph)
        for index,mat in enumerate(list(mesh.materials)):mesh.materials[index]=mapped[mat]
        mesh.transform(original.matrix_world)
        while len(mesh.uv_layers)>2:mesh.uv_layers.remove(mesh.uv_layers[-1])
        obj=bpy.data.objects.new('runtime_'+original.name,mesh);runtime_scene.collection.objects.link(obj)
    bpy.context.window.scene=runtime_scene
    for obj in runtime_scene.objects:obj.select_set(True)
    bpy.context.view_layer.objects.active=next(iter(runtime_scene.objects))
    vertex_enum=bpy.ops.export_scene.gltf.get_rna_type().properties['export_vertex_color'].enum_items.keys()
    if 'ACTIVE' not in vertex_enum:raise RuntimeError('glTF exporter lacks explicit active vertex color export')
    bpy.ops.export_scene.gltf(filepath=str(stage/'character.glb'),export_format='GLB',use_selection=True,use_active_scene=True,
                             export_apply=False,export_yup=True,export_animations=False,export_skins=False,export_morph=False,
                             export_attributes=True,export_extras=True,export_vertex_color='ACTIVE',export_all_vertex_colors=False,
                             export_cameras=False,export_lights=False,will_save_settings=False)
    payload={'schema':'bevy_stylized.genshin-input/v2','variant':recipe['id'],'materials':inputs,'palette':recipe['palette'],'day_night':recipe['lighting_overrides']['Day/Night']}
    (stage/'character.genshin.json').write_text(json.dumps(payload,indent=2)+'\n')
    for path in textures.iterdir():evidence.setdefault(path.name,{'method':'authored-image','sha256':sha(path)})
    (stage/'runtime-manifest.json').write_text(json.dumps({'schema':'bevy_stylized.runtime-export/v1','scope':authoring['scope'],
       'authoring_sha256':sha(args.authoring),'recipe':recipe,'mesh_names':['runtime_'+n for n in recipe['meshes']],
       'textures':evidence,'glb_sha256':sha(stage/'character.glb'),'export_script_sha256':sha(__file__),
       'limitations':['static pose only','metallic/specular-ramp/normal/emission disabled by explicit upstream recipe','effect meshes and upstream outlines excluded by declared scope']},indent=2)+'\n')
    shutil.move(str(stage),str(args.out));print('RUNTIME_CANDIDATE',args.out)


if __name__=='__main__':main()
