# Renderer ownership review

R5 reviewed the current renderer against Material/Instance/View ownership and
capability/scheduling contracts. The disposition is to retain the implementations;
no module split or backend replacement has demonstrated a net benefit for this
batch. This is a recorded design decision, not a claim that every path is qualified.

## Material responsibilities

| Responsibility | Owner | Why retain the current boundary |
|---|---|---|
| Cel params, mapping, specialization, event sync | material.rs | Compact baseline implementation; class-local uniform and texture semantics |
| Character extension and table material | character.rs | Descriptor conversion already reuses Cel; table shader specialization has distinct storage binding |
| Hero family controls | hero.rs | Descriptor conversion already reuses Character; explicit face/body/highlight controls should not become a generic mega-material |
| Variant/fallback | core capability + render capability/class mapping | Resolution consumes actual device features; class semantics remain separate from execution tier |
| GPU instance transport | instance.rs / instance_table.rs | MeshTag and storage table retain explicit capability/precision/lifecycle constraints |
| View and frame state | view.rs / post.rs | Global illumination/exposure/post ownership is already separate from material assets |
| External head pose and outline | face_input.rs / outline.rs | Producer state is consumed through explicit rendering adapters, without animation control |

Repeated asset-event dispatch and a few flag checks do not by themselves justify
new traits or layers. The table variant's shader definitions and handles are not
interchangeable with the baseline material. There was no newly demonstrated
behavioral defect requiring a renderer change. Two contradictory prepass comments
were corrected: the implementations already enable participation while cameras
opt in; shadows remain disabled. No executable renderer statements changed in R5.

The large capability/governor files include requirement tables, pure policy state
and test matrices. File length does not establish a missing runtime boundary.
Public exports, flags, layouts, resource bindings and embedded WGSL stay intact.
Existing limitations such as precision/transport restrictions and open ADRs remain
visible in the [current inventory](../release/03-provisional-inventory.md).

## Scheduling and lifecycle evidence

- Material flags synchronize after asset events; consistent flags avoid event echoes.
- Outline replicas synchronize after animation and before transform propagation and
  bounds calculation; producer bounds and geometry remain externally owned.
- Face input runs after Hero flag sync and transform propagation, before instance
  tags; removed bindings restore their prior override before upload.
- Instance/LOD/view ownership remains separate from shared material assets.

Existing tests cover shared adoption/release, removed head bindings, morph/skin and
outline updates, instance batching/table precision, class fallback and whole-pipeline
budgets. R6 retains these identities while grouping deformation, profile lighting
and sampling/camera cases. See the [current contract checks](../../tests/current-contracts.json).

A future split should cite a concrete editing or validation problem and demonstrate
that a small trial removes it. No performance improvement or new platform support
is asserted by preserving these boundaries.
