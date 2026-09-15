# Export and handoff contracts

## Establish input quality

Obtain the source/version manifest, candidate blend and dependencies, import log,
audit, reference views/settings, intended object/material selection and unresolved
findings. These are workflow evidence files, not a runtime schema. If starting
from an existing GLB, inspect that exact file and its provenance instead.

Locate the project's actual paths; the inspected workspace uses:

- `docs/reference/reference-toolchain.toml`: locked Blender/Gacha/artifacts.
- `tools/blender/export_reference.py`: version/signature/texture/
  attribute checks, metadata extraction and GLB export.
- `tools/blender/contracts/`: signatures, exporter settings,
  `_NPR_*` definitions, manifest schema.
- `crates/bevy-stylized-gltf/src/metadata.rs`: typed runtime sidecar/extras.
- `crates/bevy-stylized-tools/src/compile.rs`: compiler behavior.

## Do not collapse these four formats

| Format | Owner / purpose |
|---|---|
| `export-manifest.json` (`export-manifest-v1`) | Bridge provenance, hashes, settings and outputs |
| `export-sidecar.json` (`bevy_stylized.schema_version = export-sidecar-v1.1`) | Reference metadata, namespaced Genshin values and ground truth |
| `<asset>.npr.json` (`schema = bevy-stylized.sidecar/v1`) | Runtime descriptor map keyed by glTF material name |
| Material extras `bevy_stylized.material/v1` | Typed `StylizedMaterialDescriptor` for one material |

The compiler additionally reads `bevy_stylized.compile/v1` material extras or a
sidecar `compile` map for packing instructions. Read the actual payload types;
this is not the Blender ground-truth structure. Extras/sidecar disagreements
produce a conflict rather than an arbitrary winner.

Reference metadata extraction is not a general shader compiler. Establish an
explicit mapping from authored node/rig values to descriptors, renderer parameters,
instance data and view settings. Unknown values remain unsupported/unmapped until
verified. A renamed sidecar cannot provide this conversion.

## Export branch

Use a fresh candidate output directory. Normal command shape:

```sh
blender --background SOURCE.blend --python-exit-code 1 \
  --python tools/blender/export_reference.py -- --out CANDIDATE_DIR
```

Select `--role` and `--signature` only when the source genuinely matches that
contract. Never use fixture mode or weaken a signature to make a real character
pass. The installed add-on version is not necessarily the source blend's version;
record the latter independently. The current bridge cross-checks signature and
lock metadata but must not be assumed to detect every mismatch with the installed
plugin or the actual scene lineage.

Before calling the bridge, inspect export scope. The 2026-09-14 settings use
`export_apply=true`, `use_selection=false`, `use_visible=false`,
`use_renderable=false`, `use_active_scene=false`. Therefore merely hiding helpers
or selecting the character is insufficient. Prepare a runtime-only transient
scene/file, remove upstream outline geometry modifiers from that export copy,
exclude authoring widgets and verify how morphs/skin survive modifier application.
The generic bridge does not perform all of this character-specific preparation.

Gacha Shader-to-RGB graphs are not directly glTF PBR graphs. A suitable exporter
must expose supported base material inputs and preserve NPR maps separately.
The independent `tools/blender/export_character.py` is a scoped static exporter.
Its input/recipe and material restrictions are explicit; it does not promise
arbitrary rigs or effects. Do not use historical game exporters as a fallback.

Read the current `_NPR_*` domain/type contract. The inspected bridge treats missing
attributes as warnings for upstream fixtures; profile requirements determine
whether a character can actually render correctly. Preserve the original `Col`
control data rather than synthesizing all-white colors. Avoid blindly copying
Blender scale or a game's root calibration; measure units and forward after export.

## Validate, inspect and compile

From the verified workspace root:

```sh
cargo run -p bevy-stylized-tools -- validate --json CANDIDATE.glb
cargo run -p bevy-stylized-tools -- inspect --json CANDIDATE.glb
cargo run -p bevy-stylized-tools -- compile --json --out COMPILED_DIR CANDIDATE.glb
```

The built binary is `bevy-stylized`; exit codes are 0 success, 1 contract violation,
2 usage/IO. Validate emitted material counts, semantic mappings, hashes and texture
references. The compiler can create default Cel descriptors from unmapped PBR
materials, with extracted color textures. That is a structural success, not full
Genshin shading conversion. No typed metadata is legitimate only when the consuming
application supplies and verifies explicit material adoption itself.

## Current strict pipeline

The maintained path is assets prepare/build, followed by
`compile --profile genshin --strict` with the optional profile-genshin feature.
It consumes the typed .genshin.json handoff, matches every GLB material, preserves
errors and writes an atomic new GLB/sidecar/texture package. Normal default Cel
compilation remains a separate compatibility path. Runtime schemas are generated
from the Rust types, not manually duplicated in authoring JSON.
