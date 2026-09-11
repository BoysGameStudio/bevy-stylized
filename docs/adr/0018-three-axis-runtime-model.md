# ADR-0018: Three-Axis Runtime Model

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-11

## Context

The renderer needs to describe three different concerns that were previously conflated: the semantic complexity of a surface, the runtime quality/cost selected for a device, and the reference/validation implementation used for QA. Treating `Portable`, `Character`, `Hero`, and `Reference` as one hierarchy makes fallback, LOD, Cargo features and validation ambiguous.

## Options

1. One linear enum combining material semantics and quality.
2. Two axes: material class and quality, with Reference still treated as a tier.
3. Three independent axes: Surface Class, Execution Tier, and Validation Mode.

## Decision

Adopt three independent concepts:

- `SurfaceClass`: semantic surface requirements (`Cel`, `Character`, `Hero`).
- `ExecutionTier`: runtime cost/quality policy (`Portable`, `Standard`, `High`).
- `ValidationMode`: reference/QA execution, separate from shipping quality.

A concrete runtime implementation is resolved from:

`SurfaceClass × ExecutionTier × actual RenderDevice capabilities × profile fallback policy`.

A semantic downgrade from Hero to Character/Cel is not implied by reducing the execution tier; such a downgrade requires an explicit profile/LOD fallback contract.

## Evidence

The architecture review found that the former model used `Portable` both as a material/runtime class and as a quality tier, creating contradictions in LOD and fallback semantics. The revised architecture and release gates consistently use the independent-axis model.

## Consequences

Resolver APIs, tests, telemetry, Cargo feature names and documentation must never reuse one enum for these axes. Variant counts must be controlled by coarse surface classes and explicit specialization policy rather than a Cartesian explosion of feature booleans.

## Revisit trigger

Revisit only if a future Bevy rendering abstraction makes one axis redundant without losing semantic clarity.
