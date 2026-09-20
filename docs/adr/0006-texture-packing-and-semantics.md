# ADR-0006: Texture Packing and Semantic Preservation

- **Status:** Proposed

## Context

Packing/compression is selected by semantic, precision, filtering and backend support.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.
