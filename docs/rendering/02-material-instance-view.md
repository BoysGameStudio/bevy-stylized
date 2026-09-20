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

## External face pose input (2026-09-15)

`StylizedFaceInput` is an explicit renderer-side consumer of an application-owned
head entity's `GlobalTransform`. Its local forward/up vectors are supplied by the
host. It owns only the instance face threshold and side correction while attached,
restores previous values when removed, and exposes invalid inputs through
`StylizedFaceInputStatus`. Materials remain shared. The current adapter requires
MeshTag transport (threshold step 1/254), rejects storage-table slots and explicit
material-side overrides, and does not claim full-precision animated-character
parity. See [the contract and verification](../showcase/04-realtime-validation.md).

## Provisional StorageTable lifecycle (2026-09-20)

StorageTable remains experimental under ADR-0004. An idle native app keeps only an
unallocated table placeholder; a storage asset is created only for explicit entries.
Slots are dense indices in one
published table snapshot, not persistent handles. Entries are ordered by Entity
bits; removing an entry or despawning an entity compacts the next snapshot. Hosts
must not cache or assign these slot numbers. `next_slot` now reports live records.

`InstanceTableBudget.max_bytes` defaults to 16 MiB. Admission also obeys the actual
RenderDevice buffer/binding limits and the 24-bit MeshTag index range. Capacity
rounds upward within those limits and contracts with the live set. Excess entries
receive `InstanceTableCapacityExceeded`; their table material binding and slot are
suspended. Re-admission restores the saved material. Removing the entry likewise
revokes its slot and table draw binding and clears any admission failure, including
when no slot was ever allocated; adding it again may restore that binding.

Membership or capacity changes publish a fresh ShaderBuffer asset and rebind all
materials using the prior table before asset events/extraction. New tags therefore
cannot address an old prepared table if upload is delayed: the new material must
wait for its new dependency. Submitted draws retain their previous buffer through
Bevy/wgpu ownership. Numeric-only updates reuse the buffer because index ownership
is unchanged; delayed upload may retain prior values, never another entity's slot.
A render scheduling barrier places buffer preparation before table material
preparation. Unchanged records do not upload or recreate bindings.

The native schedule owns this synchronization. Hosts that take over presentation
must invoke the same system before instance tag synchronization and asset events.
The host still chooses StorageTable only on a supported tier; Portable materials
keep their independent zero-storage layouts. This lifecycle improvement does not
close ADR-0004 or claim a faster transport than MeshTag.

## Native camera ownership and timing provenance (2026-09-20)

`StylizedViewScope::AllNativeCameras` preserves the original global native mode.
Embedded hosts can insert `StylizedViewScope::MarkedCameras` and attach
`StylizedViewCamera` only to cameras whose Exposure, AmbientLight, DistanceFog and
clear color the framework may write. Removing the marker stops subsequent writes;
it does not restore prior camera components. Newly created cameras require their
own marker. Native lighting still uses the scene's shared StylizedKeyLight and a
single StylizedViewSettings resource. Marking cameras does not create independent
light worlds. Use the existing HostView/HostMaterial records and externally owned
presentation schedule for independently authored views or host-owned lights.

QualitySignals now calls its timing value `frame_time_ms`, with `timing_source`
and `observed_frame`. FrameTimeDiagnostics is recorded as FrameInterval; the renamed
`StylizedDynamicQuality.frame_time_override_ms` is HostProvided (possibly synthetic).
The observed frame is the facade's collection frame, not a GPU timestamp. Invalid
(nonfinite/nonpositive) timing input does not advance the governor. No GPU timing
claim is made by either source. The existing hysteresis/cooldown/capability policy
is unchanged; hosts must migrate the two former `gpu_frame_time_*` field names.
