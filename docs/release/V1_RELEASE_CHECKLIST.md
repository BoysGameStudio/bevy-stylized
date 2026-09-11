# V1 Release Checklist

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
- [x] Blender upstream Golden produced before runtime export.
- [x] Reference toolchain locked (Blender / Gacha Setup / digests).
- [ ] Blender export contract frozen.
- [ ] Reference upgrade policy frozen.
- [ ] Engine Reference validated against upstream Golden.
- [ ] Production variants validated against Engine Reference.
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
- [ ] Feature matrix CI passes.
- [x] Showcase suite complete.
- [x] CLI validate/inspect shipped.
- [x] Migration notes / changelog complete.
