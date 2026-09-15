# Static character inspector — first batch execution

The first migration batch from the Tenacity review is complete within the existing
`apps/showcase` application. No sibling crate, game runtime, processed game asset,
animation controller or cloth solver was introduced.

## Delivered

- `inspect --asset` loads a compiled GLB plus validated material/view sidecars and
  a contained dependency inventory through the product facade.
- Two live viewports offer orbit, zoom, pan, full/approximate-face framing and linked
  cameras. Shared light direction/exposure and independent Hero debug modes are
  available through the optional egui panel.
- Captures save both images, camera/light/debug recipes, asset hashes and inspector
  source identity in new directories. Config replay rejects a changed asset identity.
- Scene reload retires capture consumers, preserves in-flight renderer work, drains
  obsolete results and only permits captures after the new generation is ready.
- `tests/harness/src/readiness.rs` contains the shared pipeline classifier used by
  the reference lab and inspector. Shader-loading transients remain distinct from
  fatal errors. No product shader or image threshold changed.
- `tools/dev.py check --lane inspector` exercises pending-capture scene reload;
  the standard isolation job now includes this lane and inspector feature tests.

Use the [inspector manual](../showcase/03-character-inspector.md). The application
entry is `cargo run -p showcase --features inspector -- inspect --asset BUILD/compiled/character.glb`.
Source attribution remains in the [review](../research/tenacity-rendering-reuse-2026-09-15.md)
and root third-party notices. The reviewed interaction and lifecycle behavior were
adapted to the current facade/harness instead of copying an alternate renderer.

## Evidence

[Machine-readable validation](../quality/inspector-validation-2026-09-15.json) records
source identities, exact outcomes and artifact paths. Five new inspector tests
and the existing Showcase test pass with the inspector feature. The required GPU
suite retains all 19 passes, and the latest Gacha static six-case comparison still
passes in isolation. Both inspection images are pixel-identical after saved-recipe
replay and after independently rebuilding original assets in the restricted source
copy. Negative tests reject altered identity and report missing GPU as blocked.

The actual Wayland UI was launched, captured and visually inspected at
`target/stylized/inspector-window-final/window.png`. No baseline acceptance,
commit or publication was performed. General real-time performance measurement,
additional diagnostic channels and dynamic deformation-contract coverage remain
separate subsequent batches; this delivery does not claim those milestones.
