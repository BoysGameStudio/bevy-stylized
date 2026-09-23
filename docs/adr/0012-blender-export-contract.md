# ADR-0012: Blender Export Contract

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-22

## Context

The Blender → glTF boundary must be a frozen explicit contract: transforms,
mesh attributes, custom properties, exporter settings and manifests are pinned
so the runtime consumes a stable artifact shape rather than whatever an
exporter happens to emit.

## Options

1. Treat the exporter's default output as the contract and adapt the runtime
   per export.
2. Freeze the current static character export surface — artifact ownership,
   transform/attribute/custom-property tables, handoff rules — with machine
   schemas and fail-closed producer/consumer checks.
3. Defer freezing until a general-purpose interchange contract (all content
   kinds, all animation forms) is designed.

## Decision

Option 2. The
[Blender export contract](../reference/03-blender-export-contract.md) owns the
boundary: artifact ownership plus the surface/behavior tables for the current
static character scope. The machine schemas under `schemas/authoring/`
(`npr-attributes.v1.json`, `source-manifest.schema.json`,
`static-recipe.schema.json`, `character-capture.schema.json`) are the
versioned machine contract; the NPR attribute inventory generates the loader's
attribute contract and remains the active source. The fixture contract version
`blender-gltf-npr-v1` (recorded in
`fixtures/synthetic/reference-scene/export-sidecar.json`) names this export
identity.

Producer and consumer are fail-closed: `tools/blender/export_character.py`
rejects an authoring blend hash mismatch, an existing output candidate,
unsupported sampling, missing packed images, topology changes after outline
extraction and unmapped inputs; the strict compiler rejects contract
violations rather than emitting approximations.

**Explicitly not frozen here:** attribute quantization, shader-location policy
and generalized unit/negative-scale validation remain follow-up scope named by
the contract document; they are bounded by the actual profile implementation
and are not claimed stable by this acceptance. Broad exporter support beyond
the current static scope is likewise not implied. ADR-0020 (extras/sidecar
precedence) is a separate decision and stays Proposed.

## Evidence

- Producer/consumer contract tests: `crates/bevy-stylized-tools/tests/cli.rs`
  (16/16 PASS, 2026-09-22) — synthetic GLB validation, fail-closed wrong
  accessor types, machine-readable JSON output, strict profile compile
  covering every material, atomic publish, missing pack sources rejected,
  contract violations blocking emission, fallback to generic compile refused.
- The NPR attribute schema generates and is checked against the loader
  contract (`schemas/authoring/npr-attributes.v1.json` +
  `tools/checks/generate_attributes.py --check`, green in the cpu lane).
- The current qualified character build's completion manifest binds the
  authoring schemas and exporter script identities by hash; the synthetic
  reference fixture declares `blender-gltf-npr-v1` as its export contract
  version.
- Golden coverage using these artifacts (the strict character lane and the
  required GPU suites) exercises the frozen surface end to end.

## Consequences

- "Blender export contract frozen" applies to the current static character
  scope listed in the contract document. Changes to the frozen surface require
  a new contract version and migration note; they are not retroactive.
- The open sub-items (attribute quantization, shader-location policy,
  generalized unit/negative-scale validation) remain explicitly unresolved
  follow-up work; implementations must not silently depend on them being
  settled.
- extras/sidecar precedence is governed by ADR-0020 (Accepted 2026-09-23:
  conflict is an explicit failure with both values exposed, no channel
  priority); the importer exposing conflicts is unchanged.

## Revisit trigger

Revisit when a contract change is proposed (new version), when the follow-up
sub-items are resolved, or when content beyond the current static scope needs
the export path.
