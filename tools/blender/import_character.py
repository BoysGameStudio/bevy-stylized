"""Import a hash-verified Genshin source into a separate static-reference candidate.

The explicit static recipe preserves the imported rest skeleton (no Rigify or
secondary physics). It does not qualify the upstream full-rig workflow.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sys
import tempfile
import bpy
import setup_wizard

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
from assets.prepare import verify


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source',required=True,type=Path)
    p.add_argument('--manifest',required=True,type=Path)
    p.add_argument('--recipe',required=True,type=Path)
    p.add_argument('--out',required=True,type=Path)
    args=p.parse_args(sys.argv[sys.argv.index('--')+1:])
    if not bpy.app.background:
        raise RuntimeError('import task requires an isolated background process')
    if args.out.exists():
        raise RuntimeError('candidate output already exists; choose a new output directory')
    recipe=json.loads(args.recipe.read_text());manifest=json.loads(args.manifest.read_text())
    if recipe.get('ramp_policy')!='material-json-shader-closure-v1':
        raise RuntimeError('current importer requires the explicit v4 render-connected ramp policy; older recipes are historical')
    schema=json.loads((TOOLS.parent/'schemas/authoring/static-recipe.schema.json').read_text())
    if any(key not in recipe for key in schema['required']):raise RuntimeError('recipe required fields missing')
    for key,rule in schema['properties'].items():
        if 'const' in rule and recipe.get(key)!=rule['const']:raise RuntimeError(f'unsupported recipe setting: {key}')
    if not recipe['id'] or not recipe['meshes'] or len(set(recipe['meshes']))!=len(recipe['meshes']):raise RuntimeError('empty/duplicate recipe selection')
    verify(args.source,manifest)
    if bpy.app.version_string != recipe['blender'] or list(setup_wizard.bl_info['version']) != recipe['gacha_version']:
        raise RuntimeError('Blender/Gacha version differs from the explicit recipe')
    addon=Path(setup_wizard.__file__).parent
    for rel,expected in recipe['addon_files'].items():
        if not (addon/rel).resolve().is_relative_to(addon.resolve()):raise RuntimeError('addon manifest path escape')
        if hashlib.sha256((addon/rel).read_bytes()).hexdigest()!=expected:
            raise RuntimeError(f'upstream file changed: {rel}')
    # Local authoring-only compatibility shim for a confirmed upstream Python
    # name-mangling error. Missing _MainTexAlphaUse retains the shader default.
    # No global installation or source file is changed.
    from setup_wizard.material_data_import_setup.material_data_applier import MaterialDataApplier, V2_MaterialDataApplier
    setattr(V2_MaterialDataApplier, '_V2_MaterialDataApplier__handle_material_value_not_found',
            getattr(MaterialDataApplier, '_MaterialDataApplier__handle_material_value_not_found'))
    for obj in list(bpy.data.objects):bpy.data.objects.remove(obj,do_unlink=True)
    bpy.context.scene.game_type_dropdown='GENSHIN_IMPACT'
    props=bpy.context.scene.character_rigger_props
    props.disable_rigging=True
    props.enable_hair_clothes_physics=False
    bpy.context.window_manager.setup_wizard_full_run_rigging_enabled=False
    fbx=[f['path'] for f in manifest['files'] if f['path'].lower().endswith('.fbx')]
    if len(fbx)!=1:raise RuntimeError('one character FBX required')
    result=bpy.ops.genshin.import_model('EXEC_DEFAULT',filepath=str(args.source/fbx[0]),
              file_directory=str(args.source),next_step_idx=1,invoker_type='invoke_next_step_ui',
              high_level_step_name='genshin.setup_wizard_ui',game_type='GENSHIN_IMPACT')
    if result!={'FINISHED'}:raise RuntimeError(f'import failed: {result}')
    selected=[bpy.data.objects.get(n) for n in recipe['meshes']]
    if any(o is None or o.type!='MESH' for o in selected):raise RuntimeError('recipe mesh missing')
    for obj in selected:
        for modifier in obj.modifiers:
            if modifier.type=='NODES' and modifier.node_group and 'Outlines' in modifier.node_group.name:
                modifier.show_render=recipe['outlines'];modifier.show_viewport=recipe['outlines']
    # This is a named, restricted portable-reference variant, not the full
    # original material. Apply exactly the same declared restrictions upstream.
    materials={slot.material for obj in selected for slot in obj.material_slots if slot.material}
    if {m.name for m in materials} != set(recipe['material_roles']):
        raise RuntimeError('selected material roles differ from recipe')
    seen_overrides=set()
    for material in materials:
        for node in material.node_tree.nodes:
            if node.type!='GROUP' or not node.node_tree or node.node_tree.name not in {'HoYoverse - GI Body','HoYoverse - GI Face'}:
                continue
            values={**recipe['material_overrides'],**recipe['palette'],**recipe['lighting_overrides']}
            for name,value in values.items():
                socket=node.inputs.get(name)
                if socket is None:continue
                for link in list(socket.links):material.node_tree.links.remove(link)
                socket.default_value=value
                seen_overrides.add(name)
    requested=set(recipe['material_overrides'])|set(recipe['palette'])|set(recipe['lighting_overrides'])
    if requested-seen_overrides:raise RuntimeError(f'unmapped recipe overrides: {requested-seen_overrides}')
    binding_evidence={}
    if recipe.get('ramp_policy')=='material-json-shader-closure-v1':
        from blender.ramp_bindings import bind_authored_ramps
        binding_evidence=bind_authored_ramps(args.source,manifest,recipe)
    bpy.context.scene.frame_set(recipe['frame'])
    bpy.ops.file.pack_all()
    missing=[i.name for i in bpy.data.images if i.source=='FILE' and i.users and not i.packed_file and not i.packed_files and not Path(bpy.path.abspath(i.filepath)).is_file()]
    if missing:raise RuntimeError(f'missing textures: {missing}')
    args.out.parent.mkdir(parents=True,exist_ok=True)
    stage=Path(tempfile.mkdtemp(prefix='.import-',dir=args.out.parent))
    bpy.ops.wm.save_as_mainfile(filepath=str(stage/'character.blend'))
    provenance={'schema':'bevy_stylized.authoring/v1','source_manifest':manifest,
                'recipe':recipe,'recipe_sha256':hashlib.sha256(args.recipe.read_bytes()).hexdigest(),
                'import_script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'blender':bpy.app.version_string,'gacha_version':list(setup_wizard.bl_info['version']),
                'scope':'static-body-face; full original source retained, no full-rig/effect qualification',
                'compatibility_shims':['material-applier-private-method-v1'],
                'ramp_bindings':binding_evidence,
                'blend_sha256':hashlib.sha256((stage/'character.blend').read_bytes()).hexdigest()}
    (stage/'authoring.json').write_text(json.dumps(provenance,indent=2)+'\n')
    shutil.move(str(stage),str(args.out))
    print('AUTHORING_CANDIDATE',args.out,flush=True)


if __name__=='__main__':
    main()
