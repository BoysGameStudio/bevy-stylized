# Current Gacha authoring behavior contract

The supported source is the release/hash in
[the authoring lock](../../assets/sources/gacha-authoring.json), consumed through
[the current static recipe](../../assets/recipes/genshin-static-v4.json).

Current responsibilities are split deliberately:

- Source preparation validates archive/file identity and declared model scope.
- `tools/blender/import_character.py` and `inspect_character.py` establish the
  configured source scene and audit missing images, invalid drivers/constraints.
- `capture_character.py`, `character_cases.py`, `lighting_probe.py` and
  `matched_capture.py` evaluate and record the actual camera, lighting, sampling
  and comparison images. The intermediate case revisions are outputs of these
  current stages, not a promise of support for an old addon release.
- `export_character.py` exposes supported source material inputs and maps;
  `bevy-stylized-tools compile --profile genshin --strict` enforces the current
  handoff. An arbitrary Shader-to-RGB graph is not automatically a runtime material.
- `labs/reference/src/character/` checks the declared current cases against the
  independent source captures. Capture completion and parity success are distinct.

Use current source-derived audit data instead of assuming old node-group names,
lighting defaults, texture assignment or historical source maps. The recipe is a
restricted static body/face qualification, not blanket full-rig/effect acceptance.
Current thresholds remain in [comparison criteria](08-golden-tolerance-and-cross-layer-criteria.md).
