# ADR-0012: Blender Export Contract

- **Status:** V1 Blocker

## Context

Freeze transforms, mesh attributes, custom properties, exporter settings and manifests.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.
