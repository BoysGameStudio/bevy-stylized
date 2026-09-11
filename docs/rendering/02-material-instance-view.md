# Material / Instance / View 数据模型

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


这是 V1 必须完成的核心重构。

## Material

只保存：

```text
Textures
Ramp/Profile
Static Material Flags
Hair/Metal Static Parameters
Static Profile Parameters
```

---

## Instance

保存：

```text
Face Side
Face Threshold
Per-character Flags
Outline Scale
Profile Index
Instance Overrides
```

---

## View / Frame

保存：

```text
NPR Key Light
Global Palette
Day/Night
Exposure
Quality State
Camera NPR State
```

---

## 明确禁止

禁止：

```text
Face Transform
→ Material Asset 每帧修改
```

禁止：

```text
Global Day/Night
→ 每个 Material 重复写一遍
```

禁止：

```text
Camera Post Setting
→ Character Material
```

---

## Face SDF

当前 unique material path 应演化为：

```text
Shared Material
      +
StylizedInstanceGpu
```

例如：

```rust
struct StylizedInstanceGpu {
    face_threshold: f32,
    face_side: u32,
    outline_scale: f32,
    profile_index: u32,
}
```

---

## Face Light Math

以下数学：

```text
sin/cos
basis normalization
dot
acos/pow
side selection
threshold
```

如果输入对一个角色一帧保持一致，应从：

```text
Per Fragment
```

提升到：

```text
Per Instance Prepare
```

最终 Fragment 主要做：

```text
Sample SDF
+
Threshold Compare
+
Soft Transition
```

若某 Profile 需要真正 per-pixel light direction，则可以在 Hero/Reference 特化，不应拖累默认路径。

---

## StylizedInstance Data Transport（V1 Blocker）

`StylizedInstanceGpu` 只是逻辑数据模型；V1 还必须定义它如何进入 Bevy 0.19 GPU pipeline，同时保留 automatic instancing/batching。

Bevy 0.19.1 官方 automatic-instancing 示例证明：相同 Mesh + Material 可以自动合批，并使用 `MeshTag(u32)` 为每个 mesh instance 提供不同索引；shader 可通过 `mesh_functions::get_tag(instance_index)` 读取。Bevy 同时提供 `GpuArrayBuffer<T>`，在支持 storage buffer 时使用 storage，在 storage 不可用时可退回 batched dynamic uniform buffer。**但这只证明数据有 fallback storage 形式，不证明该 fallback 与 NPR mesh automatic batching 的最终组合一定最优**；dynamic offset/unbatchable 路径必须通过实际 render-phase prototype 验证。

因此必须新增：

```text
ADR: NPR Instance Data Transport
```

至少比较：

```text
A. MeshTag 直接 bit-pack 极少量数据
B. MeshTag → index into GPU/texture instance table
C. GpuArrayBuffer<T> + MeshTag/index integration
D. Texture/LUT-backed instance table for WebGL2-style baseline
E. Custom batching/phase（last resort）
```

### V1 Gate

创建 100/1000 个：

```text
same Mesh
same Material
same Pipeline
```

但拥有不同：

```text
Face threshold
Face side
Palette/Profile index
Outline instance control
```

必须验证：

```text
Instance Data 正确
+
Automatic Instancing/Batched Draw 未被无意破坏
+
Portable baseline 有非-storage-buffer fallback（若 WebGL2/GLES3.0 在支持范围内）
```

在该 ADR 通过前，不应把任何具体 storage-buffer 方案写成稳定公共 ABI。

---
