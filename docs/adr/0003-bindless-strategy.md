# ADR-0003: Fixed Binding / Bindless Strategy

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-23

## Context

Bindless (binding arrays) is an optional high-end optimization for scenes
with many materials/draws; fixed binding must remain the required baseline
because the Portable tier forbids optional GPU features and the budget
contract counts fixed slots.

## Options

1. Fixed binding only, bindless never modeled.
2. Bindless as the primary layout with fixed binding as fallback.
3. Fixed binding as the required baseline; bindless as a reserved, explicitly
   admitted optional optimization gated by the capability model.

## Decision

Option 3. Fixed binding is the required baseline everywhere:

- Every production material uses the fixed-layout bind group
  (`force_no_bindless`; `tests/integration/tests/physical_material.rs`
  pins that the fixed layout never silently selects bindless storage).
- The Portable tier must never depend on bindless:
  `PORTABLE_FORBIDDEN_REQUIREMENTS` includes `Bindless` (pinned by tests in
  `crates/bevy-stylized-core/src/capability.rs`), so a Portable-resolution
  can never be satisfied by a binding-array device feature alone.
- No runtime bindless path is implemented; introducing one is future work
  that requires the revisit evidence below.

Bindless admission is reserved and unified in the capability model
(commit `8c21489`): `StylizedGpuFeature::Bindless` maps to the complete
binding-array feature set (`TEXTURE_BINDING_ARRAY |
STORAGE_RESOURCE_BINDING_ARRAY | BUFFER_BINDING_ARRAY |
PARTIALLY_BOUND_BINDING_ARRAY`) identically for Stage A requests and Stage B
device admission (`bindless_requires_every_requested_binding_array_feature`).
A future bindless consumer must request the feature at device creation
(ADR-0022) and may never be admitted by tier resolution — tier requirements
contain no bindless entry.

## Evidence

- Fixed-binding path measured: 500 same-mesh+material instances render in
  one draw (`tests/integration/tests/instance_batching.rs` golden;
  ADR-0004's table transport keeps the same fixed layout).
- Admission unification and tests in `crates/bevy-stylized/src/capability.rs`
  (`bindless_requires_every_requested_binding_array_feature`; reconfiguration
  refusal lists an unenabled `Bindless` exactly,
  `crates/bevy-stylized/tests/reconfiguration.rs`).
- The recorded tripwire for revisiting (ADR-0001 spike summary,
  `target/tmp/adr-0001-spike/summary.md`): bind-group creation/switching
  cost exceeding 5% of frame time in a profile, or more than 64 active
  material variants — with a binding-array prototype compared on CPU prepare
  time in the same scene.

## Consequences

- Any future bindless implementation is additive: it must be an explicit
  opt-in on top of the fixed baseline, keep the Portable fallback intact,
  and update the budget accounting in `budget.rs`.
- `docs/rendering/01-surface-tier-resource-and-bindless.md` stays the
  owning contract for the resource model; bindless there is allowed at
  higher tiers with a required fallback, which this decision ratifies.
- No currently shipping shader or material may require binding arrays.

## Revisit trigger

Revisit when the recorded tripwire is crossed with measurements, when the
Bevy/wgpu baseline changes the binding model, or when a supported platform
cannot satisfy the contract.
