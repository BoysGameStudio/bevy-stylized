# Shader、Lighting、Face、Outline 与 PostProcess

> **Status:** Normative architecture specification


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

## Independent reference evidence

Current upstream validation uses Blender/Gacha captures directly. Production
regression uses approved first-party baselines and independent semantic fixtures.
The former migration renderer has been retired; it is not a current prerequisite.
See [the active reference workflow](../reference/01-reference-authoring-workflow.md).


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

Hull 副本在 PostUpdate 的引擎动画求值之后、transform propagation 和 bounds
计算之前同步源 mesh、材质 handle、skin 与 morph 组件，并移除已经不存在的
变形组件。源 mesh 或 outline 组件移除时清理副本。自定义 PostUpdate 输入系统
需要显式排在 `sync_outline_replicas` 之前。此同步只消费外部求值状态，不控制动画。
已有连续 morph GPU 回归不等价于完整的动态 bounds、LOD 或所有 prepass 验证；
ADR-0008 仍未关闭。

## Alpha / Transparency / Shadow / Prepass Matrix（V1 必需）

Bevy 0.19 Material 支持 Opaque、Mask、Blend、Premultiplied、AlphaToCoverage、Add、Multiply；同时 Bevy prepass 只处理 opaque 与 alpha-mask，透明材质会被忽略。

因此 V1 必须维护并用测试填实以下矩阵，而不是默认“所有材质都支持”：

| Surface | Main NPR | Shadow | Depth/Normal Prepass | Hull Outline | Internal Edge |
|---|---|---|---|---|---|
| Opaque | Native Cel/Character/Hero; GPU covered | Off by default; Hero casting opt-in via `shadow-casting` feature — runtime-qualified 2026-09-22 (A/B pixel evidence: shadow lands on a StandardMaterial receiver, caster and non-feature materials unaffected); receiving absent in authored shaders | Camera opt-in; geometric normals, complete skin vertex path | Native/host hull; GPU covered | Standard+ depth/normal screen effect; native GPU covered |
| Mask | Native Cel/Character/Hero; texture × factor alpha cutoff | Opt-in path shares cutout shader; masked casting not runtime-qualified | Native depth-only and depth+normal cutout regression; host shader composition covered | Hull follows pre-discard geometry by construction (runtime-qualified 2026-09-22): cutout regions are solidly hull-filled, never a floating rim; cutout-faithful hull matching not supported | Cutout content follows post-discard prepass (verified); overlay draws an emphasized boundary band around interior cutouts (~15px at width 1.5, weakly width-scaling, mechanism unpinned) — final masked edge appearance NOT qualified |
| AlphaToCoverage | No native/host selector | Not qualified | Not qualified | Not qualified | Not qualified |
| Blend | Explicit HostMaterial or pupil-highlight Hero path; scoped GPU cases | Not qualified | Excluded by engine's transparent prepass routing | Host opaque shell is not general transparency matching | No transparent depth/normal evidence |
| Premultiplied | No current selector | Not qualified | N/A by engine routing | Not qualified | Not qualified |
| Add/Multiply | No current selector | Not qualified | N/A by engine routing | Not qualified | Not qualified |

Hair cards、半透明眼睛/饰品、特效材质必须通过这张 Contract 表声明真实能力。

---

## Genshin AO regression

The explicit Hero Genshin ramp mode consumes ILM.G directly in the existing
` smoothstep(0.01, 0.4, ao) ` lighting gate. Mapping G to [0.5,1] made that gate
always fully lit and lost authored occlusion. Generic modes keep their previous
AO mapping. `genshin_ilm_zero_ao_preserves_shadow_gate` supplies a zero/full-AO
texture pair to the GPU and requires a visible lighting difference; it is part
of the required suite. Hero asset-event consistency also includes its inherited
ILM and Face SDF presence flags. This fixes the scoped behavior, not complete
upstream material equivalence.

### Externally updated bounds

Outline replicas follow producer AABB changes even when the mesh handle is stable.
When source bounds are removed, mirrored replica bounds are removed so automatic
calculation can resume. The external deformation producer owns source-bound
correctness; this synchronization does not implement deformation or physics.

### Native instance width and post ownership (2026-09-21)

Native hull replicas consume `StylizedInstanceData.outline_scale` independently of
the source's face/profile/table tag. Non-default width is clamped to [0,2] and uses
the native seven-bit scale precision; absent data and exactly 1 retain exact unit
width, including baked source shell positions. Removing the component restores
unit width. The replica's internal tag is not a public ABI or a StorageTable slot.
Host outlines continue to use the host record. Shared outline material assets are
not mutated by either path. The required `morph_skinning` suite checks zero/double
width, removal and exact restoration, plus two independent head poses sharing a
Hero material.

Attaching `StylizedPostProcess` to a native Camera3d creates its single fullscreen
effect. Global disable or all-zero effects remove the effect component and its
pass; removal of the controller restores a pre-existing manual effect, if any.
`ExternalPostProcess` still prevents native registration. The required
`post_lifecycle` GPU test checks rendered enable/disable/removal and exact pixel
restoration. The implemented effects are vignette, chromatic aberration and the
display-transform hook; lens distortion in the design list is not implemented.


### Mask prepass regression (2026-09-21)

The old type-level prepass enable flag did not preserve alpha cutouts: its default
fragment path wrote depth for transparent texels, hiding objects behind the mask.
The shared complete-skin prepass now imports each class's actual material prefix
and diffuse bindings and reuses alpha factor/cutoff and Hero cubic/backface-UV
rules. A depth-only mask has a fragment entrypoint without color outputs; normal
prepass keeps the original geometric-normal encoding. Shader imports are retained
as app-owned handles and released at app teardown.

`alpha_probe` is now registered in the required GPU lane. It places a green surface
behind the mask and compares main-pass RGBA with depth-only and depth+normal enabled
for all three native classes. Both it and `outline_advanced` wait for consecutive
ready render frames and fail on pipeline errors/timeouts; a fixed update count
could otherwise capture before an imported prepass shader was available. CPU Naga
coverage also composes native/host class variants. This does not qualify every
shadow, transparency or masked-outline combination in the table above.
