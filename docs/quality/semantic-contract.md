# NPR Semantic Contract

> **Status:** Normative V1 contract  
> **Purpose:** Define visual semantics that an Execution Tier may approximate in detail but may not silently violate.

## Cross-tier invariants

The following semantics are invariant unless a Profile explicitly declares a different fallback:

- ramp light/dark ordering must not invert;
- discrete material/class IDs must not change because of compression, filtering, quantization or precision loss;
- face left/right light selection must not invert;
- a Surface/Profile that declares silhouette outline support must retain a silhouette-outline semantic in every supported fallback;
- alpha mode, double-sided and culling semantics must remain correct;
- profile primary/shadow color relationships may be approximated but not replaced by unrelated colors;
- debug views must expose the same intermediate values actually used by shading;
- LDR fallback must not introduce NaN/Inf, invalid negative display values, or uncontrolled hue inversion;
- fallback must be deterministic and documented.

## Detail that may be reduced by Execution Tier

Subject to a profile-specific fallback contract, lower tiers may reduce or disable:

- internal/screen-space edge enhancement;
- secondary specular layers;
- expensive source-normal reconstruction;
- chromatic/lens/post effects;
- advanced environment stylization;
- sample count, kernel width or other quality-only refinements.

## Rule

> Execution-tier reduction may reduce detail and cost; it may not silently change the identity-defining NPR semantics of the surface.

## Validation

Each invariant must be covered by at least one of:

- Golden image comparison;
- feature-specific numeric probe;
- discrete classification test;
- render-semantics matrix test.

Related ADRs: ADR-0002, ADR-0006, ADR-0018, ADR-0023, ADR-0026.
