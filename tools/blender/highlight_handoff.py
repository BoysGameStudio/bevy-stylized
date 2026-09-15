"""Extract the verified UV-only Highlight closure outside the body shader."""
def extract_highlight(material,output_dir):
    output=next(n for n in material.node_tree.nodes if n.type=='OUTPUT_MATERIAL')
    mix=output.inputs['Surface'].links[0].from_node
    if mix.type!='MIX_SHADER':return None
    highlight=mix.inputs[2].links[0].from_node
    if highlight.type!='GROUP' or highlight.node_tree.name.split('.')[0]!='Highlight':return None
    alpha=mix.inputs[0].links[0].from_node
    if alpha.bl_idname!='ShaderNodeShaderToRGB' or alpha.inputs[0].links[0].from_node!=highlight:raise RuntimeError('unverified highlight alpha composition')
    image_node=highlight.inputs['Color'].links[0].from_node
    if image_node.type!='TEX_IMAGE' or not image_node.image or image_node.inputs['Vector'].is_linked:raise RuntimeError('highlight must use the primary UV texture')
    g=highlight.node_tree
    def edge(name):
        ramp=g.nodes[name].color_ramp
        if ramp.interpolation!='LINEAR' or len(ramp.elements)!=2:raise RuntimeError('unsupported highlight curve')
        if tuple(ramp.elements[0].color)!=(0,0,0,1) or tuple(ramp.elements[1].color)!=(1,1,1,1):raise RuntimeError('unsupported highlight curve colors')
        return [e.position for e in ramp.elements]
    first=edge('Color Ramp');second=edge('Color Ramp.001');opacity=edge('Color Ramp.002')
    if first[0]!=0 or opacity[0]!=0:raise RuntimeError('nonzero highlight curve origin')
    for name,kind in [('Mix','MULTIPLY'),('Mix.001','MULTIPLY'),('Mix.002','ADD')]:
        if g.nodes[name].blend_type!=kind:raise RuntimeError('unsupported highlight composition')
    image=image_node.image;dest=output_dir/'pupil_highlight.png';image.save(filepath=str(dest))
    primary=list(g.nodes['Mix.001'].inputs[7].default_value);secondary=list(g.nodes['Mix'].inputs[7].default_value);primary[3]=first[1];secondary[3]=second[0]
    return {'texture':'textures/'+dest.name,'primary':primary,'secondary':secondary,'curve':[second[1],opacity[1],0,0]}
