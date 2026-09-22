# ADR-0021: Web Baseline Policy

- **Status:** V1 Blocker

## Context

WebGPU and WebGL2 have different backend and resource constraints. A browser
compile or a Vulkan limit simulation does not establish either runtime target.

## Current scope decision, 2026-09-21

The project owner selected WebGPU verification for this hardening work and made
no WebGL2 support commitment. Build the browser consumer with Bevy's `webgpu`
feature and require an actual BrowserWebGpu adapter; do not silently fall back to
WebGL2 or a software adapter and label that hardware WebGPU qualification.

Portable retains its existing fixed-binding and non-storage material contract.
Selecting WebGPU does not waive those semantics or automatically enable higher
tiers. WebGL2 remains outside the current support promise, not a tested backend.

## Evidence and remaining gate

Bevy 0.19.1's feature declaration states that `webgpu` selects WebGPU rather than
the WebGL2 backend. The local browser consumer must additionally demonstrate
shader/pipeline readiness, real material/asset rendering and capture evidence.
Required browser flags, hardware identity and browser/driver versions are part
of that record. The ADR remains a blocker until this evidence and the matching
support-matrix update are complete; scope selection alone is not runtime success.

## Consequences

Linux native validation can continue independently of browser work. Browser
failures remain explicit; no native or headless software result substitutes for
the requested hardware browser test. A future WebGL2 commitment requires its own
resource/fallback implementation and real-browser qualification.


## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the decision, or a supported platform cannot satisfy the contract.

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
