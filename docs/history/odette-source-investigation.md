# Odette source and reimport investigation

Status: initial investigation, updated 2026-09-14. The subsequent full v3.6.4
import and source comparison are documented in
[the workflow research report](blender-gacha-bevy-workflow-research.md). The import
produced a diagnostic candidate with material/rig findings, not an accepted Golden.

## Provenance established locally

The immediate source is the sibling Tenacity checkout:
`/home/x/Projects/Tenacity/assets-src/characters/odette/`.
Its README records the original FBX, textures, material metadata and importer
archive as the portable source bundle. Tenacity commit
`0155196c9f659b1fe06a14a7148675d663728398` (2026-09-01,
`chore(assets): vendor portable Odette sources`) introduced that bundle.
Earlier commit `8603d9d6fc3dbb94f9b5759b82bcf4e3763a2ee3` introduced the game
model and GLB consumer.

- Original mesh: `Avatar_Girl_Sword_Odette.fbx`.
  SHA-256: `278df9bd75c5fa9b82f90c1123926bdff10483dd85285e8c5becc16f2e5eb02a`.
- Dependencies: `Materials/` (11 JSON files), `Textures/` (41 files).
  These include body/body01/hair diffuse, lightmap, normal and shadow ramps,
  face SDF, pupil maps, MetalMap and Specular Ramp.
- Archived original importer: `Gacha_Setup-v3.3.2.zip`.
  SHA-256: `430980237eb538e35aeee3702eb0c8a788970ce9971da56de1d9508b639a6eaa`.
- Current locally installed importer: Gacha Setup v3.6.4, Blender 5.2.1 LTS.
- Asset family: treated by this repository as third-party HoYoverse-derived
  Genshin art. HoyoToon now provides a verified matching source: character ID 10000150,
  Odette / 奥黛塔, variant Snow Swan's Dance. All 42 binary files are identical
  and 11 material JSONs are semantically equal to the local bundle. The original
  user download history and extraction tool remain unknown. See the subsequent
  research report for URLs and comparison evidence.
- Gacha Setup is the importer/shader tool, not the model distributor; its
  upstream README explicitly says that game models/textures are not included.

## Existing asset lineage

`FBX + Materials + Textures` → `Odette.blend` (original imported reference)
→ retarget/visual authoring (`Odette_Retarget.blend`,
`Odette_Kawaii_Test.blend`) → `build_odette_game.py` → `Odette_Game.blend`
→ `export_odette.py` → `models/odette.glb` + `models/odette-npr/`.

The retarget source supplies animation work; the Kawaii source supplies the
assembled game visual. This is not a simple automatic FBX-to-game conversion.
The game visual intentionally removes `Eff_001` and `Eff_002` skirt parts;
the untouched imported source retains the original reference. `Odette_Cloth.blend`
is a separate authored cloth branch. Inspection of the current runtime GLB
confirms 5 meshes, 8 materials, 1 skin, 19 animations and 8 embedded images.

Our `golden_odette` and `odette-reference-lab` consume the runtime GLB.
Our three-layer `reference-comparison` instead uses the synthetic
`reference_scene.glb`, with some Odette NPR textures.

## Reimport the original character with v3.6.4

1. Prepare a separate local-only input directory containing **only** the
   original character FBX and the complete `Materials/` and `Textures/`
   directories. Preserve names and relative layout. Do not include `animations/`.
   Inspection of installed `setup_wizard/genshin_import_character_model.py`
   shows `__find_fbx_file` recursively returns the first matching FBX; an
   isolated input removes any ambiguity with animation FBXs.
2. Open a new Blender scene using the normal user preferences and add-ons.
   Keep all existing authored `.blend` files intact.
3. In the 3D View press **N**, select **Character Setup Wizard**, choose
   **Genshin Impact**, then **Run Entire Setup** and **Select Character Folder**
   for the isolated input. Record the rigging, hair/clothes physics and
   post-processing options used. For a full new rig, leave **Disable Rigging**
   unchecked; preserving an existing rest rig is a separate import choice.
4. Let the complete pipeline finish: FBX, shader materials, material replacement,
   texture binding, outlines, material metadata, transformations and rig setup.
   Merely using Blender's generic FBX import does not recreate the toon setup.
5. Inspect body/dress/hair, face SDF, pupil and highlight textures, ramps,
   `Col` vertex colors, outline modifiers, head/light drivers and missing image
   paths. Record the resulting object/material/bone counts rather than assuming
   they match the authored game's strict export contract.
6. Save as a new local candidate, e.g.
   `target/tmp/odette-reimport-v3.6.4/Odette_Reimport_v3.6.4.blend`.
   Copy the source texture directory beside the candidate before making paths
   relative, or pack the required images. Preserve provenance and option settings.
7. Render fixed full-body and face views against the original imported source.
   Plugin installation alone does not upgrade node groups or rigs already saved
   inside old Blender files. New import validation precedes any baseline change.

Upstream instructions:
- https://gacha-setup.pages.dev/genshin/setup-character/
- https://gacha-setup.pages.dev/quickstart/
- https://github.com/PaoloESAN/gacha-setup/releases/tag/v3.6.4
- https://github.com/PaoloESAN/gacha-setup

## Re-exporting the existing game asset is a different operation

To produce an isolated candidate from the existing authored game blend, the
focused Tenacity publisher supports `--output` (verified in source):

```sh
cd /home/x/Projects/Tenacity
blender --background assets-src/characters/odette/Odette_Game.blend \
  --python-exit-code 1 \
  --python assets-src/blender/export_odette.py -- \
  --output /home/x/Projects/bevy-stylized/target/tmp/odette-export-review/odette.glb
```

This also writes sibling `odette-npr/` textures and runs the exporter's existing
validation. The command is documented here, not executed in this investigation.
It does not reimport the model or adopt v3.6.4 shaders. A newly imported character
cannot be passed directly to that strict publisher without recreating the game
visual/animation contract. Do not use `--softbody-preview` to bypass that contract.

For the current research goal, first validate the new upstream Blender import;
then assess export-bridge/schema adaptation and downstream rendering separately.
Existing project reference-toolchain locks and Golden provenance remain tied to
their recorded versions until that validation is performed.
