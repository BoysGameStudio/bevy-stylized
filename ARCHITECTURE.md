# Bevy Stylized — Architecture Index

> 本文件是架构文档系统的入口，而不是第二份 Master Plan。  
> **规范事实只在一个子文档中定义一次；这里负责导航、层级和核心不变量。**

## Architecture status

- Current state: **Independent implementation; supported static scope qualified, broader visual/release decisions remain open**
- Bevy baseline: **0.19.1**
- Reference family currently targeted for V1: **Genshin / Gacha Setup locked reference family**
- Facade Cargo package: **`bevy-stylized`**
- Canonical GitHub repository: **`BoysGameStudio/bevy-stylized`**
- Project/workspace directory: **`bevy-stylized/`**
- Rust crate identifier: **`bevy_stylized`**
- Public license: **Pending ADR-0017 / provenance review**

Current evidence is linked from [README](README.md#current-qualification-scope).
Use the [current provisional inventory](docs/release/03-provisional-inventory.md)
for implementation locations, and individual ADRs for decision status. Dated
history does not define current acceptance requirements.

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
Upstream Blender Reference  ↔  Production Renderer
                                    ↓
                         Accepted regression images

Independent semantic fixtures validate data and rendering contracts.
```

## Invariants

[Governance](docs/governance/01-invariants-risks-and-adrs.md) owns cross-cutting
invariants and decision rules. Domain contracts below own their specific data and
runtime behavior; this index does not define a second set of requirements.

## Document map

| Topic | Authoritative document |
|---|---|
| Goals, scope, three-axis model | `docs/architecture/01-goals-scope-and-three-axis-model.md` |
| Target architecture | `docs/architecture/02-target-architecture.md` |
| Repository/crate architecture | `docs/architecture/03-repository-and-crate-architecture.md` |
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
| Export contract v1（草案，pending ADR-0012） | `docs/reference/03-blender-export-contract.md` |
| Gacha Setup behavior contract v1（草案，pending ADR-0012） | `docs/reference/07-gacha-setup-behavior-contract-v1.md` |
| Blender export bridge | `docs/authoring/02-blender-reference-export-bridge.md` |
| Authoring schema / runtime packing | `docs/authoring/03-authoring-schema-runtime-packing.md` |
| Bevy glTF import contract | `docs/authoring/04-bevy-gltf-import-contract.md` |
| Platform/capability policy | `docs/platforms/01-capability-and-support-policy.md` |
| Testing / Golden / benchmark | `docs/testing/01-validation-golden-and-performance.md` |
| Visual semantic contract | `docs/quality/semantic-contract.md` |
| Performance target format | `docs/performance/targets.md` |
| Showcase / fixtures | `docs/showcase/01-showcase-examples-fixtures.md` |
| Tooling / versioning / publishing | `docs/release/01-tooling-versioning-and-publication.md` |
| V1/V2/V3 roadmap | `docs/roadmap/01-v1-v2-v3-roadmap.md` |
| V1 release gate | `docs/release/02-v1-release-gate.md` |
| Invariants / risks / ADR governance | `docs/governance/01-invariants-risks-and-adrs.md` |
| ADR index | `docs/adr/ADR_INDEX.md` |
| Requirements traceability | `TRACEABILITY.md` |
| Terms | `GLOSSARY.md` |

## Decision policy

Anything marked **Pending ADR**, **Spike**, **Candidate**, **Experimental**, or an explicit qualification state is intentionally unresolved.  
It must not silently become a runtime contract through implementation.
