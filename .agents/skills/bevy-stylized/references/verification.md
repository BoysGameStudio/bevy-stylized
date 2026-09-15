# Verification, diagnosis and upgrades

## Select evidence for the actual change

1. File/contract changes: inspect GLB, dependencies, attributes, sidecar/extras
   agreement, material coverage and compiler outputs.
2. Runtime integration: prove scene/material adoption and GPU pipeline creation,
   then capture relevant views after assets and shader warm-up.
3. Shader/lighting changes: normalize camera, pose, light/palette, color management,
   resolution and AA; compare the affected semantic region as well as global error.
4. Instance/LOD/performance changes: measure relevant counts, resolved tiers, draw
   behavior and frame-time distributions on the same device/settings. Record visual
   delta alongside timing. Avoid broad performance runs for documentation changes.

Useful targeted commands in the current workspace:

```sh
python3 tools/dev.py check --lane cpu
python3 tools/dev.py check --lane gpu --required
python3 tools/dev.py check --lane asset-integration --required --asset BUILD/compiled/character.glb
python3 tools/dev.py check --lane reference --required --case BUILD/case.json
```

GPU cases are explicitly ignored by ordinary CPU tests and executed by their GPU
lane. Missing assets/device/baseline fails a required lane. The external character
is not a default CPU fixture. Reference comparison uses the declared synthetic
scene with original maps, not full-character parity. Diagnostic capture success
is separate from the metric result; --required propagates actual failure.

The repository `docs/reference/08-golden-tolerance-and-cross-layer-criteria.md`
defines separate same-renderer reproduction and draft cross-renderer criteria.
At the inspected revision cross-layer rules include nonedge max, mean, PSNR,
shadow IoU and silhouette IoU. Read their current values; do not freeze these
numbers into a skill or relax them silently.

Performance when relevant:

```sh
cargo run -p regression-lab -- --out CANDIDATE_REPORT.json \
  --gate EXISTING_BASELINE.json --golden-dir EXISTING_GOLDEN_DIRECTORY
```

Inspect missing/skipped cells and adapter identity. The current gate rejects missing/duplicate coverage, invalid timings, changed
resolved quality/backend, mismatched GPU identity and failed Golden results. A new GPU/backend/resolution
requires qualification, not comparison to unrelated timings. Serialize Cargo builds
and performance runs; competing GPU work invalidates timing measurements.

## Diagnose in dependency order

Asset identity/variant → exported object scope and coordinates → texture/data
channels → material parameters → instance/light basis → color management/post →
unsupported effects. Preserve a controlled A/B and change one owning boundary at
a time. A geometric mismatch cannot be repaired with palette tuning; metadata
validation cannot demonstrate shader parity.

If live ECS inspection would resolve adoption/capability questions and the app
exposes BRP, use the available bevy-copilot skill/CLI after reading its instructions.
Otherwise use the application's diagnostics and controlled test scene; do not
assume a remote protocol exists merely because the app uses Bevy.

## Upgrade workflow

Treat plugin installation, source reimport, export-contract qualification and
Golden acceptance as separate checkpoints. Record actual Blender/Gacha/exporter/
Bevy versions, input/texture/shader hashes and settings. Compare old/new node group
interfaces/topology and material/rig results before updating signatures. A new
signature should be justified by semantic investigation, not copied from whatever
happened to export successfully. Preserve the old candidate and its evidence.
