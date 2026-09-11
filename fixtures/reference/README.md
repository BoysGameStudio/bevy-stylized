# Reference Fixture (self-made, committable)

`reference_scene.blend` is a 100% self-made fixture scene generated
deterministically by `tools/blender-reference-export/make_reference_scene.py`
(id `reference_scene_v1`, hashes locked in
`docs/reference/reference-toolchain.toml` `[fixture]`). It contains no
upstream-derived content and is committable per
`docs/reference/05-license-and-provenance-boundary.md`.

The binary `.blend` (~192 KB) is committed alongside its generator because
the export bridge and the render step consume a `.blend` file directly;
regenerate it with the script rather than editing it by hand.

- `reference_scene.blend` — fixed camera/sun/ambient, 512x512, EEVEE /
  sRGB / Standard; body sphere + head proxy + hair card + ground, carrying
  the `_NPR_*` attribute set from
  `tools/blender-reference-export/contracts/npr-attributes.v1.json`.
- `reference_scene.glb` — runtime export artifact produced by the export
  bridge (fixture role) with the locked `blender-gltf-npr-v1` settings.
- `export-manifest.json` / `export-sidecar.json` /
  `node-graph-signature.json` — bridge outputs (see
  `tools/blender-reference-export/README.md`).
