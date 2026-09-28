# ADR-0015: Blender Reference Export Bridge

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-22

## Context

Reference extraction must be scriptable and fail-closed instead of manual
parameter transcription: the qualified Blender/Gacha source has to flow into
runtime inputs through an orchestrated pipeline whose every stage records
explicit inputs and refuses to guess on missing or changed data.

## Options

1. Manual export from an interactive Blender session with hand-recorded
   parameters.
2. A scriptable bridge with content-addressed build attempts, per-stage logs,
   an authoring audit gate and a completion manifest verified before reuse.
3. A loose collection of independently invoked exporter scripts.

## Decision

Option 2. `tools/assets/build.py` is the single orchestration entrypoint
(`python3 tools/dev.py assets build --pack genshin-odette`): prepare → import
(`import_character.py`) → inspect (`inspect_character.py`) → reference capture
(`capture_character.py`) → runtime export (`export_character.py`) → strict
profile compilation → comparison-case stages. The bridge runs Blender in an
isolated background configuration (never touching user preferences or the
authoring scene), keys the output directory by the complete input identity
(orchestrator, scripts, schemas, recipe, source manifests, Rust lock/toolchain
and compiler source closure), and publishes `build-manifest.json`
(`bevy_stylized.build/v1`) only after every stage succeeds. Partial or failed
attempts are preserved for diagnosis and are never accepted as outputs;
completed candidates are re-validated against their manifest (paths, hashes,
containment) before reuse.

The [bridge document](../authoring/02-blender-reference-export-bridge.md) owns
the stage contract; the [reference workflow](../reference/01-reference-authoring-workflow.md)
owns executable reproduction.

## Evidence

- Orchestration and fail-closed gates in `tools/assets/build.py`: the
  authoring audit rejects invalid constraints/drivers and missing images;
  `verify_run` rejects invalid or empty completion manifests, unsafe paths
  and changed artifact hashes.
- Negative-path coverage in `tests/python/test_build_attempt.py` (7 tests):
  failed attempts preserved with retry reusing success, incomplete/killed
  attempts never blocking retry, stage failures recording partial outputs,
  legacy-success revalidation, uncommitted manifests never blocking,
  mismatched inputs rejected, corrupt completion manifests unreusable.
  Linux: 7/7 PASS (cpu lane, 2026-09-22). macOS: 7/7 — the two
  pre-existing `/var` vs `/private/var` tempdir path-equality failures
  were fixed 2026-09-28 (attempt paths resolve consistently across
  `completed_or_new` branches; the test assertions compare resolved paths).
- Fail-closed metric boundary: `labs/reference/src/character/metrics.rs`
  `empty_geometry_masks_cannot_pass` (a failed metric cannot be relabelled
  as success).
- Real bridge execution produced the current qualified character build
  recorded in the support matrix and the strict character lane evidence.

## Consequences

The bridge is the only supported export path; invoking exporters with guessed
arguments is not a compatibility surface. Build completion proves an
export/compile handoff only — runtime adoption, GPU behavior and platform
qualification remain separate checks. Third-party content stays out of the
repository; the bridge requires the prepared, source-locked inputs.

## Revisit trigger

Revisit when the reference source or recipe scope changes (ADR-0013 upgrade
policy applies), when the build/attempt schema evolves, or when a new
authoring toolchain must be admitted.
