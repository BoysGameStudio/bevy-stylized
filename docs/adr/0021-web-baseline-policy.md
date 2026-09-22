# ADR-0021: Web Baseline Policy

- **Status:** Accepted
- **Decision owner:** Project maintainer
- **Date:** 2026-09-22

## Context

WebGPU and WebGL2 have different backend and resource constraints. A browser
compile or a Vulkan limit simulation does not establish either runtime target.
Browser shader compilers (Dawn/Tint) and wgpu (naga) diverge deterministically
on float rounding, so a browser-vs-native exact pixel gate can fail on a
correct implementation; qualification needs an explicit policy for that
divergence.

## Options

1. Exact pixel equality only; any divergence fails the version forever.
2. Accept any small divergence by raising a global tolerance.
3. Per-browser-version qualification: exact equality preferred, with a
   bounded, structurally-confined compiler-divergence envelope approved
   through the tolerance-policy owner
   ([docs/reference/08 §5](../reference/08-golden-tolerance-and-cross-layer-criteria.md))
   for the browser-vs-native domain only.

## Decision

Option 3, owner-approved 2026-09-22.

- WebGPU only. The browser consumer builds with Bevy's `webgpu` feature and
  must obtain a real BrowserWebGpu hardware adapter; SwiftShader/software or
  WebGL2 fallback is BLOCKED/FAIL, never a qualification.
- Qualification is per browser/driver version: the record names browser,
  driver, adapter and required flags; a new version requires re-measurement.
- The comparison gate is exact pixel equality; on mismatch the lane evaluates
  the reference/08 §5 envelope (≤13 differing pixels, max channel delta
  ≤ 32/255, every differing pixel edge-confined by the tier2 classifier).
  The report states both outcomes unambiguously (`exact_match`,
  `envelope.pass`); exceeding the envelope fails as before.
- The envelope applies to the "browser WebGPU (Dawn) vs native Vulkan" domain
  only. Same-toolchain, Blender-parity, tier2 cross-renderer and golden
  criteria are untouched.
- This qualifies neither default-browser operation nor every material/pass
  combination; scoped coverage follows the lane report.

## Evidence

- Lane implementation: `tools/checks/browser.py` evaluates the envelope after
  an exact mismatch and reports both outcomes; envelope unit tests in
  `tests/python/test_browser_envelope.py` (exact pass; edge-confined small
  divergence pass; structural non-edge, >13 pixels, >32/255 delta and
  outside-window cases all fail) — 6/6 PASS 2026-09-22.
- The Python edge classifier is bit-exact with
  `labs/validation/src/tier2.rs::is_edge_pixel` (f32-exact port; full-map
  parity 0/524288 mismatches on the recorded pair, verified 2026-09-22).
- Measured basis: Chrome 153.0.8010.52 on Linux (NVIDIA 615.71.09, hardware
  Blackwell) — 6 differing pixels / 18 channels, max 31/255, hair specular
  highlights, byte-deterministic across runs, all edge-confined; the lane
  reports PASS with `exact_match: false`, `envelope.pass: true`. macOS Chrome
  153 (Dawn/Metal vs wgpu/Metal) — pixel-exact; the lane reports PASS with
  `exact_match: true`. The lane's native-side identity is platform-explicit
  (Vulkan on Linux, Metal on macOS — MoltenVK native Vulkan is unavailable
  upstream); software adapters stay BLOCKED/FAIL.
- Support matrix records: [SUPPORT_MATRIX](../platforms/SUPPORT_MATRIX.md)
  webgpu sections (2026-09-21 and 2026-09-22 addendum/records).

## Consequences

- The web baseline policy is closed for the recorded scope; browser runs are
  admitted as evidence under the stated per-version discipline.
- A browser/driver update invalidates the version record until re-measured;
  an envelope-exceeding divergence is a FAIL and triggers investigation, not
  a wider envelope.
- WebGL2 remains outside the support promise; admitting it requires its own
  resource/fallback implementation and real-browser qualification.

## Revisit trigger

Revisit when a browser/driver version changes the measured divergence, when
the envelope is exceeded, when the Bevy/wgpu or Dawn baselines change, or when
WebGL2 support is proposed.

## Current scope decision, 2026-09-21

The project owner selected WebGPU verification for this hardening work and made
no WebGL2 support commitment. Build the browser consumer with Bevy's `webgpu`
feature and require an actual BrowserWebGpu adapter; do not silently fall back to
WebGL2 or a software adapter and label that hardware WebGPU qualification.

Portable retains its existing fixed-binding and non-storage material contract.
Selecting WebGPU does not waive those semantics or automatically enable higher
tiers. WebGL2 remains outside the current support promise, not a tested backend.

## Executed scoped evidence, 2026-09-21

`hardening-webgpu` completed the maintained `webgpu` lane: real BrowserWebGpu
shader/pipeline creation and a 512² screenshot were pixel-identical to the native
Vulkan fixture. The [support matrix](../platforms/SUPPORT_MATRIX.md#webgpu-local-record-2026-09-21)
records browser/driver/device, required flags, source/lock identity and explicit
untested features. This satisfies the requested initial WebGPU verification.
The remaining decision is release-wide browser/version/feature qualification and
formal ADR acceptance; this result does not promise default-browser operation,
WebGL2 support or every material/pass combination.

## Evidence update, 2026-09-22

Chrome 153.0.8010.52 on Linux (NVIDIA 615.71.09, hardware Blackwell adapter,
X11/XWayland invocation — see CONTRIBUTING for the working flag set): the
`webgpu` lane now runs end-to-end (real BrowserWebGpu adapter, shader/pipeline
creation, fixture adoption and capture), but the exact-match gate FAILS on 6
deterministic pixels / 18 channels (max channel delta 31/255, hair specular
highlights of the genshin-odette fixture) from a Dawn (Tint) vs wgpu (naga)
shader divergence. The same fixture is pixel-exact on macOS Chrome 153
(Dawn/Metal vs wgpu/Metal).

Browser-version drift broke the 2026-09-21 exact record above. Acceptance
therefore additionally requires a version-qualification or tolerance policy via
[docs/reference/08](../reference/08-golden-tolerance-and-cross-layer-criteria.md);
that decision stays open and this ADR remains a V1 Blocker. The failure is
retained per policy — no tolerance was widened to pass it.

*(2026-09-22 later: resolved by the Decision above — per-version qualification
with the reference/08 §5 envelope, owner-approved today.)*
