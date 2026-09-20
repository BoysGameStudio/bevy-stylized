# Authoring Schema

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


Schema namespace 是长期数据契约，采用项目稳定品牌 namespace `bevy_stylized.*`。它的版本独立于 Cargo package 版本：未来 crate 升级不能隐式改变既有 schema 语义。

拆分 Domain：

```text
bevy_stylized.material/v1
bevy_stylized.character/v1
bevy_stylized.rig/v1
bevy_stylized.camera/v1
bevy_stylized.profile.genshin/v1
```

以上为领域划分，不能据此声称每个 namespace 都有已实现的运行时契约。
其中 `rig` 仅允许表达渲染需要的绑定或参考求值元数据；它不授权新增动画图、
IK、重定向、表情控制或物理解算 schema。实际交付以已有版本化 schema 和
实现为准，遵守[渲染职责边界](../architecture/01-goals-scope-and-three-axis-model.md)。

避免：

```text
Material Schema 改动
→ 被迫升级 Rig/Post Schema
```

---

## Authoring Data ≠ Runtime GPU Representation

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

## Texture Semantic

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

## Runtime Packing

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

## Platform Texture Pipeline

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

## Vertex Attribute Contract

Current authored attributes use the canonical `_NPR_*` semantic prefix.

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

## Source-derived schema snapshots

Runtime schemas under schemas/runtime are generated from the Rust descriptors
through the optional schema feature (Schemars 1.2.1). Authoring attribute contracts
under schemas/authoring generate the pure glTF table; the Bevy adapter is verified
against the same table. Generated files are checked, not independently hand-edited.
