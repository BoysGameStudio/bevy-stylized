#!/usr/bin/env python3
"""Dump the internal node math of node groups from a .blend file to JSON.

Runs inside Blender background mode (stdlib + bpy only):

    blender <source.blend> --background --factory-startup \
        --python tools/blender-reference-export/inspect_node_math.py -- \
        --out <dir> [--groups "Name A,Name B"]   # default: all groups

For each node group, records per node: bl_idname, name, label, parent frame,
type-specific parameters (math operation, mix blend type, ramp stops, curve
points, image reference, attribute name, map-range inputs, ...), all socket
default values, and all links (from node.socket -> to node.socket). The dump
is evidence material for docs/reference/07-gacha-setup-behavior-contract-v1.md
verification work; it changes no state and writes only JSON.
"""

import argparse
import json
import os
import sys

import bpy


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    p = argparse.ArgumentParser(description="Dump node-group internal math")
    p.add_argument("--out", required=True)
    p.add_argument("--groups", default=None, help="Comma-separated group names; default: all")
    return p.parse_args(argv)


def safe_value(value):
    if hasattr(value, "__iter__") and not isinstance(value, str):
        return [float(v) for v in value]
    if isinstance(value, (bool, int, float)):
        return value
    return str(value)


def socket_default(sock):
    if not hasattr(sock, "default_value"):
        return None
    try:
        return safe_value(sock.default_value)
    except Exception:
        return None


def node_params(node):
    """Type-specific parameters worth recording as math evidence."""
    p = {}
    for attr in ("operation", "blend_type", "data_type", "factor_mode",
                 "interpolation_type", "use_clamp", "clamp_factor", "clamp_result",
                 "transform_type", "vector_math_operation", " shading_mode"):
        if hasattr(node, attr):
            try:
                p[attr] = safe_value(getattr(node, attr))
            except Exception:
                pass
    if node.bl_idname == "ShaderNodeValToRGB":
        ramp = node.color_ramp
        p["ramp_interpolation"] = ramp.interpolation
        p["ramp_elements"] = [
            {"position": e.position, "color": list(e.color)} for e in ramp.elements
        ]
    if node.bl_idname == "ShaderNodeRGBCurve":
        p["curves"] = [
            {"curve": i,
             "points": [{"location": [pt.location[0], pt.location[1]],
                         "handle_type": pt.handle_type} for pt in c.points]}
            for i, c in enumerate(node.mapping.curves)
        ]
    if node.bl_idname == "ShaderNodeTexImage" and node.image:
        p["image"] = node.image.name
        p["interpolation"] = node.interpolation
        p["extension"] = node.extension
    if node.bl_idname in ("ShaderNodeAttribute", "GeometryNodeInputNamedAttribute"):
        p["attribute_name"] = getattr(node, "attribute_name", None)
    if node.bl_idname == "ShaderNodeVertexColor":
        p["layer_name"] = node.layer_name
    if node.bl_idname == "ShaderNodeUVMap":
        p["uv_map"] = node.uv_map
    if node.bl_idname == "FunctionNodeInputBool":
        p["boolean"] = node.boolean
    if hasattr(node, "node_tree") and node.node_tree is not None:
        p["group"] = node.node_tree.name
    if hasattr(node, "inputs") and node.bl_idname == "GeometryNodeInputIndex":
        pass
    if node.bl_idname == "ShaderNodeScript":
        p["script"] = node.script.name if node.script else None
    return p


def dump_group(group):
    nodes = {}
    for node in group.nodes:
        entry = {
            "bl_idname": node.bl_idname,
            "label": node.label or None,
            "parent_frame": node.parent.name if node.parent else None,
            "params": node_params(node),
            "inputs": {},
            "outputs": [s.name for s in node.outputs],
        }
        for sock in node.inputs:
            v = socket_default(sock)
            if v is not None or not sock.is_linked:
                entry["inputs"][sock.identifier] = {"name": sock.name, "default": v}
        nodes[node.name] = entry
    links = [
        {"from": f"{l.from_node.name}.{l.from_socket.identifier}",
         "from_socket": l.from_socket.name,
         "to": f"{l.to_node.name}.{l.to_socket.identifier}",
         "to_socket": l.to_socket.name}
        for l in group.links
    ]
    return {
        "bl_idname": group.bl_idname,
        "node_count": len(group.nodes),
        "link_count": len(group.links),
        "nodes": nodes,
        "links": links,
    }


def main():
    args = parse_args()
    out_dir = os.path.abspath(args.out)
    os.makedirs(out_dir, exist_ok=True)
    wanted = None
    if args.groups:
        wanted = {g.strip() for g in args.groups.split(",") if g.strip()}
    index = {}
    for group in bpy.data.node_groups:
        if wanted is not None and group.name not in wanted:
            continue
        data = dump_group(group)
        fname = "".join(c if c.isalnum() or c in "-_." else "_" for c in group.name) + ".json"
        with open(os.path.join(out_dir, fname), "w", encoding="utf-8") as f:
            json.dump({group.name: data}, f, indent=1, ensure_ascii=False)
            f.write("\n")
        index[group.name] = {"file": fname, "nodes": data["node_count"],
                             "links": data["link_count"]}
        print(f"[inspect] dumped {group.name!r} ({data['node_count']} nodes) -> {fname}")
    with open(os.path.join(out_dir, "_index.json"), "w", encoding="utf-8") as f:
        json.dump(index, f, indent=1)
        f.write("\n")
    print(f"[inspect] INSPECT_OK {len(index)} groups -> {out_dir}")


if __name__ == "__main__":
    main()
