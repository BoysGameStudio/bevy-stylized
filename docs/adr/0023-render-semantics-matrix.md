# ADR-0023: Render Semantics Matrix

- **Status:** V1 Blocker

## Context

Define opaque/mask/blend/shadow/prepass/outline behavior.



## Evidence required

- authoritative API/spec/source references;
- prototype or benchmark where performance/compatibility is material;
- affected Golden / pipeline tests;
- effect on Portable baseline and fallback behavior.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.

## Current bounded contract and evidence, 2026-09-21

The [owning matrix](../rendering/03-shader-lighting-outline-post.md#alpha--transparency--shadow--prepass-matrixv1-必需)
now distinguishes native opaque/mask, explicit host/pupil Blend, geometric-normal
prepass, hull extrusion and Standard+ screen edges. Unsupported selectors and
unqualified shadow/transparent-outline combinations are explicit. Native mask
prepass was reproduced hiding a rear surface and repaired by sharing the main
material's alpha inputs; its required GPU regression compares depth-only and
depth+normal paths across Cel/Character/Hero. Pipeline readiness is required before
capture. Existing deformation, hull/screen and budget tests remain independent
oracles; their baselines and tolerances are unchanged.

Remaining acceptance: runtime shadow-casting evidence for the optional feature,
masked hull/edge appearance policy and final supported-combination approval.
Compile-only shadow consumers and a type-level `enable_prepass` flag cannot close
those cells. Deferred/meshlet and general transparent NPR remain outside current
qualification rather than being inferred from Bevy's material enum.
