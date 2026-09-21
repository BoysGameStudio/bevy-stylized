"""Source-derived hull metadata for the explicit neutral static outline variant."""
import json
from pathlib import Path
import bpy

POLICY = 'genshin-source-hull-v1'


def source_positions(objects):
    """Retain the source GN weld/extrusion without baking a second hull mesh.

    CONTROL0 contains glTF-local target positions for the static hull pass.
    Surface positions/normals/colors are untouched; dynamic deformation is outside
    this recipe. The nearest lookup only transfers the upstream weld identity.
    """
    from mathutils.kdtree import KDTree
    result = {}
    for obj, mod in modifiers(objects):
        original = mod.node_group
        original_visible = mod.show_viewport
        group = original.copy()
        meshes = []
        try:
            mod.show_viewport = False
            bpy.context.view_layer.update()
            dg = bpy.context.evaluated_depsgraph_get()
            base = bpy.data.meshes.new_from_object(obj.evaluated_get(dg), preserve_all_data_layers=True, depsgraph=dg)
            meshes.append(base)
            if base.attributes.get('_NPR_CONTROL0'):
                raise RuntimeError('source already owns CONTROL0; cannot replace its meaning')
            mod.node_group = group
            mod.show_viewport = True
            output = next(n for n in group.nodes if n.type == 'GROUP_OUTPUT')
            for name in ['Merge by Distance', 'Set Position']:
                group.links.new(group.nodes[name].outputs['Geometry'], output.inputs['Geometry'])
                bpy.context.view_layer.update()
                dg = bpy.context.evaluated_depsgraph_get()
                meshes.append(bpy.data.meshes.new_from_object(obj.evaluated_get(dg), preserve_all_data_layers=True, depsgraph=dg))
            merged, extruded = meshes[1:]
            if len(merged.vertices) != len(extruded.vertices):
                raise RuntimeError('outline extrusion changed the source weld topology')
            tree = KDTree(len(merged.vertices))
            for vertex in merged.vertices: tree.insert(vertex.co, vertex.index)
            tree.balance()
            tolerance = group.nodes['Merge by Distance'].inputs['Distance'].default_value + 1e-6
            positions = []
            for vertex in base.vertices:
                _, index, distance = tree.find(vertex.co)
                if distance > tolerance: raise RuntimeError('source outline vertex cannot be traced through weld')
                point = obj.matrix_world @ extruded.vertices[index].co
                positions.append([point.x, point.z, -point.y])
            result[obj.name] = {'base': [list(v.co) for v in base.vertices], 'targets': positions}
        finally:
            mod.node_group = original
            mod.show_viewport = original_visible
            for mesh in meshes: bpy.data.meshes.remove(mesh)
            bpy.data.node_groups.remove(group)
            bpy.context.view_layer.update()
    return result


def modifiers(objects):
    return [(obj, mod) for obj in objects for mod in obj.modifiers
            if mod.type == 'NODES' and mod.node_group
            and mod.node_group.name == 'HoYoverse - Genshin Impact Outlines']


def inputs(mod):
    return {socket.name: getattr(mod.properties.inputs, socket.identifier)
            for socket in mod.node_group.interface.items_tree
            if socket.item_type == 'SOCKET' and socket.in_out == 'INPUT'}


def mappings(objects):
    for obj, mod in modifiers(objects):
        values = inputs(mod)
        for label in ['Hair', 'Body', 'Face', 'Dress', 'Dress 2', 'Other']:
            surface = values[label].value
            outline = values[label + ' Outline'].value
            if surface and surface in list(obj.data.materials):
                if outline is None:
                    raise RuntimeError(f'missing source outline: {obj.name}/{surface.name}')
                yield obj, mod, values, surface, outline


def configure(objects, recipe):
    if recipe.get('outline_policy') != POLICY:
        raise RuntimeError('outline variant requires the explicit source hull policy')
    for obj, mod in modifiers(objects):
        mod.show_render = mod.show_viewport = True
    # Apply the same declared neutral palette restriction as the body/face.
    for _, _, _, _, material in mappings(objects):
        group = material.node_tree.nodes['Outlines']
        values = {'Ambient Colour': recipe['palette']['Ambient Colour'],
                  'Sharp Lit Colour': recipe['palette']['Sharp Lit Colour'],
                  'Outline Tint': [1, 1, 1, 1], 'Outline Saturation': 1.0,
                  'Outline Intensity': 1.0}
        for name, value in values.items():
            socket = group.inputs[name]
            for link in list(socket.links): material.node_tree.links.remove(link)
            socket.default_value = value


def extract(objects, recipe, textures):
    if recipe.get('outline_policy') != POLICY:
        raise RuntimeError('unsupported outline policy')
    result = {}
    evidence = {}
    for obj, mod, values, surface, material in mappings(objects):
        def setting(name): return values[name].value
        if (setting('Screen Space') or setting('Z-Depth Offset') != 0
                or setting('Clamp Min Distance') or setting('Clamp Max Distance')
                or not setting('Base Geometry') or not setting('Use Vertex Colors?')
                or values['Vertex Colors'].type != 'ATTRIBUTE'
                or values['Vertex Colors'].attribute_name != 'Col'):
            raise RuntimeError(f'unsupported outline geometry settings: {obj.name}')
        scale = obj.matrix_world.to_scale()
        if any(abs(v-1) > 1e-5 for v in scale):
            raise RuntimeError('source hull exporter requires unit object scale')
        width_node = mod.node_group.nodes['Math.001']
        if width_node.operation != 'MULTIPLY' or width_node.inputs[1].is_linked:
            raise RuntimeError('source outline width graph changed')
        width = float(setting('Outline Thickness')) * width_node.inputs[1].default_value
        group = material.node_tree.nodes['Outlines']
        if group.node_tree.name != 'HoYoverse - GI Outlines' or not material.use_backface_culling:
            raise RuntimeError('unsupported outline material graph/culling')
        def scalar(name):
            socket = group.inputs[name]
            if socket.is_linked: raise RuntimeError(f'linked outline input not resolved: {name}')
            return float(socket.default_value)
        if scalar('Use Alpha') != 0:
            raise RuntimeError('alpha-cutout source outlines are not part of this variant')
        for name in ['Ambient Colour', 'Sharp Lit Colour', 'Outline Tint']:
            socket = group.inputs[name]
            if socket.is_linked or any(abs(v-1) > 1e-6 for v in socket.default_value):
                raise RuntimeError('outline variant requires the declared neutral lighting/tint')
        if scalar('Outline Saturation') != 1 or scalar('Outline Intensity') != 1:
            raise RuntimeError('unsupported outline color modification')
        gamma = group.node_tree.nodes['Group.032'].inputs['Gamma']
        if gamma.is_linked: raise RuntimeError('linked outline gamma')
        force = scalar('Use Face Shader')
        enabled = [scalar('Use Material '+str(i)) for i in range(2, 6)]
        if any(v not in [0, 1] for v in [force, *enabled]):
            raise RuntimeError('outline material switches must be binary')
        colors = []
        for i in range(1, 6):
            socket = group.inputs['Outline Color '+str(i)]
            if socket.is_linked: raise RuntimeError('linked source outline color')
            colors.append(list(socket.default_value))
        texture = None
        if not force:
            node = material.node_tree.nodes['Outline_Lightmap']
            if not node.image or node.interpolation != 'Cubic' or node.extension != 'REPEAT':
                raise RuntimeError('outline class map requires authored cubic/repeat sampling')
            name = ''.join(c if c.isalnum() else '_' for c in surface.name) + '_outline_class.png'
            node.image.save(filepath=str(textures/name))
            texture = 'textures/'+name
        item = {'width': width, 'colors': colors,
                'enabled_mask': sum(int(v) << i for i, v in enumerate(enabled)),
                'force_first': bool(force), 'gamma': float(gamma.default_value),
                'class_texture': texture, 'prepared_position': True}
        if surface.name in result and result[surface.name] != item:
            raise RuntimeError('one material has conflicting per-object outline settings')
        result[surface.name] = item
        evidence.setdefault(surface.name, []).append({'object': obj.name,
            'modifier': mod.name, 'source_material': material.name,
            'source_thickness': setting('Outline Thickness'),
            'width_multiplier': width_node.inputs[1].default_value,
            'vertex_width': 'Col alpha', 'class_sampling': 'cubic/repeat',
            'source_merge_distance': mod.node_group.nodes['Merge by Distance'].inputs['Distance'].default_value})
    if not result: raise RuntimeError('requested outline variant has no source hulls')
    return result, evidence
