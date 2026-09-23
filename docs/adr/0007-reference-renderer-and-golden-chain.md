# ADR-0007: Reference Renderer and Golden Chain

- **Status:** Accepted
- **Decision owner:** Project maintainer
- **Date:** 2026-09-22 (architecture decision 2026-09-14; visual qualification
  closed 2026-09-22)

## Context

A permanent dependency on a retired game's renderer prevents independent builds
and authoring. Reference independence must remain meaningful after that code
retires. The 2026-09-14 architecture decision replaced the legacy oracle with
independent upstream capture plus approved production baselines; the remaining
blocker was that family visual qualification had not passed.

## Options

1. Retain a live legacy oracle.
2. Rename the legacy renderer as a new local renderer.
3. Independent upstream Blender/Gacha capture, approved production regression
   baselines and first-party semantic fixtures.

For the qualification scope: qualify the whole family (rigs, physics, effect
meshes) before V1, or qualify the declared static matched-case scope for V1
with the rest explicitly post-V1.

## Decision

Option 3 (unchanged from 2026-09-14): independent upstream Blender/Gacha
behavior, approved production regression images, and first-party semantic
assertions. The legacy renderer stays retired; its provenance and past results
remain history. Production output never generates the upstream truth against
which it is tested.

**V1 qualification scope (owner-delegated, recorded 2026-09-22):** the V1
visual qualification scope IS the declared static matched-case family scope of
the [reference workflow](../reference/01-reference-authoring-workflow.md) —
the six calibrated full-body/face lighting cases over the current static
material variants (source outlines, metal/specular ramps, optimized normals,
emission), per ADR-0019's family scoping. Full rigs, secondary physics,
animated deformation qualification, and the excluded `Eff_001`/`Eff_002`
effect meshes are explicitly post-V1. The effects recipe
(`genshin-static-effects-v1`) now exports and shades them (decoded light
basis, ramp/ILM wiring, source vertex-color semantics, HASHED→Mask), and the
`full_body-front` matched case qualifies against the unchanged §3 criteria on
the qualified platform; `full_body-left/right` remain diagnostic on a
sub-pixel edge fill-rule residual (export contract, effect parity status).
Their full-scope qualification and the documented authoring-gap evidence
remain post-V1.

Cross-device qualification policy is unchanged: per-version/per-device measured
evidence only (docs/reference/08); the qualified comparison platform is
Linux/Vulkan. The 2026-09-22 Metal shader-translation fix
(`c177382`, naga `_tmp` redefinition in the hero fragment debug paths) restores
four Metal host suites but does not qualify macOS — the Metal golden-delta
cases stay documented in the support matrix as unqualified.

## Evidence

- 2026-09-14 (architecture): the direct-source, project-owned reference
  preparation reproduced the earlier upstream pixels exactly; required-mode
  comparison preserved the failing production metrics (about 20.96 dB) instead
  of reporting parity; the isolated workflow runs without another game
  checkout. Coverage dispositions in `tests/current-contracts.json`.
- 2026-09-21 (static qualification, per
  [reference workflow](../reference/01-reference-authoring-workflow.md)): all
  six matched cases passed on RTX 5090/Vulkan across the static variants —
  minimum PSNR 51.55–51.59 dB, maximum nonedge channel error 11–12/255,
  minimum sharp-light IoU 0.99871, minimum center-coverage IoU 0.99995;
  repeated captures pixel-identical; full network-disabled isolation workflow
  passed with byte-identical rebuilds.
- 2026-09-22 recheck on the current committed tree (`c177382`,
  `character-reference --required`, same pinned case identity): capture_status
  PASS, parity_status PASS, 6/6 cases — minimum PSNR 57.07 dB, maximum nonedge
  channel error 13/255, minimum sharp-light IoU 0.99794, minimum
  center-coverage IoU 0.99989. All within the unchanged reference/08 criteria.
- Provenance chain complete and machine-checked: source lock
  (`assets/sources/gacha-authoring.json`, v3.6.9 + SHA-256) → build completion
  manifest (`bevy_stylized.build/v1`, 13 input identities + 404 artifact
  hashes) → case file (`bevy_stylized.character-capture/v3`, recipe id, blend
  and generator hashes) → comparison report (input/device/case hashes).
- Reference integrity oracles stay independent: golden suites, adoption and
  semantic fixtures pass in the required GPU lane (42/42 on Linux/Vulkan,
  2026-09-22).

## Consequences

- Reference capture and parity acceptance keep distinct statuses; missing
  required inputs/GPU/baselines fail.
- V1 claims visual qualification for the static matched-case scope only.
  Animated deformation, rigs, secondary physics and the excluded effect meshes
  are post-V1 qualification items with explicit fallback evidence; no V1
  document may imply them.
- A source/toolchain/recipe upgrade follows ADR-0013 and produces new
  identities; old records stay valid history.
- The retired synthetic reference lane is not an acceptance input.

## Revisit trigger

Upstream, Bevy/wgpu, shader signatures, device policy or measured visual
evidence changes; or when post-V1 animated/effect qualification is scheduled.
