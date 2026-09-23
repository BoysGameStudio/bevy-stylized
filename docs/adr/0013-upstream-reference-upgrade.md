# ADR-0013: Upstream Reference Upgrade Policy

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-23

## Context

The locked upstream (currently Gacha Setup v3.6.9, pinned in
`assets/sources/gacha-authoring.json`) will eventually have a successor.
An upgrade must arrive with semantic/visual diff evidence and explicit
approval — never as a silent in-place swap.

## Options

1. Track latest upstream automatically and rebaseline as needed.
2. Upgrade only through new, separately qualified source locks and recipes;
   old versions stay reproducible as historical evidence.

## Decision

Option 2, as implemented and as described by the owning workflow
[reference/04](../reference/04-upstream-contract-upgrade-and-golden.md):

- **Target policy:** Gacha Setup support targets only the latest stable
  release at qualification time. Each qualified target is pinned by version
  and archive SHA-256 in a `bevy_stylized.source/v1` manifest
  (`assets/sources/*.json`: id, release, per-file URLs/bytes/hashes).
  Older versions become historical evidence, not ongoing compatibility
  obligations (ADR-0011's lock binds the *current* one).
- **Upgrade mechanics:** an upgrade lands as a NEW source manifest + recipe
  revision — never by editing an accepted lock or recipe in place.
  Content-addressed build attempts (`tools/assets/build.py`) mean new
  inputs produce a new build identity while accepted builds stay
  reproducible from their recorded inputs.
- **Qualification gates:** every upgrade re-passes the full lane set —
  installation/authoring (source preparation, import, audit), export
  (strict contract), runtime (required GPU suites), and image comparison
  (the character-reference lane against the strict criteria). Record exact
  Blender/Gacha/shader versions, source hashes, settings, recipe revision,
  graph signature, model scope and output hashes. A machine-installed
  plugin does not alter node graphs stored in existing Blender files.
- **Candidate discipline:** upgrades are evaluated as distinct candidates —
  node interfaces/topology, source-map bindings, rig/material audit,
  runtime coverage and relevant views compared; signatures are never
  weakened and fixture mode is never used to force a character through
  export. Missing textures, version mismatches, unmapped roles and active
  unsupported features are explicit failures.
- **No automatic baseline acceptance:** a passed upgrade produces a new
  accepted record; the previous baseline remains the accepted one until
  that record exists (reference/08 criteria unchanged).

## Evidence

- Versioned source manifests (`assets/sources/gacha-authoring.json` v3.6.9
  + SHA-256) and the recipe family (`assets/recipes/genshin-static-*` —
  older variants retained as regression controls).
- The build/attempt machinery preserving failed and accepted attempts
  separately with input-bound identities (ADR-0015, `tools/assets/`).
- The qualification lanes exist and are exercised: assets build lane
  (import → audit → export → strict compile), character-reference lane
  (6/6 static matched cases on the qualified device, ADR-0007 recheck).
- reference/04's workflow text quoted above is the normative owner of the
  per-stage gate list.

## Consequences

- "Support for a new upstream version" is a new qualification, not a
  version bump: without a full gate pass the new version is unsupported by
  definition, and the old lock remains the only qualified source.
- Recipe history is additive; reduced-feature recipes stay as regression
  controls and are not retired by an upgrade.
- Golden/baseline images are never rebaselined as part of an upgrade; new
  baselines are accepted separately with their own provenance.

## Revisit trigger

Revisit when an upstream upgrade is actually proposed, when the source
manifest schema changes, or when a supported platform cannot satisfy the
contract.
