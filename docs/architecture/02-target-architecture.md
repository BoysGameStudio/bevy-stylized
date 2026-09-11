# 总体目标架构

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


最终架构：

```text
                         User Project
                              │
                              ▼
                         NPR Facade
                              │
          ┌───────────────────┼───────────────────┐
          │                   │                   │
          ▼                   ▼                   ▼
       glTF Import         Profiles            Tools
          │                   │
          └──────────┬────────┘
                     ▼
                NPR Core Model
                     │
          ┌──────────┴───────────┐
          ▼                      ▼
   StylizedSurfaceClass         StylizedExecutionTier
   Cel/Character/Hero      Portable/Standard/High
          │                      │
          └──────────┬───────────┘
                     ▼
             Capability Resolver
                     │
                     ▼
             Resolved Runtime Variant
                     │
                     ▼
                NPR Renderer
          ┌──────────┼──────────┐
          ▼          ▼          ▼
       Material   Instance     View
          │          │          │
          └──────────┼──────────┘
                     ▼
                    GPU
```

Reference 必须是三层，而不是 Production 自己验证自己：

```text
LEVEL 0: Upstream Reference
Blender + Locked Genshin Reference Artifacts
            │
            ▼
     Direct Blender Render
            │
            ▼
       Upstream Golden
            │
            │ validates Authoring→Runtime translation
            ▼
LEVEL 1: Engine Reference Renderer
            │
            ▼
      Engine Reference Golden
            │
            │ validates Production optimization
            ▼
LEVEL 2: Production Renderer
Cel/Character/Hero × Portable/Standard/High
```

## 最重要的六个架构转变

```text
Public API tied to ExtendedMaterial
→ Renderer-independent StylizedMaterialDescriptor
```

```text
Monolithic CharacterExtension
→ Surface Class × Execution Tier
```

```text
Dynamic Material State
→ Material / Instance / View Separation
```

```text
Project-specific Shader
→ Generic Core + Reference-Family Profiles
```

```text
Single Reference Layer
→ Upstream Golden + Engine Reference + Production
```

```text
Single backend assumption
→ Whole-pipeline budget + backend-specific validation
```

## Runtime Material Backend 不应预先教条化

“直接实现 Bevy `Material`”可能是最终 Production Backend 的合适方向，但不能因为更底层就假定更快。

V1 先建立：

```text
Authoring / StandardMaterial / glTF
              ↓
       StylizedMaterialDescriptor
              ↓
       Runtime Compiler
```

然后通过 Prototype + Benchmark 比较：

```text
A. Optimized ExtendedMaterial
B. Direct Material implementation
C. Custom Render Pipeline / Phase
```

选择依据：Resource Budget、Bindless、automatic instancing/batching、PBR helper reuse、shadow/prepass、CPU prepare、GPU time、shader complexity 与维护成本。

无论内部选择哪一条路径：

> 公共 API、Authoring 和 Profile Schema 都不能绑定 StandardMaterial 的 Runtime GPU Layout。

---
