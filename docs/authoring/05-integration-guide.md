# Independent authoring and runtime integration

## Inputs and preparation

The project does not require another game checkout. Python 3.11+ handles source
manifests and deterministic synthetic fixtures. Blender/Gacha are needed only for
explicit authoring/reference tasks and must match the selected recipe/lock.

```sh
python3 tools/dev.py assets prepare --pack synthetic
python3 tools/dev.py assets prepare --pack genshin-odette
python3 tools/dev.py assets prepare --pack gacha-authoring
python3 tools/dev.py assets build --pack genshin-odette --blender blender
```

Preparation accepts `--manifest` and a hash-verified `--mirror`. Build only uses
already prepared sources; ordinary compile and tests do not fetch missing art.
The result directory is content-addressed and contains the packed source blend,
audit, runtime GLB/maps, profile input, compiled descriptors/sidecar and hashes.
Existing or incomplete candidates are not silently overwritten.

The current recipe explicitly chooses static original-rest geometry without
Rigify/secondary physics, excludes named effect meshes, and disables named
metallic/specular-ramp/normal/emission features in the upstream material variant.
It records those choices and the authoring-only private-method compatibility shim.
It does not claim complete original-rig/effect or pixel-parity support. Pupil
composition is rasterized from the upstream color node output in EEVEE, not copied
from a game exporter; face inputs are extracted from locked upstream shader images.

## Data handoff

| File | Meaning |
|---|---|
| source manifest | Original identities, URLs and file hashes |
| authoring.json / audit.json | Recipe, actual tools, source blend and inspected state |
| runtime-manifest.json | Export scope, image/mesh provenance and limitations |
| character.genshin.json | Typed profile interpretation input |
| character.view.json | Optional authored view settings, including explicit ambient radiance |
| character.npr.json | Runtime `bevy-stylized.sidecar/v1` descriptor map |
| compile-manifest.json | Emitted files and content hashes |

The legacy reference bridge's export-sidecar-v1.1 is a different format. Do not
rename it to .npr.json. Unsupported versions, conflicting sources and incomplete
strict-profile material coverage fail before a compiled package is committed.

## Bevy application setup

Configure the glTF plugin inside the default group, before its PBR extension:

```rust
app.add_plugins(DefaultPlugins.build().set(bevy_stylized::stylized_gltf_plugin()))
   .add_plugins((
       bevy_stylized::StylizedPlugin::default(),
       bevy_stylized::StylizedAssetIntegrationPlugin,
   ));
```

Attach `StylizedMaterialSet` to the application-owned scene root. Its material map
comes from the typed sidecar; `texture_prefix` is relative to the AssetServer root.
Load the GLB scene with the current Bevy scene/world-asset API. The opt-in adopter
waits for material assets, matches exact glTF material names and reports unmapped
materials through `StylizedMaterialAdoption.error`. Equivalent descriptors share
material assets across scene roots; per-instance state remains a component.

The executable integration tests exercise actual scene loading, shared materials,
despawn/reload and GPU capture. This is more than merely parsing a GLB:

```sh
python3 tools/dev.py check --lane asset-integration --required --asset BUILD/compiled/character.glb
```

## Rendering constraints

Opaque/mask and capability-resolved Cel/Character/Hero paths are explicit. Primary
materials enable prepass at the type level, with cost controlled by camera opt-in;
shadow rendering remains disabled. The older prepass-disabled description was
superseded by the implementation. Blend/effect support and Genshin-specific
Metallic equivalence must not be inferred from a material name or a bound texture.

Use the current source, semantics tests and support matrix for precise capability
claims. Asset adoption success is not upstream visual parity. Dynamic view palette
behavior and profile-specific approximations retain their documented provisional
status; no new universal equivalence is claimed by the static recipe.

The static recipe uses neutral material light colors. Its authored ambient is
exported as view schema v1.2 `ambient_override`; applications explicitly choose
whether to apply that view. Loading a material set does not overwrite application
lighting. `None` retains the existing day/night ambient policy and GPU bindings.
The exporter converts upstream day/night (0=day) to the renderer's 1=day convention.

For shared static reference cameras, calibration and truthful parity reports, use
[static character capture](../reference/09-static-character-capture.md).
