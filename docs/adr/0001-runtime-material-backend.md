# ADR-0001: Runtime Material Backend

- **Status:** Proposed

## Context

Compare optimized ExtendedMaterial, direct Material, and custom phase/pipeline using measured evidence.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.
