# ADR-0004: NPR Instance Data Transport

- **Status:** Accepted
- **Decision owner:** Architecture
- **Date:** 2026-09-22

## Context

Prototype MeshTag, GpuArrayBuffer, LUT/texture indirection and portable fallback
without breaking batching.

## Options

1. MeshTag 直接 bit-pack 极少量数据。
2. MeshTag → index into GPU/texture instance table（StorageTable）。
3. GpuArrayBuffer<T> + MeshTag/index integration。
4. Texture/LUT-backed instance table for WebGL2-style baseline。
5. Custom batching/phase（last resort）。

## Decision

V1 采用双通道合同：

- **MeshTag bit-pack**（选项 A）是粗粒度逐实例输入的稳定运输：threshold
  step 1/254、最大绝对阈值误差 1/508；profile/flags 8 bit；face flip 1 bit；
  非默认 outline scale 采用 7 bit [0,2] 量化。MeshTag 位布局自此稳定。
- **StorageTable**（选项 B，dense 快照生命周期）是 Character 完整记录的
  规范运输合同：dense slot、准入失败、撤销/重新准入、布局变更重绑、
  默认 16 MiB 预算、Portable 材质保持零 storage 布局，见
  [Material / Instance / View](../rendering/02-material-instance-view.md#storagetable-lifecycle-2026-09-20)。
- Hero 保留原生 MeshTag 粗粒度路径；V1 不引入全精度 Hero 表（未声明更细
  数值需求，原型也不具备准入/生命周期/回退/浏览器资格）。
- 选项 C/D/E 经评估未采用：对所选粗粒度非 storage 合同没有实测优势。

## Evidence

- V1 gate：100/1000 同 mesh+同 material 实例的逐实例变化不破坏合批——
  `tests/integration/tests/instance_batching.rs`（golden
  `fixtures/golden/cel-portable/instances_batch.png`）与
  `spike_instance_transport.rs`（100/1000 实例 / 1 draw spike）；
  全部在必需 GPU lane 中（42/42 PASS，Linux/Vulkan RTX 5090，de57210，
  2026-09-22）。
- StorageTable 生命周期：`tests/integration/tests/instance_table.rs` 的
  churn/像素保持用例与 `table_churn_preserves_pixels`，Portable 绑定门禁
  同属必需套件。
- Hero 精度边界：`tests/integration/tests/hero_precision.rs` 的
  512²、softness 0.01 探针在 100/1000 实例下各保持一个 opaque phase 批次；
  量化与全精度输入间 119506/143405 像素差异是已声明的精度边界，
  不是实体映射错误。CPU 密集扫描验证单调性与 1/508 误差界。
- 精度/能力边界是验收的一部分：全精度 Hero 需求须先声明数值需求、
  完成表生命周期泛化与全链路预算评审后才可提升。

## Consequences

- MeshTag 位布局与 StorageTable 生命周期成为稳定合同；外部代码可依赖。
- Hero 表原型不晋升为稳定 API；追求全精度 Hero 运输需要新需求声明。
- 运行时档位重配置（ADR-0030）降档时，表消费者的 slot 迁移按
  StorageTable 生命周期合同执行。

## Revisit trigger

Revisit when the Bevy/wgpu baseline changes, measured evidence contradicts the
decision, a supported platform cannot satisfy the contract, or a declared
requirement exceeds the documented precision boundary.
