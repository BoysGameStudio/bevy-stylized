# Blender Export Contract

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


必须建立：

```text
docs/reference/03-blender-export-contract.md
```

它是 Blender → glTF → bevy-stylized 的稳定边界。

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
- bevy-stylized；

中的哪一层。

---
