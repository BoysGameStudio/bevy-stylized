# ADR-0024: Cargo Feature Policy

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-11

## Context

Cargo features are compile-time additive capabilities and are unified across a dependency graph. They are not an appropriate runtime quality selector.

## Options

1. Cargo features named `portable`, `character`, `hero` that mirror runtime variants.
2. Compile-time features only for optional modules/capabilities, with runtime tier selection handled separately.

## Decision

Cargo features express additive compile capabilities such as `gltf`, a specific profile family, reference/dev tooling, editor/asset-compiler support, or an optional runtime backend.

Runtime `SurfaceClass` and `ExecutionTier` are not selected by Cargo features. If shipping-size constraints justify compiling out a runtime class, such features must still be additive capabilities and must not redefine runtime quality semantics.

## Evidence

Cargo's feature model unifies enabled features by union. The three-axis architecture requires runtime tier decisions to depend on device capabilities and policy, not dependency-resolution side effects.

## Consequences

Feature-matrix CI tests additive combinations. Public APIs must behave predictably when optional modules are absent, and documentation must distinguish compile capability from runtime policy.

## Revisit trigger

Revisit if Cargo gains a fundamentally different feature model or if package splitting replaces most compile-time feature gating.
