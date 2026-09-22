# Bevy Stylized

An independent Bevy rendering framework for Cel, Character and Hero surfaces,
explicit NPR material data, authored lighting, outlines, LOD and capability-based
fallback. Six product crates are separate from development apps, experiments,
asset preparation and GPU verification.

The framework owns rendering. Animation playback, expressions, retargeting, IK
and character physics belong to the host or DCC. Skin/morph fixtures verify
rendering compatibility, not a character animation system. See the
[scope contract](docs/architecture/01-goals-scope-and-three-axis-model.md).

## Start here

Use the checked-in Rust toolchain and Python 3.11+. From this directory:

```sh
cargo fetch --locked
CARGO_INCREMENTAL=0 cargo check --locked
cargo run -p showcase --features interactive -- overview
```

Ordinary builds and the procedural showcase need no Blender or third-party art.
Use [contributing](CONTRIBUTING.md) to select local CPU, consumer and GPU checks;
remote CI is not used.

For an application, begin with the [integration guide](docs/authoring/05-integration-guide.md).
For authored characters, follow the [reference workflow](docs/reference/01-reference-authoring-workflow.md)
and [export bridge](docs/authoring/02-blender-reference-export-bridge.md), then use
its compiled asset with the [character inspector](docs/showcase/03-character-inspector.md).
No sibling game checkout or personal agent setup is required.

## Current qualification scope

Gacha Setup support uses one source-locked release (currently qualified v3.6.9),
recorded in [the source lock](assets/sources/gacha-authoring.json). The external
default recipe includes source outlines, metal, specular ramps, optimized normals
and emission for the static body/face scope. Build it with
`python3 tools/dev.py assets build --pack genshin-odette --blender blender`, then
open the returned `BUILD/compiled/character.glb` in the character inspector.
The two transparent effect meshes (`Eff_001`, `Eff_002`) are explicitly excluded.
Raw and derived third-party art stays under ignored `assets/local`; the
[export contract](docs/reference/03-blender-export-contract.md) defines the
material semantics and reduced-feature regression recipes.

Current checks do not qualify every platform or upstream effect; see the
[support matrix](docs/platforms/SUPPORT_MATRIX.md). This is a private
proprietary project: publication is out of scope per the owner decision of
2026-09-23 ([ADR-0017](docs/adr/0017-license-and-provenance.md)).

## Documentation

- [Architecture](ARCHITECTURE.md): subsystem contracts and source layout.
- [Contributing](CONTRIBUTING.md): local tools, check selection and evidence.
- [Agent instructions](AGENTS.md).
- [Traceability](TRACEABILITY.md) and [decisions](docs/adr/ADR_INDEX.md).
- [Asset build recovery](docs/authoring/06-build-attempts.md).
- [Third-party notices](THIRD_PARTY_NOTICES.md).

Publication is out of scope: private proprietary project, owner decision 2026-09-23.
