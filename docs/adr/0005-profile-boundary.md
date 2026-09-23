# ADR-0005: Profile Boundary

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-23

## Context

The generic NPR core must not absorb style/game-specific policy. Profile
content (the Genshin reference family and future families) needs a hard
boundary so that core semantics stay generic while profiles remain
independently owned data packages.

## Options

1. Fold profile data into core/render crates.
2. Profile packages depending only on the core contract (descriptors,
   profile trait, schema versions), with the facade exposing them behind
   opt-in cargo features; render internals invisible to profiles.

## Decision

Option 2, as implemented:

- `bevy-stylized-core` carries only generic profile contracts
  (`StylizedProfile`, `StylizedProfileMetadata`, schema versions) — no
  Genshin-specific data or behavior.
- `crates/bevy-stylized-profile-genshin` is the single V1-locked reference
  family profile (ADR-0019): a pure-data package depending **only** on
  `bevy-stylized-core` (verified in its Cargo.toml), exposing typed mapping
  (`mapping.rs`) from bundle inputs to core descriptors. Only
  evidence-backed entries carry constants; the rest are marked
  needs-inspection rather than filled.
- The facade exposes the profile behind the default-off
  `profile-genshin` cargo feature (ADR-0024 additive capability);
  `custom_profile.rs` demonstrates that a third-party profile composes with
  the core contract (metadata + resolver) without touching render internals.
- Profile API and schema stability follow ADR-0025 (Accepted): versioned
  API/schema, no ABI promise, profiles never see render internals.

## Evidence

- Dependency closure: `bevy-stylized-profile-genshin` depends only on
  `bevy-stylized-core`; `crates/bevy-stylized/tests/no_reference_deps.rs`
  pins that the facade's dependency closure excludes reference/dev crates
  (1/1 PASS 2026-09-23).
- Third-party implementability:
  `crates/bevy-stylized/tests/custom_profile.rs`
  (`third_party_profile_works_through_core_contract_only`, 1/1 PASS) — a
  foreign profile implements `StylizedProfile` and resolves through the
  generic resolver.
- The profile crate's own discipline: only docs/reference/07-signed
  entries carry constants (`lib.rs` header; `mapping.rs` compile coverage
  exercised by the strict profile compiler tests in
  `crates/bevy-stylized-tools/tests/cli.rs`).
- Core genericity: `bevy-stylized-core` contains no Genshin identifiers
  outside the generic profile contract (`profile.rs`).

## Consequences

- New families are new crates behind their own opt-in features; core
  changes cannot smuggle style policy, and profile changes cannot force
  core changes.
- "Reference family" is a profile concept, not a renderer concept: the
  renderer consumes core descriptors produced by the profile mapping and
  stays family-agnostic.
- A profile that needs render-internal access is a boundary violation and
  must be redesigned, not merged.

## Revisit trigger

Revisit when a second reference family is admitted, when a profile
legitimately needs capabilities beyond the core contract (which would
re-open ADR-0025 first), or when the Bevy/wgpu baseline changes.
