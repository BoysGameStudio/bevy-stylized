# Current Blender reference export bridge

> **Status:** Normative architecture specification
> **Authority:** Current authoring/export responsibilities. The [reference workflow](../reference/01-reference-authoring-workflow.md) owns executable reproduction.

The bridge converts the qualified Blender/Gacha source into the framework's
current runtime inputs. It is development tooling; product crates do not depend
on Blender, Gacha, a game checkout or the bridge itself.

## Supported entrypoint

Run from the project root after preparing the source-locked current inputs:

```sh
python3 tools/dev.py assets prepare --pack genshin-odette
python3 tools/dev.py assets prepare --pack gacha-authoring
python3 tools/dev.py assets build --pack genshin-odette --blender blender
```

The current recipe is `assets/recipes/genshin-static-v4.json`. The build runner
in `tools/assets/build.py` supplies explicit paths/options to
`tools/blender/import_character.py`, `inspect_character.py`,
`capture_character.py` and `export_character.py`, followed by strict profile
compilation and the current character comparison-case preparation stages.
Use this runner rather than invoking an exporter with guessed arguments.

## Responsibilities and outputs

The bridge must verify the prepared source identities and recipe, reject missing
maps and unsupported source semantics, audit the imported authoring scene, and
keep reference capture independent of production rendering.

A successful content-addressed build directory contains:

- `authoring/character.blend`, `authoring/authoring.json` and `authoring/audit.json`:
  the scoped authoring candidate and its input/audit records;
- `reference/`: direct Blender reference captures and their metadata;
- `runtime/character.glb` and its typed profile handoff, exported by
  `tools/blender/export_character.py`;
- `compiled/character.glb` and compiled material descriptors from strict Genshin
  profile compilation;
- `character-matched/case.json`: the completed comparison input, following the
  current intermediate capture stages;
- `build-manifest.json`: the current `bevy_stylized.build/v1` completion record,
  with input identities and hashes of generated artifacts.

A manifest is published only after successful build stages. Partial or failed
attempts are not accepted outputs. Missing textures, invalid constraints/drivers,
unknown semantic mappings, changed source identities and invalid artifact hashes
must fail rather than silently guessing a compatible result.

## Boundaries and validation

Only the current qualified source/recipe is supported. The deleted synthetic
reference exporter is not an alternate path or a compatibility requirement.
Current glTF attribute and profile schemas own the runtime data contract; the
[static capture contract](../reference/09-static-character-capture.md) owns the
qualified scope and comparison stages.

Build completion proves an export/compile handoff. Independent image comparison,
runtime adoption, GPU behavior and supported-platform qualification remain
separate checks. A successful production capture does not manufacture its own
upstream reference, and a failed metric must not be relabelled as success.
