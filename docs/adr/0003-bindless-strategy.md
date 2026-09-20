# ADR-0003: Fixed Binding / Bindless Strategy

- **Status:** Proposed

## Context

Bindless is optional high-end optimization; fixed binding remains required.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.
