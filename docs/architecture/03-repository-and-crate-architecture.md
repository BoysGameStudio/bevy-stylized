# Independent repository and crate architecture

> Status: Normative architecture specification, updated by the authorized
> independent-project plan (2026-09-14).

The project is Bevy Stylized (`BoysGameStudio/bevy-stylized`). Package and directory
names use kebab-case; Rust crate identifiers and serialization namespaces follow
the established snake_case contracts. Keep six product crates:

| Product | Responsibility | Project dependencies |
|---|---|---|
| bevy-stylized-core | Domain/schema/capability data | None |
| bevy-stylized-render | Runtime material, instance, outline, LOD, post, view | core |
| bevy-stylized-gltf | Pure metadata and optional engine registration | core |
| bevy-stylized-profile-genshin | Versioned family interpretation | core |
| bevy-stylized-tools | Validate/inspect/compile/schema tooling | core, pure gltf, optional profile |
| bevy-stylized | Public exports, capability/plugin setup, optional scene adoption | core, render, optional gltf/profile |

Core uses Bevy math/color/reflection and optional serde; optional `schema` enables
Schemars for source-derived JSON Schema. It does not depend on a renderer or an
asset server. `bevy-integration` is the glTF engine feature; tools disable that
default feature. Features are additive capabilities, not runtime quality choices.
Facade without default features is not promised to be a renderer-free core: use
the core crate for that requirement.

## Development layout

`apps/showcase`, `labs/reference`, `labs/benchmarks`, `labs/regression` and
`labs/spikes/gpu-driven` own their development executables. `tests/harness` is a
product-neutral capture library. `tests/integration` is an explicit Cargo package,
not a magically discovered root test directory. GPU/file/cross-crate tests live
there; module tests stay with their implementation.

The default workspace selection is the six products. `--workspace` includes all
13 current members. No retired renderer is hidden by changing default-members.
The former migration renderer is absent from the active source graph. Historical
attribution is kept in [legacy provenance](../history/legacy-provenance.md).

Runtime WGSL is embedded from `crates/bevy-stylized-render/src/shaders`. There is
no second root shaders tree. Examples belong to the facade's examples directory.
Tools live under tools/blender, tools/assets and tools/checks. Authoring contracts
live under schemas/authoring; runtime snapshots derive from Rust types.

## Dependency enforcement

Products' normal/build closure cannot reach apps, labs, harness or integration.
Cross-crate integration tests no longer create render→facade dev cycles. Use the
complete resolved graph checker and isolated consumers to detect transitive path,
build-dependency and feature-unification mistakes. In-crate embedded paths are
valid; undeclared sibling-workspace files and escaped includes are not.

Profiles select meaningful data and modes. Renderer code does not identify a
character by name or consult an external game directory. A family algorithm may
exist as an explicit renderer mode; removing all family terminology is not an
architecture goal.

## Public lifecycle and distribution

`StylizedPlugin` installs framework rendering. `stylized_gltf_plugin()` must replace
the DefaultPlugins instance with `.set(...)` so PBR's glTF extensions see initialized
resources. `StylizedAssetIntegrationPlugin` is opt-in: a root MaterialSet maps exact
material names to descriptors; descendants receive shared materials, and the cache
releases unused entries after adoption components are removed.

MaterialSet is a one-time scene adoption configuration: replace the scene to apply
a different set. Mutating it after adoption is not a supported live editor API.
Per-instance state updates remain separate from shared material assets.

Static material data, per-instance data and view/frame settings have separate
owners. Asset compilation is not a GPU loader. Blender node graphs, controls and
physics are not silently executable runtime data.

All products remain publish=false. Packaging metadata/embedded asset inspection
and Git-free source builds are independent engineering checks; they do not resolve
the existing license/provenance decision or authorize a registry publication.
