# Bevy Stylized

An independent Bevy rendering framework for Cel, Character and Hero surfaces,
explicit NPR material data, authored lighting, outlines, LOD and capability-based
fallback. The six product crates are separate from development applications,
experiments, asset preparation and GPU verification.

The framework owns stylized rendering. Animation playback, expression control,
retargeting, IK and character physics belong to the application, engine or DCC.
Skinning and morph fixtures validate rendering compatibility with externally
supplied deformation; they do not establish a character animation system.
See the [scope contract](docs/architecture/01-goals-scope-and-three-axis-model.md).

This is a development release. Public API/profile decisions remain provisional;
upstream visual parity and publication qualification are not implied by passing
build or asset-integration tests. See [support](docs/platforms/SUPPORT_MATRIX.md)
and [release gates](docs/release/V1_RELEASE_CHECKLIST.md).

## Build and test

Use the Rust toolchain in `rust-toolchain.toml` and Python 3.11+. Ordinary builds
and CPU tests need neither Blender nor third-party character assets.

```sh
cargo fetch --locked # initial Rust dependency preparation; no character assets
cargo check --locked
python3 tools/dev.py check --lane cpu
python3 tools/dev.py check --lane gpu --required
```

The GPU command requires an actual adapter and accepted first-party baselines.
CPU-only runs explicitly leave GPU tests ignored; they do not claim GPU coverage.
The qualified local backend is currently Linux/Vulkan. Other targets retain their
stated support level until measured.

To open the showcase or capture a procedural scene:

```sh
cargo run -p showcase --features interactive -- overview
cargo run -p showcase -- overview --capture
```

## Independent character workflow

Prepare original files from versioned, hash-verified sources, then build with the
Blender version declared by the recipe:

```sh
python3 tools/dev.py assets prepare --pack synthetic
python3 tools/dev.py assets prepare --pack genshin-odette
python3 tools/dev.py assets prepare --pack gacha-authoring
python3 tools/dev.py assets build --pack genshin-odette --blender blender
```

`build` prints a content-addressed build directory. Use its `compiled/character.glb`
for integration and its authoring audit/reference metadata to inspect scope:

```sh
python3 tools/dev.py check --lane asset-integration --required --asset BUILD/compiled/character.glb
python3 tools/dev.py golden capture --asset BUILD/compiled/character.glb --out target/stylized/character.png
```

Gacha Setup support targets only the latest stable release (currently qualified
v3.6.4), pinned in `assets/sources/gacha-authoring.json`. Retired versions are
historical evidence, not compatibility requirements or acceptance blockers.

The current external recipe is an explicitly restricted static body/face variant,
not full rig/effect compatibility. Third-party raw and derived art lives only under
ignored `assets/local`. No sibling game project, processed game GLB or personal
agent configuration is required. See [authoring](docs/authoring/05-integration-guide.md).

## Interactive character inspection

```sh
cargo run -p showcase --features inspector -- inspect --asset BUILD/compiled/character.glb
```

The [character inspector](docs/showcase/03-character-inspector.md) provides linked
live A/B views, orbit/focus controls, shared lighting, material diagnostics and
versioned screenshot recipes. It uses normal MSAA rendering, independently of the
fixed Blender parity captures.

## Layout and contracts

- `crates/`: six product crates; shaders belong to the render crate.
- `apps/`: runnable showcase.
- `labs/`: independent reference comparisons, benchmarks and experiments.
- `tests/`: shared capture harness and cross-crate integration tests.
- `tools/`, `schemas/`, `assets/recipes/`: reproducible authoring and validation.
- `fixtures/`: first-party test inputs and accepted first-party image baselines.
- `.agents/skills/`: optional repository-owned workflow skills.

Start with [architecture](ARCHITECTURE.md), [contributing](CONTRIBUTING.md),
[reference validation](docs/reference/01-reference-authoring-workflow.md), and
[traceability](TRACEABILITY.md). History is retained under `docs/history` and is
not an executable or normative dependency.

A public rendering-contract change must update its specification, ADR/traceability
and relevant validation. Publication remains disabled pending the existing
license/provenance and release decisions; no registry publication is performed by
asset preparation or developer tasks.
