# ADR-0002: Whole-Pipeline Resource Budget

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-23

## Context

Resource limits must be defined per stage across the whole pipeline — view,
mesh, instance, material and profile resources — with a safety margin, so
that Portable viability is a computed fit against explicit baseline models
rather than an assumption.

## Options

1. Per-material ad-hoc limits.
2. A whole-pipeline budget model: per-stage usage decomposition
   (view + mesh + instance + material + profile + safety margin) evaluated
   against named baseline limit models, with reflection-measured gate tests.

## Decision

Option 2, as implemented in `crates/bevy-stylized-core/src/budget.rs` and
`tests/integration/tests/interim_budget.rs`:

- **Baseline limit models** (from wgpu-types 29 defaults, matching
  docs/rendering/01): `webgpu-core` (16T/16S/12U/8SB/4BG), `gles31`
  (16/16/12/4/4), `webgl2` (16/16/11/0/4). Per-model evaluation accounts
  for storage-buffer availability instead of hard-failing it silently.
- **Decomposition:** `StylizedPipelineBudget` = view (reflected real mesh
  view layout) + mesh + instance (MeshTag channel = 0 bindings today;
  storage-table usage counts here when enabled) + material (AsBindGroup
  reflection) + profile (reserved, currently no profile bindings) + safety
  margin (2T/1S/1U/0SB).
- **Gate rules:** Cel@Portable and Character@Portable must fit the WebGL2
  model (hard assertion); Hero@Portable is fully recorded with its fallback
  variant's logical usage (the fallback disables matcap/spec-ramp/packed
  normal; the fixed-layout note that slots stay bound without per-tier
  material variants is part of the contract).
- **Packing discipline (with ADR-0006):** exactly one UBO per material and
  the sampler 1:1 audit baseline (`uniform_packing_and_sampler_audit`).
- Instance-table capacity is separately budgeted by ADR-0004
  (16 MiB `InstanceTableBudget`).

The margin values (2T/1S/1U/0SB) are ratified as the current fixed budget;
recalibrating them is a measurement update under ADR-0029's evidence policy,
not an open design question.

## Evidence

- `whole_pipeline_budget_fits_baseline_models` (GPU reflection, 2026-09-23,
  Metal): view fragment 3T/3S/4U/3SB reflected from the real layout; all
  variants fit all three models — cel 8T/7S/6U/3SB, character 8T/7S/6U/3SB,
  hero 13T/12S/6U/3SB, hero-fallback 10T/9S/6U/3SB, outline 6T/5S/6U/3SB.
  Report: `target/tmp/interim-budget/pipeline_budget.json`.
- `uniform_packing_and_sampler_audit` (2/2 PASS): all four material types
  reflect exactly one UBO; sampler 1:1 baseline recorded.
- Limit-model and usage-arithmetic unit tests in
  `crates/bevy-stylized-core/src/budget.rs`.

## Consequences

- A variant that stops fitting its gate model is a contract failure, not a
  warning: it must either be reduced (fallback variant), re-tiered, or the
  budget model must be re-versioned — never silently exceeded.
- New features report their per-stage usage into the budget before claiming
  Portable viability; profile-bound resources get counted in the profile
  stage when they appear.
- Safety-margin recalibration and sampler dedup require measured
  before/after evidence (ADR-0029).

## Revisit trigger

Revisit when a variant stops fitting, when safety-margin values are
recalibrated by measurement, when a new baseline model is admitted (new
platform class), or when the Bevy/wgpu baseline changes.
