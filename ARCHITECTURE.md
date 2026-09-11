# Bevy Stylized — Architecture Index

> 本文件是架构文档系统的入口，而不是第二份 Master Plan。  
> **规范事实只在一个子文档中定义一次；这里负责导航、层级和核心不变量。**

## Architecture status

- Current state: **Architecture Pre-Freeze Review**
- Bevy baseline: **0.19.1**
- Reference family currently targeted for V1: **Genshin / Gacha Setup locked reference family**
- Facade Cargo package: **`bevy-stylized`**
- GitHub repository / project directory: **`bevy-stylized`**
- Rust crate identifier: **`bevy_stylized`**
- Public license: **Pending ADR-0017 / provenance review**

## Core architecture

```text
Authoring / Reference
        │
        ▼
NPR semantic descriptors
        │
        ▼
Surface Class
(Cel / Character / Hero)
        ×
Execution Tier
(Portable / Standard / High)
        ×
Runtime Device Capabilities
        │
        ▼
Resolved Runtime Variant
        │
        ▼
Material / Instance / View GPU data
        │
        ▼
Production Renderer
```

Validation is a separate axis:

```text
Upstream Blender Reference
        ↓
Engine Reference Renderer
        ↓
Production Renderer
```

## Non-negotiable invariants

1. Renderer ≠ Art Style.
2. Authoring Data ≠ Runtime GPU Representation.
3. Material ≠ Instance ≠ View.
4. Surface Class ≠ Execution Tier ≠ Validation Mode.
5. Reference Renderer ≠ Production Renderer.
6. Portable execution path may not require optional GPU features.
7. Whole-pipeline per-stage GPU budgets are architectural contracts.
8. Capability requests happen before RenderDevice creation; runtime selection only uses actually enabled features/limits.
9. Upstream `latest` is never a stable Golden contract.
10. glTF is a runtime data transport, not a serialization format for Blender shader graphs.
11. Any `Supported` platform claim requires real runtime evidence.
12. Optimization claims require both visual evidence and measured performance evidence.

## Document map

| Topic | Authoritative document |
|---|---|
| Goals, scope, three-axis model | `docs/architecture/01-goals-scope-and-three-axis-model.md` |
| Target architecture | `docs/architecture/02-target-architecture.md` |
| Repository/crate architecture | `docs/architecture/03-repository-and-crate-architecture.md` |
| Existing renderer adversarial review | `docs/reviews/01-current-implementation-adversarial-review.md` |
| Surface classes, tiers, resource budgets | `docs/rendering/01-surface-tier-resource-and-bindless.md` |
| Material / Instance / View | `docs/rendering/02-material-instance-view.md` |
| Shader / lighting / outline / post | `docs/rendering/03-shader-lighting-outline-post.md` |
| Profile / V1 reference family | `docs/authoring/01-profile-and-reference-family.md` |
| Reference authoring workflow | `docs/reference/01-reference-authoring-workflow.md` |
| Reference toolchain lock | `docs/reference/02-reference-toolchain-lock.md` |
| Blender export contract | `docs/reference/03-blender-export-contract.md` |
| Upstream upgrade / Golden provenance | `docs/reference/04-upstream-contract-upgrade-and-golden.md` |
| Reference source register | `docs/reference/source-register.md` |
| License / provenance boundary | `docs/reference/05-license-and-provenance-boundary.md` |
| Blender export bridge | `docs/authoring/02-blender-reference-export-bridge.md` |
| Authoring schema / runtime packing | `docs/authoring/03-authoring-schema-runtime-packing.md` |
| Bevy glTF import contract | `docs/authoring/04-bevy-gltf-import-contract.md` |
| Platform/capability policy | `docs/platforms/01-capability-and-support-policy.md` |
| Testing / Golden / benchmark | `docs/testing/01-validation-golden-and-performance.md` |
| Visual semantic contract | `docs/quality/semantic-contract.md` |
| Performance target format | `docs/performance/targets.md` |
| Showcase / fixtures | `docs/showcase/01-showcase-examples-fixtures.md` |
| Tooling / versioning / publishing | `docs/release/01-tooling-versioning-and-publication.md` |
| Tenacity migration | `docs/migration/01-tenacity-migration-plan.md` |
| V1/V2/V3 roadmap | `docs/roadmap/01-v1-v2-v3-roadmap.md` |
| V1 release gate | `docs/release/02-v1-release-gate.md` |
| Invariants / risks / ADR governance | `docs/governance/01-invariants-risks-and-adrs.md` |
| ADR index | `docs/adr/ADR_INDEX.md` |
| Requirements traceability | `TRACEABILITY.md` |
| Terms | `GLOSSARY.md` |

## Decision policy

Anything marked **Pending ADR**, **Spike**, **Candidate**, **Experimental**, or an explicit qualification state is intentionally unresolved.  
It must not silently become a runtime contract through implementation.

The original single-file plan is retained only as historical context:

`docs/archive/bevy_npr_framework_master_plan.md`
