# Static character capture execution — 2026-09-14

The authorized plan produced a reproducible, independent static comparison workflow.
The continuation now passes all six scoped static character comparisons. Historical
M1 results below remain evidence; they are superseded for the scoped character
by the [continuation validation](../quality/static-character-parity-validation-2026-09-14.json).
General upstream feature parity is not implied.
No animation/expression controller, Git commit, publication or accepted-baseline
replacement was introduced.

## Delivered workflow

- Shared versioned capture packages: source/asset identities, evaluated Blender
  inputs, exact runtime cameras, explicit view settings and finite readiness gates.
- Two views × three light directions, with two captures on each renderer per case.
- Exported first-party geometry/color calibration, an additional-Sun-off probe,
  independent opaque geometry coverage masks, full-frame metrics and difference PNGs.
- A local comparison page with blinking pairs and supplementary material-role ROI
  diagnostics for body/hair/face/pupil. The ROI masks follow original material
  identity, not error-selected areas; full-frame qualification is unchanged.
- Diagnostic versus strict execution and negative input/GPU/loading/timeout checks.
- Complete source-to-render isolation, followed by host core/glTF path checks.

See the maintained [capture contract](../reference/09-static-character-capture.md).
The final local page is `target/stylized/static-character-v4-final/index.html`.
The matching host package is
`assets/local/genshin-odette/builds/dde34331b68cf965df8f11488c09f5221c85e2e5ad9a3643eb895a6babd7c1e4/character-cases/case.json`.
ROI scripts/masks and their provenance are retained under
`target/stylized/roi-evidence`; this supplementary analysis uses the Pillow version
recorded in material-rois.json, rather than adding a dependency to CPU/product builds.

## Fixed rendering and authoring defects

1. Explicit Hero Genshin mode mapped ILM.G into [0.5,1] before a smoothstep gate
   spanning [0.01,0.4], so zero AO remained fully lit. It now consumes G directly.
   A GPU zero/full-AO fixture fails on the old shader with correctly bound textures
   and passes after correction; generic-mode AO is unchanged.
2. Hero material event handling could skip inherited ILM/Face SDF presence flag
   updates. Both are now included in the consistency check, with a CPU regression.
3. The upstream importer overwrote shared ramp nodes in directory traversal order.
   Furthermore, top-level ramp previews were disconnected from actual shading.
   Recipe **genshin-static-v4** binds original JSON-named textures through a
   per-material copy of the actual shader dependency closure; export reads that
   closure. Body, Body01, Hair, Dress and Dress01 retain their explicit source
   ramp identity. V2/V3 candidates remain historical evidence and are not restamped.
4. The cross-renderer comparator now rejects unequal width/height even if byte
   counts are equal. Readiness distinguishes transient shader loading from fatal
   processing errors. Geometry masks ignore NPR vertex-control colors in temporary
   mesh copies and restore original rendering state afterward.

The static geometry/feature scope remains unchanged. Original FBX, textures,
material JSONs, installed add-on files and authored source scenes remain intact.

## Historical M1 evidence

[Machine-readable validation](../quality/static-character-validation-2026-09-14.json)
records the exact isolated source snapshot and output evidence hashes.

| Check | Actual result |
|---|---|
| Rust CPU | 163 passed, 0 failed; 26 GPU/environment cases explicitly ignored |
| Python | 11 passed |
| Required GPU | 13 passed |
| Synthetic character import | 1 passed |
| Odette adoption/sharing/reload | 1 passed |
| Geometry/color calibration | All 6 markers passed; ≤0.5 pixel center error, ≤1/255 center color error |
| Same-candidate repeated capture | Exact pixels on both renderers, all 6 cases |
| Independent host/isolated rebuild | All 12 Blender references and 6 Bevy first captures have exact matching pixels |
| Additional Sun on/off | Maximum pixel delta 0 for the declared front full-body probe |
| Missing/invalid input checks | Expected validation/resource failure codes, no false success |
| Complete isolation | PASS, no Git/asset cache/network/sibling project dependency |
| Return to host | Core manifest guard 1 passed; glTF ordering/path tests 2 passed |
| Formatting/Clippy/docs | PASS, Clippy workspace/all-targets/all-features with -D warnings |

Recipe v4 resolved the cross-import visual drift. Face buffer ordering may differ
between imports, but canonical geometry/material/normal/UV tuples matched, and
final images match exactly. Rendering the same saved scene on the host also
reproduced its isolated images exactly.

## Historical M1 visual failures

| Case | PSNR dB | Mean normalized delta | Geometry coverage IoU |
|---|---:|---:|---:|
| Full body / front | 35.678 | 0.003267 | 0.98289 |
| Face / front | 32.047 | 0.007002 | 0.99398 |
| Full body / left | 34.841 | 0.003690 | 0.98289 |
| Face / left | 29.672 | 0.009542 | 0.99398 |
| Full body / right | 34.875 | 0.003680 | 0.98289 |
| Face / right | 29.854 | 0.009377 | 0.99398 |

These values do not mean overall PASS: non-edge errors still fail, side-light face
PSNR is below 30 dB, full-body coverage is below 0.99, and true lighting-factor
shadow masks remain unqualified. Median-luminance shadow IoU is a legacy proxy.
Strict correctly returns 1. No tolerance, image crop or accepted baseline was
changed to obtain success.

The separate locked 3.6.2 synthetic comparison also remains FAIL: about 20.73 dB
PSNR after the scoped AO change (previously about 20.96 dB). Its mean error is about
0.03778 and silhouette IoU about 0.99605. It is retained as independent evidence,
not replaced by the new character results.

## Follow-up executed in this continuation

- Isolate thin-card/feather raster coverage using first-party geometry, keeping the
  recorded EEVEE and MSAA-4 sampling policies explicit. Do not assume an AA change
  is a shader fix or relax the silhouette gate.
- Add true upstream/runtime lighting-factor diagnostics before qualifying shadow
  semantics; use the side-light face cases to isolate Face Factor/SDF composition.
- Use the material-role ROI report to select minimal shading fixtures, while
  retaining every whole-frame gate. The planned standalone normal-sphere fixture
  was replaced in this execution by actual light-attribute/zero-offset audits,
  runtime light-transform checks and the GPU AO fixture; full lighting-function
  equivalence is still not claimed.

These are rendering tasks, not animation, rigging or expression-control features.

## Completed continuation

The maintained build now produces v2 lighting diagnostics and a v3 matched HDR
sampling package from original assets. Product shaders correct band order, ramp
row orientation, safe divide, vertex AO/width conversion, face projection/control
and tint semantics, nonmetal specular and the separate pupil highlight closure.
Cubic control sampling and closest ramp sampling follow actual source nodes.
Orthographic high-quality specular now uses the camera basis rather than a
perspective approximation or reverse-Z projection row.

All six freshly rebuilt character comparisons pass unchanged RGB thresholds and
explicit geometry/lighting-factor gates. Maximum nonedge deltas are 5,12,6,12,7,12
out of 255 in case order; all PSNR values exceed 51 dB. Both renderers repeat exactly.
The final local review page is `target/stylized/static-character-qualified/index.html`.

Validation: 164 Rust CPU tests, 12 Python tests, 19 required GPU tests, synthetic
import and external asset adoption all pass. Workspace/all-target/all-feature
Clippy and documentation checks pass. The complete Bubblewrap rebuild reruns
these gates without Git, networking, existing project art cache or a sibling
project. Its exact source identity and separate legacy synthetic parity result
are recorded in the continuation validation. The old v1 AA and median-luminance
proxy metrics remain visible; they are not substituted for real semantic masks.
Animation, expressions, rigging and physics remain external responsibilities.
