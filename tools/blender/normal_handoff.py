"""Trace the locked optimized normal graph and its front/back image inputs."""
import hashlib


def extract(material, group, objects, safe, textures):
    socket = group.inputs['Normal Map']
    if len(socket.links) != 1:
        raise RuntimeError('enabled normal map requires an explicit source image path')
    lerp = socket.links[0].from_node
    if lerp.type != 'GROUP' or lerp.node_tree.name.split('.')[0] != 'UV Lerp':
        raise RuntimeError('unsupported authored normal UV selection')
    nodes = [link.from_node for item in lerp.inputs for link in item.links]
    if len(nodes) != 2 or any(n.type != 'TEX_IMAGE' or not n.image for n in nodes):
        raise RuntimeError('normal UV lerp must have two image inputs')
    if len({n.image.as_pointer() for n in nodes}) != 1:
        raise RuntimeError('different front/back normal textures require a separate adapter')
    image = nodes[0].image
    if image.colorspace_settings.name != 'Non-Color' or any(
            n.interpolation != 'Cubic' or n.extension != 'REPEAT' for n in nodes):
        raise RuntimeError('source normals require cubic/repeat Non-Color sampling')
    for obj in objects:
        if material not in list(obj.data.materials): continue
        if len(obj.data.uv_layers) < 2 or [u.name for u in obj.data.uv_layers[:2]] != ['UV0', 'UV1']:
            raise RuntimeError('normal variant requires the original UV0/UV1 pair')
        if not obj.data.uv_layers[0].active_render:
            raise RuntimeError('optimized normal basis requires active render UV0')
    basis = group.node_tree.nodes['Shading Normal / NdotH'].node_tree.nodes['Group.001'].node_tree
    if basis.name.split('.')[0] != 'Optimized Normal Map' or basis.nodes['UV Map'].uv_map:
        raise RuntimeError('unsupported normal basis graph or named UV override')
    for name, distance in [('Tangent', 1.0), ('Bi-Tangent', 1000.0)]:
        node = basis.nodes[name]
        if (node.type != 'BUMP' or not node.invert or node.inputs['Strength'].is_linked
                or node.inputs['Strength'].default_value != 1
                or node.inputs['Distance'].is_linked or node.inputs['Distance'].default_value != distance
                or node.inputs['Normal'].is_linked):
            raise RuntimeError(f'unsupported source bump basis: {name}')
    path = textures/(safe+'_normal.png')
    image.save(filepath=str(path))
    return {'texture':'textures/'+path.name, 'method':'genshin-optimized-bump/v1',
            'strength':float(group.inputs['Use Normal Map'].default_value)}, {
        path.name:{'method':'authored-optimized-normal/v1','source_image':image.name,
                   'sampling':'Cubic','extension':'REPEAT','colorspace':'Non-Color',
                   'texture_uvs':['UV0 front','UV1 back'],'basis_uv':'UV0',
                   'z_reconstruction':'sqrt(max(1-x*x+y*y,0))',
                   'bump_distances':[1,1000],'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}}
