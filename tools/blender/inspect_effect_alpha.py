"""Alpha-path wiring inside GI Body + UV layer forensics for the Eff meshes."""
import sys
from pathlib import Path
import bpy

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))


def fmt(s):
    if s.is_linked:
        l = s.links[0]
        return f'{l.from_node.name}.{l.from_socket.name}'
    v = getattr(s, 'default_value', None)
    if hasattr(v, '__len__'):
        return '[' + ','.join(f'{x:.4g}' for x in v) + ']'
    return repr(v)


def main():
    out = []
    body = bpy.data.node_groups.get('HoYoverse - GI Body')
    out.append('=== GI Body: Genshin Alpha / Alpha instances ===')
    for node in body.nodes:
        if node.type == 'GROUP' and node.node_tree and 'Alpha' in node.node_tree.name:
            out.append(f'  {node.name} group={node.node_tree.name}')
            for inp in node.inputs:
                if inp.enabled:
                    out.append(f'    in {inp.name} = {fmt(inp)}')
    out.append('=== GI Body outputs wiring (BSDF source) ===')
    for node in body.nodes:
        if node.type == 'GROUP_OUTPUT':
            for inp in node.inputs:
                out.append(f'  out {inp.name} <- {fmt(inp)}')
    for mat_name in ['HoYoverse - Odette 01', 'HoYoverse - Odette 02']:
        mat = bpy.data.materials.get(mat_name)
        out.append(f'=== {mat_name} UV Map nodes ===')
        for node in mat.node_tree.nodes:
            if node.type == 'UVMAP':
                out.append(f'  {node.name}: uv_map={node.uv_map!r}')
    for obj_name in ['Eff_001', 'Eff_002']:
        obj = bpy.data.objects[obj_name]
        mesh = obj.data
        out.append(f'=== {obj_name} UV layers: {[u.name for u in mesh.uv_layers]} active={mesh.uv_layers.active.name if mesh.uv_layers.active else None} render={next((u.name for u in mesh.uv_layers if u.active_render), None)}')
        out.append(f'    material slots: {[s.material.name if s.material else None for s in obj.material_slots]}')
        out.append(f'    blend materials surface render method: {[m.surface_render_method for m in [s.material for s in obj.material_slots if s.material]]}')
    print('\n'.join(out))


main()
