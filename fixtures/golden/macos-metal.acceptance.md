# macOS/Metal golden overlay — acceptance record

**Acceptance act:** 2026-09-23, project owner (delegated review, owner decision
2026-09-23 to qualify macOS). Capture and acceptance are separate acts per
[../README.md](../README.md#golden-platform-overlays): the images were captured
through the normal test capture path (`GOLDEN_REBASELINE=1` on the device,
overlay-candidate branch), then reviewed, then committed.

**Platform identity:** macOS 27.0 (build 26A428), Apple M4 Pro, Metal backend;
Rust 1.98.0 (`rust-toolchain.toml`). Captured at engine commit
`95446bb3` (parent `8e4994f` carries the overlay mechanism). The platform id is
`macos-metal`, derived from the actual render adapter, not a flag.

**Root-cause basis (review reference):** the 2026-09-23 diff-map investigation
recorded in
[docs/platforms/SUPPORT_MATRIX.md](../../docs/platforms/SUPPORT_MATRIX.md)
(commit `8bfa1a6`) — all deltas decompose into bulk ±1 cross-vendor rounding
plus isolated outliers at numerically hypersensitive boundaries; screen-edge
detection itself was verified correct on Metal by a direct prepass probe. No
shader change can remove these without altering the accepted Linux/Vulkan
baselines.

**Review record (canonical Linux/Vulkan baseline vs this overlay):**

| Case | Canonical → overlay max Δ | mean Δ | px > 4/255 | Notes |
| --- | --- | --- | --- | --- |
| `cel-portable/sphere_outline` | 6 | 0.0038 | 1 | one rim pixel at 6; bulk ±1 |
| `character-portable/face_sdf_instances` | 33 | 0.0348 | 2 | two px at the SDF threshold boundary |
| `outline-advanced/hull_only` | 49 | 0.0020 | 1 | one px at 49 on the hull edge |
| `outline-advanced/screen_only` | 186 | 1.7493 | 5485 | edge lines same positions; sub-pixel line coverage |
| `outline-advanced/hybrid` | 186 | 1.6809 | 5252 | same class as screen_only |

Diff maps were inspected per case: rows 1–3 show no visible structural change
(outliers are isolated pixels at the documented boundaries); rows 4–5 show the
differences confined to the screen-outline edge lines (same positions, sub-pixel
coverage; verified by the prepass probe in the root-cause note). The scope note
in the root-cause record ("three Metal golden deltas") names the three affected
suites; the outline-advanced suite contributes three cases because its earlier
lane runs stopped at the first assertion — screen_only/hybrid exceed on Metal as
well and are recorded here with the same root cause.

**Accepted images (SHA-256 of PNG bytes; FNV-1a-64 in each `.golden.json`):**

- `cel-portable/macos-metal/sphere_outline.png` — `af2aa2aa…cdef0f`
- `character-portable/macos-metal/face_sdf_instances.png` — `50e9d1ac…7112e3a`
- `outline-advanced/macos-metal/hull_only.png` — `b72cb0bf…548f3b9`
- `outline-advanced/macos-metal/screen_only.png` — `b84aa5fd…e46941`
- `outline-advanced/macos-metal/hybrid.png` — `121239f4…2952f39d`

**Scope of this acceptance:** these five images are the accepted baselines for
`macos-metal` only. They make no claim about other macOS devices, other drivers,
or other OS versions; the canonical host remains Linux/Vulkan
(docs/reference/08 §1). Non-goals unchanged: no multi-device claim, no
regression-baseline reuse across platforms.
