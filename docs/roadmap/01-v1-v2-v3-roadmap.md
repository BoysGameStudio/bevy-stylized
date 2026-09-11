# V1 / V2 / V3 路线图

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


## V1：Reusable Cross-Platform NPR Library

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

## V2：Production NPR Renderer

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
bevy-stylized compile ...
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

## V3：NPR Ecosystem / SDK

一句话：

> **让第三方围绕 Renderer 构建自己的风格和工具。**

### Stable Profile API + Versioned Schema

版本化：

```text
StylizedProfile API/Schema v1
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

## 建议 Release Roadmap

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
