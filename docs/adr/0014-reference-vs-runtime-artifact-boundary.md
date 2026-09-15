# ADR-0014: Reference vs Runtime Artifact Boundary

- **Status:** Proposed
- **Decision owner:** Project maintainer (independence plan authorized 2026-09-14)
- **Date:** 2026-09-14 (implemented boundary; final schema freeze remains open)

## Context

Blender graphs, controls and reference metadata are not executable runtime materials.
Renaming a reference sidecar cannot supply the missing semantic conversion.

## Decision

Keep original sources, packed authoring candidates, runtime-only exports, profile
interpretation inputs, typed runtime descriptors and captures as distinct artifacts.
Each writer/reader/version is explicit. Runtime shaders stay inside the owning crate;
production code never loads Blender/Gacha scripts or a sibling game's exporter.

## Evidence

The independent static recipe exports a new GLB and NPR maps; strict profile
compilation rejects incomplete coverage and unsupported active features. Compiled
packages are staged atomically and include typed .npr.json data. CPU and GPU tests
exercise source validation, material sharing, actual adoption and reload.

## Consequences

The default generic compiler remains useful without claiming full family conversion.
Source recipes preserve declared simplifications and known unsupported semantics.
Final export/schema/profile qualification remains governed by the owning ADRs; this
record does not accept arbitrary upstream versions or resolve publication licensing.

## Revisit trigger

A schema/role change, new profile, upstream signature change or additional runtime capability.
