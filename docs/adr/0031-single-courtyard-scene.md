# ADR-0031: Single Courtyard Scene with ROI-Layered Golden Judgment

- **Status:** Proposed
- **Decision owner:** Architecture
- **Date:** 2026-09-28

## Context

The showcase catalog grew into 18 flat, ungrouped scenes (`labs/scenario-model/src/catalog.rs`, `labs/scenes/src/registry.rs`): three environment variants sharing one builder, two outline scenes split only by tier, stress/performance as separate near-identical instance loads, and linear prev/next navigation with no grouping. The owner asked for a single scene.

Two constraints shape the decision. First, industry practice for golden-image regression is overwhelmingly "many small isolated scenes, one baseline each" (Unity Graphics Tests, Babylon.js validation tests, three.js/Bevy example screenshots, Skia Gold, Godot proposal #1760); composed gallery scenes exist only as showcase/teaching assets kept separate from the test suites (Unreal Content Examples, Unity Material Samples). No major renderer anchors core regression on a few composed scenes. Second, local verification (2026-09-28) found the naive single-scene data had to be corrected: the 1:1 golden frame crops the pedestal ring at the originally proposed camera (moved to eye z=16), Cel material has no emission so the night lamp field must use Character material (`crates/bevy-stylized-render/src/material.rs:44-55` vs `character.rs:204`), emission is attenuated by fog (lamp depth ≤ 38 m against the night fog end of 40), and the LOD axis must be re-solved against the 10/25/50 band thresholds from the new camera position.

## Options

1. Keep the 18 isolated scenes; add only grouping metadata. Zero risk, zero consolidation.
2. Converge to 13 scenes / 12 families per the `docs/showcase/01-showcase-examples-fixtures.md` §380-409 table. Partial consolidation; the flat-list problem shrinks but persists.
3. One composed courtyard scene whose panoramas carry whole-frame golden baselines (18 → 8 baselines). Rejected by the evidence above: any rendering change anywhere fails the same panorama baseline, fault isolation collapses, and the industry mitigation is finer granularity, not fewer baselines.
4. One composed courtyard scene for content and navigation, with **ROI-layered judgment**: whole-frame loose assertions (non-black, global statistics) plus per-region strict assertions (per-pixel threshold + fail-ratio), tier/parameter states as multiple approved baselines per capture (Unreal local/global error and Add As Alternative; Chromium Gold multiple approved images).

## Decision

Adopt option 4. The full scene design (layout, verified coordinates, lighting palettes, camera presets, parameter surface, capture matrix, ROI specification, and the multi-platform functional/interaction layer with its command-set extension and phasing) is [`docs/showcase/05-courtyard-scene-design.md`](../showcase/05-courtyard-scene-design.md); its §0 records the owner decisions (option A regression structure, mixed sphere/GLB display, day default state, GLB packaged into all lanes).

Concretely:

1. One scene `courtyard` replaces the 18 showcase scenes; all current content maps into zones/parameters (mapping table in the design doc §10). No coverage dimension is deleted.
2. Golden judgment is layered: strict per-region ROI assertions carry per-feature fault isolation; whole-frame assertions only catch wholesale breakage. One render feeds many judgments.
3. Tier-gated (hybrid outline) and asset-gated (reference GLB) content stays explicit: hybrid judges only in its Standard-tier capture; missing required assets fail the build as BLOCKED, never a silent empty pedestal.
4. The 18 old baselines are retired and replaced via the explicit GOLDEN_REBASELINE flow (GPU required; absence is BLOCKED, not PASS). Dated acceptance records remain unrewritten history.
5. Scene layout is a contract: layout changes invalidate all ROIs and require explicit recalibration.

## Consequences

Positive: one scene answers the organization complaint; navigation groups by zone instead of an 18-item flat list; regression renders drop from 18 scene builds to 8 captures with equal or better fault isolation; the parameter surface (time-of-day, crowd density, outline mode) finally uses the catalog's preset/parameter machinery for real.

Negative: ROIs must be calibrated and recorded per capture (new golden.json `regions` field, additive); the composed scene couples content changes to shared pixels, absorbed by region-level judgment instead of scene-level; layout becomes a contract that is expensive to change casually; old recipe identities tied to the retired scene ids are invalidated (accepted — old recipes are rejected explicitly by the version/catalog gate, not silently reinterpreted).

Adversarial review 2026-09-28 (same-day, pre-implementation) found and the design doc absorbed: three silent coverage cuts (post had no capture; posed flag pose had no capture; the trio dropped the off/on and threshold states present in the old face/hair/metal goldens — fixed by three comparison groups of 7 spheres and captures `courtyard-post` / `courtyard-flags`); the catalog `ParameterKind` lacks an enum kind required by the parameter surface (prerequisite); ROI calibration is automated via projected zone AABBs with pairwise-disjoint assertions instead of manual pixel picking; fault-isolation claims are narrowed honestly to material-local changes (cross-region effects — shadows, fog, global light direction — pierce ROIs by nature); the "one render, many judgments" claim is reframed as saving orchestration, not GPU time; the wire-protocol migration list (suites.json, browser/android lane derivation, catalog-scene.txt) is enumerated. One finding from the adversarial review (S4) was decided by the owner on 2026-09-28: the reference GLB **is** packaged into every lane (browser via HTTP asset source, Android via APK assets) and becomes a required dependency of the courtyard; absence fails the whole build as BLOCKED with no empty-pedestal degradation. Accepted cost: lanes without the asset pipeline ready have no courtyard coverage during the window — an explicit BLOCKED state, not a silent gap.

## Evidence

- Industry survey 2026-09-28 (isolation vs composition; local/global and multi-baseline mitigations; tolerance practice): Unity Test Framework Graphics docs, Unreal Screenshot Comparison docs, Babylon.js validation tests, three.js test/e2e README, Skia Gold docs, godot-proposals#1760, PixelEagle/FLIP write-up, NVlabs/flip.
- NPR showcase survey 2026-09-28 (content selection precedents; shaderball methodology; time-of-day via lighting parameters): UTS2 sample scenes, UTS3 (com.unity.toonshader), Unreal Content Examples, Flat Kit demo scenes, USD StandardShaderBall.
- Local verification 2026-09-28: LOD thresholds `crates/bevy-stylized-core/src/lod.rs:32-40`; emission support `crates/bevy-stylized-render/src/character.rs:204` and its absence in Cel `material.rs:44-55`; camera frustum fit at 512×512 fov 45°; fog vs clear-color split `crates/bevy-stylized-render/src/view.rs:258-260`; `with_effect_light_dir` at `character.rs:142` with no tier gate.
- Implementation evidence 2026-09-28/29 (stages 0–7 landed):
  - Scene: courtyard builder + layout/LOD band assertions + camera presets (`labs/scenes/src/{builders,metadata,registry,views}.rs`); `CourtyardSettings` parameter surface; catalog `CATALOG_LEN = 1` (`labs/scenario-model/src/catalog.rs`).
  - Regression structure: capture matrix as data (`labs/scenario-model/src/capture_spec.rs`); golden.json `regions` additive extension + AABB projection + pairwise non-overlap assertion (`tests/harness/src/regions.rs`); region double-gate diff + whole-frame luma/chroma envelope (`labs/validation/src/{diff,frame}.rs`); panorama ROI set converged to [trio, p1, p4] on projection evidence.
  - Baselines: 10-capture macos-metal overlay accepted with dated records (`fixtures/golden/showcase.courtyard.acceptance.md`; two corrective entries for outline-west camera and hybrid judgment mode); canonical baselines absent = BLOCKED, recorded (reference host not provisioned).
  - Retirement: 18 scenes removed (catalog/bindings/suites.json 18→10, `tests/integration/tests/showcase_golden.rs` rewritten as 10 capture cases, `outline_advanced.rs` removed, 18-scene golden files deleted); cpu/gpu/docs lanes green.
  - Browser lane: 9/10 captures byte-exact vs same-machine native (Dawn vs wgpu/Metal); `courtyard-outline-hybrid` diverges 2488 edge-confined pixels on screen-space outline (Dawn/naga compiler-sensitive edges, same class as the 2026-09-23 Metal screen-outline delta; recorded in `docs/platforms/SUPPORT_MATRIX.md` 2026-09-28 entry, tolerance widening left to owner decision).
  - Android lane: 10/10 PASS on Tenacity_API35 emulator (MoltenVK→host GPU) across two rounds (android-vulkan overlay seeding + non-rebaseline judgment closure; emulator evidence, not a physical-device claim); APK asset packaging (`STYLIZED_ANDROID_CATALOG_DIR`) and capture-spec control file verified end to end.
  - Character stage NPR adoption (2026-09-28 owner decision): `StylizedMaterialSet` on `WorldAssetRoot` + `StylizedAssetIntegrationPlugin` in capture hosts — resolves both the design intent (true NPR character demo) and the vanilla-PBR-black-on-Dawn/Metal host discrepancy (root cause not investigated upstream; recorded as host-limited in SUPPORT_MATRIX).
  - Interaction: SessionCommand extension + post-build camera replacement (`labs/scene-runtime`), PC V1 keymap + window-camera unification + egui overlay slice (status bar/legend/keymap help, tier grey-out, `EguiWantsInput` gating), Android resident mode (`report_and_stay`, `ANDROID_RESIDENT` logcat evidence).
  - Status remains **Proposed**: acceptance is an owner decision; this evidence is the input to it.
