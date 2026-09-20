---
name: bevy-stylized
description: Export and adapt Blender assets for the bevy_stylized framework, map NPR material and instance semantics, integrate glTF scenes, and validate rendering, fallback and performance. Use for this framework's asset or rendering workflows, not generic Bevy gameplay or unrelated stylized renderers.
---

# Bevy Stylized

Turn an authored asset into explicitly mapped runtime materials and verified
rendered output. The framework's plugin does not automatically reproduce an
arbitrary Blender node graph or every Gacha effect.

## Route and establish context

Read applicable AGENTS.md, then locate the workspace root from Cargo.toml and
its `bevy-stylized*` crates. Resolve current source and toolchain versions.
Use `blender-gacha` for upstream acquisition/import if available, or ask for the
handoff evidence described in [Export and handoff](references/export.md).

Read only the relevant detail:
- [Export and handoff](references/export.md): new asset, re-export or schema work.
- [Runtime semantics](references/runtime.md): material adoption, textures, instances,
  plugin ordering, capability routing and feature limitations.
- [Verification](references/verification.md): diagnostics, visual regression,
  performance and upgrades.
- [Local evidence](references/evidence.md): dated tested behavior and known gaps.

## Workflow

1. Inspect inputs and choose the exact operation. Distinguish a new Gacha import,
   a re-export of an existing game blend and a runtime material integration.
   Preserve authored sources and accepted baselines; use isolated candidate outputs.
2. Read the actual export runner, current recipe and source locks. Validate version, scene scope,
   outline modifiers, texture paths, units, rig and vertex attributes before export.
   An old signature must not stamp a new import with old provenance.
3. Establish the runtime mapping. The current `.genshin.json` handoff, export/build
   completion records, `<asset>.npr.json`, `bevy_stylized.material/v1` and
   `bevy_stylized.compile/v1` are different contracts.
   Never rename one JSON file and assume conversion. Unmapped material semantics
   are explicit adaptation work, not an invitation to fabricate values.
4. Run the repository's validate/inspect/compile commands as applicable. Review
   material coverage and output, not just exit codes: compiler defaults are valid
   descriptors but may carry none of the requested Genshin look.
5. Configure glTF custom attributes before GltfPlugin registration. Load the scene,
   wait for assets/entities and explicitly adopt runtime materials through the
   application's supported path. Keep static material, per-instance state and
   per-view lighting separate. Preserve batching and report resolved fallback.
6. Validate the requested behavior: structure first, GPU/render output second,
   performance only when affected. Missing GPU/assets and report-only comparisons
   cannot be reported as a visual pass. Surface Class is never silently reduced
   to satisfy a lower Execution Tier.
7. Deliver changes, commands, artifact paths and evidence with exact limitations.
   Update repository docs/contracts when authorized changes alter them. Do not
   silently rebaseline, relax tolerances or mark pending ADRs accepted.

## Helper

```sh
python3 scripts/inspect_asset.py ASSET.glb
python3 scripts/inspect_asset.py ASSET.glb --require-material-metadata
```

Resolve `scripts/` relative to this skill. This read-only inventory checks basic
GLB shape, external dependencies and descriptor coverage. It flags raw metadata
differences for the project importer to normalize and validate. It is not a
substitute for the repository validator, GPU tests or visual inspection. The
strict option applies to descriptor-driven integration; explicit application-side
material adoption can legitimately use an asset without descriptors.

## Scope discipline

Use current source when documentation drifts, and record the discrepancy.
Do not expand a skill task into a renderer rewrite: prove the missing boundary,
provide a concrete adaptation plan, and finish supported parts honestly. Odette's
mesh deletions, animation names, scale and game-specific secondary motion are example
history, not universal framework requirements. Production code must not depend
on legacy reference crates or Blender/Gacha tooling.
