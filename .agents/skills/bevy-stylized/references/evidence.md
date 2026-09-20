# Current independent implementation and evidence

The repository now owns the full source→authoring→runtime→integration chain.
Six products live under crates; apps, labs, tests and tools have separate ownership.
There is no active legacy renderer or dependency on another game's artifacts.

Current executable evidence:

- tools/dev.py assets prepare verifies raw manifests; build uses prepared inputs.
- tools/blender import/audit/export runs the declared static recipe, bakes upstream
  diffuse/pupil output and extracts traceable NPR maps.
- tools' optional profile-genshin strict compiler rejects partial coverage,
  unsupported active features and conflicting metadata, then commits a new bundle.
- tests/integration/tests/asset_adoption.rs checks actual imported geometry/materials,
  shared assets, despawn/reload and GPU output. The first-party skinned/morph fixture
  has its own case; external art is not a default CPU prerequisite.
- labs/reference compares directly against independently generated upstream images.
  Its strict mode fails on the actual existing parity gap (about 20.96 dB), while
  independent upstream regeneration matches earlier reference pixels exactly.
- tools/checks/isolated.py verifies a Git-free source copy, empty artifact cache,
  disabled networking and absence of sibling projects, including rebuilding the
  external source case. This is engineering independence, not parity acceptance.

Inspect current task reports and docs/release/V1_RELEASE_CHECKLIST.md for the
latest completed checks. Do not treat dated measurements as current-device results.
GPU verification requires a locally available device and the declared inputs.

Primary sources inside the discovered checkout:

- docs/architecture/03-repository-and-crate-architecture.md
- docs/authoring/05-integration-guide.md
- docs/reference/01-reference-authoring-workflow.md
- docs/reference/08-golden-tolerance-and-cross-layer-criteria.md
- crates/bevy-stylized/src/integration.rs
- crates/bevy-stylized-profile-genshin/src/mapping.rs
- crates/bevy-stylized-tools/src/compile.rs
- schemas/authoring and generated schemas/runtime

Obsolete research and migration copies are not operational inputs.
