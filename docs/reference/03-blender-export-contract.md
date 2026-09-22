# Blender export contract

This document owns the Blender-to-runtime boundary. The
[export bridge](../authoring/02-blender-reference-export-bridge.md) owns build stages;
the [reference workflow](01-reference-authoring-workflow.md) owns reproduction.
ADR-0012 is Accepted (2026-09-22, current static character scope); ADR-0020
remains Proposed. Describing current mechanisms does not freeze a broader
interchange API.

## Current static character export

The default recipe is `assets/recipes/genshin-static-emission-v1.json`. It includes
all qualified static material branches described below: source outlines, metal,
specular ramps, optimized normals and emission. `Eff_001` and `Eff_002` are
explicitly excluded; their shader reconstruction is not a pending requirement
for this workflow. The producer is
`tools/blender/export_character.py`, called by `tools/assets/build.py` with explicit
prepared inputs. The recipe number is not the Gacha release; source identities
come from `assets/sources/gacha-authoring.json` and the source pack manifests.

The exporter verifies the authoring blend identity and requires a fresh output
candidate. It evaluates selected recipe meshes, removes upstream GN outline
modifiers, transforms evaluated geometry into world coordinates and maps declared
body/face material roles. Production outlines are reconstructed by the renderer.
This is the declared static scope, not an animated character export promise.

The reduced-feature `genshin-static-outlines-v1` recipe isolates source hulls
against the neutral static body/face control. It is retained for regression
comparisons; ordinary builds use the complete default above. Select this control with:

```sh
python3 tools/dev.py assets build --pack genshin-odette --recipe assets/recipes/genshin-static-outlines-v1.json --blender blender
```

`outline_handoff.py` reads the modifier's material assignments, width and `Col`
alpha, five source colors/class switches, gamma and cubic/repeat class map. The
variant declares neutral outline tint, saturation, intensity and lighting. Screen
space width, depth offsets, distance clamps, alpha cutout and non-unit object
scale are rejected. Source/shader identities remain pinned by the recipe.

The source GN graph welds vertices before extrusion. The exporter evaluates that
graph and transfers its hull target positions onto the unchanged base mesh through
`_NPR_CONTROL0` (POINT / Float32 VEC3), in glTF-local coordinates. The generic
control-vector contract retains its meaning elsewhere; only the explicit
`outline_source_position` material parameter enables this static interpretation.
Surface positions, shading normals, UVs and control colors are preserved. There
is no baked second hull mesh: the runtime outline pass draws the shared mesh using
the source targets. The strict compiler and runtime reject missing target data
or skin/morph deformation for this variant.

The optional typed `outline` material input maps to `outline_source_width`,
`outline_gamma`, `outline_enabled_mask`, `outline_force_first`, the five RGB
`outline_color_ROW_CHANNEL` values and an optional `outline_class_texture_index`.
The classification texture belongs exclusively to the outline pass; it must not
enable face shading. Per-object selection is retained even when source materials
are shared: a surface without a source hull receives a separate descriptor without
outline parameters. The facade shares hull materials across equivalent instances
and clears only its own hulls on an explicit material rebind.

Beauty comparisons include hulls. Existing base-surface geometry and lighting
diagnostics exclude them on both renderers, preserving those masks' original
meaning and all numerical thresholds. This static export does not qualify animated
source targets or arbitrary upstream geometry-node graphs.

## Authored metal and specular ramp

`genshin-static-metal-v1` restores the source metal switches on the outlined
variant; `genshin-static-metal-ramp-v1` additionally restores the source specular
ramp switches. Each removes only the corresponding disabling recipe override.
It does not force the feature on every material. Build the combined variant with:

```sh
python3 tools/dev.py assets build --pack genshin-odette --recipe assets/recipes/genshin-static-metal-ramp-v1.json --blender blender
```

`metal_handoff.py` reads the active body-shader inputs and the actual nested
MetalMap/specular images. The typed optional `metal` input carries brightness,
horizontal tile scale, dark/light/shadow colors, shininess/scale/attenuation/sharp
threshold, specular colors and explicit image paths. Enabling metal without its
complete lighting/texture inputs fails strict compilation. Active normal maps
remain unsupported by this recipe.

The metal map uses cubic reconstruction, repeat addressing and sRGB decoding.
Its color-to-factor conversion is linear Rec.709 luminance, matching the source
Mix input. Specular Ramp uses linear/repeat sampling of **Non-Color data**;
`specular_ramp_index` and `specular_ramp_linear` distinguish it from the sRGB
shadow ramp. It replaces the scalar specular response, then receives the authored
ILM blue-channel multiplier before the sharp-layer override and shadow attenuation.
It is separate from the generic Hero specular-ramp additive effect.

The runtime `metal_controls` vector carries enable, brightness, tile scale and
the source ramp switch. The remaining `metal_*` vectors retain the source colors
and specular parameters before their gamma nodes. A zero light/view half-vector
stays zero; it must not become a full-strength specular highlight.

## Authored optimized normals

`genshin-static-normals-v1` additionally removes the normal-map disabling override
while retaining each material's source switch:

```sh
python3 tools/dev.py assets build --pack genshin-odette --recipe assets/recipes/genshin-static-normals-v1.json --blender blender
```

The optional typed `normal` input names a verified image, strength and the explicit
`genshin-optimized-bump/v1` method. Active normals without this input, an unknown
method or invalid strength fail profile compilation. `normal_handoff.py` checks
cubic/repeat Non-Color sampling, the shared front/back image, active UV0 and the
locked inverted Bump nodes (distances 1 and 1000).

The texture selects UV0 on front faces and UV1 on back faces; its derivative basis
always comes from UV0. The exporter retains the UV pair and the renderer accounts
for glTF's inverted V coordinate. Expanded R/G reconstruct Z as
`sqrt(max(1-x*x+y*y,0))`, matching the source graph. This is not ordinary tangent
XYZ normal-map decoding. The source's backface correction restores the same
geometric basis rather than negating the decoded vector.

Dot Creation normalizes the resulting vector for NdotL/NdotH, while the metal map
uses its raw magnitude. Bump-vector normalization uses no fixed world-space
epsilon because those vectors shrink with pixel area. The existing generic
packed-normal path remains separate; `source_conventions_0` selects the authored
method and carries its strength. Source normals do not alter outline hull targets.

## Authored emission and effect boundary

The default `genshin-static-emission-v1` removes the remaining `Use Alpha` and
`Emission Strength` overrides from the normal variant. It retains the source
switches; Odette enables emission on Body, Body01, Hair, Dress and Dress01.

```sh
python3 tools/dev.py assets build --pack genshin-odette --blender blender
```

`emission_handoff.py` requires an unbaked diffuse image and a shared color/alpha
UV path. The direct-image detector counts distinct image nodes, since each image
feeds both Color and Alpha sockets. The typed optional `emission` input carries
`genshin-alpha-emission/v1`, tint and strength. Unknown methods, missing body
lighting, baked alpha or invalid radiance fail compilation. Transparency mode
is rejected by this adapter.

For mask `m = clamp(diffuse_alpha - 0.03, 0, 1)`, the source closure is
`lit * (1-m) + diffuse_rgb * tint^2.2 * strength * m^2`. The runtime
`source_emission` vector carries gamma-decoded tint times strength in RGB and an
independent enable in W. Enabled zero strength still attenuates the lit closure.
This branch runs before the external pupil highlight layer. It retains opaque
surface coverage and does not enable generic additive Character emission or the
separate authored-energy policy.

The two excluded Odette effect meshes remain in the source blend. In the inspected
3.6.9 import, their materials use the GI Body group with Alpha disabled and noise
or mask images in the diffuse slot. Their raw effect JSON declares separate
dissolve/mask/noise/Fresnel behavior that this import does not reconstruct.
Showing those opaque fallback materials would not qualify the original effects.
Further effect work needs an explicit source graph and texture/control mapping,
then transparent ordering/depth and time-dependent reference cases; it cannot be
enabled by treating diffuse Alpha as ordinary opacity.

| Surface | Current producer behavior |
| --- | --- |
| Container and axes | GLB, glTF Y-Up |
| Selection | Current runtime scene and its selected evaluated objects |
| Geometry | Meshes evaluated before export; `export_apply=false` on the resulting objects |
| Animation / skin / morph | All disabled for this recipe; separate first-party fixtures cover runtime deformation compatibility |
| Attributes / extras | Enabled; active vertex color mode is checked against exporter RNA |
| Cameras / lights | Excluded from runtime GLB; reference capture owns its camera and lighting |
| Material interpretation | Explicit static recipe mapping; unsupported source sampling or semantics fail rather than silently approximate |
| Output lifecycle | Staged into a new candidate; existing outputs are rejected |

The script owns exact exporter arguments; no second JSON settings table overrides
it. UVs, normals, tangents and texture semantics must satisfy the selected profile
and strict compiler. Supporting another authoring shape needs its own explicit
recipe and qualification.

## Artifact ownership

| Artifact | Producer / consumer responsibility |
| --- | --- |
| `authoring/character.blend`, audit and reference captures | Preserve the scoped upstream scene and independent visual evidence; not shipping runtime dependencies |
| `runtime/character.glb` | Static runtime geometry and standard glTF material inputs |
| `runtime/character.genshin.json` | `bevy_stylized.genshin-input/v2`: explicit material inputs, recipe variant, palette and day/night value consumed by strict profile compilation |
| `runtime/runtime-manifest.json` | `bevy_stylized.runtime-export/v1`: authoring/script/GLB identities, recipe, texture evidence and limitations for this export stage |
| `compiled/` | Strict compiler's GLB and material descriptor output |
| `build-manifest.json` | `bevy_stylized.build/v1`: input identities and hashes after every required build stage succeeds; owned by the build runner |

A runtime-export manifest is not the complete-build acceptance marker. A complete
build is not proof of image parity, gameplay acceptance or platform qualification.
Preserve input, tool/script, recipe and output identities so drift can be attributed
to the owning stage; preserve camera/color/backend identity with reference captures.

## Generic NPR attributes and metadata

[Authoring schema and packing](../authoring/03-authoring-schema-runtime-packing.md)
owns semantic fields; [glTF registration](../authoring/04-bevy-gltf-import-contract.md)
owns registration before loader creation. The current attribute inventory is
`schemas/authoring/npr-attributes.v1.json`; runtime requiredness belongs to the
selected profile. A generic inventory does not imply that every source asset
provides every attribute. Do not silently invent absent semantic data.

Extras and generic sidecars are distinct input channels. The importer exposes
conflicts rather than silently choosing a winner; ADR-0020 retains precedence
policy ownership. `bevy_stylized.compile/v1` is a compiler directive namespace,
not the character profile handoff or either completion manifest. ILM channel
meaning is defined by the [Gacha behavior contract](07-gacha-setup-behavior-contract-v1.md)
and compiler implementation, not copied into an export-settings checklist.

## Unresolved generalization and schema ownership

Attribute quantization, shader-location policy, generalized unit/negative-scale
validation and broad exporter support remain the explicit follow-up scope named
by ADR-0012 (Accepted 2026-09-22) and the actual profile implementation. Current
static qualification does not settle those choices.

The current authoring schemas cover source manifests, static recipes, capture
contracts and the NPR attribute inventory. The inventory generates the loader's
attribute contract; it remains an active source. Fixture/golden provenance may
name `blender-gltf-npr-v1` as its export contract identity, which is separate from
runtime-export and complete-build record schemas.

The build runner hashes all current authoring schemas into its content key.
Removing obsolete schema inputs changes the next build identity; existing cached
artifacts keep their original provenance and are not relabelled as a new build.
