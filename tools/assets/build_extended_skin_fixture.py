#!/usr/bin/env python3
"""Build a first-party two-influence-set/morph fixture without Blender or game art."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[2]


def build():
    binary = bytearray()
    views, accessors = [], []

    def accessor(values, width, kind, code="f", bounds=False):
        while len(binary) % 4:
            binary.append(0)
        offset = len(binary)
        flat = [x for value in values for x in value] if width > 1 else values
        binary.extend(struct.pack("<" + code * len(flat), *flat))
        views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(binary) - offset})
        value = {"bufferView": len(views) - 1, "componentType": {"f": 5126, "H": 5123}[code],
                 "count": len(values), "type": kind}
        if bounds:
            axes = list(zip(*values)) if width > 1 else [values]
            value.update(min=[min(a) for a in axes], max=[max(a) for a in axes])
        accessors.append(value)
        return len(accessors) - 1

    positions = [[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.0, 0.5, 0.0]]
    attrs = {
        "POSITION": accessor(positions, 3, "VEC3", bounds=True),
        "NORMAL": accessor([[0.0, 0.0, 1.0]] * 3, 3, "VEC3"),
        "TEXCOORD_0": accessor([[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]], 2, "VEC2"),
        "JOINTS_0": accessor([[0, 0, 0, 0]] * 3, 4, "VEC4", "H"),
        "WEIGHTS_0": accessor([[0.25, 0.0, 0.0, 0.0]] * 3, 4, "VEC4"),
        "JOINTS_1": accessor([[1, 0, 0, 0]] * 3, 4, "VEC4", "H"),
        "WEIGHTS_1": accessor([[0.75, 0.0, 0.0, 0.0]] * 3, 4, "VEC4"),
    }
    attrs["_NPR_JOINTS_1"] = attrs["JOINTS_1"]
    attrs["_NPR_WEIGHTS_1"] = attrs["WEIGHTS_1"]
    indices = accessor([0, 1, 2], 1, "SCALAR", "H")
    delta = accessor([[0.0, 0.0, 0.5]] * 3, 3, "VEC3")
    identity = [1.0 if row == column else 0.0 for column in range(4) for row in range(4)]
    inverse_bind = accessor([identity, identity], 16, "MAT4")
    times = accessor([0.0, 1.0], 1, "SCALAR", bounds=True)
    translations = accessor([[0.0, 0.0, 0.0], [4.0, 0.0, 0.0]], 3, "VEC3")
    document = {
        "asset": {"version": "2.0", "generator": "bevy-stylized first-party extended-skin fixture"},
        "scene": 0, "scenes": [{"nodes": [0]}],
        "nodes": [{"name": "fixture", "children": [1, 2, 3]}, {"name": "joint-a"},
                  {"name": "joint-b"}, {"name": "surface", "mesh": 0, "skin": 0}],
        "skins": [{"joints": [1, 2], "inverseBindMatrices": inverse_bind}],
        "meshes": [{"name": "surface", "weights": [0.0], "primitives": [
            {"attributes": attrs, "indices": indices, "material": 0, "targets": [{"POSITION": delta}]}]}],
        "materials": [{"name": "surface", "pbrMetallicRoughness": {
            "baseColorFactor": [0.8, 0.3, 0.2, 1.0], "metallicFactor": 0.0, "roughnessFactor": 1.0}}],
        "animations": [{"name": "move-second-joint", "samplers": [
            {"input": times, "output": translations, "interpolation": "LINEAR"}],
            "channels": [{"sampler": 0, "target": {"node": 2, "path": "translation"}}]}],
        "buffers": [{"byteLength": len(binary)}], "bufferViews": views, "accessors": accessors,
    }
    encoded = json.dumps(document, sort_keys=True, separators=(",", ":")).encode()
    encoded += b" " * (-len(encoded) % 4)
    binary += b"\0" * (-len(binary) % 4)
    glb = (struct.pack("<4sII", b"glTF", 2, 28 + len(encoded) + len(binary))
           + struct.pack("<II", len(encoded), 0x4E4F534A) + encoded
           + struct.pack("<II", len(binary), 0x004E4942) + binary)
    sidecar = {"schema": "bevy-stylized.sidecar/v1", "materials": {
        "surface": {"name": "surface", "surface_class": "cel", "static_params": {
            "base_color_r": 0.8, "base_color_g": 0.3, "base_color_b": 0.2, "base_color_a": 1.0}}}}
    cases = {"schema": "bevy_stylized.extended-skin-fixture/v1", "source": "first-party procedural triangle",
             "glb_sha256": hashlib.sha256(glb).hexdigest(), "rest_positions": positions,
             "cases": [{"id": "rest", "time": 0.0, "morph_weight": 0.0, "translation": [0.0, 0.0, 0.0]},
                       {"id": "second-influence", "time": 1.0, "morph_weight": 0.0, "translation": [3.0, 0.0, 0.0]},
                       {"id": "morph-before-skin", "time": 1.0, "morph_weight": 1.0, "translation": [3.0, 0.0, 0.5]}],
             "negative_cases": ["missing-secondary-weights", "missing-secondary-joints", "secondary-joint-out-of-range"],
             "qualification": "input fixture only; GPU deformation checks must execute these cases"}
    encode = lambda value: (json.dumps(value, indent=2) + "\n").encode()
    return {"extended-skin.glb": glb, "extended-skin.npr.json": encode(sidecar), "cases.json": encode(cases)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--out", type=Path, default=ROOT / "fixtures/synthetic/extended-skin")
    args = parser.parse_args()
    for name, data in build().items():
        path = args.out / name
        if args.check:
            if not path.is_file() or path.read_bytes() != data:
                raise SystemExit(f"Fixture differs: {path}")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            if path.exists() and path.read_bytes() != data:
                raise SystemExit(f"Refusing to replace a different existing fixture: {path}")
            path.write_bytes(data)
    print("Extended-skin fixture: " + ("verified" if args.check else "created"))


if __name__ == "__main__":
    main()
