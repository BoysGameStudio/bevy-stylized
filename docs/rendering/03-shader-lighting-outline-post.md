# Shader、Lighting、Face、Outline 与 PostProcess

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


## Shader Entry Point

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

## Shader Permutation Policy

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

## Lighting Backend

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

## Engine Reference Renderer

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

## Ramp Fast Path

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

## ILM Decode Once

单 fragment 生命周期中：

```text
Decode ILM Once
↓
Reuse
```

Body、Specular、Emission、Debug 共享结果。

---

## Face Sampling

选择 Side 后只采一张目标 SDF。

Face Auxiliary/Correction 尽量单次采样并共享。

---

## Outline

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

## Internal Edge

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

## PostProcess

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

## Camera Pipeline

新增统一 Resolver：

```text
StylizedCameraPipeline
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

## HDR

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

## Rendering Path Contract（V1 必需）

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

## Alpha / Transparency / Shadow / Prepass Matrix（V1 必需）

Bevy 0.19 Material 支持 Opaque、Mask、Blend、Premultiplied、AlphaToCoverage、Add、Multiply；同时 Bevy prepass 只处理 opaque 与 alpha-mask，透明材质会被忽略。

因此 V1 必须维护并用测试填实以下矩阵，而不是默认“所有材质都支持”：

| Surface | Main NPR | Shadow | Depth/Normal Prepass | Hull Outline | Internal Edge |
|---|---|---|---|---|---|
| Opaque | 🚧 ADR-0023 | 🚧 ADR-0023 | 🚧 ADR-0023 | 🚧 ADR-0023 | 🚧 ADR-0023 |
| Mask | 🚧 ADR-0023 | 🚧 ADR-0023 | 🚧 ADR-0023 | 🚧 ADR-0023 | 🚧 ADR-0023 |
| AlphaToCoverage | 🚧 ADR-0023 | 🚧 ADR-0023 | 🚧 ADR-0023 | 🚧 ADR-0023 | 🚧 ADR-0023 |
| Blend | 🚧 ADR-0023 | 🚧 ADR-0023 | N/A by Bevy prepass default | 🚧 ADR-0023 | N/A or explicit alternative |
| Premultiplied | 🚧 ADR-0023 | 🚧 ADR-0023 | N/A by Bevy prepass default | 🚧 ADR-0023 | N/A or explicit alternative |
| Add/Multiply | 🚧 ADR-0023 | 🚧 ADR-0023 | N/A by Bevy prepass default | Profile-specific contract | N/A or explicit alternative |

Hair cards、半透明眼睛/饰品、特效材质必须通过这张 Contract 表声明真实能力。

---
