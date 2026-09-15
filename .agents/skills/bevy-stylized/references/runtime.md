# Runtime integration and material semantics

## Plugin registration and scene adoption

Check the installed Bevy/Cargo version and current examples rather than copying
unversioned upstream Bevy snippets. The inspected repository pins Bevy 0.19.1.
The custom glTF plugin must replace the DefaultPlugins copy before loader setup:

```rust
app.add_plugins(DefaultPlugins.build().set(bevy_stylized::stylized_gltf_plugin()))
   .add_plugins((bevy_stylized::StylizedPlugin::default(),
                 bevy_stylized::StylizedAssetIntegrationPlugin));
```

This registers custom vertex attributes; it is not automatic Gacha material
conversion. Follow the repository's current scene-loading API and wait for scene
entities/assets before material replacement. Useful real adoption examples:
`crates/bevy-stylized/src/integration.rs` and
`labs/reference/src/lib.rs` (`capture_production`). These examples
are dev-only and contain fixture-specific assumptions; extract the general lifecycle,
not their hard-coded asset paths or names. Verify which meshes received which
runtime material and which expected source maps were bound.

Current `StylizedMaterialSet` provides exact names/descriptors per scene root.
`StylizedMaterialAdoption` records errors. Compiled textures are relative to the
asset root; equivalent descriptors share assets across instances.

## Mapping checklist

| Source concept | Runtime treatment / caveat |
|---|---|
| Diffuse/base RGB | Color texture, correct sRGB decode; avoid unintended tint multiplication |
| Diffuse alpha | Genshin control/emission data may not mean transparency; choose opaque/mask by semantics |
| Packed ILM | R metallic control, G AO, B specular suppression, A material/ramp ID in the verified family; data texture, no sRGB |
| Shadow Ramp | Genshin lookup, material row and day/night addressing; do not substitute a generic 1D toon ramp |
| Face SDF | Correct texture channel plus per-instance head/light basis, side and threshold; audit softness |
| Packed normals | Profile-specific decode; not automatically ordinary XYZ tangent normals |
| MetalMap / MatCap | Upstream Metallic and engine MatCap formulas are not established equivalents |
| Specular Ramp | Bind only when the authored feature and runtime implementation call for it |
| Vertex color `Col` | Preserve control channels through export/import; verify attribute domain and packing |
| Lighting rig/palette | Capture evaluated controls and map view lighting; socket defaults can differ from driven values |
| Geometry-node outline | Reference mechanism; runtime builds its own hull/screen-space outline |
| Rigify/shape-key drivers | Bake/convert supported animation state; controls and drivers are not runtime solvers |
| Blender hair/clothes physics | Requires an explicit runtime system or bake; not provided by importing GLB |
| Compositor/display transform | Recreate supported post-processing explicitly and compare in a normalized output domain |

Material descriptors contain static class/texture/flag/parameter data. Instance
state owns face orientation/threshold, outline scale and profile selection. View
state owns global lighting/palette, day/night, exposure and camera effects.
Do not mutate shared material assets each frame to carry per-character state.

## Capability and performance boundaries

Keep Surface Class (Cel/Character/Hero), Execution Tier (Portable/Standard/High)
and Validation Mode distinct. Inspect requested and resolved tier plus fallback
report. Portable retains fixed bindings and no storage-buffer requirement.
The present Standard+ instance-table path is capability gated; MeshTag packing is
a lower-precision alternative. Read current source before assuming all fields
survive fallback without quantization. Preserve same-mesh/material batching.

For the inspected current primary materials, opaque/mask are supported, ordinary
Blend is not, `enable_shadows()` is false, and `enable_prepass()` is true with
camera-level opt-in. The old integration guide still says prepass is off; use
`character.rs`, `hero.rs`, the Cel material and `semantics_matrix.rs` as executable
evidence. Do not generalize those restrictions to every auxiliary effect material.

Known comparison gaps include upstream Metallic semantics, face-shadow overlay
slot and cast shadows. Current profile registration is not proof those features
are implemented. The source family's scope is Genshin, not every game supported
by the Blender importer.

The current compiled profile package may include `.view.json`. Apply it explicitly
at the application's view boundary if using the authored environment. View schema
v1.2 `ambient_override` expresses ambient radiance without compensating static
material light colors; None retains the previous day/night policy. Material
adoption itself does not replace application view settings.
