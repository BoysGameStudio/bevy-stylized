# 当前实现的对抗性检视

> **Review provenance**
>
> - Repository: `BoysGameStudio/Tenacity`
> - Branch under review: `ink`
> - Bevy baseline discussed: `0.19.1`
> - Exact reviewed commit: **not captured in the original review; must be recorded before treating individual findings as current-code facts**
> - Scope: the reviewed `bevy-cel` / NPR implementation snapshot used during the architecture review
>
> Findings in this document are snapshot-dependent. Once the implementation changes, re-run the review against a recorded commit and mark resolved findings explicitly.


> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


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

## 严重度总表

| 级别 | 问题 | 结论 |
|---|---|---|
| **P0** | InternalEdge Vertex/Material Uniform ABI 不一致 | 必须立即修 |
| **P0/P1** | 当前完整 CharacterMaterial 无法保证满足最低 whole-pipeline resource budget | WebGPU/GLES baseline 阻塞风险 |
| **P0/P1** | Surface Class 与 Execution Tier 混用 | 必须先修概念模型再实现 Resolver/LOD |
| **P1** | `StylizedInstanceGpu` 只有逻辑模型，没有 Bevy 0.19 batching-compatible transport contract | V1 必须增加正式 ADR/测试 |
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

## P0：InternalEdge Shader ABI

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

## P1：Character Cull Override

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

## P0/P1：GPU Resource Budget

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

## Bindless 路径被阻断

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

## Face dynamic state 所处层级错误

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

## Ramp / ILM / Face 的重复采样

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

## Outline Replica 生命周期

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

## Internal Edge 的算法问题

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

## PostProcess Ownership

目前多个逻辑可能共同控制：

- HDR；
- Bloom；
- Tonemapping；
- DepthPrepass；
- NormalPrepass。

最终不能让多个独立组件各自随意 insert/remove 同一个 Camera 状态。

应该由：

```text
StylizedCameraPipeline Resolver
```

统一决定最终 Camera Render Requirements。

---

## Gacha/Odette 数据进入 Core

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

## 测试盲区

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
