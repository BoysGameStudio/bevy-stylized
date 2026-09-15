# Import, inspect and render

## Supported upstream procedure

See [Genshin setup](https://gacha-setup.pages.dev/genshin/setup-character/),
[lighting](https://gacha-setup.pages.dev/genshin/lights-coloramp/) and
[rigging](https://gacha-setup.pages.dev/genshin/rigging/).
The documented path is a new Blender scene, N-panel → Character Setup Wizard →
Genshin Impact → Run Entire Setup → Select Character Folder. Individual Basic
Setup operations are useful for a bounded repair, not equivalent to a full import.

For the inspected v3.6.4 package, `config_ui.json` is the actual ordered pipeline:
FBX import → empty cleanup → shader import → material replacement → textures →
outline import/modifiers/lightmaps → material JSON → transformations → rigging →
head driver → color management → helper cleanup → material rename → mesh join →
compositor setup/defaults. Read this file and `import_order.py` before automating
a changed version. The complete UI operation is `genshin.setup_wizard_ui`;
individual model import is `genshin.import_model`.

The importer uses a recursive first-FBX lookup for folders. Prefer an isolated
single-character source directory or an explicitly selected character FBX.
The v3.6.4 source prefers `bpy.ops.wm.fbx_import`, with a Python importer fallback;
its bone/orientation handling differs, so record the actual importer used.

## Rigging choice

Record Disable Rigging, full-setup rigging, Hair & Clothes Physics and influence
settings. A new authoring rig can enable rigging; preserving a rest skeleton for
existing animation is a different operation. Do not change the choice as a silent
workaround. Blender constraints/drivers and optional Damped Track physics do not
become a Bevy runtime physics system through glTF export.

Use normal user preferences in both interactive and background sessions. For live
Blender control, use the actual configured bridge and its documented launch route.
If no bridge exists, background scripts can inspect assets; they are not a live
UI connection. Do not invent Blender MCP calls. Inspect output before saving a
candidate; save to a distinct path and preserve relative image paths or pack images.

## Background automation: verified mechanics, conditional acceptance

A local v3.6.4 trial successfully traversed the full chain in background mode
by calling the model step with a supplied FBX and the UI continuation properties:

```python
bpy.ops.genshin.import_model(
    'EXEC_DEFAULT', filepath=str(character_fbx), file_directory=str(input_dir),
    next_step_idx=1, invoker_type='invoke_next_step_ui',
    high_level_step_name='genshin.setup_wizard_ui', game_type='GENSHIN_IMPACT',
)
```

This is an observed internal operator interface, not a stable automation API.
Use only after verifying registration and the installed config/source. Run in an
isolated process/candidate scene with explicit settings. It produced a saved Odette
candidate but also material fallback warnings and rig issues; see the case record.
Do not ship a wrapper that treats its `FINISHED` result as full correctness.

## Inspection checklist that changes decisions

- Compare character meshes/material slots, shape keys and armature/deform bones
  against the selected source, not against a different game-edited model.
- Resolve missing images, image dimensions, packed/external status, linked libraries,
  UVs, active `Col` colors and custom attributes. No `_NPR_*` attributes is not
  automatically a failed Blender import; runtime requirements are separate.
- Inspect material JSON application logs and selected node instances. Missing
  expected parameter mappings require review even when textures are visible.
- Check constraint subtargets, invalid drivers, dependency cycles and pose response.
  Inactive helper issues may be benign; prove relevance before accepting or fixing.
- Record transforms/units and measured forward. Do not transfer a game's loader
  scale or yaw blindly to a newly imported raw model.
- Keep original effect meshes visible for reference unless the requested variant
  excludes them. Classify unsupported effects explicitly rather than deleting
  geometry to improve a comparison.

The bundled `inspect_blend.py` emits mesh, image, group input, rig and scene data.
It reports linked defaults, not evaluated linked outputs. Investigate rig-driven
palette values at the controlling object/property and rendered result.

## Reference capture

Fix pose/frame, camera transform/projection, resolution, world/sun, exposure,
view transform, compositor and renderer/device. For Gacha, also record the
Light Direction object, lighting preset/custom palette, Day/Night and shadow
threshold. A Bevy directional light alone does not reproduce the authored palette.
Use full-body and face views plus affected material closeups. Inspect every image
for clipping, missing parts, incorrect effects and blank output.

Same-source reproduction and cross-renderer parity are separate comparisons.
Store capture settings and PNG/provenance beside the candidate. Diagnostic renders
are not accepted Golden baselines. Updates to references require the requested
scope and explicit evidence; never relax thresholds just to make a comparison pass.

## Failure routing

| Symptom | First owner to investigate |
|---|---|
| Wrong model/extra animated skeleton | Selected folder and first-FBX recursion |
| Missing/gray texture | Download completeness, paths, image binding/colorspace |
| Parameters ignored but setup reports success | Material-data parser/applier and fallback log |
| Wrong facial shadow as head turns | Face map, head/light drivers and light basis |
| Skeleton warnings/pose defects | Rig creation, bone names, constraint targets and cycles |
| Effect skirt renders as opaque patterned shell | Effect material support and intended variant |
| Existing blend looks unchanged after plugin update | Embedded node groups and rig were not reimported |

The upstream Genshin Bugs & Solutions page was “Coming soon” on 2026-09-14;
consult versioned source/issues and local reproductions instead of treating that
page as a complete troubleshooting manual.

## Maintained independent recipe

For this repository's qualified static variant use tools/dev.py assets prepare/build
and inspect the recipe and build manifest. The generic full GUI workflow above is
not equivalent to that restricted recipe. Personal add-on installation, historical
source directories and prior generated game GLBs are never prerequisites. Details
are in the current Odette reference and the project's authoring guide.
