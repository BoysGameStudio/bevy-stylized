# ADR-0011: Reference Toolchain Lock

- **Status:** V1 Blocker

## Context

Lock Blender, Gacha Setup, concrete shader artifacts, render settings and provenance.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.
