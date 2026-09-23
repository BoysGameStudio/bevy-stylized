# Golden tolerance and independent comparison criteria

> Status: Existing numeric criteria retained without relaxation. ADR-0007 is
> Accepted (2026-09-22); the §3 cross-renderer thresholds are the qualified
> static-scope criteria. A failed metric is still a failure.

## 1. Same-toolchain reproduction

For the same device, toolchain and declared scene: maximum normalized channel delta
≤ 2/255, mean delta ≤ 0.5/255, PSNR ≥ 48 dB. Exact pixel equality is preferred.
Different GPU/driver qualification requires its own measured policy; no additional
platform is declared supported from the current Linux/Vulkan evidence alone.

The canonical golden baseline host is Linux/Vulkan. A non-canonical platform
qualifies through the measured **platform overlay** mechanism
([fixtures/README.md](../../fixtures/README.md#golden-platform-overlays)): the
canonical baselines stay byte-identical, and an overlay holds only the cases whose
deltas are root-caused platform truth (e.g. cross-vendor rounding at numerically
hypersensitive boundaries), each captured through the normal capture path on the
qualified device and accepted by review. An overlay never relaxes a threshold —
the overlay image replaces the comparison input for that platform, with its own
provenance. Same-toolchain reproduction criteria above apply unchanged to
repeat captures within each platform.

## 2. Normalize the compared case

Fix source/model scope, camera/projection, pose/frame, lights and authored palette,
resolution, AA, world, exposure and display transform. Current reference fixtures
use 512×512, Blender Standard/sRGB and Bevy without an additional tonemapper.
Do not resize or retune one side merely to lower an error metric. Attribute and
texture maps must be the declared inputs; missing maps cannot switch case behavior.

## 3. Existing cross-renderer thresholds

| Metric | Required value |
|---|---|
| Non-edge normalized maximum channel delta | ≤ 16/255 |
| Whole-image mean channel delta | ≤ 4/255 |
| PSNR | ≥ 30 dB |
| Shadow-mask IoU | ≥ 0.95 |
| Silhouette-mask IoU | ≥ 0.99 |

Edge exclusion and masks use the existing implementation in tests/harness/src/tier2.rs.
All applicable criteria must pass. Missing resources and baseline images are not
metric successes. `reference-comparison --required` returns failure if any actual
criterion fails; diagnostic capture success is reported separately.

## 4. Qualification scope

Use the declared current character case and source lock. Its report must name
actual inputs, adapter, metrics and pass/fail for each applicable criterion.
A prior source or renderer result is not a substitute for a current changed
case. Structural adoption, independent image criteria and platform qualification
remain separate. The retired synthetic reference lane is not an acceptance input.

## 5. Browser WebGPU (Dawn) vs native Vulkan/Metal

Owner-approved 2026-09-22: per-browser-version qualification with a documented
compiler-divergence envelope for this comparison domain ONLY (browser WebGPU
vs native — Vulkan on Linux, Metal on macOS; the expected native backend is
platform-explicit in the lane identity check, never silently substituted).

Exact pixel equality remains the preferred criterion. When exact equality fails,
the lane evaluates the approved fallback envelope (all three must hold):

1. differing_pixels ≤ 13 (0.005% of 512²);
2. max channel delta ≤ 32/255;
3. every differing pixel is edge-confined: its 7×7 window (radius 3) contains a
   tier2 edge pixel — 3×3 luminance gradient > 32/255 per
   `labs/validation/src/tier2.rs::is_edge_pixel` — in either image. Structural
   differences (flat-region or off-edge) fail regardless of count or delta.

The envelope exists because Dawn (Tint) and wgpu (naga) disagree on float
rounding in high-gradient specular regions; it never applies to same-toolchain
reproduction (§1), the cross-renderer tier2 criteria (§3), Blender parity or any
golden comparison, none of which change.

Measured basis (Chrome 153.0.8010.52, Linux, NVIDIA 615.71.09, hardware
Blackwell adapter, 2026-09-22): 6 differing pixels / 18 channels, max delta
31/255, all in hair specular highlights, byte-deterministic across runs; all 6
pixels edge-confined (verified independently by the Rust classifier and its
bit-exact Python port, full-map parity 0/524288 mismatches). The same fixture
is pixel-exact on macOS Chrome 153 (Dawn/Metal vs wgpu/Metal). Qualification is
per browser/driver version: a new browser or driver version requires
re-measurement, and any divergence larger than the envelope fails as before.
