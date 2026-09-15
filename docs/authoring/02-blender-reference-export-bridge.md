# Blender Reference Export Bridge（V1 必需）

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


既然复杂 Gacha Shader Node Graph 不能依赖 glTF 自动序列化，V1 必须有一个**最小、无 UI 依赖、可脚本化**的 Blender Reference Export Bridge。

它不是 V3 的完整 Blender Add-on/Editor。

它的唯一职责是让：

```text
Blender/Gacha Setup Reference
→ bevy-stylized Runtime Input
```

这一步可重复、可验证、可自动化。

建议：

```text
tools/blender/
├── README.md
├── export_reference.py
└── contracts/
```

运行方式应支持 Blender background mode，例如：

```text
blender source.blend --background --python export_reference.py -- <profile/options>
```

### Bridge 职责

1. 验证当前 Blender 版本与 Toolchain Lock；
2. 验证 Gacha Setup release/profile signature；
3. 定位已知的 Gacha Material/Node Group 语义；
4. 将上游行为映射到稳定 `bevy-stylized` Profile 字段；
5. 验证/生成需要的 `_NPR_*` Mesh Attributes；
6. 导出 Namespaced Custom Properties / sidecar metadata；
7. 使用锁定的 glTF Export Settings 调用 Blender Exporter；
8. 输出 `export-manifest.json`；
9. 输出 Node-Graph/Material Signature，供上游升级时检测结构变化；
10. 对未知/改变的上游节点拓扑 **Fail Closed**，而不是静默猜测。

### Fail Closed 原则

如果：

```text
Expected Node Group Missing
Socket Name Changed
Material Topology Changed
Required Texture Missing
Attribute Domain Changed
Exporter Setting Drifted
```

则：

```text
Export Failure
```

而不是：

```text
Best-effort Guess
```

Reference Pipeline 的目标是可追溯正确性，不是“尽量导出来”。

### Runtime 与 Bridge 解耦

必须保持：

```text
bevy-stylized runtime
    不依赖 bpy
    不依赖 Blender
    不依赖 Gacha Setup

blender-reference-export
    dev/tool only
```

### V1 与 V3 的边界

V1：

```text
Scriptable Reference Export Bridge
```

V3：

```text
Full Blender Add-on
├─ UI
├─ Material authoring
├─ Preview
├─ Validation
├─ Profile selection
└─ Export workflow
```

V1 不需要完整 DCC UX，但必须具备确定性的 Reference 数据提取能力。

---
