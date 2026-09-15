"""Blender read-only audit. Use: blender -b file.blend --python-exit-code 1 --python THIS -- --out NEW.json"""
import argparse
import hashlib
import json
import math
import sys
from pathlib import Path
import bpy


def value(x):
    if isinstance(x, (str, bool, int)) or x is None:
        return x
    if isinstance(x, float):
        return x if math.isfinite(x) else str(x)
    try:
        return [value(y) for y in x]
    except TypeError:
        return str(x)


def audit():
    try:
        import setup_wizard
        addon = {'version': list(setup_wizard.bl_info['version']), 'path': setup_wizard.__file__}
    except ImportError:
        addon = None
    images = []
    for img in bpy.data.images:
        if img.source not in {'FILE', 'TILED', 'SEQUENCE'}:
            continue
        packed = bool(img.packed_file or img.packed_files)
        resolved = bpy.path.abspath(img.filepath, library=img.library)
        images.append({'name': img.name, 'path': img.filepath, 'resolved': resolved,
                       'packed': packed, 'source': img.source, 'users': img.users,
                       'colorspace': img.colorspace_settings.name,
                       'exists': packed or Path(resolved).is_file()})
    meshes, armatures, constraints, drivers = [], [], [], []
    for obj in bpy.data.objects:
        if obj.type == 'MESH':
            meshes.append({'name': obj.name, 'vertices': len(obj.data.vertices),
                           'materials': [s.material.name if s.material else None for s in obj.material_slots],
                           'colors': [{'name': a.name, 'domain': a.domain, 'type': a.data_type} for a in obj.data.color_attributes],
                           'npr_attributes': [a.name for a in obj.data.attributes if a.name.startswith(('_NPR_', '_CEL_'))],
                           'modifiers': [{'name': m.name, 'type': m.type, 'render': m.show_render} for m in obj.modifiers],
                           'scale': list(obj.scale), 'render_hidden': obj.hide_render,
                           'shape_keys': len(obj.data.shape_keys.key_blocks) if obj.data.shape_keys else 0})
        if obj.type == 'ARMATURE':
            armatures.append({'name': obj.name, 'bones': len(obj.data.bones),
                              'deform_bones': sum(b.use_deform for b in obj.data.bones)})
        owners = [(obj.name, obj.constraints)]
        if obj.pose:
            owners += [(f'{obj.name}/{p.name}', p.constraints) for p in obj.pose.bones]
        for owner, cs in owners:
            for c in cs:
                target = getattr(c, 'target', None)
                subtarget = getattr(c, 'subtarget', '')
                bad_bone = bool(target and target.type == 'ARMATURE' and subtarget and subtarget not in target.data.bones)
                if bad_bone or not c.is_valid:
                    constraints.append({'owner': owner, 'constraint': c.name, 'type': c.type,
                                        'target': target.name if target else None, 'subtarget': subtarget,
                                        'missing_bone': bad_bone, 'valid': c.is_valid})
    materials = []
    for mat in bpy.data.materials:
        if not mat.node_tree:
            continue
        groups = []
        for node in mat.node_tree.nodes:
            if node.type == 'GROUP' and node.node_tree:
                groups.append({'node': node.name, 'group': node.node_tree.name,
                               'inputs': [{'name': s.name, 'linked': s.is_linked,
                                           'value': value(s.default_value) if hasattr(s, 'default_value') else None}
                                          for s in node.inputs]})
        materials.append({'name': mat.name, 'users': mat.users, 'groups': groups,
                          'images': [n.image.name for n in mat.node_tree.nodes if n.type == 'TEX_IMAGE' and n.image]})
    blocks = list(bpy.data.objects) + list(bpy.data.node_groups) + list(bpy.data.shape_keys)
    blocks += [m.node_tree for m in bpy.data.materials if m.node_tree]
    for block in blocks:
        anim = block.animation_data
        if anim:
            for curve in anim.drivers:
                if not curve.driver.is_valid:
                    drivers.append({'owner': block.name, 'data_path': curve.data_path, 'index': curve.array_index})
    scene = bpy.context.scene
    source = Path(bpy.data.filepath)
    return {'scope': 'inspection, not a visual acceptance gate; linked socket defaults are not evaluated values',
            'blend': str(source), 'sha256': hashlib.sha256(source.read_bytes()).hexdigest() if source.is_file() else None,
            'blender': bpy.app.version_string, 'addon': addon,
            'scene': {'engine': scene.render.engine, 'view_transform': scene.view_settings.view_transform,
                      'look': scene.view_settings.look, 'exposure': scene.view_settings.exposure,
                      'frame': scene.frame_current, 'unit_scale': scene.unit_settings.scale_length},
            'meshes': meshes, 'armatures': armatures, 'materials': materials, 'images': images,
            'invalid_constraints': constraints, 'invalid_drivers': drivers,
            'libraries': [x.filepath for x in bpy.data.libraries]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    report = audit()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open('x') as stream:
        json.dump(report, stream, indent=2, ensure_ascii=False, allow_nan=False)
    print('BLEND_AUDIT', args.out, 'missing_images', sum(not i['exists'] for i in report['images']),
          'invalid_constraints', len(report['invalid_constraints']), 'invalid_drivers', len(report['invalid_drivers']))


if __name__ == '__main__':
    main()
