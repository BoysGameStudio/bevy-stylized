# V1 Release Checklist

This checklist organizes release review; checked items retain their existing scope,
not a fresh run on every documentation revision. Detailed criteria live in
[release gates](02-v1-release-gate.md), and current gaps in
[the provisional inventory](03-provisional-inventory.md).

## Architecture decisions
- [x] No ADR remains in `V1 Blocker` status (ADR-0007 closed 2026-09-22, the last one).
- [x] Every `Accepted` ADR contains completed Decision, Evidence and Consequences (enforced by tools/checks/check_docs.py).
- [ ] `TRACEABILITY.md` references every normative ADR relevant to V1.
- [x] Documentation linter passes with no errors.

## Architecture
- [x] Surface Class × Execution Tier × Validation Mode implemented without semantic conflation.
- [x] Material / Instance / View separation enforced.
- [x] Instance transport ADR closed (ADR-0004 Accepted 2026-09-22).
- [ ] Runtime Material backend ADR closed.
- [ ] Whole-pipeline resource budget ADR closed.

## Reference

Gacha Setup gates apply only to the current latest-stable target pinned in
`assets/sources/gacha-authoring.json`. No historical release lane is required. Broader current-version feature scope
remains explicitly qualified.

- [x] Blender upstream Golden produced before runtime export.
- [x] Reference toolchain locked (Blender / Gacha Setup / digests).
- [x] Blender export contract frozen (ADR-0012, current static character scope; named follow-up sub-items stay explicit).
- [ ] Reference upgrade policy frozen.
- [x] Production validated against independent upstream Golden (static matched-case scope: 6/6 strict cases on RTX 5090/Vulkan, rechecked on the current tree 2026-09-22; ADR-0007).
- [x] Production variants pass accepted regression and semantic suites (required GPU lane 42/42 Linux/Vulkan; semantic matrix suites green).
- [x] Reference artifact provenance complete (source lock → build manifest → case file → comparison report, machine-checked hashes).
- [x] Reference-only code/assets excluded from default shipping build.

## Authoring
- [ ] `_NPR_*` glTF attribute schema frozen.
- [x] Bevy GltfPlugin registration ordering tested.
- [ ] extras/sidecar precedence frozen.
- [x] Blender export bridge is scriptable and fail-closed.

## Rendering semantics
- [x] Opaque/Mask/Blend/etc. support matrix published.
- [x] Native opaque/mask prepass and hull/screen outline regressions tested.
- [x] Optional runtime shadows (Hero opt-in via `shadow-casting`) qualified; masked/transparent outline combinations explicitly bounded by ADR-0023 (masked screen-edge appearance and masked shadow casting stay unqualified cells).
- [x] Skinning/morph semantics tested.
- [x] Cull inheritance tested.

## Platforms
- [x] Support matrix updated with evidence.
- [x] Web baseline ADR closed (ADR-0021 Accepted 2026-09-22: WebGPU-only, real hardware adapter required, per-version qualification with the reference/08 §5 envelope).
- [x] Backend-specific pipeline creation tests run.
- [x] Any `Supported` target has real runtime evidence.

## Quality / Performance
- [x] Golden suite passes.
- [x] Whole-pipeline resource-budget suite passes.
- [x] Automatic instancing / instance-data regression passes.
- [x] Required benchmark scenes recorded.
- [x] No performance claims without before/after evidence.

## Release / Governance
- [x] ADR-0016 Package Naming closed: `bevy-stylized`.
- [x] License/provenance ADR closed. (ADR-0017 Accepted 2026-09-22: proprietary private, no distribution; distribution reopens review)
- [x] Bevy compatibility matrix published.
- [x] Selected Linux local feature matrix passes (seven standalone consumers, schema, inspector and scoped WebGPU).
- [x] Showcase suite complete.
- [x] CLI validate/inspect shipped.
- [x] Migration notes / changelog complete.


## Current local qualification record, 2026-09-21

`target/stylized/hardening-completion.json` records the exact required-item reports,
source identities and remaining dependencies. Final offline empty-cache isolation
passed engineering independence, approved image regression and all six strict
static character cases using Gacha Setup 3.6.9 and the default emission recipe.
The same runtime source passed 41 registered GPU cases, 205 Rust CPU tests,
53 Python tests, seven standalone consumers and the browser/native capture check.

This scoped result does not check off broader family parity, pending ADR approval,
other devices or publication. Only the two separated-crowd PNGs and their provenance
were explicitly accepted. Timing collections remain candidates and the absolute
budget is intentionally undefined. The support matrix retains Experimental status.
