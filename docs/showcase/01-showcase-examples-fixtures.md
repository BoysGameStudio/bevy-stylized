# Showcase、Examples 与 Fixtures

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


## 三者职责必须分开

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

## Showcase 关卡

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

## Ramp Lab

测试：

- 1/2/3/5 bands；
- 256 authored ramp；
- smoothing；
- day/night atlas blend；
- light 360° rotation；
- temporal stability。

---

## Lighting Lab

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

## Face SDF Lab

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

## Hair Lab

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

## Metal Lab

测试：

- MatCap；
- Specular Ramp；
- Material Class；
- Camera Rotation；
- Light Rotation；
- Packed Normal；
- Day/Night。

---

## Outline Lab

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

## Animation Lab

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

## PostProcess Lab

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

## Platform Stress

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

## Genshin Reference

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

## Fixtures

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
