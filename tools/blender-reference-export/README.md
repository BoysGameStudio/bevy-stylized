# Blender Reference Export Bridge

V1 scriptable bridge from the locked Blender/Gacha Setup reference to
bevy-stylized runtime input, per `docs/authoring/02-blender-reference-export-bridge.md`
and `docs/reference/03-blender-export-contract.md`. Dev/tool only: nothing
outside `tools/` may depend on `bpy`, Blender, or Gacha Setup.

## Layout

```text
tools/blender-reference-export/
├── README.md
├── export_reference.py                  # Blender-side script (bpy, stdlib only)
├── run_batch.py                         # plain python3 batch runner (no bpy)
├── make_reference_scene.py              # deterministic self-made fixture scene builder
├── render_golden.py                     # upstream golden capture (pbr_control / gacha_upstream)
├── compare_golden.py                    # golden image diff with tolerance policy
├── reproduce_golden.sh                  # rebuild + re-render + diff (reproduction gate)
└── contracts/
    ├── export-settings.v1.json          # locked glTF exporter settings
    ├── expected-signature.genshin-v3.6.2.json  # node-group/socket signature from real artifacts
    ├── expected-signature.fixture-v1.json      # signature contract for the self-made fixture scene
    ├── manifest.schema.json             # JSON schema for export-manifest.json
    └── npr-attributes.v1.json           # _NPR_* attribute checks (+ legacy _CEL_* mapping)
```

## Reference fixture and upstream goldens (It-1b)

```text
# 1. Build the self-made fixture scene (deterministic; output is committable)
blender --background --factory-startup \
    --python tools/blender-reference-export/make_reference_scene.py -- \
    --out fixtures/reference/reference_scene.blend

# 2. Export it through the bridge (fixture role contract)
blender fixtures/reference/reference_scene.blend --background --factory-startup \
    --python tools/blender-reference-export/export_reference.py -- \
    --out fixtures/reference --signature expected-signature.fixture-v1.json --role fixture

# 3. Capture upstream goldens (EEVEE; gacha_upstream is local-only, GPL-derived)
blender --background --factory-startup \
    --python tools/blender-reference-export/render_golden.py -- \
    --blend fixtures/reference/reference_scene.blend \
    --out-dir fixtures/golden-local/upstream

# 4. Reproduction gate: fresh process rebuild + re-render + diff
tools/blender-reference-export/reproduce_golden.sh
```

The tolerance policy and cross-layer comparison criteria are drafted in
`docs/reference/08-golden-tolerance-and-cross-layer-criteria.md` (pending
ADR-0007). Captures under `fixtures/golden-local/` are gitignored and never
committed (see `fixtures/README.md`).

## Usage

Single file (inside Blender background mode):

```text
blender <source.blend> --background --python tools/blender-reference-export/export_reference.py -- \
    --out <dir> [--contract-version blender-gltf-npr-v1] [--role auto|main|outline|post]
```

Batch (plain python3):

```text
python3 tools/blender-reference-export/run_batch.py --out-root <dir> \
    [--blender /snap/bin/blender] input1.blend [input2.blend ...]
```

Each successful input produces in its output directory:

```text
<slug>.glb                   # runtime export artifact
export-manifest.json         # all fields of docs/reference/03 §Export Artifact Manifest
export-sidecar.json          # bevy_stylized.* namespaced metadata (ADR-0016 namespace)
node-graph-signature.json    # structural digest for upstream-upgrade diffs
```

The batch runner additionally writes `batch-summary.json` into the out root
and exits non-zero if any input failed.

## Fail-closed semantics

Per the normative spec, the bridge exits non-zero and leaves **no partial
outputs** (outputs are staged in a temp dir and moved only on success) when:

- Blender version does not match `docs/reference/reference-toolchain.toml`;
- an expected node group is missing;
- a socket name/type/direction changed on an expected node group;
- node-group topology changed (node types/count, link count);
- a required (unpacked, non-packed) texture is missing;
- a required `_NPR_*` attribute is missing, or a present attribute's
  domain/data type differs from `contracts/npr-attributes.v1.json`;
- a locked exporter setting drifted (unknown key or invalid enum value on
  the current `export_scene.gltf` operator).

Missing attributes on the warn list and unknown `_NPR_*` attributes are
warnings recorded in the manifest, not failures. The v1 contract marks all
`_NPR_*` attributes as warn-if-missing because the vendor Gacha Setup
v3.6.2 reference blends do not author them (they are generated later by
the setup wizard); per-asset requiredness is enforced by the runtime
profile, not this bridge.

## Contract versions

- Export contract: `blender-gltf-npr-v1`
- Manifest schema: `export-manifest-v1` (`contracts/manifest.schema.json`)
- Sidecar schema: `export-sidecar-v1`
- Signature revision: `gacha-setup-v3.6.2-069e51f3fd89`
  (derived from the actual v3.6.2 artifacts on Blender 5.2.1 LTS)
