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
| `effects` | Three spheres: view-lit, +Z override, −Z flipped (dimmer right) | `builders::effects` per-material light-basis override contrast (captured at engine commit `a7732894`) |

**Full-catalog extension (2026-09-27, same acceptance act rules):** the browser
host judges against this set uniformly, so the seven scenes already verified
through their own suites also received showcase-path baselines (captured at
engine commit `00c1ca67`, same canonical host). Their own suites keep their
specialized capture setups and coverage credentials; these images serve the
showcase-path and browser comparison. Reviewed content: `ramp` (colored ramp
banding on spheres), `face` (half-lit SDF spheres), `outline` (rotated hull
box), `animation` (posed skinned box), `post` (bloom/vignette spheres),
`stress` (1000-instance grid), `outline-advanced` (hull + screen hybrid).

- `showcase/ramp.png` — `c7647257…0a6802b9`
- `showcase/face.png` — `57ee7c04…900cf28f`
- `showcase/outline.png` — `d66ab313…b22c9f5`
- `showcase/animation.png` — `c863c2c6…85dfc66`
- `showcase/post.png` — `79a9745c…e52090a`
- `showcase/stress.png` — `01d4293a…35ab788`
- `showcase/outline-advanced.png` — `01f377dc…0acc0f1`

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
- `showcase/effects.png` — `e249cdae…340d17`

**macOS/Metal note (2026-09-23/27):** of the eleven cases, ten match these
canonical baselines on the M4 Pro within tolerance (max 1–2/255 cross-vendor
rounding, mean < 0.03). `showcase/effects` differs by exactly one pixel (delta
29 at (376, 305), on the flipped-sphere boundary — the same root-caused
platform-truth class as the existing `macos-metal` overlays) and is accepted
into the overlay at `showcase/macos-metal/effects.png` (SHA-256
`5784b37c…daff2a`; captured at engine commit `7431c331`, sidecar provenance
alongside). The full-catalog extension adds three more overlay cases, all in
the root-caused classes (reviewed per the same diff-map discipline):
`showcase/macos-metal/face` (2 px, max 21, SDF threshold boundary),
`showcase/macos-metal/stress` (4 px, max 49, hull silhouette rounding),
`showcase/macos-metal/outline-advanced` (5288 px on screen-outline edge lines,
max 186 — the same class as the accepted outline-advanced suite overlays);
captured at engine commit `cf077981`. The remaining 14 cases match canonical
within tolerance and have no overlay.

**Scope of this acceptance:** these images are the canonical baselines for the
`showcase_golden` lane suite (`tests/suites.json`, ten identities). Judgment is
the default golden tolerance (max 4/255, mean 0.5, PSNR ≥ 40 dB) against the
standard capture path; platform deltas route through the golden overlay
mechanism ([../README.md](../README.md#golden-platform-overlays)). No claim is
made about scene-motion variance: each baseline pins the deterministic
settle→observe frame of the capture profile.
