# ADR-0002: Whole-Pipeline Resource Budget

- **Status:** Proposed

## Context

Define per-stage budgets across view, mesh, instance, material and profile resources with safety margin.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.
