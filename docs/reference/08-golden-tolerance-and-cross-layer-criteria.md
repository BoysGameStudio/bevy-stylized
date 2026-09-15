# Golden tolerance and independent comparison criteria

> Status: Existing numeric criteria retained without relaxation. Cross-renderer
> qualification remains Draft / ADR-0007; a failed metric is still a failure.

## 1. Same-toolchain reproduction

For the same device, toolchain and declared scene: maximum normalized channel delta
≤ 2/255, mean delta ≤ 0.5/255, PSNR ≥ 48 dB. Exact pixel equality is preferred.
Different GPU/driver qualification requires its own measured policy; no additional
platform is declared supported from the current Linux/Vulkan evidence.

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

## 4. Current evidence

The independent direct-source recipe reproduced the prior upstream reference
pixels exactly (maximum delta 0). Production/upstream comparison remains around
20.96 dB PSNR, mean 0.0379 and shadow IoU 0.8957: **FAIL**, unchanged by the
retirement of the legacy renderer. This is a synthetic reference scene with real
source maps, not a full-character parity claim.

The independently imported static character passes structural/material adoption,
sharing and reload checks. That result does not supersede the failing upstream
metrics or qualify full-rig/effect rendering. Original images, detailed historical
experiments and previous three-renderer metrics are preserved in
[the archived criteria record](../history/pre-independence-08-golden-tolerance-and-cross-layer-criteria.md).

## 5. Baseline changes

First-party accepted images live in fixtures/golden. External source/derived art
stays under assets/local. Capture writes a candidate; comparison is read-only;
explicit acceptance requires provenance and preserves the previous version.
Neither a green Rust report-generation test nor a newly created file accepts a
Golden baseline. See the [upgrade policy](04-upstream-contract-upgrade-and-golden.md).
