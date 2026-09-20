# ADR-0004: NPR Instance Data Transport

- **Status:** V1 Blocker
- **Decision owner:** Pending assignment
- **Date:** Pending decision

## Context

Prototype MeshTag, GpuArrayBuffer, LUT/texture indirection and portable fallback without breaking batching.

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

Pending until the decision is made and evidence is recorded.

## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.

## Provisional lifecycle evidence, 2026-09-20

The StorageTable experiment now has a bounded dense snapshot lifecycle, explicit
admission failure, draw-binding revocation and material rebinding on layout
changes. The normative provisional contract is in
[Material / Instance / View](../rendering/02-material-instance-view.md#provisional-storagetable-lifecycle-2026-09-20).
CPU regressions live in `instance_table.rs`; the required integration suite adds
`table_churn_preserves_pixels` and retains batching measurements plus the Portable
binding gate. These are lifecycle findings, not acceptance of this ADR.
