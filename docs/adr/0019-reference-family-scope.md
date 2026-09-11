# ADR-0019: V1 Reference Family Scope

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-11

## Context

Gacha Setup is an integration tool that bundles multiple shader lineages for different games (Genshin, HSR, ZZZ, WuWa and others). These lineages do not form one stable shading contract.

## Options

1. Define one broad `GachaProfile`.
2. Treat every supported upstream game as V1 scope.
3. Lock V1 to one concrete reference family and add other families independently later.

## Decision

V1 reference scope is the concrete Genshin reference family derived from the locked Gacha Setup/Festivity lineage. `Gacha` may be used as an informal ecosystem label, but it is not a single runtime profile contract.

Future HSR, ZZZ, WuWa or other families must have their own versioned profile/reference contracts and may share generic NPR primitives without sharing style-specific semantics.

## Evidence

The locked Gacha Setup source explicitly credits different shader sources per supported game, and the Genshin subdirectory carries distinct main, outline and post-processing Blender artifacts.

## Consequences

Core and Hero shaders must not accumulate cross-game special cases under a generic `GachaProfile`. Reference tooling and Golden provenance identify the concrete family and artifact lineage.

## Revisit trigger

Revisit if multiple reference families prove to share a stable common schema that can be factored without semantic loss.
