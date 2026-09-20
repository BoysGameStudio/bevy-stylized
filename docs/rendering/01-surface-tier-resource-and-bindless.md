# Surface Class、Execution Tier、GPU 资源预算与 Bindless

> **Status:** Normative architecture specification


## Production Surface Classes

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

## Execution Tiers

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

## Whole-Pipeline Resource Budget

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

## Uniform Packing

CPU API 可以保持模块化 settings；GPU Prepare 按 SurfaceClass × ExecutionTier pack 成少量合理的 GPU buffers。禁止回到“每个 Feature 一个 UBO”。

## Sampler Deduplication

按 sampler state 分类共享，目标通常是 Nearest / Linear / LinearClamp 等少数状态，而不是每张纹理一个 sampler。具体数量由 backend/prototype 测量决定。

## Fixed Binding + Bindless

```text
Portable / unsupported
→ Fixed Binding

High-end capable & measured beneficial
→ Bindless
```

Bindless 是优化路径，不是语义和运行前提。

## Resource Budget Test 必须从最终 Pipeline Layout 反射/统计

CI 不能只统计 `StylizedMaterial` 自己声明了几张纹理。必须在真实 specialization 后统计最终 pipeline 对 vertex/fragment stages 的资源可见性，并与对应 baseline limits 比较。

---
