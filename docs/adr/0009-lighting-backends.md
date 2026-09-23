# ADR-0009: Lighting Backends

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-23

## Context

The renderer must support Physical, Authored and Hybrid lighting semantics
on one NPR surface model: physical Bevy lights for generic cel, explicit
key-light plus authored palette/ramp for gacha-style content, and selective
hybrid contributions (shadow influence, environment) without forcing full
PBR cost onto authored paths.

## Options

1. One uniform physical lighting model for everything.
2. One uniform authored model for everything.
3. Separate but composable backends: an opt-in physical path, an authored
   path, and a view layer that combines physical-light inputs with authored
   shading where declared.

## Decision

Option 3, as implemented. The three semantics are realized per path (there
is deliberately no single runtime `NprLightingModel` switch type; each path
is explicit in what it consumes):

- **Physical** (`bevy-stylized-render::physical`): opt-in
  `ExtendedMaterial<StandardMaterial, PhysicalCelExtension>` — Bevy
  lights/shadows/standard material handling retained, then an authored
  color-ramp remap (`physical_forward.wgsl`). It never erases emission.
- **Authored** (Character/Hero paths): the managed key light
  (`StylizedKeyLight` via view sync, direction/color from the view
  descriptor, never shadow-casting) plus authored palette, ramp rows and
  per-instance profile inputs; hair specular and face SDF follow the same
  authored inputs.
- **Hybrid** (`view.rs` + `environment.rs`): the view layer feeds
  physical-light-shaped inputs into authored shading — day/night ambient
  color+brightness lerp that never moves ramp rows, exposure mapping,
  distance fog and stylized shadow tint (multiplicative tint on the shadow
  side only, strength-0 identity), and environment shadow tint applied to
  palette shadow colors. Hero does not pay full physical PBR cost unless a
  profile explicitly asks.

Constants named by earlier drafts are ratified as-is: night ambient color,
day/night ambient brightness bases, linear fog start/end (height falloff
deferred — needs an extra data channel), and the shadow-tint channel reusing
`DistanceFog::directional_light_color` (whose native scattering semantics
our shaders do not consume).

## Evidence

- Physical path: `tests/integration/tests/physical_material.rs`
  (`physical_extension_preserves_standard_lighting_and_host_binding`,
  `physical_ramp_remaps_light_without_erasing_emission`,
  `physical_host_resource_layout_is_measured` — 3/3 PASS 2026-09-23).
- Authored path: `tests/integration/tests/profile_lighting.rs` (10/10 PASS
  2026-09-23, incl. Genshin ramp day/night row orientation, face factor with
  control map, authored band order, metal/specular ramp semantics); the
  reference lane's six calibrated full-body/face lighting cases
  (front/left/right) passed on Linux/Vulkan (ADR-0007 recheck).
- Hybrid/view layer: unit tests in `crates/bevy-stylized-render/src/view.rs`
  (exposure component roundtrip, ambient day/night lerp, authored ambient
  override with unchanged default policy, view sync spawning key light +
  exposure + ambient) and `crates/bevy-stylized-core/src/environment.rs`
  (identity defaults keep existing renders pixel-identical).
- Character golden `tests/integration/tests/golden_character.rs` covers the
  authored palette in the required lane (Linux/Vulkan).

## Consequences

- The authored path's lighting inputs (key light, palette, ramp rows,
  profile indices) are content-owned; the view layer only modulates global
  illumination inputs (ambient, key light, exposure, fog) — it never rewrites
  authoring data semantics.
- The deferred items stay explicit: height-falloff fog, stylized
  IBL/reflection probes, and a unified view-uniform environment block.
- The docs/rendering/03 lighting-backend diagram is the owning description;
  the non-existence of a runtime `NprLightingModel` enum is intentional
  (semantics are per-path, not a runtime mode).

## Revisit trigger

Revisit when the deferred environment items are scheduled, when the
Bevy/wgpu lighting model changes, when measured evidence contradicts the
decision, or when a supported platform cannot satisfy the contract.
