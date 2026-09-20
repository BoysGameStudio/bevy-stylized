# ADR-0027: Current Bevy Baseline

- **Status:** Accepted
- **Decision owner:** Workbench maintainer (explicit current-only cleanup instruction)
- **Date:** 2026-09-20

## Context

This workbench maintains one current engine/dependency graph. Supporting older
Bevy versions would retain APIs and paths that the approved cleanup removes.

## Decision

Support the Bevy version and reviewed source declared by the current Cargo
manifests and lockfiles: Bevy 0.19.1 in this checkout. Do not keep release-line
matrices, deprecated forwarding APIs or version-selected compatibility shims.
An engine upgrade replaces the current baseline and updates all current consumers
in the same change; it does not add a parallel old-version implementation.

Runtime capability selection, supported platform differences, input validation,
generation recovery and bounded instance storage remain current behavior. They
must not be removed merely because they contain fallback or recovery logic.

## Evidence

Current `Cargo.toml`, the shared workbench source overrides and local source
resolution establish the baseline. Affected client checks and extended skin,
morph and outline GPU tests pass locally; this does not certify every platform.

## Consequences

Breaking changes to obsolete APIs are intentional. Current producer/consumer
contracts, source attribution and independently required runtime guards remain.
There is no old-version acceptance requirement or archive implementation.

## Revisit trigger

Revisit when the current engine/wgpu source changes or a supported platform
requires a different capability contract. Run affected local checks before
accepting the replacement baseline.
