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

## 4. Qualification scope

Use the declared current character case and source lock. Its report must name
actual inputs, adapter, metrics and pass/fail for each applicable criterion.
A prior source or renderer result is not a substitute for a current changed
case. Structural adoption, independent image criteria and platform qualification
remain separate. The retired synthetic reference lane is not an acceptance input.
