# ADR-0011: Reference Toolchain Lock

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-22

## Context

Reference truth is only reproducible if the authoring toolchain is locked:
Blender, the Gacha Setup addon, the concrete shader artifacts and the render
settings must be pinned, and the pin must be enforced at runtime rather than
by convention.

## Options

1. Document recommended versions and rely on authors to match them.
2. Pin a source manifest (release + archive SHA-256), a recipe carrying the
   exact toolchain identity, and fail-closed runtime rechecks at import,
   capture and export.
3. Pin the Blender installer binary digest per platform as well.

## Decision

Option 2. The lock covers:

- **Source archive:** `assets/sources/gacha-authoring.json` pins Gacha Setup
  v3.6.9 by release URL, byte count and archive SHA-256.
- **Recipe identity:** `assets/recipes/genshin-static-emission-v1.json` (the
  default build recipe) records Blender `5.2.2 LTS`, the Gacha version tuple,
  the archive hash and per-file SHA-256 of the addon internals and the
  concrete shader artifact `.blend` files, plus the render/framing settings.
- **Runtime enforcement:** `tools/blender/import_character.py` hard-fails when
  the running Blender or addon version differs from the recipe and re-hashes
  every pinned addon file; `tools/blender/character_cases.py` rechecks the
  authoring blend SHA-256 and Blender version before producing comparison
  cases; the build bridge (`tools/assets/build.py`) keys outputs by the
  complete input identity.

The [toolchain lock document](../reference/02-reference-toolchain-lock.md) owns
the operational contract.

## Evidence

- The pinned files above are in the repository and machine-readable; the
  runtime checks cited are raise-on-mismatch code paths, not documentation.
- The lock is exercised by the asset build lane; a missing prepared input is
  BLOCKED (`ResourceBlocked`), never silently substituted.
- The current locked source (v3.6.9) produced the recorded character build
  whose build manifest binds the full input identity; the earlier
  `docs/quality/latest-gacha-validation-2026-09-15.json` record covers the
  previous v3.6.4 toolchain and is historical evidence for the process, not
  for the current lock.

## Consequences

- **Known boundary:** the Blender *installer binary* digest is not pinned.
  Installs are channel-managed (snap on Linux, direct download on macOS), so
  the lock binds the Blender version string and every Gacha/shader artifact
  hash instead; the version check at import catches a mismatched runtime.
- A source or toolchain upgrade produces a new input identity and requires
  re-verification; upgrade policy is owned by ADR-0013, which stays a
  separate decision and is not settled here.
- Only the current qualified source and declared static recipe scopes are
  covered; there is no historical release lane.

## Revisit trigger

Revisit when the qualified source or toolchain changes (via ADR-0013), when a
new platform channel makes installer-level pinning feasible, or when a
runtime check is shown to be bypassable.
