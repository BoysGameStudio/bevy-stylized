# ADR-0001: Runtime Material Backend

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-23

## Context

The runtime needs a material backend for Cel/Character/Hero: optimized
`ExtendedMaterial` over `StandardMaterial`, a direct custom `Material`
implementation, or a fully custom render phase/pipeline. The choice must be
made on measured evidence, not preference, and must preserve the Portable
baseline (fixed binding, whole-pipeline budgets) and NPR semantic control.

## Options

1. `ExtendedMaterial<StandardMaterial, _>` for the stylized paths — inherits
   Bevy's shadow/fog/prepass/deferred integration and base texture slots.
2. Direct `bevy_pbr::Material` implementations per class (the thin backend):
   full control of bind-group layout and WGSL, integrations attached manually.
3. A fully custom render phase/pipeline (custom draw functions, queue/bin
   logic, binding management).

## Decision

Option 2 — the thin `Material` backend is the accepted V1+ contract.
`StylizedCelMaterial` / `StylizedCharacterMaterial` / `StylizedHeroMaterial`
(and their table variants) remain the shipping forms; public semantics stay
on `bevy-stylized-core` descriptors, and any rework the decision ever
requires stays inside `bevy-stylized-render`.

- Option 1 is rejected for the core paths: its extension math hangs off the
  PBR input (composition constrained by `pbr_input`), and per-material it
  carries the full `StandardMaterial` binding cost the budget discipline
  exists to avoid. It survives only as the opt-in Physical-lighting path
  (`physical.rs`, where inheriting standard lighting is the point).
- Option 3 is deferred, not chosen: the gpu-driven spike shows the useful
  element (compute culling) works, but feeding its output into Bevy 0.19's
  binned render phase requires crate-private batch-build paths with no public
  injection point. A custom phase is justified only if measured evidence
  later shows options 1/2 insufficient; the revisit trigger below defines it.
- Integrations the thin backend does not get for free are already attached
  manually where the contract requires them: shared-skin prepass shader,
  screen-space outline overlay, opt-in Hero shadow casting (`shadow-casting`
  feature, ADR-0023), view-layer fog/environment.

## Evidence

- Backend comparison spike `tests/integration/tests/adr_0001_spike.rs`
  (A = ExtendedMaterial, B = direct `StylizedCelMaterial`; 500 instances,
  pipeline-readiness + steady-state `app.update` cost; results in
  `target/tmp/adr-0001-spike/`). Re-run 2026-09-23 (Metal, M4 Pro):
  A 696.7 ms / 5.047 ms, B 755.4 ms / 5.323 ms — comparable, and the
  qualitative trade-off in the spike summary stands.
- The thin backend carries the accepted instance transport: MeshTag bit-pack
  batching keeps 100/500/1000 same-mesh+material instances at one draw
  (`tests/integration/tests/instance_batching.rs` golden, and the ADR-0004
  table transport builds on the same layout — the strongest argument for
  keeping it).
- Integrations landed without a custom phase: complete-skin prepass WGSL,
  `screen_outline` overlay, `physical.rs` ExtendedMaterial opt-in,
  `enable_shadows` opt-in casting; all embedded WGSL validated by
  `tests/integration/tests/shader_validation.rs`.
- Gpu-driven spike (`labs/spikes/gpu-driven/`): compute culling is feasible
  and correct (CPU/GPU counts match), and the documented integration gap
  (no public injection into the binned phase) is why option 3 stays deferred.

## Consequences

- The Portable baseline keeps fixed binding and full shader/binding control;
  budget accounting (`budget.rs`) and the binding arrays stay fixed-layout.
- Any future claim that the thin backend is insufficient must come with
  measured frame-time/binding evidence; the spike's recorded thresholds
  (bind-group churn > 5% of frame time, or > 64 active material variants)
  are the tripwire for reopening option 3 or bindless (ADR-0003).
- The spike code stays a spike: `SpikeCelExtension` never enters the public
  API.

## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, when measured evidence crosses
the recorded thresholds, when a supported platform cannot satisfy the
contract, or when Bevy exposes a public batch-build injection point for a
custom phase.
