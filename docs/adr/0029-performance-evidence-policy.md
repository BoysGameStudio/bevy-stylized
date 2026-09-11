# ADR-0029: Performance Evidence Policy

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-11

## Context

Renderer changes are prone to theoretical optimizations that increase complexity without measurable benefit, or improve performance by silently reducing visual quality.

## Options

1. Accept performance claims based on source-level operation counts.
2. Require measurements only for large rewrites.
3. Require structured before/after evidence for any PR claiming performance improvement.

## Decision

Any PR that claims a performance improvement must record:

- target Surface Class and Execution Tier;
- backend/device and resolution;
- fixed test scene/workload;
- before/after CPU and/or GPU measurements;
- resource-count delta where relevant;
- visual delta against the same reference/quality contract.

Without this evidence the change may be described as a refactor or theoretical optimization, but not as a measured performance improvement.

## Evidence

GPU compilers, caching and backend behavior can invalidate source-level assumptions. The project's core goal explicitly requires both visual fidelity and measured performance.

## Consequences

Benchmark fixtures and evidence templates are part of the engineering workflow. Performance regressions can be evaluated relative to stable workload baselines rather than anecdotal frame rates.

## Revisit trigger

Revisit only to improve measurement methodology or required evidence fields.
