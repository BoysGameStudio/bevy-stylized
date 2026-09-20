# Static character capture packages

The versioned development contract `bevy_stylized.character-capture/v1`–`v3` lives under
schemas/authoring. It is separate from runtime material/view schemas and from the
retired synthetic reference. These revisions identify current pipeline stages:
initial cases, lighting diagnostics, then matched samples. The current recipe uses
the source-locked Gacha release.

Build prepared original sources with the existing asset workflow. The build now
also emits `character-cases/case.json`, six two-repeat Blender captures, evaluated
light/camera records, two geometry coverage images, and a first-party calibration
fixture exported through glTF. Source scenes and accepted baselines are untouched.

```sh
python3 tools/dev.py assets build --pack genshin-odette --blender blender
python3 tools/dev.py check --lane character-reference --case BUILD/character-matched/case.json --out target/stylized/NEW_RUN
python3 tools/dev.py check --lane character-reference --required --case BUILD/character-matched/case.json --out target/stylized/NEW_STRICT_RUN
```

Both commands require a new output directory and explicit prepared input. Diagnostic
success means the report was generated, not that parity passed. Strict returns 1
on failed qualification. Missing resources or inability to capture returns 2;
invalid inputs rejected by the Python preflight return 1. `index.html` provides
paired images, blinking comparison, difference images and the complete metrics.

## Identity and readiness

The package inventories runtime GLB, descriptors, view, textures, authoring/audit
records, calibration inputs and reference outputs with SHA-256. The Python entry
and Rust lab check package identity before GPU capture. Python also validates
upstream actual settings independently of the requested camera and checks declared
texture inventory. Rust checks actual upstream camera/direction and actual runtime
camera, orthographic extent, exposure and directional-light transform.

The lab uses the real product plugin/material adoption path. It waits for scene
and declared textures, complete material adoption, successful pipeline compilation
and 24 consecutive ready updates before capture, with a 60-second readiness limit.
Shader-not-loaded/import-not-yet-available are transient states; compilation errors
are failures. `STYLIZED_REFERENCE_TIMEOUT_MS` can shorten readiness for negative
verification. Capture itself has a separate 30-second limit.

Cameras are computed once from evaluated Blender geometry and converted from
Blender `(x,y,z)` to glTF `(x,z,-y)`. Bevy does not compute a second fit. Both sides
use 512×512, linear background 0.12, unit exposure and Standard/sRGB output without
an extra tonemapper. Sampling methods are recorded: EEVEE samples and Bevy MSAA 4x
are distinct algorithms, not a claim of identical rasterization.

## Light evidence and calibration

`lightDir` is an evaluated Euler attribute. The locked upstream Dot Creation
rotates -Z; Face Factor rotates +Z. Those conventions remain separately recorded.
The controller's rotation is not copied directly into a unit vector. Head basis
attributes are audited as static inputs; no animation or expression controller is
implemented. Disabling the additional Sun is a single-variable reference probe.

Six first-party emissive markers exercise exported geometry, asymmetric screen
positions and linear-to-sRGB colors. Before character capture, the lab checks their
center colors (≤2/255) and projected centers (≤1 pixel). This calibration does not
qualify the full NPR shading formula.

## Metrics and qualification

Legacy Tier-2 metrics retain their algorithm and thresholds. Its median-luminance
shadow mask remains a diagnostic proxy; it cannot qualify true character shadows.
V1 retains the historical open semantic-shadow gate. V2 adds diagnostic passes.
V3 qualifies sharp NPR lighting factors and center-sampled geometry independently;
median-luminance and unmatched AA coverage remain visible as legacy diagnostics.

`white-geometry-coverage/v1` is an independent silhouette diagnostic for this opaque
static variant: each renderer draws its own geometry in white, with a fixed sRGB
threshold 197 over linear 0.12 background. The Bevy diagnostic temporarily uses
mesh copies without vertex colors because those colors carry NPR controls; it
restores the original meshes/materials afterward. Both foreground masks must contain
at least 100 pixels, and IoU must reach 0.99. This does not qualify alpha-mask,
outline expansion, dynamic bounds or cast-shadow semantics.

The report separates repeatability, calibrated inputs, coverage, legacy numerical
metrics and overall qualification. It retains full-frame violations; differences
are not cropped out to improve scores. Cross-renderer dimensions must match in both
axes, not merely byte count. V3 requires real lighting-factor masks; it does not qualify cast shadows or a
character control system.

## Recipe v4 and cross-build determinism

`genshin-static-v4` keeps the v2 geometry/feature scope and resolves each body/hair
material's ramp by the original JSON `_PackedShadowRampTex.Name`. The previous
upstream import could overwrite shared Body ramp groups in directory iteration
order, producing different Body/Body01 ramps on a clean rebuild. The authoring
adapter copies the actual shader dependency closure per material and binds the exact verified
PNG; it records JSON/texture hashes in `authoring.json.ramp_bindings`. This also
preserves Dress01's explicitly named Body01 ramp. It does not modify the global
add-on or the original files. The v2/v3 recipes and candidates remain historical
evidence; they are not silently restamped with the new recipe identity.

The v3 experiment bound only the top-level ramp preview. Inspection proved that
this preview was disconnected: the body shader used nested shared ramp groups.
V4 binds both the actual render-connected groups and the preview, and the exporter
reads the actual shader closure. Older recipes are rejected by the current importer
instead of silently inheriting the new policy. Frozen saved-scene reproduction
was pixel-identical across host and isolation; cross-import reproduction is checked
separately after this fix.

## Matched sampling and actual lighting factors (v3)

The maintained asset build additionally emits `character-diagnostics/case.json`
and `character-matched/case.json`. `tools/blender/lighting_probe.py` reads the
frozen source shader's exposed Lit Factor and Lit Sharp Factor outputs. The
product Hero debug view renders the corresponding runtime factors. Blue identifies
geometry; green identifies sharp lighting. A nonempty sharp-shadow mask must reach
IoU 0.95. Independent center coverage must reach IoU 0.99. Both passes disable AA
and use black backgrounds; they do not inherit the legacy white-mask AA domain.

`tools/blender/matched_capture.py` renders a regular 4× grid per output pixel into
linear RGBA32F. Bevy uses the same grid in RGBA16F. Each side averages linear HDR
samples before clipping/display conversion and writes 512×512 Standard/sRGB.
The supported grids are 2, 4 and 8; the exact grid and producer hash are inventoried.
Source and runtime each capture twice. The lab recreates cameras when switching
between HDR beauty and LDR diagnostic targets to prevent retained view state.

The RGB limits remain max nonedge 16/255, mean 4/255 and PSNR 30 dB. No error-derived
mask, crop, rebaseline or tolerance increase is used. A matched package with an
unknown diagnostic/sampling policy is rejected before qualification. Different
sampling grids are different candidate captures, not replacements for accepted
baselines.

## Static Genshin handoff v2

The exported handoff preserves actual AO toggles, band parameters, evaluated head
basis, face-control textures/tints and the UV-only pupil highlight closure. Runtime
uses the source band order 1,4,3,5,2; PNG ramp rows account for Blender's opposite
UV orientation. Vertex AO/width controls use fragment-stage `pow(value,0.4545)`;
zero-width division follows the authored safe divide. Ramp modulation, soft/sharp
palette factors and nonmetal specular are distinct operations.

Raw ILM and face controls use cubic B-spline reconstruction at level zero. Authored
shadow ramps use closest sampling. The exporter rejects changed control sampling
modes. Direct diffuse images retain their original data with an explicit
`cubic_base_color` flag; composite diffuse inputs retain a traceable bake. Cubic
reconstruction costs four bilinear taps per affected texture and adds no bindings.
Ordinary Hero materials keep their existing sampling path. The pupil highlight
shares a mutually exclusive control binding with face controls and retains depth
writes during blending. Metallic/normal/spec-ramp features remain outside this
static recipe. These are rendering inputs, not animation or expression systems.

Orthographic specular uses the camera's world-space +Z basis, constant across the
image; perspective uses normalized camera-minus-surface position. Projection depth
rows are unsuitable because reverse-Z changes their sign. The source convention
was checked against EEVEE's `coordinate_incoming` / `drw_world_incident_vector`
implementation and actual Gacha Dot Creation nodes. A first-party GPU fixture
checks equal orthographic NdotH on separated surfaces.
