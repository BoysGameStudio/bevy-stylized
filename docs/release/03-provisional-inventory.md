# Provisional implementation inventory

This is the current implementation map, updated during R2 on 2026-09-15. ADR
statuses remain owned by the [ADR index](../adr/ADR_INDEX.md); implementation or
test success does not accept an ADR. The complete [current validation workflow](../../CONTRIBUTING.md)
is preserved with its original measurements and limitations.

## Current implementation and remaining decisions

| Domain | Current code and evidence | Remaining decision / limitation |
|---|---|---|
| Material backend | [render plugin](../../crates/bevy-stylized-render/src/lib.rs), Cel/Character/Hero Material implementations | ADR-0001; no backend replacement or speed claim follows from this inventory |
| Whole-pipeline budgets | [budget tests](../../tests/integration/tests/interim_budget.rs) | ADR-0002/0003; interim limits and bindless policy remain provisional |
| Instance transport | [MeshTag](../../crates/bevy-stylized-render/src/instance.rs), [instance table](../../crates/bevy-stylized-render/src/instance_table.rs), [tests](../../tests/integration/tests/instance_table.rs) | ADR-0004; StorageTable exists, Character has the table material variant; it does not imply Hero full-precision face transport |
| Fallback and LOD | [capability](../../crates/bevy-stylized-core/src/capability.rs), [render LOD](../../crates/bevy-stylized-render/src/lod.rs) | Preserve class semantics and explicit tier/feature reductions; three-axis ADR acceptance does not settle every fallback algorithm |
| View, lighting, environment | [view](../../crates/bevy-stylized-render/src/view.rs), [post](../../crates/bevy-stylized-render/src/post.rs) | ADR-0009; authored mappings and environment limitations remain explicit |
| Outline | [replica synchronization](../../crates/bevy-stylized-render/src/outline.rs), [screen outline](../../crates/bevy-stylized-render/src/screen_outline.rs) | ADR-0008; updated/removed producer bounds and deformation compatibility are tested, not a general deformation solver |
| External face input | [adapter](../../crates/bevy-stylized-render/src/face_input.rs), [rendering-input contract](../showcase/04-realtime-validation.md) | External propagated head pose drives existing quantized instance input; no animation controller or general table support |
| Alpha/prepass/shadows | [semantic matrix](../../tests/integration/tests/semantics_matrix.rs) | ADR-0023; supported combinations stay explicitly bounded |
| Profile and texture interpretation | [Genshin mapping](../../crates/bevy-stylized-profile-genshin/src/mapping.rs), [Hero](../../crates/bevy-stylized-render/src/hero.rs) | ADR-0005/0006; current static qualification does not establish every family effect |
| glTF metadata and setup | [metadata](../../crates/bevy-stylized-gltf/src/metadata.rs), [setup](../../crates/bevy-stylized-gltf/src/setup.rs) | ADR-0020/0028; conflict rejection and pre-registration are implemented, final API decisions remain separate |
| Reference chain | [current dated qualification](../quality/realtime-validation-2026-09-15.json), [current source lock](../../assets/sources/gacha-authoring.json) | ADR-0007/0011/0012/0015; only qualified current static scope, historical releases opt-in |
| Platform/feature cost | [support matrix](../platforms/SUPPORT_MATRIX.md), [disabled cost tests](../../tests/integration/tests/disabled_feature_cost.rs) | ADR-0010/0021/0026/0027; no unmeasured backend becomes Supported |
| Distribution | [release checklist](V1_RELEASE_CHECKLIST.md), [provenance](../reference/05-license-and-provenance-boundary.md) | ADR-0017; publication remains disabled |

## Evidence interpretation

The current source lock selects the accepted authoring release. Current static
character parity is recorded separately from engineering independence, first-party
image regression and performance. Dated records are not rewritten when scope grows.
The task-level [report contract](../testing/02-task-results.md) prevents stale run
summaries from becoming new qualification evidence; it does not close visual ADRs.
