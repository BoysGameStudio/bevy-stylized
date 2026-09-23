# ADR-0010: Platform Support Policy

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-23

## Context

Platform support claims require evidence appropriate to the claim. Compile
success, shader validation, pipeline creation, real runtime rendering,
device qualification and performance measurement are different evidence
states, and none may be inferred from another.

## Options

1. Declare support by platform name once anything compiles there.
2. Per-device, per-version qualification with an evidence ledger: claims
   are recorded only at the measured state, never inherited across devices,
   drivers, versions or cells.

## Decision

Option 2, as practiced in
[docs/platforms/SUPPORT_MATRIX.md](../platforms/SUPPORT_MATRIX.md):

- **Evidence states:** `Validated` (evidence exists for the cell),
  `Experimental` (partial implementation/evidence), `Not tested` (no
  evidence), `Blocked` (known blocker), `Unsupported` (intentionally out),
  N/A. `Supported` as a public status is never inferred from compile
  success; a target with only compile/shader/pipeline evidence is
  Experimental; consoles are Port-ready until real SDK/backend/device
  validation exists.
- **No inherited claims:** qualification is per device + driver + version.
  A record names the actual adapter, driver, versions, flags and source/
  lock identity; a different device, driver or version starts unqualified
  until measured. Cross-device golden comparison requires its own measured
  policy (reference/08 §1) and is not implied by single-device acceptance.
- **Current honest ledger (2026-09-23):** Linux/Vulkan — full-lane
  evidence on the recorded RTX 5090 (cpu lane, required GPU lane 42/42,
  character-reference 6/6; single-device Experimental by the matrix's own
  vocabulary). Android — Experimental (API-35 emulator host-Vulkan smoke
  and driver replay evidence; not physical-device qualification). Web —
  Experimental (per-version qualification with the reference/08 §5
  envelope; Chrome 153 records on Linux and macOS). macOS — Not qualified
  (the three golden deltas are root-caused platform rounding, recorded in
  the matrix). Windows/D3D12, Linux/GLES, iOS, Android/GLES, consoles —
  Not qualified / no current evidence. WebGL2 — outside the support
  promise (ADR-0021).
- **Failure honesty:** a failed or divergent measurement is recorded as
  such (e.g. the Chrome 153 Dawn/naga envelope, the Metal golden deltas);
  it is never edited into a pass or silently rebaselined.

## Evidence

- The support matrix itself, kept current with dated records
  (2026-09-21/22/23 entries for Android emulator, WebGPU, Metal fixes and
  the golden-delta root-cause notes).
- The lane infrastructure that produces cell evidence: cpu/gpu
  `--required` lanes (42/42 on the qualified Linux host), character-reference
  lane, webgpu lane, Android smoke/replay paths.
- Tier requirement tests in `crates/bevy-stylized-core/src/capability.rs`
  and the capability lifecycle (ADR-0022) that keep per-device admission
  explicit.

## Consequences

- New support claims require the corresponding lane evidence recorded in
  the matrix before they are stated anywhere; the matrix is updated in the
  same change as the evidence.
- "Supported" as a multi-device release claim requires its own explicit
  policy upgrade — nothing in the current ledger grants it.
- Platform-specific failures stay in the ledger with their root cause;
  support scope never silently expands to hide them.

## Revisit trigger

Revisit when a new target/device/driver/version is measured, when a
multi-device release-support policy is proposed, or when the Bevy/wgpu
baseline changes.
