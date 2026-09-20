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

Gacha Setup support uses one source-locked release (currently qualified v3.6.4),
recorded in [the source lock](assets/sources/gacha-authoring.json). The external
recipe covers an explicitly restricted static body/face variant. Raw and derived
third-party art stays under ignored `assets/local`.

This is a development release. Public API/profile decisions, broader upstream
parity and publication gates remain open. Current checks do not qualify every
platform or upstream effect. See the [support matrix](docs/platforms/SUPPORT_MATRIX.md),
[provisional inventory](docs/release/03-provisional-inventory.md) and
[release checklist](docs/release/V1_RELEASE_CHECKLIST.md).

## Documentation

- [Architecture](ARCHITECTURE.md): subsystem contracts and source layout.
- [Contributing](CONTRIBUTING.md): local tools, check selection and evidence.
- [Agent instructions](AGENTS.md).
- [Traceability](TRACEABILITY.md) and [decisions](docs/adr/ADR_INDEX.md).
- [Asset build recovery](docs/authoring/06-build-attempts.md).
- [Third-party notices](THIRD_PARTY_NOTICES.md).

Publication remains disabled pending license/provenance and release decisions.
