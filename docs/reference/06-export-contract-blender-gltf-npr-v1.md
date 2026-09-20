# Export Contract `blender-gltf-npr-v1`（Blender → glTF → bevy-stylized）

> **Status:** Draft（pending ADR-0012，未经维护者确认前不构成 runtime 契约）
> **上游规范：** 本文档是 docs/reference/03-blender-export-contract.md 的具体 v1 落地；03 是规范性边界，本文档是它的版本化实例。
> **配套文档：** 行为侧契约见 docs/reference/07-gacha-setup-behavior-contract-v1.md；桥实现见 tools/blender/README.md。
> **机器可读载体：** `schemas/authoring/` 下的 `export-settings.v1.json`、`npr-attributes.v1.json`、`manifest.schema.json`、`expected-signature.genshin-v3.6.2.json`。本文档与这些 JSON 冲突时，以 JSON 为准并修订本文档。

## 1. 合同版本与版本规则

- 合同标识：`blender-gltf-npr-v1`。
- 任何对以下内容的修改都必须提升合同版本号并走 docs/reference/04-upstream-contract-upgrade-and-golden.md 的受控迁移流程：
  - 锁定的 glTF exporter 设置集合或任一值；
  - `_NPR_*` attribute 的名称 / domain / data type / accessor 形态；
  - manifest schema（当前 `export-manifest-v1`）；
  - sidecar schema（当前 `export-sidecar-v1.1`）。

> **Draft 修订记录（2026-09-12，pending ADR-0012 确认时一并审议）**：sidecar schema 从 `export-sidecar-v1` 增量扩展为 `export-sidecar-v1.1`——新增可选字段 `bevy_stylized.genshin.ground_truth`（palette/ramp/材质实例 ground truth 提取，仅 main 角色产出）。该变更为纯增量（additive-only），不破坏 v1 消费方；因本合同尚处 Draft（未经 ADR-0012 冻结），以修订记录形式纳入而非提升合同版本号。若 ADR-0012 确认时认为应触发合同版本升级，则随冻结一并升版。
- Node-graph signature 的 revision（当前 `gacha-setup-v3.6.2-069e51f3fd89`）跟随上游 Gacha Setup commit，不属于合同版本，但其变化按 04 的流程分类（A–F）处理。
- 运行期禁止“同一合同版本号下的静默行为漂移”：桥在每次导出时把实际应用的设置做 canonical JSON SHA-256（`export_settings_hash`）写入 manifest，与锁定值比对。

## 2. Scene / Transform policy

以下每条注明依据：`export-settings.v1.json` 的锁定键，或本文档的显式草案决策。

| 策略项 | 规则 | 依据 |
|---|---|---|
| 坐标系 | glTF Y-Up；由 exporter 完成坐标转换，不在脚本内手工变换 | `export_yup=true` |
| 单位 | 1 Blender 单位 = 1 米；exporter 不提供单位缩放键，锁定集中也没有缩放键，因此合同要求 source fixture 以米为单位建模 | 草案决策（待定，pending ADR-0012） |
| Apply Modifier | 导出时应用 modifier | `export_apply=true` |
| GN Outline modifier | 上游 `HoYoverse - Genshin Impact Outlines`（GeometryNodeTree）是 Reference-only 机制；runtime export 的网格**不得**携带该 modifier，否则 `export_apply=true` 会把描边几何烘焙进 GLB。runtime 描边由 bevy-stylized 自己的 outline 架构重建 | 草案决策（待定，pending ADR-0012）；上游证据见 docs/reference/07-gacha-setup-behavior-contract-v1.md §Outline |
| Object/Armature scale | Reference fixture 导出前必须应用 object scale；**禁止** negative scale 与 non-uniform scale 出现在 runtime export 的网格节点上（锁定设置无法表达该项，桥暂不做 fail-closed 检查，列为 V1 fixture 制作规范） | 草案决策（待定，pending ADR-0012） |
| Armature 姿态 | 以 rest position 导出，导出时重置 pose bones | `export_rest_position_armature=true`、`export_reset_pose_bones=true` |
| 导出范围 | 整场景导出，不按选中/可见/可渲染过滤 | `use_selection=false`、`use_visible=false`、`use_renderable=false`、`use_active_scene=false` |
| Camera / Light | 不导出。Reference Camera / Lighting 属 Reference Artifact（03 §A），不进入 runtime GLB | `export_cameras=false`、`export_lights=false` |
| 容器格式 | GLB 单文件 | `export_format="GLB"` |
| 覆盖与持久化 | 直接覆盖输出；不把设置写回 Blender 偏好 | `check_existing=false`、`will_save_settings=false` |

## 3. Mesh export matrix

| 数据 | 导出 | 锁定键 | 理由 |
|---|---|---|---|
| UVs | 是 | `export_texcoords=true` | Base/Lightmap/Ramp 等全部纹理路径依赖 UV |
| Normals | 是 | `export_normals=true` | cel ramp 与 NdotL/NdotV 语义的基础输入 |
| Tangents | 是 | `export_tangents=true` | Normal map 路径需要；上游存在 `Optimized Normal Map` 节点组，runtime 侧 normal path 必须有确定切线 |
| Vertex Colors | 是 | `export_vertex_color="MATERIAL"`、`export_all_vertex_colors=true`、`export_active_vertex_color_when_no_material=true` | 上游 body shader 有 `Use Vertex Color AO` 输入；outline GN 有 `Use Vertex Colors?` 输入 |
| Custom Attributes（`_NPR_*` 等下划线前缀） | 是 | `export_attributes=true` | NPR 语义载体，schema 见 §4 |
| Morph Targets | 是（含 normal，不含 tangent） | `export_morph=true`、`export_morph_normal=true`、`export_morph_tangent=false` | 面部/表情资产需要；morph tangent 用不上且增大体积 |
| Skinning | 是（4 influences） | `export_skins=true`、`export_all_influences=false` | 角色资产必需；4 影响为 portable baseline |
| Animations | 是（采样） | `export_animations=true`、`export_force_sampling=true` | Reference 动画姿态可复现优先于体积 |
| Materials / Images | 导出标准 PBR 输入，图片格式自动 | `export_materials="EXPORT"`、`export_image_format="AUTO"` | 只承载“可导出的标准 Material 输入”（03 §B）；Gacha 节点图不序列化进 GLB |
| Extras（Custom Properties） | 是 | `export_extras=true` | 见 §5 |
| Draco 压缩 | 否 | `export_draco_mesh_compression_enable=false` | Bevy 0.19.1 loader 路径未验证；V1 保持 vanilla glTF |
| 未使用图片/纹理 | 不导出 | `export_unused_images=false`、`export_unused_textures=false` | 控制产物体积与确定性 |

设置键在导出时对当前 `bpy.ops.export_scene.gltf` operator RNA 逐一校验：未知键或非法枚举值是 fail-closed 条件（实测于 Blender 5.2.1 LTS，io_scene_gltf2 5.2.40）。

## 4. `_NPR_*` Mesh Attribute Schema

机器可读正本：`schemas/authoring/npr-attributes.v1.json`。

### 4.1 统一约定

- **glTF accessor 形态**：`FLOAT` → `SCALAR` / componentType `FLOAT`(5126)；`FLOAT_VECTOR` → `VEC3` / componentType `FLOAT`(5126)；`normalized=false`。
- **量化规则（待定，pending ADR-0012）**：v1 草案建议**不做量化**——全部以未压缩 float32 传输。依据 docs/authoring/03-authoring-schema-runtime-packing.md：glTF 2.0 禁止 application-specific attribute 使用 `UNSIGNED_INT`，且 Bevy 0.19.1 不支持 `KHR_mesh_quantization`；U8/Float16 压缩待 Bevy 侧能力验证后在未来合同版本中决定，v1 不凭感觉定格式。
- **Shader location / runtime packing（待定，pending ADR-0012）**：v1 草案建议——经 `GltfPlugin::add_custom_vertex_attribute` 注册为 Bevy 自定义顶点属性（与 legacy `crates/legacy/bevy-gltf-metadata/src/attributes.rs` 的既有机制同构），shader location 由 profile 在材质编译期分配，不固定在导出合同中。§4.3 保留历史命名对应；当前只注册规范 `_NPR_*`，不加载旧实现。
- **Requiredness**：桥层面 v1 全部 attribute 为 warn-if-missing（vendor Gacha Setup v3.6.2 reference blend 不作者 `_NPR_*`，它们由 setup wizard / 资产管线后生成）；逐资产必需性由 runtime profile 强制，不由桥强制。
- **Domain drift**：任何**存在**的 `_NPR_*` attribute 若 domain 或 data type 与本表不符，是 fail-closed 条件。

### 4.2 Attribute 表

| Attribute | Blender Domain | Blender Data Type | glTF Semantic / Accessor | Bevy VertexFormat | Default | Missing Fallback |
|---|---|---|---|---|---|---|
| `_NPR_MASK` | POINT | FLOAT | `_NPR_MASK` / SCALAR float | Float32 | 0.0 | shader-default 0.0 |
| `_NPR_CLOTH_PIN` | POINT | FLOAT | `_NPR_CLOTH_PIN` / SCALAR float | Float32 | 0.0 | no cloth pinning |
| `_NPR_EMISSION` | POINT | FLOAT | `_NPR_EMISSION` / SCALAR float | Float32 | 0.0 | no per-vertex emission |
| `_NPR_FACE_SDF` | POINT | FLOAT | `_NPR_FACE_SDF` / SCALAR float | Float32 | 0.0 | face SDF texture path |
| `_NPR_FACE_SIDE` | POINT | FLOAT | `_NPR_FACE_SIDE` / SCALAR float | Float32 | 1.0 | front face |
| `_NPR_HAIR_MASK` | POINT | FLOAT | `_NPR_HAIR_MASK` / SCALAR float | Float32 | 0.0 | no hair highlight |
| `_NPR_HAIR_TANGENT` | POINT | FLOAT_VECTOR | `_NPR_HAIR_TANGENT` / VEC3 float | Float32x3 | [0.0, 1.0, 0.0] | object-space +Y |
| `_NPR_MATERIAL_ID` | POINT | FLOAT | `_NPR_MATERIAL_ID` / SCALAR float | Float32 | 0.0 | material slot index |
| `_NPR_LEVEL_ROLE` | POINT | FLOAT | `_NPR_LEVEL_ROLE` / SCALAR float | Float32 | 0.0 | generic surface |
| `_NPR_CONTROL0` | POINT | FLOAT_VECTOR | `_NPR_CONTROL0` / VEC3 float | Float32x3 | [0.0, 0.0, 0.0] | zero control vector |

## 5. Custom Properties：extras 与 sidecar

ADR-0020（glTF Extras vs Sidecar Precedence）仍为 Proposed，因此本合同只冻结**中性机制**，不冻结优先级：

1. 桥同时产生两种输出：glTF `extras`（`export_extras=true`，Blender custom properties 随 GLB 携带）与 versioned sidecar（`export-sidecar.json`，schema `export-sidecar-v1.1`（v1 + 增量 ground_truth 字段，见 §1 修订记录），承载 `bevy_stylized.*` namespaced metadata，命名空间规范见 ADR-0016 方向与 docs/authoring/03-authoring-schema-runtime-packing.md）。
2. 两种输出的字段集在 v1 保持一致（sidecar 为正本，extras 为随包副本）。
3. **消费方优先级与冲突解决规则显式推迟到 ADR-0020**；在该 ADR 关闭前，runtime 导入器不得对二者冲突做静默取舍，遇到不一致必须以诊断/错误暴露。

### 5.1 编译期指令键（provisional，V2-f 引入）

`bevy_stylized.compile/v1`：材质 extras 或 sidecar 顶层 `compile` 键（按材质名索引）可声明资产编译指令（当前唯一指令：ILM 四通道打包的源贴图映射，通道布局 R=metal / G=AO / B=specular-suppression / A=material-ID，见 docs/reference/07-gacha-setup-behavior-contract-v1.md §3.2）。payload 必须带 `schema_version`，major 不符则编译 fail-closed。消费方为 `bevy-stylized compile`（crates/bevy-stylized-tools）；冲突诊断规则同 §5 第 3 条。该键的语义在 ADR-0012/0020 关闭前为 provisional。

## 6. Export Manifest 字段

`export-manifest.json` 正本 schema：`schemas/authoring/manifest.schema.json`（`schema_version="export-manifest-v1"`，`additionalProperties=false`）。字段覆盖 docs/reference/03-blender-export-contract.md §Export Artifact Manifest 的全部要求：

| 字段 | 内容 |
|---|---|
| `schema_version` | 固定 `export-manifest-v1` |
| `blender_version` | 导出机的 `bpy.app.version_string`（锁定 5.2.1 LTS） |
| `gacha_setup_version` / `gacha_setup_commit` / `gacha_setup_asset_digest` | 上游 release / commit / release asset SHA-256，来自 toolchain lock |
| `gltf_exporter_version` | io_scene_gltf2 bl_info 版本（实测 5.2.40） |
| `export_contract_version` | 固定 `blender-gltf-npr-v1` |
| `export_settings_hash` | 实际应用的锁定设置 canonical JSON 的 SHA-256 |
| `node_graph_signature.revision` / `.signature_sha256` | 上游节点图签名的修订号与摘要 |
| `render_engine` | 观测值 `BLENDER_EEVEE` |
| `color_management.{display_device, view_transform, view_look, sequencer_colorspace}` | 观测值 sRGB / Standard / None / sRGB |
| `canonical_environment_id` | Reference Family ID（当前 `genshin-festivity-gacha-setup-v1`） |
| `source_fixture.{path, sha256, role}` | 源 blend 路径、SHA-256、角色（main / outline / post） |
| `outputs.{glb, glb_sha256, sidecar, node_graph_signature}` | 产物路径与摘要 |
| `status` | 成功时固定 `ok`；失败时不产生 manifest（fail-closed，无部分输出） |
| `warnings` | 可选；warn 级事件（如可选 `_NPR_*` 缺失）逐条记录 |

## 7. 显式未决项（汇总）

| 未决项 | 状态 | 归属 |
|---|---|---|
| `_NPR_*` 量化格式（U8/Float16/不量化） | 待定（pending ADR-0012）；v1 草案建议不量化 | ADR-0012 |
| Shader location / runtime packing 分配 | 待定（pending ADR-0012）；v1 草案建议 profile 编译期分配 | ADR-0012 |
| extras vs sidecar 消费优先级 | 显式推迟 | ADR-0020 |
| 单位/scale/negative-scale 的桥内 fail-closed 检查 | 草案决策，当前仅为 fixture 制作规范 | ADR-0012 |
| 逐资产 `_NPR_*` requiredness | 由 runtime profile 强制，profile schema 未定稿 | 当前 profile crate 与 authoring schema 明确 requiredness |

---
