# ADR-0023: Render Semantics Matrix

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-22

## Context

The renderer must define opaque/mask/blend/shadow/prepass/outline behavior
explicitly instead of inheriting "everything works" from Bevy's material
enum. The [owning matrix](../rendering/03-shader-lighting-outline-post.md#alpha--transparency--shadow--prepass-matrixv1-必需)
distinguishes native opaque/mask, explicit host/pupil Blend, geometric-normal
prepass, hull extrusion and Standard+ screen edges. The 2026-09-21 record
(below, kept as evidence) closed the matrix structure and the masked prepass
repair, leaving three acceptance items: runtime shadow-casting evidence for
the optional feature, a masked hull/edge appearance policy, and the final
supported-combination approval.

## Options

1. Qualify every combination Bevy's `AlphaMode` enum can express.
2. Qualify only combinations with direct GPU evidence; keep the rest as
   explicit "not qualified" cells; ratify current implementation behavior
   only where it is verified coherent.
3. Defer acceptance until masked screen-edge appearance and masked shadow
   casting are fully qualified.

## Decision

Option 2. The matrix is approved as the V1 semantics contract with the
following rulings:

1. **Shadow casting (Opaque row):** stylized materials never cast shadows by
   default and never *receive* them in authored shaders. The opt-in
   `shadow-casting` Cargo feature enables casting for `StylizedHeroMaterial`
   only (Cel/Character/outline/overlay materials stay hard-disabled). Runtime
   evidence is required and now exists; masked casting stays unqualified.
2. **Masked × hull outline:** ratified as implemented — the inverted-hull
   shell is pure geometric extrusion and never samples base alpha, so the
   hull follows the *pre-discard* silhouette: cutout regions are solidly
   filled with the outline color by construction (a contiguous fill, never a
   floating rim). Cutout-faithful hull outlining is not supported.
3. **Masked × screen-space edges (Standard+):** the cutout *content* follows
   the post-discard prepass (holes reveal the background; prepass is
   byte-exact against the forward pass per the alpha probe). However the
   overlay currently draws an emphasized boundary band around interior
   cutouts (~15px at default width 1.5, weakly width-scaling, mechanism not
   pinned down). The dedicated masked edge appearance therefore stays **not
   qualified**; the cell records the observed behavior and the reason.
4. **Blend / AlphaToCoverage / Premultiplied / Add / Multiply:** unchanged —
   explicit host/pupil Blend paths stay scoped; all other modes have no
   selector and are not qualified. Deferred/meshlet and general transparent
   NPR remain outside qualification rather than inferred from Bevy's enum.
5. `tests/integration/tests/semantics_matrix.rs` is the contract suite; its
   shadow assertion is feature-aware so that default and `shadow-casting`
   builds each pin their own truth.

## Evidence

- Matrix contract suite `tests/integration/tests/semantics_matrix.rs`
  (alpha/cull matrix, Hero flags/keys, no per-frame material mutation, Blend
  unreachable, feature-aware prepass/shadow contract) — PASS 2026-09-22.
- Runtime shadow evidence `tests/integration/tests/shadow_casting.rs`
  (GPU, run with and without `--features shadow-casting`): with the feature,
  the Hero caster darkens a StandardMaterial receiver measurably
  (5510 differing pixels in the A/B light-toggle diff, all in the caster's
  shadow region; the Cel in-scene control and the caster's own pixels are
  untouched — casting never shades the caster). Without the feature the A/B
  pair is byte-identical (0 differing pixels) — off-feature behavior is
  exactly the previous behavior. Metal (Apple M4 Pro) and Vulkan (RTX 5090)
  runs both PASS.
- Masked outline evidence `tests/integration/tests/masked_outline.rs`
  (GPU): `masked_hull_follows_geometric_silhouette` — zero background pixels
  inside the hull silhouette, solid fill ≈49% of the interior (a thin rim
  would be <5%); `masked_screen_edges_follow_post_discard_silhouette` —
  cutout band reveals the background (>30% of the interior), deep hole
  interior carries zero edge pixels (no floating outline), boundary emphasis
  exists and is recorded as the unqualified-appearance gap above.
- Regression safety: `capture_all` 17/17 byte-identical, regression golden
  cells byte-identical, 19/19 integration binaries unchanged, required GPU
  lane 42/42 on Linux/Vulkan (2026-09-22).

## Consequences

- The matrix's supported combinations are approved as-is; every unsupported
  or unqualified combination stays an explicit cell. New selectors or a
  masked-edge appearance fix require new GPU evidence and a matrix update.
- The `shadow-casting` feature is scoped to Hero casting on shadow-mapped
  receivers; enabling it changes `prepass_and_shadow_contract_is_explicit`'s
  expected value for Hero only, by construction.
- Hosts needing cutout-faithful outlines on masked surfaces have no qualified
  path until the masked screen-edge appearance is revisited (revisit trigger).

## Revisit trigger

Revisit when the masked screen-edge emphasis band is root-caused, when masked
shadow casting or a cutout-faithful hull is required by content, when the
Bevy/wgpu baseline changes, or when measured evidence contradicts the matrix.

## Current bounded contract and evidence, 2026-09-21

The [owning matrix](../rendering/03-shader-lighting-outline-post.md#alpha--transparency--shadow--prepass-matrixv1-必需)
now distinguishes native opaque/mask, explicit host/pupil Blend, geometric-normal
prepass, hull extrusion and Standard+ screen edges. Unsupported selectors and
unqualified shadow/transparent-outline combinations are explicit. Native mask
prepass was reproduced hiding a rear surface and repaired by sharing the main
material's alpha inputs; its required GPU regression compares depth-only and
depth+normal paths across Cel/Character/Hero. Pipeline readiness is required before
capture. Existing deformation, hull/screen and budget tests remain independent
oracles; their baselines and tolerances are unchanged.

Remaining acceptance: runtime shadow-casting evidence for the optional feature,
masked hull/edge appearance policy and final supported-combination approval.
Compile-only shadow consumers and a type-level `enable_prepass` flag cannot close
those cells. Deferred/meshlet and general transparent NPR remain outside current
qualification rather than being inferred from Bevy's material enum.

*(2026-09-22: all three items resolved above — shadow evidence landed, the
masked hull policy is ratified with the masked edge appearance explicitly
unqualified, and the combination approval is the Decision section.)*
