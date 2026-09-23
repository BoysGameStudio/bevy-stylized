# ADR-0008: Outline Architecture

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-23

## Context

The renderer needs an outline architecture: a portable baseline that works
without optional GPU features, plus a higher-quality path for internal edges
that is explicitly tiered.

## Options

1. Inverted-hull extrusion only.
2. Screen-space edge detection only.
3. Inverted hull as the portable baseline, with screen-space depth/normal
   edges as a tiered (Standard+) opt-in per camera; hybrid = hull silhouette
   plus screen-space internal edges.

## Decision

Option 3, as implemented:

- **Hull (Portable baseline):** `portable_outline.wgsl` inverted-hull
  extrusion — vertex-normal expansion, front-face cull, no geometry
  shader/tessellation/compute, physical-pixel width independent of viewport.
  Hull replicas are per-source synchronized copies
  (`sync_outline_replicas` in `outline.rs`): the replica mirrors source mesh,
  material handle, skin/morph components after engine animation evaluation
  and before transform propagation/bounds computation, and is cleaned up when
  the source or the outline component is removed; externally updated bounds
  are mirrored and removed when the source bounds go away.
- **Screen-space internal edges (Standard+):** per-camera opt-in
  (`StylizedScreenOutline` component; attachment via the scene workbench
  camera recipe), gated on the resolved tier per frame — Standard renders
  prepass + overlay, lower tiers fall back to hull-only and the downgrade
  path strips prepass/overlay again (ADR-0030 reconfiguration).
- **Hybrid:** hull silhouette plus screen-space internal edges is the
  `outline-advanced` composition; hull and screen edges stay independent
  paths with independent identities.
- Masked (alpha-cutout) interaction is owned by ADR-0023's matrix: hull
  follows pre-discard geometry (ratified), and the masked screen-edge
  appearance stays an explicitly unqualified cell — this ADR does not
  qualify it.

## Evidence

- Golden coverage: `fixtures/golden/outline-advanced/{hull_only,screen_only,hybrid}.png`
  with the required `outline_advanced_hull_screen_hybrid` suite
  (passes in the Linux/Vulkan required lane; the Metal host deltas are
  root-caused platform rounding, SUPPORT_MATRIX 2026-09-23 note).
- Hull lifecycle GPU evidence in `morph_skinning`:
  `live_morph_outline_tracks_surface_and_returns_to_rest`,
  `native_outline_instance_width_changes_and_restores_shared_material`,
  `external_head_pose_drives_face_shading_without_material_clones` —
  live input tracking, instance width with shared-material restoration,
  removal/rest semantics (5/5 PASS on Metal, 2026-09-23).
- Masked policy GPU evidence `tests/integration/tests/masked_outline.rs`
  (hull pre-discard solid fill, screen-edge post-discard content; 2/2 PASS).
- Outline-mode gating and replica lifecycle unit tests in
  `crates/bevy-stylized-render/src/screen_outline.rs` and `outline.rs`
  (mirrors render layers, removes replicas on removal, downgrade cleanup).
- Cel outline golden `fixtures/golden/cel-portable/sphere_outline.png`
  (`tests/integration/tests/golden_cel.rs`).

## Consequences

- The portable outline never requires prepass, storage buffers or any
  optional GPU feature; the tiered path is an explicit per-camera cost with
  a documented fallback (hull-only), never a silent upgrade.
- The replica sync's ownership rules (mirrors render layers, removes its own
  replicas only, does not mutate shared material assets) are the ratified
  lifecycle contract; fuller bounds/LOD policy for new content remains with
  the owning matrix rows, not an outline redesign.
- The masked screen-edge appearance gap (ADR-0023) is the named follow-up;
  fixing it revisits this ADR's screen-edge path, not the hull baseline.

## Revisit trigger

Revisit when the masked screen-edge appearance is root-caused, when the
Bevy/wgpu baseline changes the prepass/MSAA model, when measured evidence
contradicts the decision, or when a supported platform cannot satisfy the
contract.
