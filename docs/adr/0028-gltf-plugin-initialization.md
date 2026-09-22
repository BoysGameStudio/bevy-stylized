# ADR-0028: glTF Plugin Initialization Ordering

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-22

## Context

Custom `_NPR_*` vertex attributes must be registered on `GltfPlugin` before its
loader is created; adding `DefaultPlugins` first and `StylizedPlugin` later cannot
retroactively register them. The facade must expose a correct setup path.

## Options

1. Ask users to manually construct `GltfPlugin` with the right registrations.
2. Facade-provided configured `GltfPlugin` (`stylized_gltf_plugin()`) plus a
   fail-closed order guard that rejects late setup.
3. Post-hoc loader patching after `GltfPlugin` is added.

## Decision

Option 2. Use the facade's configured `stylized_gltf_plugin()` in
`DefaultPlugins.build().set(...)` before adding the asset integration plugin.
The underlying builder registers `_NPR_*` and secondary skin attributes before
`GltfPlugin` creates its loader; `check_gltf_plugin_not_added` rejects late setup
with a structured error. The
[integration guide](../authoring/05-integration-guide.md#bevy-application-setup)
and `crates/bevy-stylized-gltf/src/setup.rs` own this contract.

## Evidence

- `crates/bevy-stylized/tests/gltf_ordering.rs`:
  `facade_configured_app_loads_npr_attribute_and_extras` and fail-closed
  `configuring_gltf_after_plugin_added_fails_closed` (2/2 PASS, verified
  2026-09-22 on the accepted tree).
- Registration guard and attribute registration:
  `crates/bevy-stylized-gltf/src/setup.rs`, `attributes.rs`.
- Real asset adoption covered by the required GPU lane (42/42 PASS on
  Linux/Vulkan RTX 5090, run at de57210, 2026-09-22).
- The standalone WebGPU consumer uses the same facade configuration and adopts
  every declared primitive: macOS Chrome 153 pixel-exact vs native Metal;
  Linux Chrome 153 hardware WebGPU (NVIDIA Blackwell) — 6 deterministic
  pixel-level Dawn/naga divergences documented in CONTRIBUTING (2026-09-22).
- Portable baseline unaffected: the configured plugin changes registration
  order only, not device requests or fallback behavior.

## Consequences

`stylized_gltf_plugin()` is the stable public setup path; the provisional
markers on the facade and glTF crate docs are lifted. Late `GltfPlugin`
addition remains a hard configuration error. This decision freezes the
registration mechanism only; per-asset metadata and attribute policies are
governed by their own contracts.

## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the
decision, or a supported platform cannot satisfy the contract.
