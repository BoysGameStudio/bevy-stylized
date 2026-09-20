# ADR-0021: Web Baseline Policy

- **Status:** V1 Blocker

## Context

Decide WebGPU-only support versus WebGPU + WebGL2 fallback and document separate limits.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.
