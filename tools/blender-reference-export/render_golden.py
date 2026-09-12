#!/usr/bin/env python3
"""Capture Upstream Golden renders of the reference fixture scene.

Runs inside Blender background mode:

    blender --background --factory-startup \
        --python tools/blender-reference-export/render_golden.py -- \
        --blend fixtures/reference/reference_scene.blend \
        --out-dir fixtures/golden-local/upstream \
        --vendor-blend "target/tmp/gacha-setup/extracted/setup_wizard/shaders/gi/HoYoverse - Genshin Impact v3.4.blend"

Cases:
- `pbr_control`    — fixture as authored (self-made Principled materials).
- `gacha_upstream` — fixture meshes re-materialized with the Gacha Setup
                     v3.6.2 node groups (`HoYoverse - GI Body` / `HoYoverse - GI Face`)
                     appended from the vendor blend. This capture is a
                     GPL-upstream derivative: LOCAL ONLY (fixtures/golden-local/).

Exit codes: 0 = all cases captured; 2 = gacha_upstream blocked (fail-closed,
BLOCKED record written, pbr_control still captured); 1 = hard failure.
"""

import argparse
import hashlib
import json
import os
import platform
import sys
import traceback

import bpy

DEFAULT_VENDOR_BLEND = os.path.join(
    "target", "tmp", "gacha-setup", "extracted", "setup_wizard", "shaders", "gi",
    "HoYoverse - Genshin Impact v3.4.blend")

GACHA_BODY_GROUP = "HoYoverse - GI Body"
GACHA_FACE_GROUP = "HoYoverse - GI Face"
GACHA_HELPER_OBJECTS = ["Light Direction", "Head Forward", "Head Origin", "Head Up"]

# Wizard-injection emulation (local-only textures, never committed):
# Tenacity odette-npr sidecar textures, mapped per the verified channel
# layout (docs/reference/07 §3.2: R=metal, G=AO, B=spec-suppress, A=material-id).
DEFAULT_TEXTURE_DIR = "/home/x/Projects/Tenacity/crates/client-bevy/assets/models/odette-npr"

# (node group, node name prefix, texture filename, colorspace)
GROUP_IMAGE_ASSIGNMENTS = [
    ("Body Shadow Ramp", "Body_Shadow_Ramp", "shadow_ramp.png", "sRGB"),
    ("Hair Shadow Ramp", "Hair_Shadow_Ramp", "shadow_ramp.png", "sRGB"),
    ("Face Factor", "Face_Lightmap", "face_lightmap.png", "Non-Color"),
    ("Face Factor", "Face_Shadow", "face_shadow.png", "Non-Color"),
    ("Face Shadow", "Face_Shadow", "face_shadow.png", "Non-Color"),  # optional: not in GI Body/Face append closure
    ("Genshin Metallic", "MetalMap", "metal_map.png", "sRGB"),
    ("Genshin Specular", "Image Texture", "specular_ramp.png", "sRGB"),
]
OPTIONAL_IMAGE_GROUPS = {"Face Shadow"}

TEXTURE_PROVENANCE = ("Tenacity odette-npr sidecar textures (local-only, "
                      "HoYoverse-derived; never committed to bevy-stylized)")


def log(msg):
    print(f"[render-golden] {msg}", flush=True)


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def sha256_pixels(path):
    """Content hash of decoded pixels (PNG file bytes embed timestamps)."""
    import struct
    img = bpy.data.images.load(path)
    pixels = img.pixels[:]
    h = hashlib.sha256()
    h.update(struct.pack("<II", img.size[0], img.size[1]))
    h.update(bytearray(struct.pack(f"<{len(pixels)}f", *pixels)))
    bpy.data.images.remove(img)
    return h.hexdigest()


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    p = argparse.ArgumentParser(description="Capture upstream goldens")
    p.add_argument("--blend", required=True)
    p.add_argument("--out-dir", required=True)
    p.add_argument("--vendor-blend", default=DEFAULT_VENDOR_BLEND)
    p.add_argument("--cases", default="pbr_control,gacha_upstream")
    p.add_argument("--texture-dir", default=DEFAULT_TEXTURE_DIR,
                   help="Wizard-injection emulation texture dir (local-only); "
                        "gacha_upstream fails closed if missing")
    return p.parse_args(argv)


def render_to(png_path):
    scn = bpy.context.scene
    assert scn.render.engine == "BLENDER_EEVEE", scn.render.engine
    scn.render.filepath = png_path
    bpy.ops.render.render(write_still=True)
    if not os.path.exists(png_path):
        raise RuntimeError(f"render produced no file: {png_path}")


def golden_sidecar(case, png_path, blend_path):
    scn = bpy.context.scene
    return {
        "reference_toolchain": "gacha-blender-v1",
        "blender": bpy.app.version_string,
        "gacha_setup": "v3.6.2",
        "gacha_setup_commit": "069e51f3fd89cdc66b4c2741798ede11a5fb16b5",
        "gacha_setup_asset_digest": "f5ab8206464130962234a7ccab19c7d212a048c848ce3e69518f84fc41a39999",
        "export_contract": "PENDING (blender-gltf-npr-v1 drafted in It-1a; not yet maintainer-confirmed)",
        "profile_schema": "PENDING (gacha/v1 drafted; [profile].genshin_schema still PENDING in reference-toolchain.toml)",
        "fixture": str(bpy.context.scene.get("bevy_stylized.fixture_id", "unknown")),
        "fixture_sha256": sha256_file(blend_path),
        "renderer": "blender-eevee",
        "case": case,
        "image": os.path.basename(png_path),
        "image_sha256": sha256_file(png_path),
        "image_pixels_sha256": sha256_pixels(png_path),
        "resolution": [scn.render.resolution_x, scn.render.resolution_y],
        "render_engine": scn.render.engine,
        "color_management": {
            "display_device": scn.display_settings.display_device,
            "view_transform": scn.view_settings.view_transform,
            "view_look": scn.view_settings.look,
        },
        "hardware": "NVIDIA RTX 5090 (local baseline)",
        "platform": platform.platform(),
        "license_scope": "local-only (fixtures/golden-local/); contains GPL-upstream-derived rendering" if case == "gacha_upstream"
                         else "self-made fixture render; still captured local-only per It-1b layout",
    }


def load_texture(texture_dir, filename, colorspace):
    path = os.path.join(texture_dir, filename)
    if not os.path.exists(path):
        raise RuntimeError(f"texture not found: {path}")
    img = bpy.data.images.load(path, check_existing=True)
    img.colorspace_settings.name = colorspace
    return img


def assign_group_images(texture_dir):
    """Assign wizard-injected images into the shared group-internal nodes."""
    used = {}
    for group_name, node_prefix, filename, colorspace in GROUP_IMAGE_ASSIGNMENTS:
        group = bpy.data.node_groups.get(group_name)
        if group is None:
            if group_name in OPTIONAL_IMAGE_GROUPS:
                log(f"optional texture group {group_name!r} not appended; skipping")
                continue
            raise RuntimeError(f"expected node group missing for texture wiring: {group_name!r}")
        matched = [n for n in group.nodes
                   if n.bl_idname == "ShaderNodeTexImage" and n.name.startswith(node_prefix)]
        if not matched:
            raise RuntimeError(f"no texture node {node_prefix!r} in group {group_name!r}")
        img = load_texture(texture_dir, filename, colorspace)
        for node in matched:
            node.image = img
        used[f"{group_name}/{node_prefix}*"] = {
            "file": filename,
            "sha256": sha256_file(os.path.join(texture_dir, filename)),
            "colorspace": colorspace,
            "nodes": [n.name for n in matched],
        }
    return used


def wire_lightmap(nt, group_node, texture_dir, filename):
    """Wire a real ILM/lightmap texture into a GI Body group instance."""
    img = load_texture(texture_dir, filename, "Non-Color")
    tex = nt.nodes.new("ShaderNodeTexImage")
    tex.image = img
    nt.links.new(tex.outputs["Color"], group_node.inputs["Lightmap Color"])
    nt.links.new(tex.outputs["Alpha"], group_node.inputs["Lightmap Alpha"])


def gacha_material(name, group, base_color, body_hair=None, texture_dir=None,
                   lightmap=None):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nt = mat.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    node = nt.nodes.new("ShaderNodeGroup")
    node.node_tree = group
    if node.inputs.get("Diffuse Color"):
        node.inputs["Diffuse Color"].default_value = (*base_color, 1.0)
    if body_hair is not None and node.inputs.get("Body / Hair"):
        node.inputs["Body / Hair"].default_value = body_hair
    # Neutral "no authored maps" defaults for the GI Body group; its
    # interface defaults assume wizard-generated lightmaps/vertex colors and
    # evaluate to black without them.
    neutral_defaults = {
        "Diffuse Alpha": 1.0,
        "Lightmap Color": (1.0, 1.0, 1.0, 1.0),
        "Lightmap Alpha": 1.0,
        "Normal Map": (0.5, 0.5, 1.0),
        "Use Lightmap AO": 0.0,
        "Use Vertex Color AO": 0.0,
        "Use Normal Map": 0.0,
        "Use Specular Ramp": 0.0,
        "Enable Metallics?": 0.0,
    }
    if texture_dir is not None and lightmap is not None:
        # Wizard-injection emulation: real lightmap wired, upstream vendor
        # material defaults restored where the texture path supports them.
        neutral_defaults.update({
            "Use Lightmap AO": 1.0,     # vendor default (docs/reference/07 §3.2 G=AO)
            "Enable Metallics?": 1.0,   # vendor default (R=metal mask, >0.9)
        })
    for sock_name, value in neutral_defaults.items():
        sock = node.inputs.get(sock_name)
        if sock is not None and not sock.is_linked:
            sock.default_value = value
    if texture_dir is not None and lightmap is not None:
        wire_lightmap(nt, node, texture_dir, lightmap)
    shader_out = node.outputs.get("Shader") or node.outputs.get("BSDF")
    if shader_out is None:
        raise RuntimeError(f"group {group.name!r} exposes no shader output")
    nt.links.new(shader_out, out.inputs["Surface"])
    return mat


def setup_gacha_upstream(vendor_blend, texture_dir=None):
    """Append the Gacha node groups (and their helper objects) from the vendor blend."""
    if not os.path.exists(vendor_blend):
        raise RuntimeError(f"vendor blend not found: {vendor_blend}")
    with bpy.data.libraries.load(vendor_blend, link=False) as (data_from, data_to):
        missing = [g for g in (GACHA_BODY_GROUP, GACHA_FACE_GROUP)
                   if g not in data_from.node_groups]
        if missing:
            raise RuntimeError(f"expected node groups missing in vendor blend: {missing}")
        data_to.node_groups = [GACHA_BODY_GROUP, GACHA_FACE_GROUP]
        data_to.objects = [o for o in GACHA_HELPER_OBJECTS if o in data_from.objects]
    appended = {g: bpy.data.node_groups.get(g) for g in (GACHA_BODY_GROUP, GACHA_FACE_GROUP)}
    if any(v is None for v in appended.values()):
        raise RuntimeError("append failed: Gacha node groups not present after libraries.load")
    for obj in bpy.data.objects:
        if obj.name in GACHA_HELPER_OBJECTS and obj.name not in bpy.context.scene.objects:
            bpy.context.scene.collection.objects.link(obj)

    textures_used = {}
    if texture_dir is not None:
        textures_used = assign_group_images(texture_dir)

    body_mat = gacha_material("GachaMatBody", appended[GACHA_BODY_GROUP],
                              (0.72, 0.42, 0.30), body_hair=0.0,
                              texture_dir=texture_dir, lightmap="body_lightmap.png")
    hair_mat = gacha_material("GachaMatHair", appended[GACHA_BODY_GROUP],
                              (0.32, 0.20, 0.11), body_hair=1.0,
                              texture_dir=texture_dir, lightmap="hair_lightmap.png")
    face_mat = gacha_material("GachaMatFace", appended[GACHA_FACE_GROUP],
                              (0.86, 0.66, 0.56))
    if face_mat.node_tree:
        for node in face_mat.node_tree.nodes:
            if node.bl_idname == "ShaderNodeGroup" and node.inputs.get("[Loli/Boy/Girl/Male/Lady]"):
                node.inputs["[Loli/Boy/Girl/Male/Lady]"].default_value = 1.0

    assignments = {
        "FixtureBody": body_mat,
        "FixtureHairCard": hair_mat,
        "FixtureHead": face_mat,
    }
    for obj_name, mat in assignments.items():
        obj = bpy.data.objects.get(obj_name)
        if obj is None:
            raise RuntimeError(f"fixture object missing: {obj_name}")
        obj.data.materials.clear()
        obj.data.materials.append(mat)
    return textures_used


def main():
    args = parse_args()
    out_dir = os.path.abspath(args.out_dir)
    os.makedirs(out_dir, exist_ok=True)
    blend = os.path.abspath(args.blend)
    vendor = os.path.abspath(args.vendor_blend)
    cases = [c.strip() for c in args.cases.split(",") if c.strip()]

    results = {}
    exit_code = 0
    for case in cases:
        # Fresh file load per case for isolation.
        bpy.ops.wm.open_mainfile(filepath=blend)
        png = os.path.join(out_dir, f"{case}.png")
        textures_used = None
        try:
            if case == "gacha_upstream":
                texture_dir = os.path.abspath(args.texture_dir)
                if not os.path.isdir(texture_dir):
                    raise RuntimeError(f"texture dir not found: {texture_dir}")
                textures_used = setup_gacha_upstream(vendor, texture_dir)
            elif case != "pbr_control":
                raise RuntimeError(f"unknown case {case!r}")
            render_to(png)
            sidecar = golden_sidecar(case, png, blend)
            if textures_used:
                sidecar["textures"] = textures_used
                sidecar["texture_provenance"] = TEXTURE_PROVENANCE
                sidecar["wizard_injection_emulated"] = True
            sidecar_path = os.path.join(out_dir, f"{case}.golden.json")
            with open(sidecar_path, "w", encoding="utf-8") as f:
                json.dump(sidecar, f, indent=1)
                f.write("\n")
            results[case] = "ok"
            log(f"CAPTURE_OK {case} -> {png} (sha256 {sidecar['image_sha256'][:12]}...)")
        except Exception as exc:
            traceback.print_exc()
            results[case] = f"blocked: {exc}"
            blocked_path = os.path.join(out_dir, f"{case}.BLOCKED.json")
            with open(blocked_path, "w", encoding="utf-8") as f:
                json.dump({"case": case, "status": "blocked", "reason": str(exc),
                           "traceback": traceback.format_exc()}, f, indent=1)
                f.write("\n")
            log(f"FAIL_CLOSED case {case}: {exc} (recorded in {blocked_path})")
            exit_code = 2

    log(f"results: {results}")
    return exit_code


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception:
        traceback.print_exc()
        sys.exit(1)
