# Export and handoff contracts

## Establish inputs and scope

Locate the actual project root. Read the current source locks, recipe, authoring
audit and producer before selecting an export path. Source version, installed
add-on version and an accepted reference capture are distinct facts. For an
existing GLB, inspect that exact file and its provenance.

The maintained workspace uses:

- `assets/sources/gacha-authoring.json` and the character source manifest for qualified identities;
- `assets/recipes/genshin-static-v4.json` for the current restricted static scope;
- `tools/assets/build.py` for the full pipeline and completion marker;
- `tools/blender/export_character.py` for the runtime export stage;
- `schemas/authoring/npr-attributes.v1.json` for the generated attribute contract;
- `crates/bevy-stylized-gltf/src/metadata.rs` and the tools compiler for typed runtime inputs.

## Keep artifact responsibilities distinct

| Artifact / schema | Role |
| --- | --- |
| Authoring blend, audit and reference captures | Scoped upstream source and independent visual evidence |
| `character.genshin.json`, `bevy_stylized.genshin-input/v2` | Material/recipe/palette handoff to strict Genshin compilation |
| `runtime-manifest.json`, `bevy_stylized.runtime-export/v1` | Export-stage identities, texture evidence and limitations |
| `build-manifest.json`, `bevy_stylized.build/v1` | Complete-build input identities and generated artifact hashes |
| `<asset>.npr.json`, `bevy-stylized.sidecar/v1` | Runtime descriptor map keyed by material name |
| Material extras `bevy_stylized.material/v1` | One typed material descriptor |

`bevy_stylized.compile/v1` is a compiler directive namespace. Fixture/reference
sidecars and provenance tags are not interchangeable with these current handoffs.
Read the payload types; renaming a JSON file cannot convert its meaning.
Extras/sidecar disagreements require an explicit diagnostic and resolution.

## Build the current character candidate

From the verified project root, with its declared tools and qualified sources:

```sh
python3 tools/dev.py assets prepare --pack genshin-odette
python3 tools/dev.py assets prepare --pack gacha-authoring
python3 tools/dev.py assets build --pack genshin-odette --blender blender
```

Use the runner's returned build directory and `compiled/character.glb`. It supplies
stage paths and options, creates a new immutable attempt and preserves failures.
A runtime-export record alone is not the full build's success marker.

The current exporter evaluates recipe-selected meshes into a separate runtime
scene, removes upstream outline geometry, applies the evaluated world transforms,
and exports the selected runtime objects. It uses `export_apply=false` because
geometry was evaluated before export. Animation, skin and morph export are disabled
for this static recipe. Cameras/lights stay with the reference captures. This
workflow does not qualify arbitrary rigs, effects or another authoring scope.

Preserve authored UVs, control colors, texture semantics and intended material
coverage. Runtime profile requirements decide whether an absent attribute is
acceptable; generic schema defaults cannot manufacture missing art semantics.
Measure units and forward axes instead of copying another game's calibration.
Never substitute a production screenshot for an independent upstream reference.

## Validate and compile

```sh
CARGO_INCREMENTAL=0 cargo run -p bevy-stylized-tools -- validate --json CANDIDATE.glb
CARGO_INCREMENTAL=0 cargo run -p bevy-stylized-tools -- inspect --json CANDIDATE.glb
CARGO_INCREMENTAL=0 cargo run -p bevy-stylized-tools --features profile-genshin -- compile --profile genshin --strict --out COMPILED_DIR CANDIDATE.glb
```

Strict Genshin compilation needs the matching `.genshin.json` and complete
material coverage. Generic Cel compilation is a separate supported asset path;
its default descriptors do not establish a Genshin conversion. Explicit host-side
material adoption can legitimately consume assets without runtime descriptors,
but that host mapping must be validated.

Inspect emitted materials, textures, schema identities and limitations as well
as exit status. Build success, runtime integration, GPU correctness, visual parity
and performance are separate qualifications. Runtime schemas come from Rust types;
do not maintain another handwritten copy or relax signatures/goldens to pass.
