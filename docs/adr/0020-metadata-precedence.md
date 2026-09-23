# ADR-0020: glTF Extras vs Sidecar Precedence

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-23

## Context

Stylized material metadata can arrive through two channels: glTF `extras`
embedded in the asset and typed `.npr.json` sidecar files. The contract
needs one canonical schema and deterministic behavior when both channels
disagree.

## Options

1. Extras always win.
2. Sidecar always wins.
3. One canonical typed schema for both channels; conflict is an explicit
   failure, never a silent pick.

## Decision

Option 3, as implemented in `crates/bevy-stylized-gltf/src/metadata.rs`:

- The canonical form is the typed `bevy_stylized.material/v1` descriptor;
  extras and sidecar are both parsed into it with fail-closed validation
  (wrong schema id, wrong field types and unknown versions are hard errors
  carrying the JSON path).
- One channel present: adopted as-is.
- Both channels present with semantically equal content: adopted (the
  duplicate is recorded).
- Both channels present and conflicting: the import returns
  `material: None` plus a `ExtrasSidecarConflict` diagnostic carrying BOTH
  channel values — no precedence is applied and nothing is silently
  picked. The caller must resolve the conflict explicitly; a strict import
  therefore fails closed rather than guessing.

This is the complete precedence rule: there is no channel priority by
design, because renaming a sidecar must never be able to override embedded
authoring data, and editing extras must never silently defeat the typed
handoff.

## Evidence

- Implementation and unit tests in `crates/bevy-stylized-gltf/src/metadata.rs`
  (13/13 PASS 2026-09-23): `conflict_is_diagnosed_never_silently_picked`
  (conflicting extras vs sidecar yields `material: None` + one
  ExtrasSidecarConflict diagnostic with both values), wrong-schema and
  wrong-field-type sidecars rejected with the JSON path, single-channel
  adoption for both channels.
- The same fail-closed discipline is exercised end-to-end by the strict
  compiler CLI tests (`crates/bevy-stylized-tools/tests/cli.rs`).

## Consequences

- Producers must not write both channels with different content for the
  same material; a conflict is a build/authoring error, not a warning to
  ignore.
- Any future channel-priority rule is a contract change requiring its own
  ADR; until then `material: None` + diagnostic is the only allowed
  conflict outcome.
- ADR-0012's export contract is updated: its "ADR-0020 retains precedence
  policy ownership" note is superseded by this acceptance (the rule above).

## Revisit trigger

Revisit when a channel-priority rule is proposed, when a new metadata
channel is introduced, or when the Bevy/wgpu baseline changes.
