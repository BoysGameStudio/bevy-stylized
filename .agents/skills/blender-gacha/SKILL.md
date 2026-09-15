---
name: blender-gacha
description: Acquire and trace Gacha character assets, install or update Gacha Setup, import and inspect characters in Blender, and prepare reference renders and a verified handoff. Use for Gacha Setup authoring or reimport tasks; downstream bevy_stylized export and runtime integration belong to bevy-stylized.
---

# Blender Gacha

Produce an inspectable Blender character and a reproducible source record.
Genshin is the verified family; other games require their own upstream workflow
and must not inherit Genshin shader or texture assumptions.

## Choose the operation

- New asset or source question: read [Sources](references/sources.md).
- Install, upgrade, reimport, materials, rig or reference capture: read
  [Authoring](references/authoring.md).
- Passing the result to an engine: read [Handoff](references/handoff.md).
- Odette specifically: read [Odette evidence](references/odette.md).

Read the relevant repository's AGENTS.md and locate its current toolchain locks.
Discover Blender and the asset root; do not assume a user name, checkout path or
installed version. Follow the requested scope: research is not permission to
replace authored scenes, and an import does not authorize accepting new Golden
baselines. Continue reversible, authorized work in a separate candidate.

## Execute

1. Establish source identity: game, character ID/internal name, variant, channel,
   download URL and retrieval time. Keep original bytes and hash them. JSON
   semantic equality may explain byte differences but does not replace file hashes.
2. Check actual `bpy.app.version_string`, enabled `setup_wizard` module/version,
   plugin release archive and relevant shader files. A newly installed plugin
   does not update node groups or rigs embedded in old `.blend` files.
3. Preserve normal Blender preferences. Use the existing tool/bridge when live
   inspection is needed; ordinary background Blender is appropriate for scripts.
   Do not reset preferences or use `--factory-startup` unless requested.
4. Isolate the chosen character FBX, Materials and Textures from animations and
   other variants. Run the version-appropriate complete setup or requested step.
   Record rigging, physics, compositor and lighting choices.
5. Inspect the output and logs. `FINISHED`, a saved blend, or a successful process
   exit is not proof that all material appliers, drivers and constraints worked.
   Use the bundled read-only audit, review relevant findings, then render.
6. Deliver candidate paths, source/version records, reference images and known
   limitations. Hand off to `bevy-stylized` when runtime work is requested; that
   skill need not be installed to produce the portable handoff described below.

## Helpers

Paths below are relative to this skill directory; resolve them before execution.

```sh
python3 scripts/source_manifest.py CHARACTER_INPUT --compare PREVIOUS_INPUT
blender --background CANDIDATE.blend --python-exit-code 1 \
  --python scripts/inspect_blend.py -- --out NEW_AUDIT.json
```

The source helper exits 1 for ambiguous FBX selection/symlinks and 2 for IO/usage
errors. The Blender helper writes only a new JSON file and never saves the blend;
its output is evidence to review, not an automated visual acceptance decision.
Packed files count as available; sequence/UDIM placeholders need tile/frame review.
Linked socket defaults are recorded as defaults, not evaluated rig-driven colors.

## Completion

Match evidence to the request. For a reimport: source/version/options known,
character saved separately, relevant dependencies present, rig/material findings
classified, and requested views inspected. If the upstream importer fails, report
the owning step and preserve a diagnostic candidate; do not hide the failure by
silently disabling rigging or rewriting source materials. If a scoped adaptation
is required, explain it and execute it only within the authorized work.
