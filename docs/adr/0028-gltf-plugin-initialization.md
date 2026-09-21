# ADR-0028: glTF Plugin Initialization Ordering

- **Status:** V1 Blocker

## Context

Register custom `_NPR_*` attributes before GltfLoader creation; facade must expose a correct setup path.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.

## Implemented choice and review evidence, 2026-09-21

Use the facade's configured `stylized_gltf_plugin()` in
`DefaultPlugins.build().set(...)` before adding the asset integration plugin.
The underlying builder registers `_NPR_*` and secondary skin attributes before
GltfPlugin creates its loader; the explicit order guard rejects late setup.
The [integration guide](../authoring/05-integration-guide.md#bevy-application-setup)
and `crates/bevy-stylized-gltf/src/setup.rs` own this contract.

CPU plugin-order/schema checks and real synthetic/external asset-adoption tests
cover this path; the standalone WebGPU consumer uses the same facade configuration
and adopts every declared primitive. Material sharing, descriptor conflict errors,
reload and teardown have separate facade coverage. No app-specific exporter or
personal path is required. The remaining ADR action is formal acceptance of this
existing builder/guard API; successful loading alone does not freeze every glTF
metadata or attribute policy.
