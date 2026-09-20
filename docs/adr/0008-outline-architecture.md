# ADR-0008: Outline Architecture

- **Status:** Proposed
- **Decision owner:** Pending assignment
- **Date:** Pending decision

## Context

Keep inverted hull as portable baseline; internal edge is tiered optional quality.

## Options

Pending: enumerate and evaluate options during the owning spike/review.

## Decision

Pending: this ADR is intentionally unresolved until its evidence gate is satisfied.

## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.

## Consequences

2026-09-14 implementation review added CPU replica lifecycle and continuous morph
GPU evidence (`outline_tracks_live_inputs_and_removes_stale_deformation` and
`live_morph_outline_tracks_surface_and_returns_to_rest`). These close live input
copy defects, not the full bounds/LOD/pass compatibility decision. Status remains
Proposed; see [current validation workflow](../../CONTRIBUTING.md).

Pending until the decision is made and evidence is recorded.

## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.
