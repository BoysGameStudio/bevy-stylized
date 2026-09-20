# ADR-0007: Reference Renderer and Golden Chain

- **Status:** V1 Blocker
- **Decision owner:** Project maintainer (independence plan authorized 2026-09-14)
- **Date:** 2026-09-14 (architecture decision; visual qualification remains open)

## Context

A permanent dependency on a retired game's renderer prevents independent builds
and authoring. Reference independence must remain meaningful after that code retires.

## Options

Retain a live legacy oracle; rename it as a new local renderer; or use independent
upstream capture, approved production regression baselines and semantic fixtures.
Renaming preserves the same architectural dependency and does not solve the problem.

## Decision

Adopt independent upstream Blender/Gacha behavior, approved production regression
images, and first-party semantic assertions. Remove the active legacy renderer after
coverage migration; retain its provenance and past results as history. Current
production output must never generate the upstream truth against which it is tested.

## Evidence

The direct-source, project-owned reference preparation reproduces the earlier
upstream pixels exactly. Required-mode comparison preserves the failing production
metrics (about 20.96 dB), rather than reporting the capture as parity success.
The isolated workflow runs without another game checkout or preprocessed character.
Coverage dispositions are recorded in tests/current-contracts.json.

## Consequences

Reference capture and parity acceptance have distinct statuses. Missing required
inputs/GPU/baselines fail. The ADR remains V1 Blocker because family visual
qualification has not passed; the engineering architecture decision does not close
that requirement or relax its thresholds.

## Revisit trigger

Upstream, Bevy/wgpu, shader signatures, device policy or measured visual evidence changes.
