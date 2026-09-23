"""Bisect the effect-panel brightness: render full_body-front with the source
scene, toggling effect material features. Reports the skirt-band mean color.

Usage: blender --background <blend> --python tools/blender/effect_bisect.py -- <mode>
mode: stock | no-normal | no-ao | no-ramp | emission-diffuse
"""
import sys
from pathlib import Path
import bpy
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))


def main():
    mode = sys.argv[sys.argv.index('--') + 1]
    out_png = Path(f'/tmp/eff_bisect_{mode}.png')
    scene = bpy.context.scene
    for obj in scene.objects:
        if obj.type == 'MESH':
            obj.hide_render = obj.name not in {'Body', 'Eff_001', 'Eff_002'}
    mats = [bpy.data.materials['HoYoverse - Odette 01'], bpy.data.materials['HoYoverse - Odette 02']]
    groups = [next(n for n in m.node_tree.nodes if n.type == 'GROUP' and n.node_tree
                   and n.node_tree.name.split('.')[0] == 'HoYoverse - GI Body') for m in mats]
    if mode == 'no-normal':
        for g in groups:
            g.inputs['Use Normal Map'].default_value = 0.0
    elif mode == 'no-ao':
        for g in groups:
            g.inputs['Use Lightmap AO'].default_value = 0.0
            g.inputs['Use Vertex Color AO'].default_value = 0.0
    elif mode == 'no-ramp':
        for g in groups:
            g.inputs['Use Shadow Ramp'].default_value = 0.0
    elif mode == 'emission-diffuse':
        # pure diffuse reference: what the base texture alone looks like
        for m, g in zip(mats, groups):
            out = next(n for n in m.node_tree.nodes if n.type == 'OUTPUT_MATERIAL')
            em = m.node_tree.nodes.new('ShaderNodeEmission')
            sock = g.inputs['Diffuse Color']
            if sock.is_linked:
                m.node_tree.links.new(sock.links[0].from_socket, em.inputs['Color'])
            for link in list(out.inputs['Surface'].links):
                m.node_tree.links.remove(link)
            m.node_tree.links.new(em.outputs[0], out.inputs['Surface'])
    elif mode != 'stock':
        raise RuntimeError(f'unknown mode {mode}')
    # light proxy per the case: direction_to_light [0,0,1] runtime -> Blender (0,-1,0)
    from mathutils import Vector
    bpy.data.objects['Light Direction'].rotation_euler = Vector((0, 0, -1)).rotation_difference(Vector((0, -1, 0))).to_euler('XYZ')
    bpy.context.view_layer.update()
    scene.render.engine = 'BLENDER_EEVEE'
    scene.eevee.taa_render_samples = 1
    scene.render.filter_size = .01
    scene.render.dither_intensity = 0
    scene.render.resolution_x = scene.render.resolution_y = 512
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = False
    scene.render.use_compositing = False
    scene.view_settings.view_transform = 'Standard'
    scene.view_settings.look = 'None'
    world = bpy.data.worlds.new('W')
    world.use_nodes = True
    world.node_tree.nodes['Background'].inputs['Color'].default_value = (0.12, 0.12, 0.12, 1)
    scene.world = world
    cd = bpy.data.cameras.new('C')
    cam = bpy.data.objects.new('C', cd)
    scene.collection.objects.link(cam)
    scene.camera = cam
    cd.type = 'ORTHO'
    # case full_body-front: position [-0.00039,0.799,5.083] target z=-0.062 (runtime y-up)
    conv = lambda v: Vector((v[0], -v[2], v[1]))
    pos = conv([-0.00039289891719818115, 0.7989959120750427, 5.083198070526123])
    tgt = conv([-0.00039289891719818115, 0.7989959120750427, -0.06217893958091736])
    cam.location = pos
    cam.rotation_euler = (tgt - pos).to_track_quat('-Z', 'Y').to_euler()
    cd.ortho_scale = 2.0581508830189703
    scene.render.filepath = str(out_png)
    scene.render.image_settings.file_format = 'PNG'
    bpy.ops.render.render(write_still=True)
    a = np.array(bpy.data.images.load(str(out_png)).pixels[:]).reshape(512, 512, 4)[..., :3]
    band = a[209:321, 165:347]
    print(f'BISECT {mode}: skirt-band mean rgb = {band.mean(axis=(0,1)).round(4)}')


main()
