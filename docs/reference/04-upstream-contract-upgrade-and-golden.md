# Genshin Reference Family / Gacha Setup Upstream Contract

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


Gacha Setup 不应作为“黑盒神奇插件”。

`profile-genshin` 必须明确记录它从上游 Reference 中复现哪些行为。

建议建立：

```text
docs/reference/04-upstream-contract-upgrade-and-golden.md
```

按功能记录：

| Reference Area | Upstream Behavior | bevy-stylized Representation |
|---|---|---|
| Shadow Ramp | Blender/Gacha material graph | Ramp/Profile data |
| ILM/Lightmap | Channel/class semantics | Control texture decode |
| Face | Direction transfer + threshold maps | Face SDF |
| Normal | Source packed normal behavior | Hero/Profile normal path |
| Metal | MatCap/specular behavior | Hero metallic module |
| Outline | Source outline classes/width | Outline profile |
| Lighting | Source key light/palette | Authored Lighting backend |
| Post | Source display/compositor behavior | Reference/Post profile |

每个行为必须标记：

```text
Exact
Approximation
Intentional Difference
Unsupported
```

对每个 `Exact/Approximation` 项，还必须记录它的输入来源：

```text
Texture Channel
Vertex Attribute
Object/Material Custom Property
Profile Constant
View/Frame State
Derived Instance State
```

这样可以防止 Runtime Shader 反向依赖 Blender Node Name、Socket Index 或某个未版本化的节点拓扑。

Reference 工具可以读取/检查 Blender/Gacha Setup 的节点结构，但最终 Runtime Contract 必须落到稳定的 `bevy-stylized` 语义上。


这样 Production Renderer 的目标不是模糊的“看起来像 Genshin”，而是有明确 Contract。

---

## Reference Baseline Upgrade Policy

更新 Blender 或 Gacha Setup 属于**受控 Reference Migration**。

禁止：

```text
dependency update
→ Golden update
→ merge
```

正确流程：

```text
1. Open Reference Upgrade PR
2. Update toolchain lock
3. Regenerate Blender reference export
4. Run bevy-stylized validator
5. Render old Reference implementation
6. Render new upstream-derived Reference
7. Generate semantic/visual diff
8. Classify changes
9. Update Profile Contract if needed
10. Optimize Production path
11. Update Golden only after approval
```

变化分类：

```text
A. Upstream Bug Fix
B. Upstream Intentional Visual Change
C. Blender Export Behavior Change
D. Gacha Setup Material/Node Change
E. bevy-stylized Importer Bug
F. Production Renderer Regression
```

Golden 不能成为“测试失败以后随手更新”的文件。

---

## Golden Provenance

每张 Reference Golden 必须能够追溯：

```text
Production/Engine Golden
   ↓
Upstream Blender Golden
   ↓
Reference Fixture
   ↓
Reference Manifest / Node-Graph Revision
   ↓
Export Artifact Hash
   ↓
Blender Version
   ↓
Canonical Blender Render Environment
   ↓
Gacha Setup Version/Commit/Digest
   ↓
Profile Contract Version
   ↓
Reference Renderer Commit
```

建议每个 Golden 附带：

```text
<case>.golden.json
```

例如：

```json
{
  "reference_toolchain": "gacha-blender-v1",
  "blender": "<exact validated Blender version>",
  "gacha_setup": "v3.6.2",
  "gacha_setup_commit": "069e51f3fd89cdc66b4c2741798ede11a5fb16b5",
  "export_contract": "blender-gltf-npr-v1",
  "profile_schema": "gacha/v1",
  "fixture": "hero_face_day_v1",
  "renderer": "reference-gacha"
}
```

这使 Golden 成为真正的工程证据，而不是匿名 PNG。

---
