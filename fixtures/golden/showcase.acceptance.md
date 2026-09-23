# Showcase golden set — acceptance record

**Acceptance act:** 2026-09-23, project owner (delegated review). These ten
images are the canonical (Linux/Vulkan) baselines for the previously
demonstration-only PC showcase scenes, captured through the standard scenes
registry → driver capture path (`showcase::capture_case`) on the canonical
golden host and reviewed per scene below. Capture and acceptance are separate
acts per [../README.md](../README.md).

**Platform identity:** Linux x86_64, NVIDIA RTX 5090 (Vulkan), driver
615.71.09; Rust 1.98.0. Captured at engine commit `080893d4` (the
`showcase_golden` judgment-suite commit).

**Scene-by-scene review** (each image inspected for the content its builder
declares; non-trivial content confirmed, no placeholder frames):

| Case | Baseline shows | Why this is the right content |
| --- | --- | --- |
| `overview` | Three full-frame material spheres (white/cream/gray-blue) in a row | `builders::overview` 三材质球 + 轮廓概览; the class-contrast row is the scene |
| `lighting` | Single cream sphere with visible key-light terminator | `builders::lighting` 灯光/阴影模型; the lit/shadow boundary is the subject |
| `hair` | Dark sphere with hair-family highlight response | `builders::hair` hair material semantics |
| `metal` | Two spheres with distinct matcap/metal responses | `builders::metal` metal material class contrast |
| `reference` | Synthetic reference figure (body + head + hair plate on ground plane) | `builders::reference` loads `fixtures/synthetic/reference-scene/reference_scene.glb`; content matches the fixture's two-form figure |
| `performance` | Hero sphere plus the 100-instance grid | `builders::performance` hero_1 + portable_100 capture surface (measurement is a separate contract) |
| `lod` | Two shaded spheres plus two low-poly LOD stand-ins | `builders::lod` distance-band class layout |
| `environment-day` | Fog-blue sky/ground with three spheres, day palette | `views::environment_day` preset over `builders::environment` |
| `environment-dusk` | Same layout, warm dusk palette | `views::environment_dusk` preset |
| `environment-night` | Same layout, dark night palette | `views::environment_night` preset |

The three environment images share one builder with three view presets; each
preset owns its own baseline, matching the per-scene catalog entries.

**Accepted images** (SHA-256 of PNG bytes; FNV-1a-64 in each `.golden.json`):

- `showcase/overview.png` — `3ad1e3e9…ce3dc1`
- `showcase/lighting.png` — `ab3cfb9e…d472b8`
- `showcase/hair.png` — `576c7f9c…c53025`
- `showcase/metal.png` — `9a6b7cd2…c2e97a`
- `showcase/reference.png` — `0244695d…9a25bb`
- `showcase/performance.png` — `b55451de…74b21b`
- `showcase/lod.png` — `ecc8029b…eba3e4`
- `showcase/environment-day.png` — `3abb9223…d47291`
- `showcase/environment-dusk.png` — `f5b6cf29…9079c6`
- `showcase/environment-night.png` — `cf70c315…6124b0`

**Scope of this acceptance:** these images are the canonical baselines for the
`showcase_golden` lane suite (`tests/suites.json`, ten identities). Judgment is
the default golden tolerance (max 4/255, mean 0.5, PSNR ≥ 40 dB) against the
standard capture path; platform deltas route through the golden overlay
mechanism ([../README.md](../README.md#golden-platform-overlays)). No claim is
made about scene-motion variance: each baseline pins the deterministic
settle→observe frame of the capture profile.
