# Independent reference and authoring workflow

> Status: Current normative workflow. Only the source-locked current authoring target is supported.

There are two renderer sources and three forms of evidence: upstream Blender/Gacha
behavior, regression against accepted production images, and independent semantic
fixtures/assertions. The current production image is never used to manufacture its
own upstream truth. The retired engine-reference implementation is not compiled.

## Current upstream acceptance

Only the latest stable Gacha Setup release is a support target. Verified on
2026-09-21: [v3.6.9](https://github.com/PaoloESAN/gacha-setup/releases/tag/v3.6.9),
with Blender 5.2.2 LTS.
The exact qualified archive/hash is pinned in `assets/sources/gacha-authoring.json`;
ordinary builds do not silently download a moving version. When upstream releases
a new stable version, qualify a fresh candidate and advance this single target.
No compatibility matrix for retired Gacha versions is required.

```sh
python3 tools/dev.py assets prepare --pack genshin-odette
python3 tools/dev.py assets prepare --pack gacha-authoring
python3 tools/dev.py assets build --pack genshin-odette --blender blender
python3 tools/dev.py check --lane character-reference --required --case BUILD/character-matched/case.json --out target/stylized/NEW_RUN
CARGO_INCREMENTAL=0 cargo run --locked -p showcase --features inspector -- inspect --asset BUILD/compiled/character.glb
```

Use the build command's returned `build` directory as `BUILD` for both validation
and preview. Ordinary builds and `check --lane independence --required` select
`genshin-static-emission-v1`: source outlines, metal/specular ramps, optimized
normals and emission. No feature-stage selection is needed. `Eff_001` and
`Eff_002` are explicitly excluded from this workflow and require no further input.
The original geometry remains in the source blend.

The current static scope and unchanged image gates are defined by the
[capture contract](09-static-character-capture.md). The isolation gate rebuilds
this workflow from raw inputs and requires strict parity to pass. A rendering
failure is not accepted merely because it was already known. The reduced-feature
recipes below remain explicit regression controls for the same locked source;
their dated measurements retain their original scopes.

## Initial v3.6.9 static qualification

The v3.6.9 candidate was rebuilt from the verified original Odette source pack.
Its Genshin shader, outline/compositor libraries and material-data applier match
the previous qualified bytes. The runner now marks its existing private Blender
process as an upstream isolated worker so the new asynchronous UI launcher does
not return before import finishes; see [build attempts](../authoring/06-build-attempts.md).
The static scope, material restrictions and image thresholds are unchanged.

The local `gacha369-character-reference` strict run passed all six matched
full-body/face lighting cases on NVIDIA GeForce RTX 5090 / Vulkan. Minimum PSNR
was 51.55 dB, maximum nonedge channel error was 11/255, minimum sharp-light IoU
was 0.99871 and minimum center-coverage IoU was 0.99995. Both renderers' repeated
captures were pixel-identical. The legacy unmatched-AA full-body coverage
diagnostic remains below 0.99 (0.98289); it is retained separately from the
existing v3 matched center-coverage gate. This qualifies the declared static
variant, not full rigs, secondary physics, outlines or excluded effects.

Local CPU, all 32 registered GPU cases, synthetic/declared asset integration and
inspector reload checks passed. `gacha369-independence-fixed` also passed the full
network-disabled isolation workflow using the standalone registry dependency
graph, including raw-source rebuild and all six strict comparison cases. Its
twelve Blender/Bevy beauty images match the host build pixel-for-pixel. A coplanar
overlap in the day/night ramp test was corrected by separating its three swatches;
the original color assertions remain unchanged. Performance was smoke-tested
only; no new performance budget or platform qualification is implied.

## Optional source outlines

`assets/recipes/genshin-static-outlines-v1.json` adds source-derived outlines to
the same restricted static variant. The no-outline control recipe and its accepted
images stay unchanged. Use the explicit recipe selection in the
[export contract](03-blender-export-contract.md), then validate the returned build's
asset and matched cases with the same required lanes.

The local `gacha369-outline-reference-3` run passed all six unchanged strict image
criteria: minimum PSNR 51.58 dB, maximum nonedge channel error 12/255. Source colors,
class-map sampling and source welded/extruded vertex positions are preserved;
EyeStar remains unoutlined as in the source even though it shares a face material.
The static hull targets do not qualify skin/morph deformation, alpha-cutout hulls,
screen-space source outlines or the excluded effect meshes.

## Optional metal and specular ramp

The metal-only and combined metal/ramp recipes retain the static source outlines.
Both passed all six unchanged strict cases in `gacha369-metal-reference` and
`gacha369-metal-ramp-reference`. The combined variant restores six authored metal
materials, five with active specular ramps; minimum PSNR was 51.59 dB and maximum
nonedge error was 12/255. Texture sampling and the typed handoff are specified by
the [export contract](03-blender-export-contract.md).

Normal maps, emission and excluded effect meshes remain outside this qualification;
neither these image results nor static hull targets qualify animated deformation.

## Optional optimized normals

`genshin-static-normals-v1` restores the five authored normal-map materials on the
combined outline/metal/ramp variant. `gacha369-normal-reference` passed all six
unchanged strict cases, with minimum PSNR 51.59 dB and maximum nonedge error 12/255.
A native Blender plane probe records the front/back basis and raw versus normalized
vector behavior; the export contract specifies its exact decoding and UV policy.
The face, brow and pupil retain their source normal switches. This extends the
static material scope; emission, excluded effect meshes and animation remain
outside this qualification.

## Default complete static material recipe

`genshin-static-emission-v1` restores the five source-enabled body/hair/dress
emission materials on the normal variant. Diffuse Alpha remains an emission mask,
not opacity; the source blend retains the excluded effect geometry. Native Blender
probes cover disabled, zero-alpha, partial/full-alpha, tinted and zero-strength
closures. The typed handoff and formula are specified by the
[export contract](03-blender-export-contract.md#authored-emission-and-effect-boundary).

The local `gacha369-emission-reference` run passed all six unchanged strict cases.
The direct diffuse path now preserves original image Alpha instead of taking a
color-only bake when Color and Alpha share the same image nodes. All six beauty
pairs were visually inspected. This qualifies static emission without bloom;
it does not qualify source transparency, effect meshes or animation.

An unmodified upstream-effect diagnostic shows opaque patterned shells over the
skirt. Gacha 3.6.9 assigns GI Body materials with noise/mask diffuse images and
Alpha disabled, leaving the raw effect shader's dissolve/Fresnel behavior absent.
Those fallback shells are evidence of an authoring gap, not a runtime reference
for the original effects. The export contract describes the required separate
adaptation boundary.

The `gacha369-emission-full-isolation-2` run additionally rebuilt the emission
variant from verified raw inputs with networking disabled, no sibling repositories
and an empty character cache. CPU, registered GPU cases, asset adoption, inspector,
realtime replay and all six strict character cases passed. All twelve source/runtime
beauty images were pixel-identical to the reviewed host captures. Performance was
smoke-tested only. This is now the default for `check --lane independence --required`; see
[isolation setup](../../CONTRIBUTING.md#isolation). This closes the independent
rebuild gap for the supported static variant. The two effect meshes are outside
the agreed scope and do not block its acceptance.
