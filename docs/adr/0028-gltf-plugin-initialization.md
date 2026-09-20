# ADR-0028: glTF Plugin Initialization Ordering

- **Status:** V1 Blocker

## Context

Register custom `_NPR_*` attributes before GltfLoader creation; facade must expose a correct setup path.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.
