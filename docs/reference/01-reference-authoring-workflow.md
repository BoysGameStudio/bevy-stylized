# Blender + Gacha Setup / Genshin Reference Authoring Workflow

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


Blender 与 Gacha Setup 必须正式纳入 Reference Workflow，但它们属于 **dev/reference toolchain**，不是 Runtime dependency。

V1 Reference Scope 明确为：

```text
Genshin Reference Family
```

而不是所有 Gacha Setup 支持的游戏。

完整工作流必须分成三个验证层：

```text
Legally usable Reference Fixture
          │
          ▼
Reference Toolchain Lock
├─ Exact Blender version
├─ Exact Gacha Setup release/tag/commit
├─ Genshin main/outline/post artifact digests
├─ Render engine + color-management contract
├─ Export contract
└─ Genshin profile schema
          │
          ▼
Blender + Gacha Setup + locked Genshin artifacts
          │
          ├─────────────────────────────────────┐
          │                                     │
          ▼                                     ▼
LEVEL 0: Upstream Reference                Runtime Export Preparation
Direct Blender Render                      ├─ Mesh/UV/Normal/Tangent
├─ Camera                                  ├─ Skin/Morph/Animation
├─ Lighting                                ├─ _NPR_* Attributes
├─ Material Node Graph                     ├─ Textures
├─ Compositor/Post                         └─ bevy_stylized extras/sidecar
└─ Color Management                              │
          │                                      ▼
          ▼                              Blender glTF 2.0 Export
   Upstream Golden                               │
          │                                      ▼
          │                              GLB/GLTF + Sidecar/Manifest
          │                                      │
          │                                      ▼
          │                              bevy-stylized validate
          │                                      │
          │                                      ▼
          │                              StylizedMaterialDescriptor
          │                                      │
          └──────────────────┬───────────────────┘
                             ▼
                LEVEL 1: Engine Reference Renderer
                             │
                             ▼
                   Engine Reference Golden
                             │
            ┌────────────────┴────────────────┐
            │ Translation validation         │
            │ against Upstream Golden        │
            └────────────────┬────────────────┘
                             ▼
                 LEVEL 2: Production Renderer
             SurfaceClass × ExecutionTier
                             │
                             ▼
                    Production Regression
```

这个结构分别回答两个问题：

1. **Authoring/Export/Profile Translation 是否正确？** → Engine Reference 与 Upstream Golden 比。
2. **Production Optimization 是否正确？** → Production 与 Engine Reference（以及关键 Upstream probes）比。

禁止让 Engine Reference 与 Production 共用同一错误翻译后再互相验证，从而出现“两个实现一起错、测试仍通过”。

### glTF 的边界

Blender glTF 负责稳定传输：

```text
Mesh / Primitive
UV
Normal / Tangent
Vertex Color
Application-specific _NPR_* attributes
Skinning / Morph / Animation
Standard glTF material inputs
Textures
Custom Properties / extras
```

复杂 Gacha/Genshin Shader Node Graph、Compositor Graph、Blender driver/setup 行为不是 vanilla glTF 的 Runtime Shader Serialization 格式。

因此：

```text
Blender Node Graph = Upstream behavior truth
Runtime glTF/sidecar = runtime input data
Genshin Profile = explicit semantic mapping
Production Renderer = optimized implementation
```

游戏运行、Cargo Build、Portable Runtime 都不能要求安装 Blender 或 Gacha Setup。

---
