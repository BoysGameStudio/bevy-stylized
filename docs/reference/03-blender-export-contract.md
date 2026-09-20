# Blender export contract

This document owns the Blender-to-runtime boundary. The
[export bridge](../authoring/02-blender-reference-export-bridge.md) owns build stages;
the [reference workflow](01-reference-authoring-workflow.md) owns reproduction.
ADR-0012 and ADR-0020 remain unresolved; describing current mechanisms does not
freeze a broader interchange API.

## Current static character export

The active recipe is `assets/recipes/genshin-static-v4.json`. The producer is
`tools/blender/export_character.py`, called by `tools/assets/build.py` with explicit
prepared inputs. The recipe number is not the Gacha release; source identities
come from `assets/sources/gacha-authoring.json` and the source pack manifests.

The exporter verifies the authoring blend identity and requires a fresh output
candidate. It evaluates selected recipe meshes, removes upstream GN outline
modifiers, transforms evaluated geometry into world coordinates and maps declared
body/face material roles. Production outlines are reconstructed by the renderer.
This is the declared static scope, not an animated character export promise.

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

## Unresolved generalization and retained schema files

Attribute quantization, shader-location policy, generalized unit/negative-scale
validation and broad exporter support remain bounded by ADR-0012 and the actual
profile implementation. Current static qualification does not settle those choices.

`schemas/authoring/export-settings.v1.json` and `manifest.schema.json` describe the
retired generic exporter draft. They do not configure this static producer, and
`export-manifest-v1` is not its completion format. They are excluded from the
current reproduction path. Removing these data files requires a separate consumer
check; this documentation change does not migrate any stored asset schema.
