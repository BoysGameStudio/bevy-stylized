#!/usr/bin/env python3
"""Blender Reference Export Bridge (blender-gltf-npr-v1).

Runs inside Blender background mode:

    blender <source.blend> --background --python export_reference.py -- \
        --out <dir> [--contract-version blender-gltf-npr-v1] [--role auto]

Implements bridge responsibilities 1-10 of
docs/authoring/02-blender-reference-export-bridge.md and the manifest
contract of docs/reference/03-blender-export-contract.md. Stdlib + bpy only.

Fail-closed: any contract violation exits non-zero and leaves no partial
outputs in the output directory (outputs are staged in a temp dir first).
"""

import argparse
import hashlib
import json
import os
import shutil
import sys
import tempfile
import tomllib

import bpy
import addon_utils

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
CONTRACTS_DIR = os.path.join(REPO_ROOT, "tools", "blender-reference-export", "contracts")
TOOLCHAIN_LOCK = os.path.join(REPO_ROOT, "docs", "reference", "reference-toolchain.toml")

WARNINGS = []


class FailClosed(Exception):
    pass


def log(msg):
    print(f"[export-bridge] {msg}", flush=True)


def warn(msg):
    WARNINGS.append(msg)
    log(f"WARN: {msg}")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def sha256_json(obj):
    return hashlib.sha256(json.dumps(obj, sort_keys=True).encode("utf-8")).hexdigest()


def load_json(path):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    p = argparse.ArgumentParser(description="Blender Reference Export Bridge")
    p.add_argument("--out", required=True, help="Output directory for export artifacts")
    p.add_argument("--contract-version", default="blender-gltf-npr-v1")
    p.add_argument("--role", default="auto",
                   help="Artifact role; validated against the signature contract")
    p.add_argument("--signature", default="expected-signature.genshin-v3.6.2.json",
                   help="Signature contract filename under contracts/")
    return p.parse_args(argv)


def verify_blender_version(lock):
    """Responsibility 1: verify Blender version against the toolchain lock."""
    expected = lock["blender"]["version"]
    if expected == "PENDING":
        raise FailClosed("toolchain lock [blender].version is PENDING; baseline not locked")
    running = bpy.app.version_string
    if running != expected:
        raise FailClosed(f"Blender version mismatch: running {running!r}, locked {expected!r}")
    series = lock["blender"].get("series", "")
    if series:
        major_minor = ".".join(str(v) for v in bpy.app.version[:2])
        if not series.startswith(major_minor):
            raise FailClosed(f"Blender series mismatch: running {major_minor}, locked {series!r}")
    log(f"Blender version OK: {running}")


def gltf_exporter_version():
    for mod in addon_utils.modules():
        if mod.__name__ == "io_scene_gltf2":
            return ".".join(str(v) for v in mod.bl_info.get("version", []))
    raise FailClosed("io_scene_gltf2 addon not found; cannot export glTF")


def detect_role(signature, requested):
    present = set(bpy.data.node_groups.keys())
    if requested != "auto":
        if requested not in signature["roles"]:
            raise FailClosed(f"unknown role {requested!r} for this signature contract")
        marker = signature["roles"][requested]["detect_if_present"]
        if marker is not None and marker not in present:
            raise FailClosed(
                f"requested role {requested!r} but marker node group {marker!r} is absent")
        scene_marker = signature["roles"][requested].get("detect_scene_property")
        if scene_marker is not None and bpy.context.scene.get(scene_marker) != requested:
            raise FailClosed(
                f"requested role {requested!r} but scene property {scene_marker!r} "
                f"!= {requested!r}")
        return requested
    for role, spec in signature["roles"].items():
        marker = spec["detect_if_present"]
        if marker is not None and marker in present:
            return role
    raise FailClosed("cannot detect artifact role: no known marker node group present")


def interface_sockets(group):
    sockets = []
    for item in group.interface.items_tree:
        if item.item_type == "SOCKET":
            sockets.append({
                "name": item.name,
                "in_out": item.in_out,
                "socket_type": item.socket_type,
            })
    return sockets


def verify_signature(signature, role):
    """Responsibilities 2, 3 and 10: verify Gacha node-group signature, fail closed."""
    spec = signature["roles"][role]
    observed = {}
    for entry in spec["required_node_groups"]:
        name = entry["name"]
        group = bpy.data.node_groups.get(name)
        if group is None:
            raise FailClosed(f"expected node group missing: {name!r}")
        if group.bl_idname != entry["bl_idname"]:
            raise FailClosed(
                f"node group {name!r} type changed: {group.bl_idname} != {entry['bl_idname']}")
        sockets = interface_sockets(group)
        exp_names = [(s["name"], s["in_out"], s["socket_type"]) for s in entry["interface"]]
        got_names = [(s["name"], s["in_out"], s["socket_type"]) for s in sockets]
        if got_names != exp_names:
            exp_set, got_set = set(exp_names), set(got_names)
            raise FailClosed(
                f"socket name changed on node group {name!r}: "
                f"missing={sorted(exp_set - got_set)} unexpected={sorted(got_set - exp_set)}")
        node_types = sorted(n.bl_idname for n in group.nodes)
        if sha256_json(node_types) != entry["node_types_sha256"]:
            raise FailClosed(f"material topology changed (node types) in node group {name!r}")
        if len(group.nodes) != entry["node_count"] or len(group.links) != entry["link_count"]:
            raise FailClosed(
                f"material topology changed in node group {name!r}: "
                f"nodes {len(group.nodes)}/{entry['node_count']}, "
                f"links {len(group.links)}/{entry['link_count']}")
        observed[name] = {
            "bl_idname": group.bl_idname,
            "node_count": len(group.nodes),
            "link_count": len(group.links),
            "interface_sha256": sha256_json(sockets),
            "node_types_sha256": sha256_json(node_types),
        }
    for mat in spec.get("required_materials", []):
        if mat not in bpy.data.materials:
            raise FailClosed(f"expected material missing: {mat!r}")
    signature_sha = sha256_json(observed)
    log(f"node-group signature OK for role {role!r} "
        f"({len(observed)} groups, sha256={signature_sha[:12]}...)")
    return observed, signature_sha


def check_textures():
    """Fail closed on required texture missing (unpacked FILE image whose file is gone)."""
    for img in bpy.data.images:
        if img.source != "FILE":
            continue
        if img.packed_file is not None:
            continue
        path = bpy.path.abspath(img.filepath) if img.filepath else ""
        if not path or not os.path.exists(path):
            raise FailClosed(f"required texture missing: image {img.name!r} filepath {img.filepath!r}")


def check_npr_attributes(contract):
    """Responsibility 5: verify _NPR_* mesh attributes per contract data."""
    by_name = {a["name"]: a for a in contract["attributes"]}
    for mesh in bpy.data.meshes:
        present = {a.name: a for a in mesh.attributes if a.name.startswith("_NPR_")}
        for attr_name in contract.get("fail_if_missing", []):
            if attr_name not in present:
                raise FailClosed(f"required attribute missing: {attr_name} on mesh {mesh.name!r}")
        for attr_name in contract.get("warn_if_missing", []):
            if attr_name not in present:
                warn(f"optional attribute missing: {attr_name} on mesh {mesh.name!r}")
        for attr_name, attr in present.items():
            spec = by_name.get(attr_name)
            if spec is None:
                warn(f"unknown _NPR_* attribute {attr_name!r} on mesh {mesh.name!r}")
                continue
            if attr.domain != spec["domain"]:
                raise FailClosed(
                    f"attribute domain changed: {attr_name} on mesh {mesh.name!r} "
                    f"{attr.domain} != {spec['domain']}")
            if attr.data_type != spec["data_type"]:
                raise FailClosed(
                    f"attribute data type changed: {attr_name} on mesh {mesh.name!r} "
                    f"{attr.data_type} != {spec['data_type']}")


def group_node_instances(group_name):
    for mat in bpy.data.materials:
        if not mat.node_tree:
            continue
        for node in mat.node_tree.nodes:
            if node.bl_idname == "ShaderNodeGroup" and node.node_tree \
                    and node.node_tree.name == group_name:
                yield mat, node


def build_metadata(signature, role, contract_version, source_path, source_sha):
    """Responsibilities 4 and 6: map upstream inputs to bevy_stylized.* metadata."""
    mapped = {}
    for entry in signature.get("metadata_map", {}).get(role, []):
        values = []
        for mat, node in group_node_instances(entry["group"]):
            sock = node.inputs.get(entry["socket"])
            if sock is None:
                continue
            value = getattr(sock, "default_value", None)
            if hasattr(value, "__iter__") and not isinstance(value, str):
                value = [float(v) for v in value]
            elif isinstance(value, (int, float)):
                value = float(value)
            values.append({"material": mat.name, "value": value})
        if not values:
            group = bpy.data.node_groups.get(entry["group"])
            if group is not None:
                item = group.interface.items_tree.get(entry["socket"])
                value = getattr(item, "default_value", None) if item else None
                if value is not None:
                    if hasattr(value, "__iter__") and not isinstance(value, str):
                        value = [float(v) for v in value]
                    elif isinstance(value, (int, float)):
                        value = float(value)
                    values.append({"material": None, "value": value})
        if values:
            mapped[entry["key"]] = values[0]["value"] if len(values) == 1 else values
    sidecar = {
        "bevy_stylized.schema_version": "export-sidecar-v1",
        "bevy_stylized.export_contract_version": contract_version,
        "bevy_stylized.source": {
            "path": os.path.basename(source_path),
            "sha256": source_sha,
            "role": role,
            "gacha_setup_release": signature.get("source", {}).get("release"),
            "gacha_setup_commit": signature.get("source", {}).get("commit"),
        },
        "bevy_stylized.metadata": mapped,
    }
    scene = bpy.context.scene
    scene["bevy_stylized.export_contract_version"] = contract_version
    scene["bevy_stylized.source_role"] = role
    release = signature.get("source", {}).get("release")
    commit = signature.get("source", {}).get("commit")
    if release:
        scene["bevy_stylized.gacha_setup_release"] = release
    if commit:
        scene["bevy_stylized.gacha_setup_commit"] = commit
    return sidecar


def locked_export_kwargs(settings_contract, glb_path):
    """Responsibility 7: locked glTF export settings, with drift detection."""
    rna_props = bpy.ops.export_scene.gltf.get_rna_type().properties
    known = {p.identifier: p for p in rna_props}
    kwargs = {}
    for key, value in settings_contract["settings"].items():
        prop = known.get(key)
        if prop is None:
            raise FailClosed(f"exporter setting drifted: {key!r} no longer exists on export_scene.gltf")
        if prop.type == "ENUM":
            items = list(prop.enum_items) or list(getattr(prop, "enum_items_static", []))
            valid = {i.identifier for i in items}
            if valid and value not in valid:
                raise FailClosed(
                    f"exporter setting drifted: {key}={value!r} not in {sorted(valid)}")
        kwargs[key] = value
    kwargs["filepath"] = glb_path
    return kwargs


def main():
    args = parse_args()
    if args.contract_version != "blender-gltf-npr-v1":
        raise FailClosed(f"unknown export contract version: {args.contract_version!r}")

    with open(TOOLCHAIN_LOCK, "rb") as f:
        lock = tomllib.load(f)
    signature = load_json(os.path.join(CONTRACTS_DIR, args.signature))
    settings_contract = load_json(os.path.join(CONTRACTS_DIR, "export-settings.v1.json"))
    attr_contract = load_json(os.path.join(CONTRACTS_DIR, "npr-attributes.v1.json"))

    source_path = bpy.data.filepath
    if not source_path or not os.path.exists(source_path):
        raise FailClosed(f"no source blend loaded (bpy.data.filepath={source_path!r})")
    source_sha = sha256_file(source_path)

    verify_blender_version(lock)
    if signature["contract_version"] != args.contract_version:
        raise FailClosed("signature contract version mismatch")
    sig_commit = signature.get("source", {}).get("commit")
    if sig_commit and sig_commit != lock["gacha_setup"]["commit"]:
        raise FailClosed("Gacha Setup commit mismatch between signature contract and toolchain lock")

    role = detect_role(signature, args.role)
    log(f"artifact role: {role} (source: {os.path.basename(source_path)})")

    observed, signature_sha = verify_signature(signature, role)
    check_textures()
    check_npr_attributes(attr_contract)

    out_dir = os.path.abspath(args.out)
    os.makedirs(out_dir, exist_ok=True)
    staging = tempfile.mkdtemp(prefix=".tmp-export-", dir=out_dir)
    try:
        slug = os.path.splitext(os.path.basename(source_path))[0]
        slug = "".join(c if c.isalnum() or c in "-_" else "_" for c in slug)
        glb_name = f"{slug}.glb"
        glb_path = os.path.join(staging, glb_name)

        sidecar = build_metadata(signature, role, args.contract_version, source_path, source_sha)

        kwargs = locked_export_kwargs(settings_contract, glb_path)
        settings_hash = sha256_json(settings_contract["settings"])
        bpy.ops.export_scene.gltf(**kwargs)
        if not os.path.exists(glb_path):
            raise FailClosed("glTF exporter returned without producing output")

        scene = bpy.context.scene
        manifest = {
            "schema_version": "export-manifest-v1",
            "blender_version": bpy.app.version_string,
            "gacha_setup_version": lock["gacha_setup"]["release"],
            "gacha_setup_commit": lock["gacha_setup"]["commit"],
            "gacha_setup_asset_digest": lock["gacha_setup"]["sha256"],
            "gltf_exporter_version": gltf_exporter_version(),
            "export_contract_version": args.contract_version,
            "export_settings_hash": settings_hash,
            "node_graph_signature": {
                "revision": signature["revision"],
                "signature_sha256": signature_sha,
            },
            "render_engine": scene.render.engine,
            "color_management": {
                "display_device": scene.display_settings.display_device,
                "view_transform": scene.view_settings.view_transform,
                "view_look": scene.view_settings.look,
                "sequencer_colorspace": scene.sequencer_colorspace_settings.name,
            },
            "canonical_environment_id": lock["reference_family"]["id"],
            "source_fixture": {
                "path": os.path.basename(source_path),
                "sha256": source_sha,
                "role": role,
            },
            "outputs": {
                "glb": glb_name,
                "glb_sha256": sha256_file(glb_path),
                "sidecar": "export-sidecar.json",
                "node_graph_signature": "node-graph-signature.json",
            },
            "status": "ok",
            "warnings": WARNINGS,
        }
        node_graph_signature = {
            "contract_version": args.contract_version,
            "revision": signature["revision"],
            "role": role,
            "signature_sha256": signature_sha,
            "groups": observed,
        }

        for name, payload in (
            ("export-manifest.json", manifest),
            ("export-sidecar.json", sidecar),
            ("node-graph-signature.json", node_graph_signature),
        ):
            with open(os.path.join(staging, name), "w", encoding="utf-8") as f:
                json.dump(payload, f, indent=1, ensure_ascii=False)
                f.write("\n")

        for name in os.listdir(staging):
            shutil.move(os.path.join(staging, name), os.path.join(out_dir, name))
    finally:
        shutil.rmtree(staging, ignore_errors=True)

    log(f"EXPORT_OK role={role} out={out_dir} glb={glb_name} warnings={len(WARNINGS)}")


if __name__ == "__main__":
    try:
        main()
    except FailClosed as exc:
        log(f"FAIL_CLOSED: {exc}")
        sys.exit(1)
    except SystemExit:
        raise
    except Exception:
        import traceback
        traceback.print_exc()
        log("FAIL_CLOSED: unexpected error (see traceback)")
        sys.exit(1)
