# V1 Release Checklist

This checklist organizes release review; checked items retain their existing scope,
not a fresh run on every documentation revision. Detailed criteria live in
[release gates](02-v1-release-gate.md), and current gaps in
[the provisional inventory](03-provisional-inventory.md).

## Architecture decisions
- [ ] No ADR remains in `V1 Blocker` status.
- [ ] Every `Accepted` ADR contains completed Decision, Evidence and Consequences.
- [ ] `TRACEABILITY.md` references every normative ADR relevant to V1.
- [x] Documentation linter passes with no errors.

## Architecture
- [x] Surface Class × Execution Tier × Validation Mode implemented without semantic conflation.
- [x] Material / Instance / View separation enforced.
- [ ] Instance transport ADR closed.
- [ ] Runtime Material backend ADR closed.
- [ ] Whole-pipeline resource budget ADR closed.

## Reference

Gacha Setup gates apply only to the current latest-stable target pinned in
`assets/sources/gacha-authoring.json`. No historical release lane is required. Broader current-version feature scope
remains explicitly qualified.

- [x] Blender upstream Golden produced before runtime export.
- [x] Reference toolchain locked (Blender / Gacha Setup / digests).
- [ ] Blender export contract frozen.
- [ ] Reference upgrade policy frozen.
- [ ] Production validated against independent upstream Golden.
- [ ] Production variants pass accepted regression and semantic suites.
- [ ] Reference artifact provenance complete.
- [x] Reference-only code/assets excluded from default shipping build.

## Authoring
- [ ] `_NPR_*` glTF attribute schema frozen.
- [x] Bevy GltfPlugin registration ordering tested.
- [ ] extras/sidecar precedence frozen.
- [x] Blender export bridge is scriptable and fail-closed.

## Rendering semantics
- [x] Opaque/Mask/Blend/etc. support matrix published.
- [x] Shadow/prepass/outline behavior tested.
- [x] Skinning/morph semantics tested.
- [x] Cull inheritance tested.

## Platforms
- [x] Support matrix updated with evidence.
- [ ] Web baseline ADR closed.
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
- [ ] License/provenance ADR closed.
- [x] Bevy compatibility matrix published.
- [ ] Selected local feature matrix passes for the release candidate.
- [x] Showcase suite complete.
- [x] CLI validate/inspect shipped.
- [x] Migration notes / changelog complete.
