# Legacy Reference Crates

These crates were copied verbatim from the Tenacity project
(local source: `/home/x/Projects/Tenacity`, commit `b5b26ac`, imported 2026-09-11)
as the **Legacy Reference Path** required by
[`docs/migration/01-tenacity-migration-plan.md`](../../docs/migration/01-tenacity-migration-plan.md).

- `bevy-cel` (package `bevy_cel`) — cel/NPR rendering foundations built on
  `ExtendedMaterial<StandardMaterial, CharacterExtension>`.
- `bevy-cel-metadata` (package `bevy_gltf_metadata`) — glTF metadata integration
  and validation (`_CEL_*` vertex attributes, `CEL_metadata` extension, sidecar
  schema, `cel_gltf_validate` CLI).

## Status

- **Frozen reference oracle, not the production API.** The public API of these
  crates is not a stable Bevy Stylized contract. The production renderer is
  re-architected (Surface Class × Execution Tier, Material/Instance/View
  separation) in later migration phases; these crates exist so the new
  implementation can be validated against a known-good reference.
- Package names are unchanged so `embedded://bevy_cel/...` shader paths keep
  working.
- The optional `kawaii` feature of `bevy_gltf_metadata` (dependency on
  Tenacity's `bevy_kawaii_physics`) was **stripped** during import; it was
  unused by `bevy_cel`.
- Both crates carry their original MIT license declaration. The repository-wide
  license remains pending ADR-0017.
- GPU screenshot fixtures under `bevy-cel/examples/` are compile-checked only;
  running them requires a GPU and the original asset directories.
