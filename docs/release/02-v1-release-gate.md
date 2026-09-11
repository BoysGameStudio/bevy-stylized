# V1 Release Gate 与 Definition of Done

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


## API

- 用户只需依赖单一 facade package：`bevy-stylized`（Rust identifier：`bevy_stylized`）；
- Programmatic/no-glTF 基础接入目标 ≤ 10 行；
- glTF/_NPR_* 接入也必须 ≤ 10 行，并通过 facade 提供的 Stylized-aware `GltfPlugin` 初始化 helper 完成；
- Profile 可由第三方实现；
- Core 中不存在 Tenacity/Odette；
- Renderer internal 不成为稳定公共 API。

---

## Architecture

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

## Portable Execution Tier

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

## Platforms

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

## Quality

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

## Performance

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

## Tools

至少：

```text
bevy-stylized validate
bevy-stylized inspect
Blender Reference Export Bridge
```

Reference Export Bridge 必须支持无交互/批处理运行，并在上游 Gacha Setup 结构与锁定 Contract 不一致时 fail closed。

---

## Showcase

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

## Definition of Done

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
