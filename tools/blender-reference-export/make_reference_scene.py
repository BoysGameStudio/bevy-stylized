#!/usr/bin/env python3
"""Build the self-made bevy-stylized reference fixture scene (deterministic).

Runs inside Blender background mode:

    blender --background --factory-startup \
        --python tools/blender-reference-export/make_reference_scene.py -- \
        --out fixtures/reference/reference_scene.blend

The scene is 100% self-made (no upstream-derived content) so the resulting
.blend and any GLB exported from it are committable to the public repo
(docs/reference/05-license-and-provenance-boundary.md).

Contents:
- fixed camera (position/track target/50 mm lens), fixed sun key light,
  fixed world ambient, fixed 512x512 resolution, EEVEE / sRGB / Standard
  (locked per docs/reference/reference-toolchain.toml);
- self-made fixture geometry: body sphere, head proxy sphere, hair-card
  sheet, ground plane;
- the full `_NPR_*` attribute set from contracts/npr-attributes.v1.json
  applied with contract-correct domains/data types, so the export bridge
  exercises the real attribute checks.

Determinism: pure analytic geometry, no RNG, no time dependence.
"""

import argparse
import math
import os
import sys

import bpy
from mathutils import Vector

SCENE_VERSION = "reference_scene_v2"

# Fixed scene constants (do not change without bumping SCENE_VERSION and
# re-capturing goldens).
CAMERA_LOCATION = (0.0, -4.2, 1.1)
CAMERA_TARGET = (0.0, 0.0, 0.9)
CAMERA_LENS_MM = 50.0
SUN_DIRECTION_FROM = (2.0, -3.0, 4.0)  # sun sits here, points at origin
SUN_ENERGY = 3.0
SUN_COLOR = (1.0, 0.956, 0.898)
WORLD_COLOR = (0.03, 0.03, 0.045)
RESOLUTION = 512


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    p = argparse.ArgumentParser(description="Build reference fixture scene")
    p.add_argument("--out", required=True, help="Output .blend path")
    return p.parse_args(argv)


def track_to(obj, target):
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat("-Z", "Y").to_euler()


def add_float_attr(mesh, name, values):
    attr = mesh.attributes.new(name=name, type="FLOAT", domain="POINT")
    attr.data.foreach_set("value", values)


def add_vector_attr(mesh, name, vectors):
    attr = mesh.attributes.new(name=name, type="FLOAT_VECTOR", domain="POINT")
    flat = [c for v in vectors for c in v]
    attr.data.foreach_set("vector", flat)


def principled_material(name, base_color, roughness=0.6, metallic=0.0):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = (*base_color, 1.0)
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    return mat


def make_body(mat):
    bpy.ops.mesh.primitive_uv_sphere_add(radius=0.9, segments=48, ring_count=32,
                                         location=(0.0, 0.0, 0.0))
    obj = bpy.context.active_object
    obj.name = "FixtureBody"
    mesh = obj.data
    mesh.name = "FixtureBody"
    n = len(mesh.vertices)
    zs = [v.co.z for v in mesh.vertices]
    # Cloth pin weight grows toward the bottom hemisphere (deterministic).
    add_float_attr(mesh, "_NPR_CLOTH_PIN", [max(0.0, -z / 0.9) for z in zs])
    add_float_attr(mesh, "_NPR_EMISSION", [0.0] * n)
    add_float_attr(mesh, "_NPR_MATERIAL_ID", [1.0] * n)
    add_float_attr(mesh, "_NPR_MASK", [1.0] * n)
    # Control vector: normalized position (object-space control field).
    add_vector_attr(mesh, "_NPR_CONTROL0",
                    [(v.co.x / 0.9, v.co.y / 0.9, v.co.z / 0.9) for v in mesh.vertices])
    mesh.materials.append(mat)
    return obj


def make_head(mat):
    bpy.ops.mesh.primitive_uv_sphere_add(radius=0.55, segments=48, ring_count=32,
                                         location=(0.0, 0.0, 1.45))
    obj = bpy.context.active_object
    obj.name = "FixtureHead"
    obj.scale = (0.92, 0.85, 1.05)
    mesh = obj.data
    mesh.name = "FixtureHead"
    n = len(mesh.vertices)
    # Face SDF proxy: 1.0 at the face front (-Y), falling off toward the back.
    sdf = [max(0.0, min(1.0, 0.5 - 0.5 * (v.co.y / 0.55))) for v in mesh.vertices]
    add_float_attr(mesh, "_NPR_FACE_SDF", sdf)
    add_float_attr(mesh, "_NPR_FACE_SIDE", [1.0] * n)
    add_float_attr(mesh, "_NPR_MASK", [1.0] * n)
    add_float_attr(mesh, "_NPR_MATERIAL_ID", [2.0] * n)
    mesh.materials.append(mat)
    return obj


def make_hair_card(mat):
    """Curved sheet over the top of the head, built by from_pydata."""
    cols, rows = 16, 8
    verts, faces = [], []
    for r in range(rows + 1):
        v = r / rows
        for c in range(cols + 1):
            u = c / cols
            angle = (u - 0.5) * math.pi * 1.1
            radius = 0.62 + 0.18 * v
            x = radius * math.sin(angle)
            y = 0.10 + 0.20 * v - 0.25 * radius * math.cos(angle)
            z = 1.78 - 0.85 * v
            verts.append((x, y, z))
    for r in range(rows):
        for c in range(cols):
            i = r * (cols + 1) + c
            faces.append((i, i + 1, i + cols + 2, i + cols + 1))
    mesh = bpy.data.meshes.new("FixtureHairCard")
    mesh.from_pydata(verts, [], faces)
    # Deterministic planar UVs (grid coordinates) so lightmap/ramp texture
    # paths can bind to this mesh.
    uv_layer = mesh.uv_layers.new(name="UVMap")
    for poly in mesh.polygons:
        for li in poly.loop_indices:
            vi = mesh.loops[li].vertex_index
            r, c = divmod(vi, cols + 1)
            uv_layer.data[li].uv = (c / cols, r / rows)
    mesh.update()
    obj = bpy.data.objects.new("FixtureHairCard", mesh)
    bpy.context.collection.objects.link(obj)
    n = len(mesh.vertices)
    # Strand direction: object-space, flowing down the card (-Z with -Y tilt).
    add_vector_attr(mesh, "_NPR_HAIR_TANGENT", [(0.0, -0.2, -0.98)] * n)
    add_float_attr(mesh, "_NPR_HAIR_MASK", [1.0] * n)
    add_float_attr(mesh, "_NPR_MATERIAL_ID", [3.0] * n)
    mesh.materials.append(mat)
    return obj


def make_ground(mat):
    bpy.ops.mesh.primitive_plane_add(size=12.0, location=(0.0, 0.0, -0.9))
    obj = bpy.context.active_object
    obj.name = "FixtureGround"
    mesh = obj.data
    mesh.name = "FixtureGround"
    add_float_attr(mesh, "_NPR_LEVEL_ROLE", [1.0] * len(mesh.vertices))
    mesh.materials.append(mat)
    return obj


def main():
    args = parse_args()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scn = bpy.context.scene

    # Locked render environment (docs/reference/reference-toolchain.toml).
    scn.render.engine = "BLENDER_EEVEE"
    scn.render.resolution_x = RESOLUTION
    scn.render.resolution_y = RESOLUTION
    scn.render.resolution_percentage = 100
    scn.render.image_settings.file_format = "PNG"
    scn.render.image_settings.color_mode = "RGB"
    scn.render.film_transparent = False
    scn.display_settings.display_device = "sRGB"
    scn.view_settings.view_transform = "Standard"
    scn.view_settings.look = "None"
    scn.sequencer_colorspace_settings.name = "sRGB"

    body_mat = principled_material("FixtureMatBody", (0.72, 0.42, 0.30))
    head_mat = principled_material("FixtureMatHead", (0.86, 0.66, 0.56))
    hair_mat = principled_material("FixtureMatHair", (0.32, 0.20, 0.11), roughness=0.35)
    ground_mat = principled_material("FixtureMatGround", (0.18, 0.20, 0.24), roughness=0.9)

    make_body(body_mat)
    make_head(head_mat)
    make_hair_card(hair_mat)
    make_ground(ground_mat)

    bpy.ops.object.camera_add(location=CAMERA_LOCATION)
    cam = bpy.context.active_object
    cam.name = "ReferenceCamera"
    cam.data.lens = CAMERA_LENS_MM
    track_to(cam, CAMERA_TARGET)
    scn.camera = cam

    bpy.ops.object.light_add(type="SUN", location=SUN_DIRECTION_FROM)
    sun = bpy.context.active_object
    sun.name = "ReferenceKeyLight"
    sun.data.energy = SUN_ENERGY
    sun.data.color = SUN_COLOR
    track_to(sun, (0.0, 0.0, 0.6))

    world = bpy.data.worlds.new("ReferenceWorld")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (*WORLD_COLOR, 1.0)
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 1.0
    scn.world = world

    # Marker consumed by the export bridge fixture-role contract.
    scn["bevy_stylized.fixture_role"] = "fixture"
    scn["bevy_stylized.fixture_id"] = SCENE_VERSION

    out = os.path.abspath(args.out)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=out, check_existing=False)
    print(f"[make-scene] SCENE_OK {SCENE_VERSION} -> {out}")


if __name__ == "__main__":
    main()
