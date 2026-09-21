# Static character inspector

The existing Showcase application now includes an opt-in static character inspector.
It uses production glTF/material adoption, two live render targets, perspective
orbit cameras, shared light controls and Hero diagnostic views. Neither viewport
is a Blender reference or an accepted regression baseline.

## Run

Build assets using the independent authoring workflow, then use the printed `build`
path as `BUILD` below. The default includes source outlines, metal/specular ramps,
optimized normals and emission, excluding `Eff_001` and `Eff_002`.
A compiled GLB must have matching `.npr.json` and `.view.json` sidecars.
All declared texture/buffer dependencies must be present within that bundle.

```sh
python3 tools/dev.py assets build --pack genshin-odette --blender blender
CARGO_INCREMENTAL=0 cargo run --locked -p showcase --features inspector -- inspect --asset BUILD/compiled/character.glb
```

The `inspector` feature enables the development UI and X11/Wayland window support;
normal library builds do not depend on egui or window creation. No game checkout,
processed game asset, Blender process or AI tool is required at runtime.

- Drag either image to orbit; right-drag to pan; scroll to zoom.
- Full body fits the imported scene bounds in metres. Face focus uses an optional
  `inspector-focus.json` containing a model-space `[x,y,z]` point. Otherwise it uses the bounds of a face-SDF surface when available, then an
  explicitly labeled height-based approximation. No mesh name is required.
- Link cameras to apply interaction from either side to both views.
- Shared lighting controls azimuth, elevation and exposure. Each view independently
  selects Beauty, Lighting factors, Base color, Before palette or Normal/half-vector.
  Diagnostics require Hero descriptors; other surface classes retain Beauty.
  Source hulls are visible in Beauty and hidden independently in diagnostic
  views so their colors do not contaminate lighting-factor output.
- Reset light and Reset inspection restore launch settings; double-click resets
  the selected camera (and its linked peer).
- Save A/B + recipe creates a new take under `target/stylized/inspector`, or `--out`.
  It never overwrites accepted baselines or the source bundle.

The fixed perspective FOV is 45 degrees; each viewport defaults to 1024×1024,
MSAA 4x, Standard/sRGB output without offline supersampling. `--resolution` accepts
128–2048. Display scaling does not silently change render resolution.

## Reproduce and validate

Each take contains `a.png`, `b.png`, `recipe.json`, and `capture.json`. The recipe
records both cameras, debug modes, shared lighting, exposure, resolution and a
SHA-256 inventory. Supply the asset explicitly when replaying; identity checking
allows relocating the whole bundle without embedding a personal asset root.

```sh
cargo run -p showcase --features inspector -- inspect --asset BUILD/compiled/character.glb --config TAKE/recipe.json
python3 tools/dev.py check --lane inspector --required --asset BUILD/compiled/character.glb --out target/stylized/NEW_INSPECTOR_CHECK
```

The developer lane runs without a window, deliberately rebuilds the scene while
readback is pending, verifies a retired capture and saves only the new generation.
`--case TAKE/recipe.json` supplies a saved inspector recipe to this lane.
A missing GPU or incomplete load is not a successful test. The lane checks capture
identity and lifecycle, not upstream visual parity or performance budgets.

`--headless` performs one offscreen capture and exits. `--capture-once` opens the
actual UI, saves both render targets plus `window.png`, and exits. Both require a
new output directory. `--exercise-reload` deliberately exercises the pending
capture cancellation path. These flags also work through the direct Cargo command.

## Ownership and lifecycle

The facade still owns material adoption. Diagnostic variants are app-owned and
shared within each viewport; changing one view's debug mode does not alter the
other view or source descriptors. Both cameras see the same managed key light.

Readiness uses the shared harness pipeline classifier, loaded texture dependencies,
exact adopted primitive count, scene bounds and 24 distinct ready render frames.
Missing/import-pending shaders remain transient; processing failures and finite
load/readback timeouts are visible failures. Recipe/camera/debug changes invalidate
pending captures. Retired GPU requests are drained without prematurely deleting
renderer-owned in-flight work.

Reload scene rebuilds the same immutable compiled bundle. Changed source bytes are
rejected: restart against the new build so GPU resources and declared identity
cannot disagree. Reload launch recipe rereads `--config`, checks exact bundle
identity, and permits changing camera/light/debug settings. Changing resolution
requires a restart because it changes the render targets.

Full-body bounds currently use transformed mesh AABBs for initial static framing.
They do not qualify dynamic skinned/morph bounds. External deformation tests remain
separate. The inspector adds no animation playback, expression control, IK, cloth
solver or gameplay systems.

## Relationship to reference validation

The fixed six-case Blender comparison continues through `character-reference` and
its existing versioned capture packages. Interactive fitting never writes those
camera records. The shared readiness refactor changes no shader formulas or image
thresholds. Gacha Setup acceptance continues to target only the latest stable
qualified release.

Camera/viewport interaction and generation-safe capture were informed by the
[current attribution](../../THIRD_PARTY_NOTICES.md); the
inspector is implemented within this repository and has no sibling dependencies.

## Internal ownership

The inspector entry module assembles the app. `options.rs` owns CLI options;
`runtime.rs` owns scene lifecycle, view synchronization, fitting and readiness.
`state.rs`, `panel.rs`, `capture.rs` and `validation.rs` retain recipe/UI, generation
capture and replay/measurement responsibilities. The module move preserves update
ordering, MSAA, warm-up counts and request retirement.

Its input policy remains separate from the fixed reference package; see
[reference/inspector input boundaries](../testing/03-capture-input-boundaries.md).

Capture metadata uses `inspector_source_policy = module-sources/v2`. Its SHA-256
includes every inspector module in lexical relative-path order, hashing each
filename and source with NUL separators. A source inventory test guards newly
added modules. This corrects the old incomplete source list; screenshot pixels
and generation/revision rules are unchanged.
