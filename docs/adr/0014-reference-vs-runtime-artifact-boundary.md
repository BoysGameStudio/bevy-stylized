# ADR-0014: Reference vs Runtime Artifact Boundary

- **Status:** Accepted
- **Decision owner:** Project maintainer
- **Date:** 2026-09-14 (implemented boundary); closed 2026-09-23

## Context

Blender graphs, controls and reference metadata are not executable runtime
materials. Renaming a reference sidecar cannot supply the missing semantic
conversion.

## Decision

Keep original sources, packed authoring candidates, runtime-only exports,
profile interpretation inputs, typed runtime descriptors and captures as
distinct artifacts. Each writer/reader/version is explicit. Runtime shaders
stay inside the owning crate; production code never loads Blender/Gacha
scripts or a sibling game's exporter.

Boundary as implemented and verified:

- Original third-party sources and derived art stay out of the repository
  entirely: `assets/local/` is git-ignored (`.gitignore`), documented in
  AGENTS.md; provenance is recorded in `THIRD_PARTY_NOTICES.md` instead.
- Authoring candidates and their completion records are content-addressed
  (`bevy_stylized.build/v1` manifests bind every input identity and artifact
  hash); runtime-only exports (`runtime/character.glb`) and strict-compiled
  packages (`compiled/`) are separate stages with separate identities.
- Profile interpretation inputs (typed `.npr.json` descriptors) are
  generated only by the strict compiler — never hand-written into fixtures
  or silently inferred from generic inventory.
- Captures and comparison assets belong to the reference chain
  (`labs/reference`, `fixtures/`), not to the runtime shipping set;
  reference-only code lives outside the product crates and the
  `profile-genshin` feature is default-off, so the reference profile is
  excluded from default builds.

## Evidence

- The independent static recipe exports a new GLB and NPR maps; strict
  profile compilation rejects incomplete coverage and unsupported active
  features (`crates/bevy-stylized-tools/tests/cli.rs`: strict profile
  covers every material and publishes atomically; fallback to generic
  compile refused).
- Compiled packages are staged atomically and include typed `.npr.json`
  data; build completion manifests record inputs and artifact hashes
  (verified for the current character build).
- CPU and GPU tests exercise source validation, material sharing, actual
  adoption and reload (`tests/integration/tests/asset_adoption.rs`,
  required asset-integration lane).
- `docs/authoring/02-blender-reference-export-bridge.md` (ADR-0015
  Accepted) owns the stage contract; product crates carry no Blender,
  Gacha or game-checkout dependency (facade dependency closure pinned by
  `no_reference_deps.rs`).

## Consequences

- The default generic compiler remains useful without claiming full family
  conversion; source recipes preserve declared simplifications and known
  unsupported semantics.
- The owning qualifications named here are now closed: export contract
  (ADR-0012), toolchain lock (ADR-0011), export bridge (ADR-0015),
  reference/golden chain (ADR-0007) are all Accepted — the 2026-09-14
  "final schema freeze remains open" caveat is resolved at the current
  static scope. Broader family coverage stays post-V1 (ADR-0007).
- This record does not accept arbitrary upstream versions (ADR-0013 stays
  the upgrade-policy owner) and does not reopen publication licensing
  (ADR-0017: private proprietary, no distribution).

## Revisit trigger

A schema/role change, a new profile family, an upstream signature change,
or an additional runtime capability.
