"""Read the locked source metal branch without substituting generic PBR values."""
import hashlib


def switch(group, name):
    socket = group.inputs.get(name)
    if socket is None: return False
    if socket.is_linked or socket.default_value not in (0, 1):
        raise RuntimeError(f'unresolved/nonbinary feature switch: {name}')
    return bool(socket.default_value)


def extract(group, safe, textures, use_ramp):
    def scalar(name):
        socket = group.inputs[name]
        if socket.is_linked: raise RuntimeError(f'linked metal input: {name}')
        return float(socket.default_value)
    def color(name):
        socket = group.inputs[name]
        if socket.is_linked: raise RuntimeError(f'linked metal color: {name}')
        return list(socket.default_value)
    metallic = group.node_tree.nodes['Metallic'].node_tree
    specular = group.node_tree.nodes['Specular'].node_tree
    specular_node = group.node_tree.nodes['Specular']
    for name in ['Global Specular Scale', 'Metallic Specular Intensity', 'Non-Metallic Specular Intensity']:
        socket = specular_node.inputs[name]
        if socket.is_linked or socket.default_value != 1:
            raise RuntimeError(f'unsupported source specular modifier: {name}')
    evidence = {}
    def image(node, suffix, sampling, colorspace):
        if not node.image or node.interpolation != sampling or node.extension != 'REPEAT':
            raise RuntimeError(f'unsupported metal texture sampling: {node.name}')
        if node.image.colorspace_settings.name != colorspace:
            raise RuntimeError(f'unsupported metal texture encoding: {node.image.name}')
        path = textures/(safe+suffix+'.png')
        node.image.save(filepath=str(path))
        evidence[path.name] = {'method':'locked-source-metal-image','source_image':node.image.name,
            'sampling':sampling,'extension':'REPEAT','colorspace':colorspace,
            'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
        return 'textures/'+path.name
    matcap = image(metallic.nodes['MetalMap'], '_metal_matcap', 'Cubic', 'sRGB')
    # The source's Mix factor is scalar: RGB is converted to Rec.709 luminance,
    # not used independently per channel. A native RGB-basis render verifies it.
    evidence[safe+'_metal_matcap.png']['factor_conversion'] = 'linear Rec.709 luminance'
    ramp = image(specular.nodes['Image Texture'], '_metal_specular_ramp', 'Linear', 'Non-Color') if use_ramp else None
    return {'matcap':matcap,'specular_ramp':ramp,
        'brightness':scalar('Metallic Matcap Brightness'),'tile_scale':scalar('Metallic Matcap Tile Scale'),
        'dark':color('Metallic Matcap Dark Color'),'light':color('Metallic Matcap Light Color'),
        'shadow':color('Metallic Matcap Shadow Multiply Color'),
        'specular':[scalar('Metallic Specular Shininess'),scalar('Metallic Specular Scale'),
                    scalar('Metallic Specular Attenuation'),scalar('Metallic Specular Sharp Layer Offset')],
        'specular_color':color('MT Specular Color'),'sharp_color':color('MT Sharp Layer Color')}, evidence
