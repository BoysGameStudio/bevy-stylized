"""Bind named ramps on the render-connected shader closure, not only UI previews."""
def bind_authored_ramps(source,manifest,recipe):
    import bpy
    from assets.material_sources import ramp_bindings
    evidence=ramp_bindings(source,manifest,recipe)
    def is_ramp(node):
        return node.type=='TEX_IMAGE' and node.image and 'shadow_ramp' in node.image.name.lower()
    for name,binding in evidence.items():
        material=bpy.data.materials[name]
        image=bpy.data.images.load(str(source/binding['texture']),check_existing=True);image.colorspace_settings.name='sRGB'
        contains_cache={};copies={};replaced=[]
        def contains(tree):
            key=tree.as_pointer()
            if key not in contains_cache:
                contains_cache[key]=False
                contains_cache[key]=any(is_ramp(n) or (n.type=='GROUP' and n.node_tree and contains(n.node_tree)) for n in tree.nodes)
            return contains_cache[key]
        def clone(tree):
            if not contains(tree):return tree
            key=tree.as_pointer()
            if key in copies:return copies[key]
            candidate=tree.copy();copies[key]=candidate
            for node in candidate.nodes:
                if is_ramp(node):node.image=image;replaced.append(candidate.name+'/'+node.name)
                elif node.type=='GROUP' and node.node_tree:node.node_tree=clone(node.node_tree)
            return candidate
        changed_shaders=[]
        for node in material.node_tree.nodes:
            if node.type=='GROUP' and node.node_tree:
                original=node.node_tree;node.node_tree=clone(original)
                if original!=node.node_tree and original.name.split('.')[0]=='HoYoverse - GI Body':changed_shaders.append(node.name)
        if not changed_shaders or not replaced:raise RuntimeError(f'no render-connected ramp binding: {name}')
        binding.update(method='per-material shader-closure copy; original JSON texture name',shader_nodes=changed_shaders,bound_image_nodes=replaced)
    return evidence
