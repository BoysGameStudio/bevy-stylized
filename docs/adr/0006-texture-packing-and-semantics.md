# ADR-0006: Texture Packing and Semantic Preservation

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-23

## Context

Texture packing and compression must be selected by semantic, precision,
filtering and backend support — packed layouts are a representation
optimization that must never silently change channel meaning, sRGB/data
roles, or sampling semantics.

## Options

1. Free-form per-source texture sets with per-feature uniforms.
2. Semantic packing: fixed channel maps per texture class with the decode
   shared once per fragment, one packed UBO per material, and authored
   reconstruction rules for precision-critical maps.

## Decision

Option 2, as implemented:

- **ILM/control texture semantic map** (character and hero fragment decode,
  `character_forward.wgsl` / `hero_forward.wgsl`): **R = metal mask**
  (threshold `> 0.9`), **G = AO** (generic mode `mix(0.5, 1.0, g)`; the
  Genshin ramp mode consumes `g` directly — the AO regression in
  docs/rendering/03 §Genshin AO pins both), **B = highlight suppression**
  (decoded as `1.03 − b`), **A = material band ID** (band count by
  thresholds 0.2/0.4/0.6/0.8; the Genshin profile remaps band order to
  authored `1,4,3,5,2`). AO gates lighting via
  `smoothstep(0.01, 0.4, ao)`. Emission is **not** an ILM channel — the
  emission mask lives in base-color alpha; this is a deliberate, recorded
  decision.
- **Sampling semantics preserved:** authored ILM/face maps use cubic
  B-spline reconstruction; locked upstream packed images force level zero
  (`textureSampleLevel(..., 0.0)`) — no silent mip filtering of authored
  masks. sRGB vs data roles stay explicit per slot; packed normal, matcap
  and spec-ramp are authored Hero paths behind presence flags, disabled by
  the Portable fallback (`apply_hero_portable_fallback`).
- **Uniform packing:** every material packs into **exactly one UBO**
  (never one-UBO-per-feature — pinned by the
  `uniform_packing_and_sampler_audit` reflection test in
  `tests/integration/tests/interim_budget.rs`); sampler deduplication is
  recorded as a 1:1 audit baseline — sharing samplers by state is a
  recorded optimization opportunity, not a shipped behavior.
- The authoring-data ↔ runtime representation split is owned by
  [authoring/03](../authoring/03-authoring-schema-runtime-packing.md);
  ILM channel meaning is defined by the
  [Gacha behavior contract](../reference/07-gacha-setup-behavior-contract-v1.md)
  and the compiler implementation, not duplicated into checklists.

## Evidence

- Shader decode (cited above) exercised by the required golden lanes:
  `golden_character.rs` (ILM G=AO fixture + face SDF),
  `profile_lighting.rs` (10/10 PASS 2026-09-23: ILM zero/full AO gate,
  ramp day/night rows, metal ramp linear-data blue mask, band order).
- `tests/integration/tests/interim_budget.rs`
  `uniform_packing_and_sampler_audit` (2/2 PASS 2026-09-23): all four
  material types reflect exactly 1 UBO; sampler 1:1 baseline recorded.
- Presence-flag fallbacks: `apply_hero_portable_fallback` semantics
  covered in `profile_lighting.rs` (`hero_material_and_instance_fallback_disable_source_extras_after_sync`).

## Consequences

- Changing a channel's meaning, threshold or decode is a schema-level
  change requiring a new contract version and a baseline re-acceptance;
  it can never ride along in a "packing optimization".
- New packed maps must declare channel semantics, precision and sampling
  rules up front and pass the same decode-once and one-UBO audits.
- The sampler-dedup optimization, when implemented, must carry measured
  before/after evidence and update the audit baseline (ADR-0029).

## Revisit trigger

Revisit when a packed layout changes semantics, when sampler dedup is
implemented, when the Bevy/wgpu baseline changes texture capabilities, or
when a supported platform cannot satisfy the contract.
