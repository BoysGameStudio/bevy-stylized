> [!WARNING]
> **ARCHIVED / NON-NORMATIVE**
>
> This file is retained only for historical traceability. It is no longer a source of architectural truth.
> Current architecture starts at [`/ARCHITECTURE.md`](../../ARCHITECTURE.md), with subsystem specifications and ADRs under `docs/`.
> Do not update ADR numbering, release gates, platform status or implementation contracts here.

# Bevy NPR Framework
## 独立 GitHub 仓库架构、跨平台渲染设计、对抗性检视、重构计划与 V1/V2/V3 路线图

> **文档状态**：Master Architecture Plan  
> **目标**：将 Tenacity 中现有 NPR 实现演化为一个独立、可复用、跨平台、可验证、可扩展，并在不同 GPU 能力层级上取得最佳质量/性能平衡的 Bevy NPR Rendering Framework。  
> **核心方法**：保留现有高还原度实现作为 Reference Renderer；重新建设 Production Renderer；通过 Capability、Resource Budget、Golden Render 和 Benchmark 驱动架构决策，而不是依赖“理论上应该更快/更兼容”的假设。

---

## 目录

1. [目标、定义与非目标](#1-目标定义与非目标)
2. [当前实现的对抗性检视](#2-当前实现的对抗性检视)
3. [总体目标架构](#3-总体目标架构)
4. [独立 GitHub 仓库与 Crate 架构](#4-独立-github-仓库与-crate-架构)
5. [Surface Class、Execution Tier、GPU 资源预算与 Bindless](#5-surface-classexecution-tiergpu-资源预算与-bindless)
6. [Material / Instance / View 数据模型](#6-material--instance--view-数据模型)
7. [Shader、Lighting、Face、Outline 与 PostProcess](#7-shaderlightingfaceoutline-与-postprocess)
8. [Profile、Blender/Genshin Reference Workflow、Authoring 与 Asset Pipeline](#8-profileblendergenshin-reference-workflowauthoring-与-asset-pipeline)
9. [跨平台 Capability 与支持策略](#9-跨平台-capability-与支持策略)
10. [测试、Golden、性能与质量验证](#10-测试golden性能与质量验证)
11. [Showcase、Examples 与 Fixtures](#11-showcaseexamples-与-fixtures)
12. [工具链、版本、发布与公共仓库规则](#12-工具链版本发布与公共仓库规则)
13. [从 Tenacity 迁移的实施路线](#13-从-tenacity-迁移的实施路线)
14. [V1 / V2 / V3 路线图](#14-v1--v2--v3-路线图)
15. [V1 Release Gate 与 Definition of Done](#15-v1-release-gate-与-definition-of-done)
16. [架构不变量、风险与 ADR](#16-架构不变量风险与-adr)
17. [最终结论](#17-最终结论)

---

# 1. 目标、定义与非目标

## 1.1 项目定位

目标不是做一个“好看的 Toon Shader”，而是建设：

```text
Bevy NPR Rendering Framework / SDK
```

它必须满足：

- 独立 GitHub 仓库维护；
- 可通过 Cargo 被其它 Bevy 项目直接依赖；
- Core 不包含 Tenacity / Odette 专属逻辑；
- 数据驱动、Profile 驱动；
- 支持 glTF Authoring 与程序化 Authoring；
- 支持第三方自定义 NPR Profile；
- 覆盖 D3D12 / Vulkan / Metal / GLES / WebGPU；
- 为主机移植预留清晰 backend 边界；
- 同一份 Authoring Data 可在不同 GPU Tier 下选择不同实现；
- 低能力设备保持正确视觉语义；
- 高端设备充分使用更优 GPU 路径；
- 提供 Reference Renderer、Production Renderer、Showcase、Golden Test、Pipeline Test、Resource Budget Test、Benchmark 和 CLI；
- 长期可扩展为 Editor / DCC / Profile Ecosystem。

---

## 1.2 “完美”的工程定义

不存在一个实现能在所有 GPU、所有场景、所有分辨率下同时取得绝对最高性能。

因此本文中的“效果与性能最优”定义为：

> **在明确的视觉误差、平台能力、资源预算、帧时间和内存预算约束内，选择经过测量证明的 Pareto-optimal 实现。**

任何 Production Tier 都必须有：

```text
Visual Error Budget
Frame-Time Budget
GPU Resource Budget
Memory Budget
Feature Set
Fallback Contract
```

任何“性能优化”都必须通过 Benchmark 证明。

任何“效果等价”都必须通过 Reference/Golden 证明。

---

## 1.3 “全平台”的定义与三轴模型

错误定义：

```text
所有平台
→ 同一个 Shader
→ 同一个 BindGroup Layout
→ 同一套 Pass
```

正确设计必须拆成三个正交维度：

```text
Authoring Data
      │
      ▼
NprSurfaceClass          内容/风格语义
├─ Cel
├─ Character
└─ Hero
      ×
NprExecutionTier         设备/性能预算
├─ Portable
├─ Standard
└─ High
      ×
NprValidationMode         验证路径
├─ Production
├─ UpstreamReference
└─ EngineReference
      │
      ▼
Resolved Runtime Variant
```

三个概念不能混用：

- **Surface Class** 回答“这个材质/角色必须表达什么语义”；
- **Execution Tier** 回答“当前设备允许花多少 GPU/内存成本”；
- **Reference Mode** 回答“当前是在验证上游真值，还是执行 Production 实现”。

因此一个 Hero 在低端设备上应优先解析为：

```text
Hero @ Portable
```

而不是简单退化成另一个 Surface Class：

```text
Hero → Character → Portable   // 不再作为默认模型
```

只有 Profile 明确定义了语义允许的 Surface-Class LOD 时，才允许跨 Class 降级。

建议 Execution Tier：

| Tier | 目标 |
|---|---|
| **Portable** | 最低能力路径；不得依赖可选 GPU Feature |
| **Standard** | 主流 iOS/Android Vulkan/普通 PC |
| **High** | 高端 PC、未来主机级 GPU |

Reference 不属于 Execution Tier。Reference 是独立的验证后端/模式。

“全平台”的工程定义是：

> 同样的核心视觉语义拥有明确、确定、可测试的 fallback，并且 Support Claim 与真实 backend/device 验证等级一致。

对于 Web，必须进一步区分 WebGPU 与 WebGL2/GLES3 baseline，不能笼统写成“Web”。

---

## 1.4 V1 非目标

以下内容明确不进入 V1：

- Mesh Shader；
- GPU Driven Culling；
- Indirect Draw；
- 通用 Shader Graph；
- Marketplace；
- 大型 Editor；
- 完整动态质量系统；
- 完整 Texture Streaming；
- 大规模 Profile Marketplace；
- 通用 DCC Graph Compiler。

这些属于 V2/V3。

V1 的唯一战略目标：

> **建立正确、稳定、可复用、可验证、后续无需推倒重来的 Renderer Architecture。**

---

# 2. 当前实现的对抗性检视

现有 Tenacity `bevy-cel` 已具备非常有价值的功能：

- Toon Ramp；
- ILM / Control Map；
- Face SDF；
- Hair Highlight；
- Toon Specular；
- Rim Lighting；
- Genshin Packed Normal；
- Metallic MatCap；
- Specular Ramp；
- Inverted Hull；
- Morph / Skinning Outline；
- Stylized Fullscreen PostProcess；
- Versioned Metadata；
- Blender / Gacha Upstream Reference-Parity。

这些不是需要“重写掉”的东西。

它们应该成为：

```text
Algorithm Assets
+
Reference Renderer
+
Production Renderer 的正确性基线
```

---

## 2.1 严重度总表

| 级别 | 问题 | 结论 |
|---|---|---|
| **P0** | InternalEdge Vertex/Material Uniform ABI 不一致 | 必须立即修 |
| **P0/P1** | 当前完整 CharacterMaterial 无法保证满足最低 whole-pipeline resource budget | WebGPU/GLES baseline 阻塞风险 |
| **P0/P1** | Surface Class 与 Execution Tier 混用 | 必须先修概念模型再实现 Resolver/LOD |
| **P1** | `NprInstanceGpu` 只有逻辑模型，没有 Bevy 0.19 batching-compatible transport contract | V1 必须增加正式 ADR/测试 |
| **P1** | Reference Truth 只有 Engine Reference，没有独立 Upstream Golden 层 | 有“翻译错但两边一起错”的循环验证风险 |
| **P1** | 单一 `GachaProfile` 把多个不同 Shader lineage 混成一套语义 | V1 应锁定具体 Genshin Reference Family |
| **P1** | CharacterExtension 覆盖 Base Material culling | 必须修 |
| **P1** | Monolithic CharacterExtension 资源过肥 | 必须重构 |
| **P1** | Extension 非 bindless 导致 ExtendedMaterial 整体失去 bindless | 高端扩展性问题 |
| **P1/P2** | Face dynamic state 通过 unique material + per-frame mutation 表达 | 通用库必须重构 |
| **P1/P2** | Ramp / ILM / Face 存在可消除的重复采样 | Production 路径应修 |
| **P1/P2** | WebGPU、GLES3.1、WebGL2/GLES3.0 baseline 未分开 | 平台声明不够精确 |
| **P2** | Alpha/Transparency/Shadow/Prepass Contract 缺失 | 可复用 Renderer 必须补齐 |
| **P2** | Outline replica 生命周期依赖 Added-only 构造 | 必须明确 immutable/mutable contract |
| **P2** | Internal Edge 使用 raw depth magic scaling | 视觉稳定性风险 |
| **P2** | PostProcess Camera Component ownership 不够统一 | 组合性问题 |
| **P2** | Genshin/Odette-specific 常量进入 Core Shader | 可复用性问题 |
| **P2** | 大量 Shader Contract Test 只检查源码字符串 | GPU/backend ABI 测试盲区 |
| **P2** | Cargo package 名与许可证在架构文档中被提前假定 | 发布前必须由 ADR/审查决定 |

---

## 2.2 P0：InternalEdge Shader ABI

当前 InternalEdge Material 的 binding 0 对应一个较小的：

```text
InternalEdgeAccentSettings
```

但其 Vertex Shader 复用 Outline Vertex，并把同一 binding 0 解释为完整：

```text
OutlineUniform
```

且实际读取：

- source state；
- source params；
- masks；
- outline width。

因此存在：

```text
Rust Material Binding Layout
!=
Vertex Shader Expected Binding Layout
```

这不是“风格问题”，而是 GPU ABI 契约问题。

### 正确修复

```text
shared_deformation.wgsl
        │
        ├── outline_vertex.wgsl
        │      ├── skin/morph
        │      └── outline extrusion
        │
        └── internal_edge_vertex.wgsl
               ├── skin/morph
               └── ordinary clip transform
```

Internal Edge Vertex 不读取 Outline Uniform。

---

## 2.3 P1：Character Cull Override

Base Material 已经根据自身 Cull Mode 与 View Invert Culling 决定最终 Raster Cull。

当前 CharacterExtension 又在后续 specialization 覆盖：

```text
descriptor.primitive.cull_mode
```

这可能破坏：

- 双面材质；
- Hair Card；
- Cloth；
- Mirror/inverted view；
- Base Material 的标准 culling contract。

### 目标

默认：

```text
NprCullMode::Inherit
```

如果保留 override：

```rust
enum NprCullMode {
    Inherit,
    Back,
    Front,
    None,
}
```

任何情况下 `Inherit` 应为默认。

---

## 2.4 P0/P1：GPU Resource Budget

当前 CharacterExtension 约包含：

```text
9 textures
9 samplers
9 uniform buffers
```

再叠加 StandardMaterial 与 Bevy View Bindings。

保守估计：

```text
                   Texture   Sampler   UBO
CharacterExtension    9         9       9
StandardMaterial      5         5       1
Bevy View             3+        3+      4+
------------------------------------------------
Total                 17+       17+     14+
```

因此不能将：

```text
“WGSL 能编译”
```

等价为：

```text
“WebGPU minimum device 必然支持”
```

### 结论

如果 Portable 明确包含 WebGPU minimum，则 GPU Resource Budget 必须成为 CI Contract。

---

## 2.5 Bindless 路径被阻断

Bevy 的组合 Material 只有 Base 和 Extension 都满足 bindless contract 时才能启用 bindless。

当前 CharacterExtension 是普通 `AsBindGroup`。

结果：

```text
StandardMaterial Bindless
+
CharacterExtension Non-Bindless
↓
Combined Material Non-Bindless
```

对于少量 Hero 不重要。

对于：

- 大量角色；
- 大量不同材质；
- 大规模战斗；
- 高端 PC；
- 未来 GPU-driven pipeline；

这是长期瓶颈。

最终必须支持：

```text
Portable → Fixed Binding
High     → Bindless
```

---

## 2.6 Face dynamic state 所处层级错误

当前思路大致：

```text
Character
↓
Clone CharacterMaterial
↓
Unique Material
↓
每帧写 Face Basis
```

这对少量角色可以工作。

但作为通用 Renderer：

```text
Instance Data
```

不应该通过：

```text
Material Asset Mutation
```

表达。

目标必须变成：

```text
1 Shared Material
+
N Character Instance Records
```

---

## 2.7 Ramp / ILM / Face 的重复采样

当前存在多个 source-level 可消除采样：

### Ramp

普通：

```text
smoothing = 0
page_blend = 0
```

理论上接近 1 个 Ramp sample 即可。

现有逻辑可能提前求值：

```text
primary current
primary next
secondary current
secondary next
```

### ILM

同一 fragment 不应多次 decode 同一 ILM。

目标：

```text
Decode Once
↓
Body
Specular
Emission
Debug
```

### Face

Right Side 不应：

```text
Sample Left
↓
Sample Right
```

而应该直接选择目标纹理。

Face correction/tint 也应尽量共享一次 auxiliary sample。

这些优化必须由 GPU Profile 验证实际收益，但 Production 代码不应保留明显无必要的 source-level work。

---

## 2.8 Outline Replica 生命周期

Added-only construction 并非天然错误。

关键取决于 API contract。

必须明确选择：

### A. Immutable after attach

```text
CharacterOutline 插入后：
Mesh/Skin/Outline Topology 不允许动态改变
```

则：

- 写进 API 文档；
- Debug Build Assert；
- Runtime 简化。

### B. Runtime Mutable

则必须实现 reconciliation：

```text
Desired Replicas
vs
Actual Replicas
↓
Create / Update / Remove
```

不能处于“代码默认为 immutable，但 API 没写”的灰区。

---

## 2.9 Internal Edge 的算法问题

固定 1.5/2 pixel kernel 本身可以是正确设计。

真正值得质疑的是：

```text
raw depth delta × magic constant
```

Raw depth：

- 非线性；
- 与 projection 有关；
- 与 near/far 有关；
- 与距离有关。

High-quality Internal Edge 后续应研究：

```text
Linear/View Depth
Relative Depth Difference
Normal Difference
Distance-aware Threshold
4/8 Neighbor Quality Mode
Temporal Stability
```

---

## 2.10 PostProcess Ownership

目前多个逻辑可能共同控制：

- HDR；
- Bloom；
- Tonemapping；
- DepthPrepass；
- NormalPrepass。

最终不能让多个独立组件各自随意 insert/remove 同一个 Camera 状态。

应该由：

```text
NprCameraPipeline Resolver
```

统一决定最终 Camera Render Requirements。

---

## 2.11 Gacha/Odette 数据进入 Core

以下内容可以存在于 Reference/Profile：

```text
Source material class mapping
Specific shadow thresholds
Specific specular tables
Specific face day/night colors
Source calibration constants
```

但不能长期进入 Generic Core。

原则：

```text
Generic Algorithm
→ Core

Art-style Policy
→ Profile

Character-specific Value
→ Content
```

---

## 2.12 测试盲区

现有源码 Contract Test 很有价值：

```rust
assert!(shader.contains("..."));
```

它适合防止 Upstream Reference-Parity 逻辑被误删。

但不能证明：

- shader 真正可创建；
- bind group size 正确；
- stage visibility 正确；
- Pipeline ABI 正确；
- Resource Budget 正确；
- GPU 输出正确。

必须补真实 Pipeline Test。

---

# 3. 总体目标架构

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
   NprSurfaceClass         NprExecutionTier
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

## 3.1 最重要的六个架构转变

```text
Public API tied to ExtendedMaterial
→ Renderer-independent NprMaterialDescriptor
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

## 3.2 Runtime Material Backend 不应预先教条化

“直接实现 Bevy `Material`”可能是最终 Production Backend 的合适方向，但不能因为更底层就假定更快。

V1 先建立：

```text
Authoring / StandardMaterial / glTF
              ↓
       NprMaterialDescriptor
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

# 4. 独立 GitHub 仓库与 Crate 架构

## 4.1 仓库工作名、Cargo Package 名与许可证

GitHub 仓库可以暂时使用工作名：

```text
bevy-npr
```

但**不能把 `bevy-npr` 当成已确认可发布的 crates.io Package 名**。截至本次审查，docs.rs 已存在 `bevy_npr 0.1.0`；crates.io 的名称碰撞规则会把 `-`/`_` 差异视为冲突。因此正式发布名必须通过 `Package Naming ADR` 确定。

同样，仓库许可证不能在尚未完成 GPL/Reference provenance 审查前直接假定为 MIT/Apache 双许可。Gacha Setup 和当前 Genshin Reference artifact lineage 均声明 GPL-3.0；独立实现最终可以采用何种许可证取决于实际代码/资产来源与法律审查。

V1 仓库结构中的 crate 目录名以下均是**工作目录名**，不等价于最终 registry package 名：

```text
bevy-npr/
│
├── Cargo.toml
├── Cargo.lock
├── README.md
├── ARCHITECTURE.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── LICENSE.md                 # final license TBD by ADR/review
├── THIRD_PARTY_NOTICES.md
│
├── crates/
│   ├── facade/                # published name TBD
│   ├── core/
│   ├── render/
│   ├── gltf/
│   ├── profile-genshin/       # V1: concrete Genshin Reference Family
│   └── tools/
│
├── shaders/
│   ├── common/
│   ├── cel/
│   ├── character/
│   ├── hero/
│   ├── outline/
│   ├── post/
│   └── reference/
│
├── examples/
├── showcase/
├── fixtures/
├── tests/
├── benches/
├── tools/blender/
├── docs/
│   ├── adr/
│   ├── architecture/
│   ├── authoring/
│   ├── profiles/
│   ├── platforms/
│   ├── reference/
│   ├── performance/
│   ├── quality/
│   └── migration/
└── .github/workflows/
```

必须新增两个早期 ADR：

```text
Package Naming ADR
License & Provenance ADR
```

---

## 4.2 V1 不应过度拆 Crate

V1 控制到：

```text
facade
core
render
gltf
profile-genshin
tools
```

Outline / Post / Runtime Asset 先作为 `bevy-npr-render` module。

只有出现清晰独立 API 与独立版本价值时，再拆：

```text
bevy-npr-outline
bevy-npr-post
bevy-npr-assets
```

---

## 4.3 依赖方向

必须单向：

```text
Showcase / Tools / Examples
            │
            ▼
Profiles / glTF
            │
            ▼
Renderer
            │
            ▼
Core
```

禁止：

```text
Core → Gacha
Core → Showcase
Renderer → Tenacity
```

推荐：

```text
bevy-npr
├── core
├── render
├── optional gltf
└── optional profile-genshin

render
└── core

gltf
└── core

profile-genshin
├── core
└── 尽量不接触 render internals

tools
├── core
└── gltf
```

---

## 4.4 Facade

对外必须只有一个稳定的 facade crate/API，但**最终 crates.io package 名在 Package Naming ADR 通过前保持 TBD**。

文档示例统一使用逻辑占位名：

```toml
[dependencies]
<published-npr-package> = "1"
```

Rust import 名也由最终 package/lib name 决定。

Facade 必须同时提供：

```text
NprPlugin                  # programmatic/no-glTF path
NPR-aware GltfPlugin helper # _NPR_* registration before loader initialization
```

不能承诺“DefaultPlugins 加完以后再加 NprPlugin 就能 retroactively 注册 glTF custom attributes”。

Facade 只负责：

- Re-export；
- Plugin Group；
- compile capability routing；
- 稳定公共 API。

Renderer internals 尽量 `pub(crate)`，不要把 Bevy Render World internals 变成 1.0 稳定 API。

---

## 4.5 Core

`bevy-npr-core` 只包含：

- Domain Model；
- `NprMaterialDescriptor`；
- `NprInstanceDescriptor`；
- `NprViewDescriptor`；
- `NprSurfaceClass` / `NprExecutionTier` / Validation Mode；
- Capability Model；
- Quality Policy；
- Profile Contract；
- Schema-neutral serialization types。

尽量避免依赖完整 `bevy` crate。

能使用：

```text
bevy_math
bevy_color
bevy_reflect（需要时）
```

就不要为了几个类型依赖完整 Renderer。

Core 不应出现：

```text
RenderPipelineDescriptor
BindGroup
TextureView
CommandEncoder
RenderPhase
```

---

# 5. Surface Class、Execution Tier、GPU 资源预算与 Bindless

## 5.1 Production Surface Classes

Production 语义 Class：

```text
Cel
Character
Hero
```

它们描述**内容需要什么**，不是设备质量档位。

### Cel

核心功能：Base Color、Ramp、Basic Control/ILM、Basic Toon Lighting、Hull Outline。

### Character

在 Cel 上增加：Face SDF、Hair、Rim、Emission、Enhanced Specular。

### Hero

在 Character 上增加具体 Profile 所需的高规格语义，例如 Genshin Reference Family 的 authored ramp、packed normal、matcap/specular ramp、face correction、explicit key light、source palette。

同一个 Surface Class 必须能够存在不同 Execution Tier：

```text
Hero @ Portable
Hero @ Standard
Hero @ High
```

Profile 可以为某些高级语义定义 approximation/fallback，但不能由 Resolver 随机改变角色语义。

## 5.2 Execution Tiers

```text
Portable
Standard
High
```

### Portable

不得依赖：bindless、FP16、subgroup、compute、HDR。若目标包含 WebGL2/GLES3.0，则也不能要求 storage buffer。

### Standard

允许使用主流现代移动/桌面能力，但必须仍有 Portable fallback。

### High

可以使用 bindless、更高资源预算和更昂贵 pass，但所有 Feature 必须有明确 fallback。

## 5.3 Whole-Pipeline Resource Budget

原先的：

```text
Material textures <= 6
samplers <= 3
UBO <= 2
```

保留为**暂定 Material Sub-budget**，但它不是平台兼容性的最终判断对象。

真正的 per-stage contract 必须统计：

```text
View
+
Mesh
+
Instance
+
Material
+
Profile
+
Safety Margin
=
Whole-Pipeline Stage Budget
```

原因：WebGPU/wgpu 的 `maxSampledTexturesPerShaderStage`、`maxSamplersPerShaderStage`、`maxUniformBuffersPerShaderStage` 是对整个 shader stage 可见 layout 计数，而不是单独 Material bind group。

Baseline 至少分别验证：

```text
WebGPU Core / modern wgpu defaults
GLES 3.1 / wgpu downlevel_defaults
GLES 3.0 + WebGL2 / wgpu downlevel_webgl2_defaults（若支持）
```

WebGL2/GLES3.0 baseline 特别要考虑：storage buffer = 0，uniform buffers per shader stage = 11。

## 5.4 Uniform Packing

CPU API 可以保持模块化 settings；GPU Prepare 按 SurfaceClass × ExecutionTier pack 成少量合理的 GPU buffers。禁止回到“每个 Feature 一个 UBO”。

## 5.5 Sampler Deduplication

按 sampler state 分类共享，目标通常是 Nearest / Linear / LinearClamp 等少数状态，而不是每张纹理一个 sampler。具体数量由 backend/prototype 测量决定。

## 5.6 Fixed Binding + Bindless

```text
Portable / unsupported
→ Fixed Binding

High-end capable & measured beneficial
→ Bindless
```

Bindless 是优化路径，不是语义和运行前提。

## 5.7 Resource Budget Test 必须从最终 Pipeline Layout 反射/统计

CI 不能只统计 `NprMaterial` 自己声明了几张纹理。必须在真实 specialization 后统计最终 pipeline 对 vertex/fragment stages 的资源可见性，并与对应 baseline limits 比较。

---

# 6. Material / Instance / View 数据模型

这是 V1 必须完成的核心重构。

## 6.1 Material

只保存：

```text
Textures
Ramp/Profile
Static Material Flags
Hair/Metal Static Parameters
Static Profile Parameters
```

---

## 6.2 Instance

保存：

```text
Face Side
Face Threshold
Per-character Flags
Outline Scale
Profile Index
Instance Overrides
```

---

## 6.3 View / Frame

保存：

```text
NPR Key Light
Global Palette
Day/Night
Exposure
Quality State
Camera NPR State
```

---

## 6.4 明确禁止

禁止：

```text
Face Transform
→ Material Asset 每帧修改
```

禁止：

```text
Global Day/Night
→ 每个 Material 重复写一遍
```

禁止：

```text
Camera Post Setting
→ Character Material
```

---

## 6.5 Face SDF

当前 unique material path 应演化为：

```text
Shared Material
      +
NprInstanceGpu
```

例如：

```rust
struct NprInstanceGpu {
    face_threshold: f32,
    face_side: u32,
    outline_scale: f32,
    profile_index: u32,
}
```

---

## 6.6 Face Light Math

以下数学：

```text
sin/cos
basis normalization
dot
acos/pow
side selection
threshold
```

如果输入对一个角色一帧保持一致，应从：

```text
Per Fragment
```

提升到：

```text
Per Instance Prepare
```

最终 Fragment 主要做：

```text
Sample SDF
+
Threshold Compare
+
Soft Transition
```

若某 Profile 需要真正 per-pixel light direction，则可以在 Hero/Reference 特化，不应拖累默认路径。

---

## 6.7 NprInstance Data Transport（V1 Blocker）

`NprInstanceGpu` 只是逻辑数据模型；V1 还必须定义它如何进入 Bevy 0.19 GPU pipeline，同时保留 automatic instancing/batching。

Bevy 0.19.1 官方 automatic-instancing 示例证明：相同 Mesh + Material 可以自动合批，并使用 `MeshTag(u32)` 为每个 mesh instance 提供不同索引；shader 可通过 `mesh_functions::get_tag(instance_index)` 读取。Bevy 同时提供 `GpuArrayBuffer<T>`，在支持 storage buffer 时使用 storage，在 storage 不可用时可退回 batched dynamic uniform buffer。**但这只证明数据有 fallback storage 形式，不证明该 fallback 与 NPR mesh automatic batching 的最终组合一定最优**；dynamic offset/unbatchable 路径必须通过实际 render-phase prototype 验证。

因此必须新增：

```text
ADR: NPR Instance Data Transport
```

至少比较：

```text
A. MeshTag 直接 bit-pack 极少量数据
B. MeshTag → index into GPU/texture instance table
C. GpuArrayBuffer<T> + MeshTag/index integration
D. Texture/LUT-backed instance table for WebGL2-style baseline
E. Custom batching/phase（last resort）
```

### V1 Gate

创建 100/1000 个：

```text
same Mesh
same Material
same Pipeline
```

但拥有不同：

```text
Face threshold
Face side
Palette/Profile index
Outline instance control
```

必须验证：

```text
Instance Data 正确
+
Automatic Instancing/Batched Draw 未被无意破坏
+
Portable baseline 有非-storage-buffer fallback（若 WebGL2/GLES3.0 在支持范围内）
```

在该 ADR 通过前，不应把任何具体 storage-buffer 方案写成稳定公共 ABI。

---

# 7. Shader、Lighting、Face、Outline 与 PostProcess

## 7.1 Shader Entry Point

真正拆成不同 compilation unit：

```text
portable_forward.wgsl
character_forward.wgsl
hero_forward.wgsl
reference_gacha_forward.wgsl
```

共享：

```text
common.wgsl
ramp.wgsl
face.wgsl
hair.wgsl
metal.wgsl
lighting.wgsl
packing.wgsl
```

Portable 编译时不能包含无关 Hero-only 代码。

---

## 7.2 Shader Permutation Policy

禁止指数排列：

```text
FACE × HAIR × RIM × METAL × EMISSION × NORMAL × ...
```

只有影响：

- Resource Layout；
- 大块 Shader Code；
- Render State；

的 Feature 才进入 compile-time specialization。

建议：

```text
Surface Class:
Cel / Character / Hero

Compile-time candidates:
Normal Mapping
Face
Metal
Alpha Mode
```

Runtime：

```text
Hair Intensity
Rim Intensity
Palette
Day/Night
Debug Mode
Light Direction
```

最终以 Pipeline Count 和 Profile 数据决定是否继续减少/增加 permutation。

---

## 7.3 Lighting Backend

统一 NPR Surface，多个 Lighting Backend：

```text
             NPR Surface
                  │
       ┌──────────┴───────────┐
       ▼                      ▼
PhysicalLighting       AuthoredLighting
       │                      │
 Bevy Lights             NPR Key Light
 Shadows                 Source Palette
 Environment             Authored Ramp
       │                      │
       └──────────┬───────────┘
                  ▼
              NPR Compose
```

公开模型：

```rust
enum NprLightingModel {
    Physical,
    Authored,
    Hybrid,
}
```

### Physical

适合 Generic Cel：

```text
Bevy Physical Lighting
→ NPR Remap
```

### Authored

适合 Gacha/Hero：

```text
Explicit Key Light
+
Authored Profile
```

### Hybrid

有选择地引入：

- Physical shadow；
- local light influence；
- environment contribution。

不要让所有 Hero 永久支付完整 Physical PBR 成本，除非 Profile 明确需要。

---

## 7.4 Engine Reference Renderer

当前 Tenacity 中高还原、Genshin-targeted 的 shader 应在修复明确 correctness 问题后冻结为：

```text
Engine Reference Renderer
```

它的目标是尽量复现锁定的 Upstream Blender/Genshin Reference，但**真正的一级视觉 Oracle 是 Blender 直接生成的 Upstream Golden**。

Engine Reference 的职责：

```text
验证 Blender/glTF/Profile Translation
+
为 Production optimization 提供 engine-side oracle
```

它不以 Production Performance 为目标。Shipping 默认不编译 Reference；推荐 `reference` compile capability 默认关闭或放入独立 dev/test package。

---

## 7.5 Ramp Fast Path

当：

```text
smoothing == 0
page_blend == 0
```

应实现真正最小采样路径。

只有真正需要：

- next band；
- secondary page；

时才采样。

---

## 7.6 ILM Decode Once

单 fragment 生命周期中：

```text
Decode ILM Once
↓
Reuse
```

Body、Specular、Emission、Debug 共享结果。

---

## 7.7 Face Sampling

选择 Side 后只采一张目标 SDF。

Face Auxiliary/Correction 尽量单次采样并共享。

---

## 7.8 Outline

Inverted Hull 大方向保留：

```text
Vertex Extrusion
+
Front-face Cull
```

优点：

- 无 Geometry Shader；
- 无 Tessellation；
- 无 Compute 依赖；
- 适合 D3D12/Vulkan/Metal/GLES/WebGPU；
- 可支持 Skinning/Morph。

必须定义：

- lifecycle；
- mesh/skin/morph update；
- LOD；
- bounds；
- despawn；
- immutable/mutable contract。

---

## 7.9 Internal Edge

不是 Portable Required Feature。

推荐：

```text
Portable → Hull
Standard → Hull
High → Hull + optimized internal edge
Reference → upstream-reference-matching edge
```

High path 再优化：

- View/linear depth；
- Relative depth threshold；
- Normal edge；
- 4/8 neighbor mode；
- Distance scaling；
- Temporal stability。

---

## 7.10 PostProcess

保留 Single Fullscreen Pass 思想：

```text
Vignette
Lens Distortion
Chromatic Aberration
Display Transform
```

Disabled Feature 应减少不必要采样。

例如 Chromatic=0：

```text
只采中心颜色
```

而不是额外 R/B sample。

---

## 7.11 Camera Pipeline

新增统一 Resolver：

```text
NprCameraPipeline
```

综合决定：

- HDR；
- Bloom；
- Tonemapping；
- Display Transform；
- Depth/Normal Prepass；
- Stylized Fullscreen。

避免多个功能互相 insert/remove Camera Component。

---

## 7.12 HDR

HDR 不得成为 NPR Core Requirement。

Portable 必须允许：

```text
LDR
```

同时保持核心 NPR 语义。

HDR 是：

```text
Quality/Camera Capability
```

---

## 7.13 Rendering Path Contract（V1 必需）

V1 必须明确每个 SurfaceClass × ExecutionTier 对以下 Render Path 的状态：

```text
Main Forward
Shadow Caster
Depth Prepass
Normal Prepass
Motion Vector Prepass（若使用）
Deferred
Meshlet
```

建议 V1 以 `Forward-first` 作为候选策略，但必须通过 ADR/Prototype 决定；Deferred/Meshlet 若未验证，应明确标记 Unsupported/Experimental，而不是留空。

同一材质的：alpha discard、culling、skinning、morph、double-sided 语义必须在 Main/Shadow/Prepass 一致。

## 7.14 Alpha / Transparency / Shadow / Prepass Matrix（V1 必需）

Bevy 0.19 Material 支持 Opaque、Mask、Blend、Premultiplied、AlphaToCoverage、Add、Multiply；同时 Bevy prepass 只处理 opaque 与 alpha-mask，透明材质会被忽略。

因此 V1 必须维护并用测试填实以下矩阵，而不是默认“所有材质都支持”：

| Surface | Main NPR | Shadow | Depth/Normal Prepass | Hull Outline | Internal Edge |
|---|---|---|---|---|---|
| Opaque | TBD/Tested | TBD | TBD | TBD | TBD |
| Mask | TBD/Tested | TBD | TBD | TBD | TBD |
| AlphaToCoverage | TBD | TBD | TBD | TBD | TBD |
| Blend | TBD | TBD | N/A by default | TBD | N/A/alternative |
| Premultiplied | TBD | TBD | N/A by default | TBD | N/A/alternative |
| Add/Multiply | TBD | TBD | N/A by default | profile-specific | N/A/alternative |

Hair cards、半透明眼睛/饰品、特效材质必须通过这张 Contract 表声明真实能力。

---

# 8. Profile、Blender/Genshin Reference Workflow、Authoring 与 Asset Pipeline

## 8.1 Renderer ≠ Art Style

Core 提供：

- Ramp；
- Face；
- Hair；
- Rim；
- Specular；
- MatCap；
- Outline；
- Lighting；
- Post primitives。

Profile 决定如何组合。

---

## 8.2 Profile Contract

Profile 负责 Art-style policy，不负责 GPU backend 选择。

建议逻辑接口（仅表达语义，不承诺最终 Rust ABI）：

```rust
pub trait NprProfile {
    fn surface_class(&self) -> NprSurfaceClass;

    fn prepare_material(
        &self,
        source: &NprSourceMaterial,
        output: &mut NprMaterialDescriptor,
    );

    fn prepare_instance(
        &self,
        context: &NprInstanceContext,
        output: &mut NprInstanceData,
    );

    fn fallback_contract(
        &self,
        tier: NprExecutionTier,
    ) -> NprProfileFallbackContract;
}
```

关键边界：

```text
Profile
→ 声明 Surface semantics + fallback contract

Capability Resolver
→ 根据实际 RenderDevice + Execution Policy 选择实现
```

Profile 不应直接查询/绑定 wgpu backend internals。

V1 需要稳定的是**Profile 语义与 Versioned Schema Contract**；具体 Rust trait signature 在 1.0 前可以调整。V3 所谓“稳定 Profile”默认指 Stable API + Versioned Schema，而不是未经设计的二进制 Rust ABI。

---

## 8.3 Reference Family：V1 锁定 Genshin，而不是抽象“Gacha”

Gacha Setup 是一个聚合工具，当前同时覆盖 Genshin、HSR、ZZZ、NTE、WuWa 等，而这些游戏引用的是不同 shader lineage。它们不能在 V1 被假定为共享同一套 shading contract。

V1 只定义一个具体、可追溯的：

```text
Genshin Reference Family
```

其上游至少包括：

```text
Gacha Setup locked release
+
Festivity/Blender-miHoYo-Shaders lineage
+
locked Genshin main .blend artifact
+
locked outline .blend artifact
+
locked post-processing .blend artifact
```

V1 profile crate/模块使用工作名：

```text
profile-genshin
```

未来如支持 HSR/ZZZ/WuWa，应分别建立：

```text
HsrReferenceFamily
ZzzReferenceFamily
WuwaReferenceFamily
```

它们可以在更高层被某个 `gacha` collection/facade 聚合，但不能把所有 shader family 的特殊逻辑塞进一个 Hero shader/profile。

文档中的“Upstream Reference-Parity”统一解释为：

> **Upstream Reference-Parity：与锁定的 Blender/Gacha Setup/具体 shader artifact Reference 保持定义范围内的一致。**

它不等价于“与原始商业游戏 renderer 100% 一致”。若未来需要 Game Capture Parity，必须建立独立验证轨道。

---

## 8.4 Blender + Gacha Setup / Genshin Reference Authoring Workflow

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
├─ Compositor/Post                         └─ bevy_npr extras/sidecar
└─ Color Management                              │
          │                                      ▼
          ▼                              Blender glTF 2.0 Export
   Upstream Golden                               │
          │                                      ▼
          │                              GLB/GLTF + Sidecar/Manifest
          │                                      │
          │                                      ▼
          │                              <npr-cli> validate
          │                                      │
          │                                      ▼
          │                              NprMaterialDescriptor
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

## 8.5 Reference Toolchain Lock

Golden Reference 必须绑定**明确版本**。

禁止：

```text
Blender = latest
Gacha Setup = latest
```

作为 Golden 的永久定义。

每一套 Reference Baseline 都必须保存：

```text
Blender exact version
Gacha Setup release
Gacha Setup tag
Gacha Setup commit SHA
Release package SHA-256
Reference Family ID
Main Shader Artifact path + digest
Outline Artifact path + digest
Post Artifact path + digest
Reference Render Engine / Color Management / View Transform
Canonical Reference OS / GPU backend / GPU+driver (when renderer-dependent)
Export Contract version
bevy-npr Genshin Reference Profile schema version
Reference fixture revision
Camera/Lighting preset revision
Golden revision
```

建议建立：

```text
docs/reference/toolchain-lock.md
```

以及机器可读：

```text
reference-toolchain.toml
```

示意结构：

```toml
[blender]
series = "5.2 LTS"
version = "<exact validated Blender version>"

[gacha_setup]
release = "v3.6.2"
tag = "v3.6.2"
commit = "069e51f3fd89cdc66b4c2741798ede11a5fb16b5"
release_asset = "Gacha_Setup-v3.6.2.zip"
sha256 = "f5ab8206464130962234a7ccab19c7d212a048c848ce3e69518f84fc41a39999"

[export]
contract = "blender-gltf-npr-v1"

[reference_family]
id = "genshin-festivity-gacha-setup-v1"

[reference_family.artifacts]
main = "setup_wizard/shaders/gi/HoYoverse - Genshin Impact v3.4.blend"
main_git_blob = "7637f1d18a63188ca08870fb554d949dafefaecf"
outline = "setup_wizard/shaders/gi/HoYoverse - Genshin Impact Outlines v3.blend"
outline_git_blob = "532e0860065acea1e427f1d4033f69626793036b"
post = "setup_wizard/shaders/gi/HoYoverse - Genshin Impact Post-Processing.blend"
post_git_blob = "7cafd3d5fcb77e5e7efa8037531e085887f57d5e"

[profile]
genshin_schema = "v1"
```

上面的 Gacha Setup `v3.6.2` 是 **2026-09-11 文档修订时的最新正式 Release 和候选 Reference Upstream**，不是“永远自动使用最新版本”的规则。真正进入 Golden Baseline 前仍必须完成完整 Reference Validation。

若将来最新 release 变成 v3.7/v4：

```text
New Upstream Release
        ↓
Candidate Upgrade Branch
        ↓
Reference Import + Export
        ↓
Golden Compare
        ↓
Semantic Diff Review
        ↓
Accept / Reject
```

不能让 CI 自动跟随最新 release。

---

## 8.6 Blender Export Contract

必须建立：

```text
docs/reference/blender-export-contract.md
```

它是 Blender → glTF → bevy-npr 的稳定边界。

至少冻结：

### Scene / Transform

- glTF Y-Up；
- Object/Armature scale policy；
- Apply Modifier policy；
- negative/non-uniform scale policy；
- coordinate conversion policy。

### Mesh

必须明确是否导出：

```text
UVs
Normals
Tangents
Vertex Colors
Custom Attributes
Morph Targets
Skinning
```

对于 NPR 依赖的数据，不允许使用“Blender 默认值碰巧正确”。

### NPR Mesh Attributes

Blender 5.2 glTF Exporter 支持导出名称以下划线开头的 Mesh Attribute。

因此 V1 应正式定义：

```text
_NPR_CONTROL0
_NPR_HAIR_TANGENT
...
```

的：

- Blender Attribute Domain；
- Blender Data Type；
- glTF Semantic；
- Bevy VertexFormat；
- Shader Location / Runtime Packing；
- Quantization Rules；
- Default Value；
- Missing Attribute Fallback。

不得仅依赖“名称约定”而缺乏 Schema。

### Custom Properties

Blender glTF Export 可将 Custom Properties 写入 glTF `extras`。

因此：

```text
Material/Rig/Profile metadata
```

可以有两种规范化输出方式：

1. glTF `extras`；
2. Versioned sidecar。

选择规则必须记录在 Authoring Contract 中。

### Reference Artifact 与 Runtime Export Artifact 必须分离

完整工作流必须产生两类不同证据：

#### A. Reference Artifact

```text
source.blend
reference-render/
reference-manifest.json
```

它保存：

- Gacha Setup 实际 Node Graph；
- Blender-only setup；
- Reference Camera；
- Reference Lighting；
- Compositor/Post；
- Reference Render；
- 上游版本信息。

这是视觉行为的 Source of Truth。

注意：若 `source.blend` 包含无法公开再分发的第三方资产，它只能存在于合法的本地/私有 Reference Suite；公开仓库只提交可再分发的 synthetic/self-owned Reference Fixture、manifest、允许公开的 Golden 与行为 Contract。

#### B. Runtime Export Artifact

```text
export.glb
export-manifest.json
sidecar.json（若存在）
runtime-textures/
```

它保存：

- Runtime Geometry；
- UV/Normal/Tangent；
- `_NPR_*` Attributes；
- Skin/Morph/Animation；
- 可导出的标准 Material 输入；
- Texture；
- Namespaced Metadata。

Runtime Artifact 不需要、也不应该尝试保存完整 Blender/Gacha Shader Node Graph。

### Export Artifact Manifest

`export-manifest.json` 至少记录：

```text
Blender Version
Gacha Setup Version
Gacha Setup Commit
Gacha Setup Release Asset Digest
Blender glTF Exporter Version
Export Contract Version
Export Settings Hash
Reference Node-Graph Signature/Revision
Reference Render Engine / Color Management
Canonical Reference Environment ID
Source Fixture Hash
Output Hash
```

这样才能判断一次 Golden 变化来自：

- Blender；
- Gacha Setup；
- Exporter；
- Source Asset；
- bevy-npr；

中的哪一层。

---

## 8.7 Genshin Reference Family / Gacha Setup Upstream Contract

Gacha Setup 不应作为“黑盒神奇插件”。

`profile-genshin` 必须明确记录它从上游 Reference 中复现哪些行为。

建议建立：

```text
docs/reference/gacha-setup-contract.md
```

按功能记录：

| Reference Area | Upstream Behavior | bevy-npr Representation |
|---|---|---|
| Shadow Ramp | Blender/Gacha material graph | Ramp/Profile data |
| ILM/Lightmap | Channel/class semantics | Control texture decode |
| Face | Direction transfer + threshold maps | Face SDF |
| Normal | Source packed normal behavior | Hero/Profile normal path |
| Metal | MatCap/specular behavior | Hero metallic module |
| Outline | Source outline classes/width | Outline profile |
| Lighting | Source key light/palette | Authored Lighting backend |
| Post | Source display/compositor behavior | Reference/Post profile |

每个行为必须标记：

```text
Exact
Approximation
Intentional Difference
Unsupported
```

对每个 `Exact/Approximation` 项，还必须记录它的输入来源：

```text
Texture Channel
Vertex Attribute
Object/Material Custom Property
Profile Constant
View/Frame State
Derived Instance State
```

这样可以防止 Runtime Shader 反向依赖 Blender Node Name、Socket Index 或某个未版本化的节点拓扑。

Reference 工具可以读取/检查 Blender/Gacha Setup 的节点结构，但最终 Runtime Contract 必须落到稳定的 `bevy-npr` 语义上。


这样 Production Renderer 的目标不是模糊的“看起来像 Genshin”，而是有明确 Contract。

---

## 8.8 Reference Baseline Upgrade Policy

更新 Blender 或 Gacha Setup 属于**受控 Reference Migration**。

禁止：

```text
dependency update
→ Golden update
→ merge
```

正确流程：

```text
1. Open Reference Upgrade PR
2. Update toolchain lock
3. Regenerate Blender reference export
4. Run <npr-cli> validator
5. Render old Reference implementation
6. Render new upstream-derived Reference
7. Generate semantic/visual diff
8. Classify changes
9. Update Profile Contract if needed
10. Optimize Production path
11. Update Golden only after approval
```

变化分类：

```text
A. Upstream Bug Fix
B. Upstream Intentional Visual Change
C. Blender Export Behavior Change
D. Gacha Setup Material/Node Change
E. bevy-npr Importer Bug
F. Production Renderer Regression
```

Golden 不能成为“测试失败以后随手更新”的文件。

---

## 8.9 Golden Provenance

每张 Reference Golden 必须能够追溯：

```text
Production/Engine Golden
   ↓
Upstream Blender Golden
   ↓
Reference Fixture
   ↓
Reference Manifest / Node-Graph Revision
   ↓
Export Artifact Hash
   ↓
Blender Version
   ↓
Canonical Blender Render Environment
   ↓
Gacha Setup Version/Commit/Digest
   ↓
Profile Contract Version
   ↓
Reference Renderer Commit
```

建议每个 Golden 附带：

```text
<case>.golden.json
```

例如：

```json
{
  "reference_toolchain": "gacha-blender-v1",
  "blender": "<exact validated Blender version>",
  "gacha_setup": "v3.6.2",
  "gacha_setup_commit": "069e51f3fd89cdc66b4c2741798ede11a5fb16b5",
  "export_contract": "blender-gltf-npr-v1",
  "profile_schema": "gacha/v1",
  "fixture": "hero_face_day_v1",
  "renderer": "reference-gacha"
}
```

这使 Golden 成为真正的工程证据，而不是匿名 PNG。

---

## 8.10 Reference Source Register

建议建立：

```text
docs/reference/source-register.md
```

至少包含以下权威来源：

| Source | Role |
|---|---|
| https://github.com/PaoloESAN/gacha-setup | Gacha Setup upstream repository |
| https://github.com/PaoloESAN/gacha-setup/releases | Versioned Gacha Setup release source |
| https://github.com/festivities/Blender-miHoYo-Shaders | V1 Genshin shader lineage / upstream reference |
| https://gacha-setup.pages.dev/ | Gacha Setup project documentation |
| https://docs.blender.org/manual/zh-hans/5.2/addons/scene_gltf2.html | Blender 5.2 LTS glTF 2.0 export contract/reference |
| Blender/Gacha Setup documentation used by a specific profile | Behavior/reference documentation |
| bevy-npr ADR/Profile contracts | Local normative specification |

规则：

> 外部文档描述“上游做什么”；`bevy-npr` 自己的 Profile/ADR 描述“我们承诺什么”。

上游改变不能隐式改变 `bevy-npr` 的稳定行为。

---

## 8.11 License / Provenance Boundary

当前 Gacha Setup 仓库声明 GPL-3.0，并明确其仓库不分发游戏模型、纹理或专有游戏资产。

因此公开 `bevy-npr` 仓库必须区分：

```text
Behavioral / Visual Reference
vs
Copied Source Code
vs
Copied Assets
```

要求：

1. 不把未授权 HoYoverse/其它游戏资产提交到公开仓库；
2. 公共 Fixture 使用自制、CC0 或明确允许再分发的资产；
3. 复制/修改任何 GPL 上游源码前必须单独进行 License Review；
4. 不因为“只是 Shader”就默认上游代码可以复制进 permissive core；
5. `THIRD_PARTY_NOTICES.md` 记录实际使用的第三方代码、工具与许可；
6. Reference 文档记录来源与行为映射，但 Runtime Core 尽量保持独立实现；
7. 私有 Upstream Reference-Parity 资产如无法合法公开，可进入 optional local/private reference suite，不得成为公开 CI 的必要依赖。

这不是为了削弱 Gacha Setup 的参考价值，而是保证：

```text
Reference Provenance 清晰
+
公开仓库可合法复用
+
Runtime License 边界明确
```

---

## 8.12 Blender Reference Export Bridge（V1 必需）

既然复杂 Gacha Shader Node Graph 不能依赖 glTF 自动序列化，V1 必须有一个**最小、无 UI 依赖、可脚本化**的 Blender Reference Export Bridge。

它不是 V3 的完整 Blender Add-on/Editor。

它的唯一职责是让：

```text
Blender/Gacha Setup Reference
→ bevy-npr Runtime Input
```

这一步可重复、可验证、可自动化。

建议：

```text
tools/blender/
├── README.md
├── export_reference.py
└── contracts/
```

运行方式应支持 Blender background mode，例如：

```text
blender source.blend --background --python export_reference.py -- <profile/options>
```

### Bridge 职责

1. 验证当前 Blender 版本与 Toolchain Lock；
2. 验证 Gacha Setup release/profile signature；
3. 定位已知的 Gacha Material/Node Group 语义；
4. 将上游行为映射到稳定 `bevy-npr` Profile 字段；
5. 验证/生成需要的 `_NPR_*` Mesh Attributes；
6. 导出 Namespaced Custom Properties / sidecar metadata；
7. 使用锁定的 glTF Export Settings 调用 Blender Exporter；
8. 输出 `export-manifest.json`；
9. 输出 Node-Graph/Material Signature，供上游升级时检测结构变化；
10. 对未知/改变的上游节点拓扑 **Fail Closed**，而不是静默猜测。

### Fail Closed 原则

如果：

```text
Expected Node Group Missing
Socket Name Changed
Material Topology Changed
Required Texture Missing
Attribute Domain Changed
Exporter Setting Drifted
```

则：

```text
Export Failure
```

而不是：

```text
Best-effort Guess
```

Reference Pipeline 的目标是可追溯正确性，不是“尽量导出来”。

### Runtime 与 Bridge 解耦

必须保持：

```text
bevy-npr runtime
    不依赖 bpy
    不依赖 Blender
    不依赖 Gacha Setup

blender-reference-export
    dev/tool only
```

### V1 与 V3 的边界

V1：

```text
Scriptable Reference Export Bridge
```

V3：

```text
Full Blender Add-on
├─ UI
├─ Material authoring
├─ Preview
├─ Validation
├─ Profile selection
└─ Export workflow
```

V1 不需要完整 DCC UX，但必须具备确定性的 Reference 数据提取能力。

---

## 8.13 Authoring Schema

Schema namespace 是长期数据 ABI，不应跟尚未确定的 crates.io package name 自动绑定。V1 API Freeze 前必须由 `Schema Namespace ADR` 固定；以下 `bevy_npr.*` 只是工作名。

拆分 Domain：

```text
bevy_npr.material/v1
bevy_npr.character/v1
bevy_npr.rig/v1
bevy_npr.camera/v1
bevy_npr.profile.genshin/v1
```

避免：

```text
Material Schema 改动
→ 被迫升级 Rig/Post Schema
```

---

## 8.14 Authoring Data ≠ Runtime GPU Representation

DCC 中可以存在：

```text
ShadowRamp
ILM
FaceLeft
FaceRight
FaceCorrection
MetalMap
SpecRamp
PackedNormal
Emission
```

Runtime 应经过 Compiler/Prepare：

```text
Authoring Representation
        ↓
Runtime Compiler
        ↓
Optimized GPU Representation
```

---

## 8.15 Texture Semantic

所有 Texture 必须声明语义：

```text
Color
Normal
Continuous Mask
Discrete Classification
Ramp
MatCap
```

这决定：

- sRGB/Linear；
- Compression；
- Filtering；
- Mipmap；
- Packing；
- Precision。

尤其 Discrete Classification 不能因为压缩导致 Material ID 漂移。

---

## 8.16 Runtime Packing

可研究：

```text
Color Textures
├─ Base
├─ Ramp Atlas
└─ MatCap

Control Textures
├─ ILM/Control
├─ Face Atlas
├─ Packed Normal
└─ Mask/Emission
```

但不应规定“一定要合成几张纹理”。

Packing 必须由：

```text
Semantic
+
Filter
+
Precision
+
Platform Budget
+
Benchmark
```

决定。

---

## 8.17 Platform Texture Pipeline

V2 可以采用 KTX2/Basis/平台原生压缩作为候选离线 Packaging Strategy，但不能把它写成“单一万能 Runtime 格式”。

必须区分：

```text
Texture Semantic
×
Target Backend Capability
×
Bevy/wgpu Loader Capability
```

候选策略：

```text
Desktop → BC family when supported
Apple   → ASTC when appropriate
Android → ASTC / ETC2
WebGPU  → adapter/browser supported format or uncompressed fallback
WebGL2  → explicitly validated compatible path
```

### Bevy 0.19.1 限制

Bevy 0.19.1 的 glTF loader 文档明确：

```text
KHR_texture_basisu      unsupported extension syntax
KHR_mesh_quantization   unsupported
EXT_mesh_gpu_instancing unsupported
```

Bevy 可以加载 KTX2/某些压缩图片资产，并不等价于 `KHR_texture_basisu` 的 glTF extension 语法可直接使用。

因此 V1/V2 Asset Pipeline 必须明确：

- 不依赖 `KHR_texture_basisu` 作为 Bevy 0.19.1 的必需 glTF 路径；
- 不依赖 `KHR_mesh_quantization` 表达 `_NPR_*`；
- Runtime automatic instancing 使用 Bevy ECS/render batching，而不是 `EXT_mesh_gpu_instancing`；
- 所有压缩格式必须按 Color/Normal/Continuous Mask/Discrete ID/Ramp 等 Texture Semantic 分别做 Golden 与精度验证。

KTX2/Basis 是候选技术，不是架构不变量。

---

## 8.18 Vertex Attribute Contract

当前多个 `_CEL_*` Attribute 不应无限扩张。新 schema 以 `_NPR_*` 为稳定语义前缀。

候选收敛：

```text
_NPR_CONTROL0
_NPR_HAIR_TANGENT
```

但 V1 **不能凭感觉先决定 U8/Float16/量化格式**。

glTF 2.0 规定 application-specific attribute semantic 必须以 `_` 开头，并且不能使用 `UNSIGNED_INT` component type。Bevy 0.19.1 又明确不支持 `KHR_mesh_quantization`。

因此每个 Attribute 必须在 schema 中冻结：

```text
semantic name
accessor type
component type
normalized flag
Blender attribute domain/type
Bevy MeshVertexAttribute format/id
shader location/packing
quantization rule（若未来支持）
default/fallback
```

V1 优先选择 vanilla glTF + Bevy 0.19.1 已验证支持的 accessor/format，避免把 `KHR_mesh_quantization` 变成 Portable Requirement。

## 8.19 Bevy 0.19.1 glTF Import Contract

这条工作流已经能落到 Bevy 官方 API，但必须显式配置。

### Custom Vertex Attributes

Bevy 0.19.1 `GltfPlugin` 提供：

```text
add_custom_vertex_attribute(name, MeshVertexAttribute)
```

并要求 glTF attribute name 以 `_` 开头。

因此 `NprPlugin`/glTF integration 必须在 glTF loader 注册前把所有 V1 `_NPR_*` attributes 注册到 `GltfPlugin`。不能假定未知 custom semantic 会被默认保存。

### extras

Bevy 0.19.1 暴露：

```text
GltfExtras
GltfMaterialExtras
GltfMeshExtras
GltfSceneExtras
```

其内容是未经类型化的 JSON string。

因此 `bevy_npr` convention 必须自己定义稳定 JSON root，例如：

```json
{
  "bevy_npr": {
    "schema": 1,
    "profile": "genshin/v1"
  }
}
```

注意：glTF `extras` 本身没有规范化 namespace；`bevy_npr` 只是本项目 application convention。

Importer 必须：

```text
parse
→ validate schema/version
→ reject unknown required fields/profile version
→ resolve extras/sidecar precedence
→ detect conflicting duplicate metadata
→ produce typed Npr descriptors
```

V1 必须选择并文档化唯一的 metadata precedence policy。推荐原则是“一个 canonical source，另一个只允许补充或必须一致”；禁止 extras 与 sidecar 对同一 required field 给出不同值后静默择一。

正式 Runtime Contract 不允许直接在 shader/runtime 系统里到处解析 arbitrary JSON。

### 不支持的 glTF extension 不得成为 V1 前提

Bevy 0.19.1 明确未支持：

```text
KHR_mesh_quantization
KHR_texture_basisu extension syntax
EXT_mesh_gpu_instancing
```

所以 V1 importer/fixtures/CI 必须覆盖“纯 vanilla glTF + registered custom attributes + extras/sidecar”的标准路径。

若未来 Bevy 新版本新增支持，应通过 Bevy Compatibility ADR/测试升级，不应隐式改变 V1 schema。

---

## 8.20 glTF Plugin Initialization Ordering Contract（V1 必需）

Bevy `DefaultPlugins` 在启用 `bevy_gltf` feature 时已经包含 `GltfPlugin`。而 `_NPR_*` custom vertex attributes 必须通过：

```rust
GltfPlugin::add_custom_vertex_attribute(...)
```

在 glTF loader 注册配置阶段提供。

因此不能把下面这段伪装成“完整 glTF 集成必然成立”：

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(NprPlugin::default());
```

因为 `NprPlugin` 如果只在 `DefaultPlugins` 之后加入，不能假定自己还能 retroactively 改写已经配置好的 `GltfPlugin` custom attribute registry。

### V1 Facade 必须提供初始化前 Helper

目标 API 形态可以是以下之一（最终由 API ADR 决定）：

```rust
App::new()
    .add_plugins(DefaultPlugins.set(npr_gltf_plugin()))
    .add_plugins(NprPlugin::default());
```

或等价的：

```text
NprDefaultPlugins / NprGltfIntegration helper
```

要求：

1. `_NPR_*` registry 在任何 NPR glTF asset load 前完成；
2. helper 与最终 Package Name 解耦；
3. 不重复添加第二个 GltfPlugin；
4. no-gltf 用户仍可只使用 `NprPlugin`；
5. 初始化顺序有 compile/run integration test；
6. 文档不能承诺比 Bevy 真实 Plugin 初始化模型更“魔法”的行为。

### V1 接入体验目标

“基础接入 ≤10 行”仍保留，但要分成：

```text
Programmatic/No-glTF
→ DefaultPlugins + NprPlugin

glTF + _NPR_* Authoring
→ DefaultPlugins configured with NPR-aware GltfPlugin + NprPlugin
```

两条路径都必须有可复制运行的 Example。

---

# 9. 跨平台 Capability 与支持策略

## 9.0 Portable Baseline 必须分开定义

Portable 不能同时模糊表示 WebGPU、GLES3.1、WebGL2/GLES3.0。

正式 baseline：

```text
Baseline A: WebGPU Core / modern wgpu limits
Baseline B: GLES 3.1 / wgpu downlevel_defaults
Baseline C: GLES 3.0 + WebGL2 / wgpu downlevel_webgl2_defaults
```

wgpu 明确保证：`downlevel_defaults()` 对 GLES 3.1 兼容；`downlevel_webgl2_defaults()` 对 GLES 3.0/WebGL2 兼容，后者 storage buffer/storage texture 保证值为 0，per-stage UBO 保证值为 11。

V1 必须在 ADR 中明确 Web Policy：

```text
Option 1: WebGPU-only web support
Option 2: WebGPU + WebGL2 fallback
```

在该 ADR 通过前，README 不能笼统声明“Web supported”。Bevy 0.19 官方仍把 WebGPU 标记为 experimental，因此即使 NPR 自身满足 WebGPU limits，也必须把 Bevy upstream status 纳入 Support Claim。

---

## 9.1 Capability Resolver 必须分成 Renderer-Init 与 Runtime 两阶段

wgpu/Bevy 中必须区分：

```text
Adapter supports feature/limit
!=
Device actually requested/enabled feature/limit
```

Bevy 0.19 的 `WgpuSettings` 在 RenderDevice 创建前控制 backend、priority、features、limits、constrained_limits 等；RenderDevice 创建完成后，应使用 `RenderDevice::features()` / `RenderDevice::limits()` 与 `RenderAdapterInfo` 获取**实际启用**的能力。

因此架构必须分两阶段：

### Stage A：RenderDevice Creation Policy（初始化前）

```text
Application policy
+
Bevy RenderPlugin/WgpuSettings
+
Backend/Web policy
→ requested Device features/limits
```

V1 默认 NPR 插件必须是**被动兼容**的：

- 不要求用户为了 Cel/Portable 修改 WgpuSettings；
- 不在 RenderDevice 创建以后尝试“补申请” feature；
- High/Bindless 等路径只在实际 RenderDevice 已启用所需能力时选择。

如果某个未来 High feature 确实需要在 device creation 阶段显式 request，则 facade 必须提供**初始化前**的配置 helper/文档，并说明它必须与 `DefaultPlugins.set(RenderPlugin { ... })` 一起使用。普通 `.add_plugins(NprPlugin)` 不能 retroactively 修改已创建 Device。

### Stage B：Runtime Capability Resolution（初始化后）

权威输入：

```text
RenderDevice::features()
RenderDevice::limits()
RenderAdapterInfo
Compiled Capabilities
Application Quality Policy
Surface Requirements
```

输出：

```text
NprRuntimeCapabilities
      ↓
Resolved SurfaceClass × ExecutionTier Variant
```

禁止：

```text
adapter 理论支持
→ 假定 device 一定启用了该 feature
```

也禁止把 pipeline creation failure 当 feature detection。

### Test Profiles

CI/QA 应显式覆盖不同 Device Policy，例如：

```text
WebGPU-like limits
GLES3.1 downlevel limits
WebGL2/GLES3.0 limits
Functionality/high-end limits
```

并记录 Bevy/WgpuSettings priority 与实际 RenderDevice limits。

---

## 9.2 Quality Policy

默认：

```rust
NprQualityPolicy::Auto
```

同时允许：

```rust
NprQualityPolicy::Force(NprExecutionTier::Portable)
```

用途：

- QA；
- Benchmark；
- Compatibility；
- Low-spec simulation。

Reference 使用显式 Debug/QA 模式，不应被 Auto 自动选择。

---

## 9.3 确定性降级策略

每个高级 Feature 必须声明 fallback。

| Feature | High Execution / Hero Surface（按适用条件） | Portable Execution Fallback |
|---|---|---|
| Internal Edge | Depth/Normal Edge | Hull only |
| Packed Source Normal | Full reconstruction | Standard/vertex normal approximation |
| Metallic Spec Ramp | Full | Simplified specular |
| Face Correction | Full | Basic Face SDF |
| HDR Emission | HDR | LDR-safe equivalent |
| Chromatic Post | Optional | Disabled |
| Bindless | When supported/beneficial | Fixed binding |

Capability Resolver 只能选择显式注册的 fallback。

禁止：

```text
资源不够
→ 随机关掉一张 Texture
```

---

## 9.4 平台支持状态必须分层

公开 README 不应只写“Supported”。

应区分：

```text
Architecture-Compatible
Compile-Validated
Shader-Validated
Pipeline-Validated
Render-Validated
Device-Validated
Performance-Qualified
```

推荐矩阵：

| Platform/Backend | Architecture | Compile | Pipeline | Render | Device | Performance |
|---|---|---|---|---|---|---|
| Windows / D3D12 | ✓ | 状态 | 状态 | 状态 | 状态 | 状态 |
| Linux / Vulkan | ✓ | 状态 | 状态 | 状态 | 状态 | 状态 |
| Linux / GLES | ✓ | 状态 | 状态 | 状态 | 状态 | 状态 |
| macOS / Metal | ✓ | 状态 | 状态 | 状态 | 状态 | 状态 |
| iOS / Metal | ✓ | 状态 | 状态 | 状态 | 状态 | 状态 |
| Android / Vulkan | ✓ | 状态 | 状态 | 状态 | 状态 | 状态 |
| Android / GLES | ✓ | 状态 | 状态 | 状态 | 状态 | 状态 |
| Web / WebGPU | Architecture target | 状态 | 状态 | 状态 | 状态 | 状态 |
| Web / WebGL2 | Policy TBD | 状态 | 状态 | 状态 | 状态 | 状态 |
| Consoles | Port-ready only | — | — | — | — | — |

---

## 9.5 主机平台

在没有：

- 平台 SDK；
- 厂商 backend；
- 真实主机；
- 性能验证；

之前只能声明：

```text
Console-port-ready architecture
```

不能声明：

```text
Console supported
```

---

## 9.6 Supported / Experimental / Port-ready 的严格定义

公开仓库只允许三种对外状态：

### Supported

必须同时满足：

```text
Compile
+
Shader Validation
+
Pipeline Validation
+
Representative Real Runtime Render
+
Showcase Smoke Test
+
Release Regression Suite
```

对于标记为 `Supported` 的平台，不能用模拟器、交叉编译或 Pipeline 创建代替真实平台运行证据。

### Experimental

至少：

```text
Compile
+
Shader/Pipeline Validation
```

可以缺少完整 Device/Performance Coverage，但 README 必须明确限制。

### Port-ready

仅表示：

```text
Core Architecture 没有已知平台锁定
```

不代表已经存在 backend、包体、输入层或真实硬件验证。

主机在没有真实 SDK/Backend/Device 前只能是 `Port-ready`。

---

## 9.7 V1 平台发布门槛

如果 V1 对某个平台写 `Supported`，Release Checklist 必须保存至少一份可追踪的真实运行记录。

最低目标矩阵：

```text
Windows / D3D12
Linux / Vulkan
macOS / Metal
iOS / Metal
Android / Vulkan
Android / GLES（若项目声明支持 GLES）
Web / WebGPU
Web / WebGL2（仅当 Web Policy 声明支持）
```

WebGPU 还应至少覆盖发布时选定的代表性浏览器/OS 组合，并记录浏览器版本。

不要求所有这些验证都运行在 GitHub-hosted CI，但必须：

```text
Automated CI
+
Release Device Matrix
```

共同组成发布证据。

---

# 10. 测试、Golden、性能与质量验证

## 10.1 测试层级

| 测试 | 目的 |
|---|---|
| Unit | 数学、Clamp、数据转换 |
| Schema | Version、Migration |
| glTF Import | `_NPR_*` registration、extras parse、sidecar、unsupported-extension rejection |
| Plugin Init | DefaultPlugins/GltfPlugin/NprPlugin ordering and registration |
| Shader | WGSL/Naga Validation |
| Pipeline | 真实 Pipeline Creation |
| Render | Offscreen Render |
| Golden | Visual Regression |
| Resource Budget | GPU Limits |
| Feature Matrix | Cargo Feature 组合 |
| Portability | Backend/Capability |
| Performance | Regression |

---

## 10.2 Shader Test

覆盖：

```text
portable_forward
character_forward
hero_forward
reference_gacha_forward
outline
internal_edge
postprocess
```

源码 Token Contract Test 可以继续存在，但不能代替 Shader/Pipeline Validation。

---

## 10.3 Backend-Specific Pipeline Creation Test

仅做 WGSL parse/Naga validation 不够。某些错误会在 WGSL/Naga 通过后，直到特定 backend 的 GLSL/MSL/HLSL 翻译或 pipeline creation 才失败。

测试层级至少：

```text
WGSL Parse / Naga
↓
Core Pipeline Layout Validation
↓
D3D12 Pipeline
Vulkan Pipeline
Metal Pipeline
GLES Pipeline
WebGL2 Pipeline（若支持）
Browser WebGPU Pipeline
↓
Real Device Render
```

必须实际创建 BindGroupLayout、ShaderModule、PipelineLayout 和 RenderPipeline，捕获 Uniform Size、Binding Type、Stage Visibility、backend translation 与资源限制问题。

InternalEdge ABI 类错误必须在这里被阻止进入主分支。

---

## 10.4 Whole-Pipeline Resource Budget Test

对每个：

```text
SurfaceClass × ExecutionTier × Backend Baseline
```

自动统计最终 specialized pipeline 的：

- sampled textures per stage；
- samplers per stage；
- uniform buffers per stage；
- storage buffers/textures per stage；
- bind groups；
- dynamic offsets；
- vertex attributes/buffers；
- inter-stage components；
- render pass/prepass requirements。

测试对象是：

```text
View + Mesh + Instance + Material + Profile
```

而不是只统计 Material bind group。

Portable 若超对应 baseline budget：CI Fail，除非 ADR 正式改变支持范围。

---

## 10.5 Golden Render

目录：

```text
fixtures/golden/
├── portable/
├── character/
├── hero/
├── outline/
└── gacha/
```

Case：

```text
reference.png
scene.ron
camera.ron
metadata.json
```

比较指标：

- SSIM；
- RMSE；
- Max Error；
- Edge Error；
- Feature-specific probe。

---

## 10.6 跨 GPU Golden 策略

不要假设不同 GPU/Backend 的浮点结果 bit-exact。

建议两层：

### Canonical Golden

在固定受控 backend/环境生成，作为主要视觉基准。

### Cross-backend Tolerance

其它 backend：

```text
Feature-specific tolerance
+
SSIM/RMSE
```

而不是逐像素完全一致。

如果某个 Feature 必须严格离散一致，例如 Material Class/ID，则应单独做数值 Probe，而不是依赖整图容差。

---

## 10.7 Visual Metrics

建议：

- Mean Error；
- Max Error；
- SSIM；
- Edge Alignment；
- Shadow Transition Error；
- Face Threshold Error；
- Metal Highlight Error；
- Ramp Class Error。

重点区域：

```text
Body
Face
Hair
Metal
Outline
```

重点状态：

```text
Day
Sunset
Night
Motion
```

---

## 10.8 Performance Benchmark

记录：

- CPU Extract；
- CPU Prepare；
- Material Prepare；
- Instance Prepare；
- Bind Group Changes；
- Draw Calls；
- Vertex GPU；
- Fragment GPU；
- Outline GPU；
- Post GPU；
- VRAM；
- GPU resource count；
- Shader compile time。

跨机器主要比较：

```text
Regression %
```

而不是绝对毫秒数。

---

## 10.9 性能优化 PR 的证据格式

必须说明：

```text
Target Tier
Target Backend
Test Scene
Resolution
Before
After
Visual Delta
CPU Delta
GPU Delta
Resource Delta
```

例：

```text
Hero / Vulkan / 1440p / 3 characters

Fragment GPU:
1.42 ms → 1.08 ms

SSIM:
0.9987

Texture Samples:
12 → 8
```

没有数据的优化只能称为：

```text
Refactor / Theoretical Optimization
```

不能宣称性能提升。

---

## 10.10 Benchmark Scene

至少：

```text
1 Full Hero
3 Full Hero
32 Character
100 Portable
500 Portable
1000 Portable
Hero + Morph + Skin
Hero + Outline
Hero + Full Post
```

---

## 10.11 不可降级的视觉语义

Quality Tier 可以降低实现复杂度，但不能随机改变风格定义。

建议建立：

```text
docs/quality/semantic-contract.md
```

至少定义以下跨 Tier 不变量：

- Ramp 的明暗顺序不能反转；
- Material Class/ID 不能因为压缩或精度产生错误分类；
- Face 左/右光照选择不能反转；
- Silhouette Outline 在声明支持 Outline 的 Tier 中必须存在；
- Alpha/Double-sided/Culling 语义必须正确；
- Profile 定义的主色调/阴影色语义不能被 fallback 无规则替换；
- Debug View 必须与实际用于 shading 的中间量一致；
- LDR fallback 不得产生 NaN、负 HDR 溢出或明显色相翻转。

可降低：

- Internal Edge；
- Secondary specular layer；
- 高成本 packed-normal reconstruction；
- Chromatic/Post 特效；
- 高阶环境效果。

也就是说：

> **Tier 降级允许降低细节，不允许破坏角色识别和 NPR 风格的核心语义。**

---

## 10.12 Performance Target 必须外部化

架构文档不硬编码未来硬件上的毫秒目标。

仓库应维护：

```text
docs/performance/targets.md
```

其中按 Release 明确：

```text
Target Device
Resolution
Refresh/Frame Budget
Scene
Tier
CPU Budget
GPU Budget
Memory Budget
Thermal Duration（移动端）
```

例如结构：

```text
Target: Android Mid-tier Reference Device
Resolution: 1920x1080 dynamic
Scene: 100 Portable
Tier: Portable
GPU budget: <release-defined>
CPU render budget: <release-defined>
Sustained duration: 15 min
```

所有“性能最优”判断必须相对于这里定义的目标和同等视觉质量基线。

---

## 10.13 Automatic Instancing / Instance Data Regression

V1 必须有专门测试证明 Instance Data 设计没有破坏 Bevy automatic instancing/batching。

固定场景：

```text
same mesh + same material
100 / 1000 instances
unique per-instance NPR values
```

验证：

- 渲染输出每实例数据正确；
- draw/batch 行为符合预期；
- Main/Outline/Shadow 路径不会因为 Instance transport 无意变成每实体独立 material/draw；
- GLES/WebGL2 fallback（若支持）不要求 storage buffer。

---

# 11. Showcase、Examples 与 Fixtures

## 11.1 三者职责必须分开

### Examples

```text
最小 API 教程
```

### Showcase

```text
完整 Feature Demo + Manual QA + Debug + Benchmark Host
```

### Fixtures

```text
最小、稳定、可重复的自动测试资产
```

不能互相替代。

---

## 11.2 Showcase 关卡

推荐：

```text
00 Overview
01 Ramp Lab
02 Lighting Lab
03 Face SDF Lab
04 Hair Lab
05 Metal Lab
06 Outline Lab
07 Animation Lab
08 PostProcess Lab
09 Platform Stress
10 Genshin Reference
11 Performance Arena
```

---

## 11.3 Ramp Lab

测试：

- 1/2/3/5 bands；
- 256 authored ramp；
- smoothing；
- day/night atlas blend；
- light 360° rotation；
- temporal stability。

---

## 11.4 Lighting Lab

对比：

```text
Physical
Authored
Hybrid
```

控制：

- Key Light；
- Physical Influence；
- Palette；
- Environment。

---

## 11.5 Face SDF Lab

测试：

```text
Front
Left
Right
Back
Top
Bottom
```

以及：

- Generic Face；
- Gacha Face；
- Mirrored Right；
- Face Correction；
- Hard/Soft Factor。

---

## 11.6 Hair Lab

测试：

- tangent；
- shift；
- power；
- threshold；
- softness；
- mask；
- light rotation；
- camera motion；
- skinned/mirrored geometry。

---

## 11.7 Metal Lab

测试：

- MatCap；
- Specular Ramp；
- Material Class；
- Camera Rotation；
- Light Rotation；
- Packed Normal；
- Day/Night。

---

## 11.8 Outline Lab

模型：

```text
Sphere
Cube
Concave
Skinned
Morph
Hair Cards
Thin Geometry
Large/Tiny Scale
```

宽度：

```text
1/2/4/8/24 px
```

模式：

- Screen-space Hull；
- Source Local-space Hull；
- Internal Edge。

---

## 11.9 Animation Lab

测试：

- Skinning；
- Morph；
- Skin+Morph；
- Extreme Pose；
- Fast Motion；
- Mesh Swap；
- LOD；
- Bounds。

---

## 11.10 PostProcess Lab

独立控制：

- HDR；
- Bloom；
- Vignette；
- Distortion；
- Chromatic；
- Display Transform；
- Tonemap；
- Prepass。

必须显示：

```text
Resolved Camera Pipeline
```

---

## 11.11 Platform Stress

数量：

```text
100
300
500
1000
```

可切：

- Outline；
- Face；
- Hair；
- Post。

显示：

- FPS；
- CPU/GPU frame；
- Draw Calls；
- Material Count；
- Visible Instances；
- VRAM；
- Resolved Tier；
- Bindless On/Off。

---

## 11.12 Genshin Reference

固定：

- Character；
- Pose；
- Camera；
- Light；
- Exposure；
- Background；
- Resolution。

Preset：

```text
Default
Sunrise
Day
Sunset
Night
Rain
```

输出 Golden。

---

## 11.13 Fixtures

只使用：

- 自制；
- CC0；
- 明确允许再分发；

的资产。

推荐最小 fixture：

```text
toon_sphere.glb
cube.glb
concave_mesh.glb
face_sdf_head.glb
two_bone_skin.glb
morph_cube.glb
hair_strip.glb
```

不要让自动测试依赖最终 Showcase Hero 资产。

---

# 12. 工具链、版本、发布与公共仓库规则

## 12.0 Package Naming 与 License/Provenance Gate

### Package Naming

截至本次审查，`bevy_npr` 已存在公开 crate；crates.io 又禁止仅通过 `-` 与 `_` 区分包名。因此：

```text
bevy-npr
```

只能作为仓库/架构工作名，不能作为已确认 registry package 名。

在 M0/M1 必须完成 `Package Naming ADR`，检查 crates.io、docs.rs、GitHub namespace 与 Rust import ergonomics。

### License

当前 Reference 上游 Gacha Setup 及其内置 Genshin shader lineage 声明 GPL-3.0。文档不能预先宣称新仓库一定 MIT/Apache。

必须完成 `License & Provenance ADR`：

```text
Independent implementation
Reference-only behavior
Copied/transformed code
Bundled shader artifact
Bundled asset
```

逐类记录来源与许可。公开发布前应进行专门的许可证/法律审查。

---

## 12.1 CLI

V1：

```bash
<npr-cli> validate character.glb
<npr-cli> inspect character.glb
```

并必须提供可脚本化的：

```text
Blender Reference Export Bridge
```

用于固定 Blender/Gacha Setup Reference → glTF/metadata/manifest 的转换。

Validate：

- UV；
- Tangent；
- Custom Attributes；
- Face Rig；
- Texture Format；
- Profile Requirements；
- Portable Compatibility。

Inspect：

- Required Class；
- Fallback；
- Texture Count；
- Sampler Count；
- Vertex Attributes；
- Profile；
- Resource Budget。

V2：

```text
compile
optimize
golden
benchmark
```

---

## 12.2 Cargo Feature：只表达 Compile Capability，不表达 Runtime Tier

Cargo features 是 additive union；不能把它当成 runtime quality switch。

推荐 V1 Feature 主要用于可选子系统：

```toml
[features]
default = ["gltf"]

gltf = ["dep:<gltf-package>"]
profile-genshin = ["dep:<genshin-profile-package>"]
reference = ["profile-genshin"]
devtools = []
bindless-runtime = []        # compile capability only
webgl2-compat = []           # only if Web Policy accepts this target
```

如果为了最终二进制体积需要裁掉生产 SurfaceClass shader，可以额外使用 additive：

```text
runtime-character
runtime-hero
```

但运行时最终 Variant 必须始终由：

```text
Compiled Capabilities
∩ Device Capabilities
∩ Surface Requirements
∩ Runtime Quality Policy
=
Resolved Variant
```

决定。

---

## 12.3 Feature Matrix CI

必须测试：

- default features；
- no-default-features；
- minimal/default；
- runtime-character on/off；
- runtime-hero on/off；
- gltf on/off；
- profile-genshin；
- reference；
- bindless compile path。

可以使用 `cargo hack` 或等价工具自动覆盖 feature powerset 的合理子集。

---

## 12.4 Bevy 版本策略

Bevy minor 之间可能有明显 Renderer/API 变化。

因此：

```text
bevy-npr release line
→ 明确支持一个 Bevy minor 或经过验证的兼容范围
```

每次升级 Bevy 必须执行：

- API Migration；
- Shader Suite；
- Pipeline Suite；
- Golden；
- Performance；
- Examples；
- Showcase。

不承诺“同一个 release 自动支持所有 Bevy minor”。

---

## 12.5 Rust/MSRV 策略

公开仓库应明确：

```text
MSRV / Rust toolchain policy
```

可以选择：

- 跟随 Bevy MSRV；
- 或锁定 workspace toolchain。

CI 至少测试官方声明的最低 Rust 版本和当前 stable。

---

## 12.6 SemVer

`0.x`：

- 可 breaking；
- 必须 CHANGELOG；
- 必须 Migration Note。

`1.0` 后稳定：

- facade API；
- Authoring Schema；
- Profile Contract；
- 公开 Serialization Contract。

Renderer internal 类型尽量保持私有。

---

## 12.7 资产与版权

公共仓库不能直接分发未授权第三方游戏资产。

允许：

```text
公开 synthetic fixtures
+
自有 Showcase assets
+
明确许可资产
```

如果某些 Upstream Reference-Parity 测试必须依赖私有资产：

```text
Public Synthetic Suite
+
Optional Private/Local Reference Suite
```

公共 CI 不能依赖无法合法分发的文件。

必须维护：

```text
THIRD_PARTY_NOTICES.md
```

---

## 12.8 Reference 的 Shipping 边界

Reference 默认：

```text
off
```

普通发行只包含 Production Runtime。

QA：

```text
--features reference
```

避免：

- 大型 reference shader；
- debug path；
- private test asset hooks；

进入 Shipping Build。

---

## 12.9 Observability

Renderer 必须能解释自己的决策。

至少显示：

- Resolved Tier；
- Material Class；
- Lighting Backend；
- Bindless；
- Texture/Sampler/UBO Count；
- Draw Calls；
- Visible NPR Instances；
- Outline Mode；
- Internal Edge；
- HDR/Post State。

V2 增加：

- GPU timings；
- LOD distribution；
- Dynamic Quality reason；
- streaming residency。

---

# 13. 从 Tenacity 迁移的实施路线

迁移必须双轨进行：

```text
Legacy Reference Path
+
New Production Path
```

不能：

```text
先删旧 Shader
→ 凭肉眼重新实现
```

否则失去 Upstream Reference-Parity Oracle。

---

## Phase 0：Reference Freeze

1. 固定当前 Gacha/Blender Reference Shader；
2. 创建 `Reference Toolchain Lock`，锁定 Blender 精确版本、Gacha Setup release/tag/commit、release asset digest；
3. 固定 Blender glTF Export Contract；
4. 建立 Gacha Setup Upstream Behavior Contract；
5. 建立 Blender Reference Export Bridge，并让它输出 Node-Graph Signature 与 Export Manifest；
6. 修复影响 Reference 正确性的明确 bug；
7. 固定 Reference Scene、Camera、Lighting、Resolution；
8. 建立带 Provenance Metadata 的 Golden；
9. 建立 Reference Upgrade Policy；
10. 将 Reference 与 Shipping Production 解耦。

---

## Phase 1：Correctness

优先完成：

1. InternalEdge ABI；
2. Character Cull Inherit；
3. Real Pipeline Creation Test；
4. WGSL/Naga Validation；
5. Outline Lifecycle Contract；
6. Camera/Post Ownership。

---

## Phase 2：Core Extraction

建立：

```text
bevy-npr-core
```

抽离：

- Ramp；
- Face settings；
- Lighting model；
- Material descriptor；
- Instance descriptor；
- Capability；
- Quality；
- Profile contract。

这一阶段不追求 GPU 优化。

---

## Phase 3：Cel Surface + Portable Execution Baseline

实现：

```text
Cel @ Portable
```

先满足：

- minimum resource budget；
- fixed binding；
- core ramp/control；
- registered `_NPR_*` glTF attributes + typed extras/sidecar import；
- hull outline；
- basic physical/authored lighting；
- Pipeline/Golden。

这是整个新 Renderer 的地基。

---

## Phase 4：Material / Instance / View

完成：

```text
Material
Instance
View
```

分离。

迁移：

- Face Runtime State；
- Key Light；
- Global Palette；
- Day/Night；
- Outline Instance Control。

消除核心路径的 per-frame Material Asset mutation。

---

## Phase 5：Character

加入：

- Face；
- Hair；
- Rim；
- Emission；
- Enhanced Specular。

必须定义 Character @ Portable fallback/approximation。

---

## Phase 6：Hero Surface + Genshin Reference Family

加入：

- Gacha ILM；
- Authored Ramp；
- Packed Normal；
- MatCap；
- Spec Ramp；
- Face Correction；
- Gacha Palette。

同时建立：

```text
Upstream Golden → Engine Reference → Hero Production
```

Golden 对比。

---

## Phase 7：GPU Resource Architecture

执行：

- UBO packing；
- sampler dedup；
- texture budget；
- vertex control packing；
- bindless prototype；
- fixed/bindless comparison。

在这里通过 ADR 决定最终 Runtime Material Backend。

---

## Phase 8：Showcase / Tools / Docs

完成：

- Showcase labs；
- validate/inspect CLI；
- Integration docs；
- Custom Profile example；
- Migration docs。

---

## Phase 9：V1 Freeze

停止新增大 Feature。

只处理：

- correctness；
- portability；
- API cleanup；
- performance regression；
- documentation。

---

# 14. V1 / V2 / V3 路线图

## 14.1 V1：Reusable Cross-Platform NPR Library

一句话：

> **把正确的架构建立起来。**

必须有：

- independent repo；
- facade；
- core/render/gltf/profile/tools；
- Cel/Character/Hero Surface Classes；
- Portable/Standard/High Execution Tiers；
- Reference；
- Material/Instance/View；
- Capability Resolver；
- deterministic fallback policy；
- glTF authoring；
- Genshin Reference Family/Profile；
- Showcase；
- Fixtures；
- Pipeline/Golden/Budget Test；
- 基础 Benchmark；
- CLI；
- Support Matrix。

### V1 不做

- GPU-driven；
- Meshlet；
- Dynamic Quality；
- Full Asset Compiler；
- Large Editor；
- Shader Graph；
- Marketplace。

---

## 14.2 V2：Production NPR Renderer

一句话：

> **把 V1 的正确架构扩展到商业项目规模。**

### Bindless Production

成熟实现：

```text
Material ID
→ GPU Material Table
→ Resource Index
→ Bindless Arrays
```

### GPU Instance Database

统一：

- Face；
- Outline；
- Palette；
- Profile；
- LOD；
- Quality Override；
- Lighting Override。

### GPU-driven Rendering

评估：

- GPU Culling；
- Indirect Draw；
- Instance Compaction；
- GPU LOD。

必须以实际 Bevy/wgpu 能力和 Benchmark 为前提。

### NPR LOD

```text
Hero Near     → Hero @ High
Hero Mid      → Hero @ Standard
Hero Far      → Hero @ Portable
Very Far      → Profile-declared simplified variant
```

同时变化：

```text
Geometry LOD
+
Material LOD
+
Shader Feature LOD
```

### Dynamic Quality

输入：

```text
Capabilities
GPU Time
Resolution
Thermal/Battery（平台可得时）
Scene Complexity
```

输出：

```text
Tier / Feature / LOD
```

必须有：

- hysteresis；
- cooldown；
- stable transition；
- observability。

### Asset Compiler

目标：

```bash
<npr-cli> compile ...
```

生成：

- Runtime Material；
- Packed Maps；
- Platform Texture Variants；
- Profile Data；
- LOD Data。

可以复用 Bevy Asset Processor；不强制发明 `.nprpkg`。

### Advanced Outline

- depth-aware；
- normal edge；
- material edge；
- hybrid geometry/screen；
- temporal stability。

### Environment NPR

- stylized IBL；
- reflection；
- fog；
- atmosphere palette；
- stylized shadow。

### Tooling

- Material Inspector；
- Ramp Editor；
- Face Debugger；
- Outline Debugger；
- Capability Inspector；
- Performance Overlay。

### Full Regression/Performance Lab

自动 sweep：

```text
Count × Resolution × Tier × Backend × Pose × Lighting
```

---

## 14.3 V3：NPR Ecosystem / SDK

一句话：

> **让第三方围绕 Renderer 构建自己的风格和工具。**

### Stable Profile API + Versioned Schema

版本化：

```text
NprProfile API/Schema v1
```

描述：

- Material Requirements；
- Instance Data；
- View Data；
- Shader Modules；
- Fallback；
- Asset Compiler Hook；
- Editor Metadata。

### Third-party Profiles

例如：

```text
genshin
ink
comic
watercolor
```

### Declarative Profile

可研究：

```text
.profile.ron
.profile.bsn
```

描述：

- Ramp；
- Lighting；
- Face；
- Outline；
- Post；
- Fallback。

### NPR Domain Graph

V3 才考虑。

不是通用 Shader Graph。

节点限定在 NPR Domain：

- Ramp；
- Half-Lambert；
- Face；
- MatCap；
- Hair；
- Rim；
- ILM；
- Palette；
- Outline；
- Shadow。

必须支持：

- 静态 Resource Analysis；
- Portable Compatibility Analysis；
- 自动 fallback 检查。

### DCC

Blender Add-on：

- Material Authoring；
- Ramp Preview；
- Face Preview；
- Outline Preview；
- Profile；
- Export Validation；
- Platform Compatibility。

### Platform Specialization

高端可选：

- FP16；
- subgroup；
- advanced bindless；
- future mesh shader/backend feature。

Portable 标准路径永远保留。

---

## 14.4 建议 Release Roadmap

| Version | Milestone |
|---|---|
| `0.1` | Reference Baseline |
| `0.2` | Core + Cel@Portable |
| `0.3` | Instance/View + Character |
| `0.4` | Hero + Genshin Reference Family |
| `0.5` | Resource Architecture |
| `0.6` | Capability/Fallback |
| `0.7` | Pipeline/Golden/Budget CI |
| `0.8` | Showcase + Tools |
| `0.9` | API/Schema Freeze |
| **`1.0`** | V1 |
| `1.x` | Compatibility/Bugfix |
| **`2.0`** | Production Scale |
| **`3.0`** | SDK/Ecosystem |

---

# 15. V1 Release Gate 与 Definition of Done

## 15.1 API

- 用户只需依赖最终选定的单一 facade package（Package Naming ADR 后确定）；
- Programmatic/no-glTF 基础接入目标 ≤ 10 行；
- glTF/_NPR_* 接入也必须 ≤ 10 行，并通过 facade 提供的 NPR-aware `GltfPlugin` 初始化 helper 完成；
- Profile 可由第三方实现；
- Core 中不存在 Tenacity/Odette；
- Renderer internal 不成为稳定公共 API。

---

## 15.2 Architecture

必须完成：

```text
Material != Instance != View
```

以及：

```text
Surface Classes: Cel / Character / Hero
Execution Tiers: Portable / Standard / High
Validation Modes: Production / UpstreamReference / EngineReference
```

职责清晰。

---

## 15.3 Portable Execution Tier

Portable Tier 必须：

- Whole-Pipeline Resource Budget 通过；
- Fixed Binding 可用；
- 不依赖 bindless；
- 不依赖 FP16；
- 不依赖 subgroup；
- 不依赖 compute；
- 不依赖 HDR；
- 对每个 SurfaceClass 有明确 fallback；
- 若 WebGL2/GLES3.0 在支持范围，则不能要求 storage buffer，并按 wgpu `downlevel_webgl2_defaults` 预算验证。

`Cel @ Portable` 是最低完整基线；`Character/Hero @ Portable` 的保留语义由各 Profile Contract 明确规定。

---

## 15.4 Platforms

对于项目公开声明为 `Supported` 的平台，V1 必须同时具备：

```text
Compile
Shader Validation
Pipeline Validation
Real Runtime Render
Showcase Smoke Test
Release Regression Evidence
```

目标 backend：

```text
D3D12
Vulkan
Metal
GLES（若公开支持）
WebGPU
```

任何只完成 Compile/Shader/Pipeline 的目标必须标记：

```text
Experimental
```

不能写成 `Supported`。

Console 在真实 SDK/backend/device 验证完成前只能标记：

```text
Port-ready
```

---

## 15.5 Quality

必须首先具备完整 Reference Provenance：

- Blender 精确版本锁；
- Gacha Setup release/tag/commit/digest 锁；
- Blender Export Contract；
- Genshin Reference Family Upstream Contract；
- Golden Provenance Metadata；
- Reference Upgrade Policy。

并拥有：

- Generic Golden；
- Upstream Blender Golden；
- Engine Genshin Reference Golden；
- Face；
- Hair；
- Metal；
- Outline；
- Lighting；
- Post；

的可重复验证。

---

## 15.6 Performance

至少固定：

```text
1 Hero
100 Portable
500 Portable
1000 Portable
```

Benchmark。

每个平台建立自己的 baseline，而不是全世界共享一个绝对毫秒指标。

---

## 15.7 Tools

至少：

```text
<npr-cli> validate
<npr-cli> inspect
Blender Reference Export Bridge
```

Reference Export Bridge 必须支持无交互/批处理运行，并在上游 Gacha Setup 结构与锁定 Contract 不一致时 fail closed。

---

## 15.8 Showcase

至少：

```text
Overview
Ramp
Lighting
Face
Hair
Metal
Outline
Animation
Post
Stress
Reference
Performance
```

---

## 15.9 Definition of Done

任何 Feature 只有同时满足：

```text
API
+
GPU ABI
+
Fallback
+
Tests
+
Showcase
+
Documentation
+
Performance Evidence
```

才算 Production Ready。

例如新增 Hair Feature 必须回答：

- 属于哪个 Class？
- Portable fallback？
- 增加几个 binding？
- 增加几个 sample？
- 是否影响 bindless？
- 是否影响 vertex format？
- Golden fixture？
- Showcase？
- Backend compatibility？
- Benchmark cost？

---

# 16. 架构不变量、风险与 ADR

## 16.1 Architecture Invariants

以下规则应长期保持：

1. Renderer ≠ Art Style。
2. Authoring Data ≠ Runtime GPU Representation。
3. Material ≠ Instance ≠ View。
4. Surface Class ≠ Execution Tier ≠ Reference Mode。
5. Upstream Reference ≠ Engine Reference ≠ Production Renderer。
6. Portable Execution Tier 不依赖可选 GPU Feature。
7. High Path 必须有明确 fallback。
8. Disabled Feature 应尽可能接近 Zero GPU Cost。
9. Capability 必须在 Pipeline 创建前 Resolve。
10. Core 不允许 Game-specific 数据。
11. 优化必须有视觉与性能证据。
12. GPU Resource Budget 是 **whole-pipeline per-stage** Contract。
13. Examples ≠ Showcase ≠ Fixtures。
14. WGSL Token/Naga Test 不能替代 backend-specific Pipeline Validation。
15. Runtime 不依赖 Editor/Tooling/Blender/Gacha Setup。
16. 公开 Support Claim 必须与实际验证等级对应。
17. Reference-only 资源默认不能进入 Shipping Build。
18. Tier 降级只能降低细节，不能破坏定义好的视觉语义。
19. Reference Golden 必须可追溯到精确 Blender/Gacha Setup/具体 Shader Artifact/Exporter/Profile 版本。
20. 上游 `latest` 永远不是稳定 Golden Contract；升级必须通过 Reference Migration。
21. glTF 只承载明确支持的 Runtime Data；复杂 Blender Node Graph 不是隐式 Runtime ABI。
22. Reference 数据提取必须脚本化、版本化、fail closed。
23. Cargo Feature 只表达 compile capability；Runtime Variant 由 Resolver 决定。
24. Rust Profile 扩展承诺 Stable API/Schema，不虚构未设计的稳定 Rust ABI。
25. Package name 与 License 必须由 ADR/现实 registry/provenance 审查决定。
26. V1 glTF Contract 不依赖 Bevy 0.19.1 明确未支持的 `KHR_mesh_quantization` / `KHR_texture_basisu` / `EXT_mesh_gpu_instancing`。
27. Runtime Resolver 只能依据实际 RenderDevice 已启用能力；需要额外 feature/limit 的路径必须在 RenderDevice 创建前配置。
28. `_NPR_*` glTF attributes 必须在 `GltfPlugin` loader 配置阶段注册；NprPlugin 不得假装能在事后自动补注册。
29. Golden 必须记录 canonical Blender render environment；extras/sidecar 冲突不得静默解析。
30. Schema namespace 是稳定数据 Contract，不跟 crates.io package name 隐式联动。

---

## 16.2 PR Review Checklist

### Architecture

- 是否把 Instance Data 放进 Material？
- 是否把 View Data 放进 Material？
- 是否把 Profile-specific 数据放进 Core？
- 是否破坏单向依赖？

### GPU

- 是否增加 Binding？
- 是否增加 Texture Sample？
- 是否增加 Vertex Attribute？
- 是否增加 Pass？
- 是否超对应 baseline 的 Whole-Pipeline Budget？
- Disabled 状态是否仍执行无意义 GPU work？

### Platform

- 是否要求新 GPU Feature？
- fallback 是什么？
- WebGPU/GLES3.1/WebGL2 baseline（按支持范围）是否仍满足？
- GLES/D3D12/Vulkan/Metal 是否受影响？

### Validation

- Unit？
- Shader？
- Pipeline？
- Golden？
- Showcase？
- Benchmark？
- Docs？

---

## 16.3 主要风险

| 风险 | 后果 | 控制措施 |
|---|---|---|
| 过早重写 Reference | 丢失视觉正确性 | Reference Freeze + Golden |
| 过度 Crate 化 | API/依赖复杂 | V1 只保留 6 个主要 crate |
| Shader permutation 爆炸 | 编译时间/缓存膨胀 | Coarse Material Class |
| Portable 被 Hero Feature 污染 | Web/Mobile 失败 | Resource Budget Gate |
| 过度追求 bindless | 低端不兼容 | Fixed + Bindless 双路径 |
| Asset packing 破坏数据语义 | Material ID/Face 错误 | Texture Semantic |
| 跨 GPU Golden 过严 | CI 不稳定 | Canonical + Tolerance |
| “理论优化”无实际收益 | 增加复杂度 | Benchmark Evidence |
| Bevy API 快速变化 | 维护成本 | Compatibility Matrix + ADR |
| 未授权资产进入公开仓库 | 版权/发布问题 | Synthetic/Public Fixtures |
| V2 功能提前进入 V1 | 项目失控 | Feature Freeze |
| SurfaceClass 与 Tier 混用 | Resolver/LOD 语义错误 | 三轴模型 |
| Instance transport 破坏 batching | CPU/draw-call 回退 | Instance Transport ADR + batching test |
| Engine Reference 自证 | 翻译错误无法发现 | Upstream Golden 三层验证 |
| Cargo 包名冲突 | 无法按计划发布 | Package Naming ADR |
| License provenance 不清 | 发布/合规风险 | License & Provenance ADR |
| Golden 依赖未记录的 Blender/GPU 环境 | Reference 漂移 | Canonical Reference Environment |
| extras/sidecar 双源冲突 | 同一资产行为不确定 | Metadata precedence contract |

---

## 16.4 ADR 列表

至少建立：

```text
docs/adr/
├── 0001-runtime-material-backend.md
├── 0002-whole-pipeline-resource-budget.md
├── 0003-bindless-strategy.md
├── 0004-material-instance-view.md
├── 0005-instance-data-transport.md
├── 0006-surface-class-vs-execution-tier.md
├── 0007-profile-reference-family-boundary.md
├── 0008-reference-three-layer-validation.md
├── 0009-texture-packing.md
├── 0010-outline-architecture.md
├── 0011-lighting-backend.md
├── 0012-render-path-alpha-prepass.md
├── 0013-web-webgpu-webgl2-policy.md
├── 0014-render-device-creation-policy.md
├── 0015-platform-support-policy.md
├── 0016-reference-toolchain.md
├── 0017-blender-export-contract.md
├── 0018-upstream-reference-upgrade.md
├── 0019-reference-vs-runtime-artifact-boundary.md
├── 0020-package-naming.md
├── 0021-license-provenance.md
├── 0022-gltf-plugin-initialization.md
└── 0023-schema-namespace-and-metadata-precedence.md
```

每个 ADR 至少记录：Context、Options、Decision、Evidence、Consequences、Revisit Trigger。

其中 Runtime Material Backend、Instance Transport、Web Policy、Package Name 与 License 都必须在 V1 API Freeze 前作出有证据的决定。

---

## 16.5 当前 Reference Upstream Snapshot（非永久 Contract）

截至本文本次修订（2026-09-11）：

```text
Gacha Setup latest stable release:
v3.6.2

Published:
2026-09-09

Tag commit:
069e51f3fd89cdc66b4c2741798ede11a5fb16b5

Release asset:
Gacha_Setup-v3.6.2.zip

Release asset SHA-256:
f5ab8206464130962234a7ccab19c7d212a048c848ce3e69518f84fc41a39999

Gacha Setup stated Blender target:
Blender 5.2+

Current Blender LTS candidate at document review time:
Blender 5.2.1 LTS (2026-08-25)
Formal baseline remains TBD until Reference Validation passes.
Canonical Blender render environment (OS/GPU backend/device/driver where relevant): TBD and must be locked with the baseline.

V1 Genshin Reference Family artifacts at the locked commit:
Main: setup_wizard/shaders/gi/HoYoverse - Genshin Impact v3.4.blend
Git blob: 7637f1d18a63188ca08870fb554d949dafefaecf

Outline: setup_wizard/shaders/gi/HoYoverse - Genshin Impact Outlines v3.blend
Git blob: 532e0860065acea1e427f1d4033f69626793036b

Post: setup_wizard/shaders/gi/HoYoverse - Genshin Impact Post-Processing.blend
Git blob: 7cafd3d5fcb77e5e7efa8037531e085887f57d5e

Lineage:
Festivity / Blender-miHoYo-Shaders (GPL-3.0)

Accuracy scope:
Upstream Reference-Parity; upstream shader author explicitly does not claim 100% in-game accuracy.
```

官方/上游入口：

- Gacha Setup Repository: https://github.com/PaoloESAN/gacha-setup
- Gacha Setup Releases: https://github.com/PaoloESAN/gacha-setup/releases
- Blender 5.2 LTS glTF Manual: https://docs.blender.org/manual/zh-hans/5.2/addons/scene_gltf2.html

注意：

> 这只是文档修订时的 upstream snapshot。只有通过 Reference Validation 后，某个具体版本组合才能成为项目的正式 Golden Baseline。



## 16.6 本次深度检视的权威依据

关键事实必须以官方规范/上游源码为准。当前主要依据：

- Cargo registry naming restrictions: https://doc.rust-lang.org/cargo/reference/registry-index.html
- Existing `bevy_npr` crate: https://docs.rs/bevy_npr/latest/bevy_npr/
- Rust layout/ABI guarantees: https://doc.rust-lang.org/reference/type-layout.html
- glTF 2.0 application-specific attributes: https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html
- Blender glTF custom properties/attributes: https://docs.blender.org/manual/en/5.2/addons/scene_gltf2.html
- wgpu limits / GLES/WebGL2 baselines: https://docs.rs/wgpu-types/latest/wgpu_types/struct.Limits.html
- Bevy 0.19.1 automatic instancing and MeshTag: Bevy v0.19.1 `examples/shader/automatic_instancing.rs`
- Bevy 0.19.1 glTF custom attributes/extras/extensions: `crates/bevy_gltf/src/lib.rs`, `loader/mod.rs`, `assets.rs`
- Bevy DefaultPlugins/GltfPlugin integration: Bevy `DefaultPlugins` includes `GltfPlugin`; use PluginGroup configuration (`set`) for loader-time custom attributes
- Bevy 0.19.1 GPU array buffers: Bevy v0.19.1 `bevy_render::render_resource::GpuArrayBuffer`
- Bevy 0.19.1 WebGPU/WebGL examples/status: Bevy v0.19.1 examples README
- Bevy 0.19.1 renderer initialization/features/limits: `bevy_render::settings::WgpuSettings` and `WgpuSettingsPriority`
- Gacha Setup: https://github.com/PaoloESAN/gacha-setup and Releases
- Genshin reference lineage: https://github.com/festivities/Blender-miHoYo-Shaders

这些 URL/版本不是“永远不变的 truth”；正式 Release 时应将已验证版本写入 Toolchain Lock/Support Matrix。

---

# 17. 最终结论

现有 Tenacity NPR 实现最大的价值，不是它已经是最终通用 Renderer，而是它已经提供了：

- 丰富的 NPR Algorithm；
- 很强的 Blender/Gacha Upstream Reference-Parity；
- 一个非常有价值的 Reference Renderer；
- Face / Hair / Metal / Outline / Post 的完整验证表面；
- 可迁移的 Authoring/Metadata 基础。

下一阶段不应该继续向当前 `CharacterExtension` 纵向添加功能。

应该进入第二代架构：

```text
Renderer-independent NprMaterialDescriptor
+
Benchmark-selected Runtime Backend
+
Cel / Character / Hero Surface Classes × Portable / Standard / High Execution Tiers
+
Material / Instance / View Separation
+
Generic Core + Reference-Family Profiles
+
Upstream Reference / Engine Reference / Production Three-Layer Validation
+
Capability Resolver
+
Fixed Binding / Bindless Dual Path
+
Resource Budget Contract
+
Golden / Pipeline / Feature Matrix / Benchmark CI
+
Independent GitHub Repository
```

版本目标：

```text
V1
→ 正确、可复用、跨平台、可验证的 NPR Library

V2
→ 面向商业项目规模的 Production Renderer

V3
→ Third-party Profile / Tooling / DCC NPR SDK Ecosystem
```

最终判断标准不是“代码看起来高级”，而是：

```text
Correctness
+
Visual Fidelity
+
Measured Performance
+
Whole-Pipeline Portable Resource Budget
+
Clear Fallback
+
Stable Public API
+
Repeatable Validation
```

只有所有这些同时成立，才接近本项目所定义的：

> **完美、可复用、跨全平台、效果与性能最优。**
