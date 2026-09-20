# 测试、Golden、性能与质量验证

> **Status:** Normative architecture specification


## 测试层级

| 测试 | 目的 |
|---|---|
| Unit | 数学、Clamp、数据转换 |
| Schema | Version、Migration |
| glTF Import | `_NPR_*` registration、extras parse、sidecar、unsupported-extension rejection |
| Plugin Init | DefaultPlugins/GltfPlugin/StylizedPlugin ordering and registration |
| Shader | WGSL/Naga Validation |
| Pipeline | 真实 Pipeline Creation |
| Render | Offscreen Render |
| Golden | Visual Regression |
| Resource Budget | GPU Limits |
| Feature Matrix | Cargo Feature 组合 |
| Portability | Backend/Capability |
| Performance | Regression |

---

## Shader Test

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

## Backend-Specific Pipeline Creation Test

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

## Whole-Pipeline Resource Budget Test

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

## Golden Render

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

## 跨 GPU Golden 策略

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

## Visual Metrics

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

## Performance Benchmark

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

## 性能优化 PR 的证据格式

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

## Benchmark Scene

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

## 不可降级的视觉语义

Quality Tier 可以降低实现复杂度，但不能随机改变风格定义。

正式语义契约：

[`docs/quality/semantic-contract.md`](../quality/semantic-contract.md)

核心跨 Tier 不变量包括：

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

## Performance Target 必须外部化

架构文档不硬编码未来硬件上的毫秒目标。

正式性能目标格式：

[`docs/performance/targets.md`](../performance/targets.md)

按 Release/Target 明确：

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

## Automatic Instancing / Instance Data Regression

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
