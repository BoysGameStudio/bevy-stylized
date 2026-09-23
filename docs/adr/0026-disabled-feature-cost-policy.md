# ADR-0026: Disabled Feature Cost Policy

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-23

## Context

Optional effects, when disabled or absent, should cost (approximately)
nothing at runtime: no extra passes, no extra samples, no per-frame writes.
The policy must be pinned by tests, not asserted.

## Options

1. Always-run systems that check flags at runtime.
2. Component-driven zero-cost gating: a feature's systems early-out cheaply
   when its driving components/events are absent, and produce nothing.

## Decision

Option 2, as implemented and pinned by
`tests/integration/tests/disabled_feature_cost.rs`:

- **Outline:** without `StylizedOutline` components, the replica sync
  produces zero replicas and changes zero entities over repeated frames.
- **Material flag sync:** without material asset events, no `Modified`
  writes occur — flag synchronization is event-driven, never per-frame
  mutation (echo-free by consistency check).
- **Post process:** without a `StylizedPostProcess` camera component, no
  fullscreen effect entity is created.
- **Screen outline:** without a `StylizedScreenOutline` camera component,
  no overlay is spawned and no prepass components are inserted — prepass
  cost is exactly zero; on tier downgrade the previously granted
  prepass/overlay is stripped again (ADR-0008/0030).

Enabling a feature pays its declared cost once (component insert, prepass
attachment, pipeline variant); disabling or never declaring it must not
leave residual passes, entities or asset writes behind.

## Evidence

- `outline_sync_is_noop_without_outline_components`,
  `flag_sync_is_noop_without_material_events`,
  `post_sync_is_noop_without_post_component`,
  `screen_outline_absent_means_zero_prepass_work` — 4/4 PASS 2026-09-23.
- The no-per-frame-mutation regression
  (`tests/integration/tests/semantics_matrix.rs::
  no_per_frame_material_mutation_with_shared_material`, in the required
  lane) pins the same discipline across 100 shared-material instances.

## Consequences

- New optional features must gate through their driving component or
  event stream with a cheap early-out, and their cost tests must show
  absence produces zero entities/passes/writes.
- A feature that cannot reach zero cost when disabled must declare its
  residual cost explicitly in the owning contract, not hide it in frame
  time.

## Revisit trigger

Revisit when a feature cannot meet the zero-cost rule, when the
Bevy/wgpu scheduling model changes, or when measured evidence contradicts
the decision.
