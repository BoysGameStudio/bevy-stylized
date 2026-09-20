# ADR-0015: Blender Reference Export Bridge

- **Status:** V1 Blocker

## Context

Build scriptable, fail-closed reference extraction instead of manual parameter transcription.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.
