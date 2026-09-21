"""Preserve the source Alpha group's emissive closure and reject other modes."""
import math
from blender.metal_handoff import switch


def extract(group, direct):
    if not switch(group, 'Use Alpha'):
        return None
    if switch(group, '0 = Emission, 1 = Transparency'):
        raise RuntimeError('source transparency requires its own surface and outline adapter')
    if not direct:
        raise RuntimeError('source emission requires an unbaked diffuse alpha channel')
    # Color and alpha must travel through the same image/UV selection. A color
    # bake alone cannot preserve an independently authored emission mask.
    color = group.inputs['Diffuse Color']
    alpha = group.inputs['Diffuse Alpha']
    if (len(alpha.links) != 1 or alpha.links[0].from_node != color.links[0].from_node
            or alpha.links[0].from_socket.name != 'Alpha'):
        raise RuntimeError('unsupported independent diffuse alpha path')
    values = {}
    for name, key in [('Emission Tint', 'tint'), ('Emission Strength', 'strength')]:
        socket = group.inputs[name]
        if socket.is_linked:
            raise RuntimeError(f'unresolved emission input: {name}')
        values[key] = list(socket.default_value) if key == 'tint' else float(socket.default_value)
    if any(not math.isfinite(v) or v < 0 for v in [*values['tint'], values['strength']]):
        raise RuntimeError('invalid source emission input')
    values['method'] = 'genshin-alpha-emission/v1'
    return values
