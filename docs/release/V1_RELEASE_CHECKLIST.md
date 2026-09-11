# V1 Release Checklist

## Architecture decisions
- [ ] No ADR remains in `V1 Blocker` status.
- [ ] Every `Accepted` ADR contains completed Decision, Evidence and Consequences.
- [ ] `TRACEABILITY.md` references every normative ADR relevant to V1.
- [ ] Documentation linter passes with no errors.

## Architecture
- [ ] Surface Class × Execution Tier × Validation Mode implemented without semantic conflation.
- [ ] Material / Instance / View separation enforced.
- [ ] Instance transport ADR closed.
- [ ] Runtime Material backend ADR closed.
- [ ] Whole-pipeline resource budget ADR closed.

## Reference
- [ ] Blender upstream Golden produced before runtime export.
- [ ] Engine Reference validated against upstream Golden.
- [ ] Production variants validated against Engine Reference.
- [ ] Reference artifact provenance complete.
- [ ] Reference-only code/assets excluded from default shipping build.

## Authoring
- [ ] `_NPR_*` glTF attribute schema frozen.
- [ ] Bevy GltfPlugin registration ordering tested.
- [ ] extras/sidecar precedence frozen.
- [ ] Blender export bridge is scriptable and fail-closed.

## Rendering semantics
- [ ] Opaque/Mask/Blend/etc. support matrix published.
- [ ] Shadow/prepass/outline behavior tested.
- [ ] Skinning/morph semantics tested.
- [ ] Cull inheritance tested.

## Platforms
- [ ] Support matrix updated with evidence.
- [ ] Web baseline ADR closed.
- [ ] Backend-specific pipeline creation tests run.
- [ ] Any `Supported` target has real runtime evidence.

## Quality / Performance
- [ ] Golden suite passes.
- [ ] Whole-pipeline resource-budget suite passes.
- [ ] Automatic instancing / instance-data regression passes.
- [ ] Required benchmark scenes recorded.
- [ ] No performance claims without before/after evidence.

## Release / Governance
- [x] ADR-0016 Package Naming closed: `bevy-stylized`.
- [ ] License/provenance ADR closed.
- [ ] Bevy compatibility matrix published.
- [ ] Feature matrix CI passes.
- [ ] Migration notes / changelog complete.
