# 架构不变量、风险与 ADR

> **Status:** Normative architecture specification


## Architecture Invariants

以下规则应长期保持：

1. Renderer ≠ Art Style。
2. Authoring Data ≠ Runtime GPU Representation。
3. Material ≠ Instance ≠ View。
4. Surface Class ≠ Execution Tier ≠ Reference Mode。
5. Upstream Reference ≠ Engine Reference ≠ Production Renderer。
6. Portable Execution Tier 不依赖可选 GPU Feature。
7. High Path 必须有明确 fallback。
8. Disabled Feature 应尽可能接近 Zero GPU Cost。
9. Capability 必须在 Pipeline 创建前 Resolve。
10. Core 不允许 Game-specific 数据。
11. 优化必须有视觉与性能证据。
12. GPU Resource Budget 是 **whole-pipeline per-stage** Contract。
13. Examples ≠ Showcase ≠ Fixtures。
14. WGSL Token/Naga Test 不能替代 backend-specific Pipeline Validation。
15. Runtime 不依赖 Editor/Tooling/Blender/Gacha Setup。
16. 公开 Support Claim 必须与实际验证等级对应。
17. Reference-only 资源默认不能进入 Shipping Build。
18. Tier 降级只能降低细节，不能破坏定义好的视觉语义。
19. Reference Golden 必须可追溯到精确 Blender/Gacha Setup/具体 Shader Artifact/Exporter/Profile 版本。
20. 上游 `latest` 永远不是稳定 Golden Contract；升级必须通过 Reference Migration。
21. glTF 只承载明确支持的 Runtime Data；复杂 Blender Node Graph 不是隐式 Runtime ABI。
22. Reference 数据提取必须脚本化、版本化、fail closed。
23. Cargo Feature 只表达 compile capability；Runtime Variant 由 Resolver 决定。
24. Rust Profile 扩展承诺 Stable API/Schema，不虚构未设计的稳定 Rust ABI。
25. Package name 与 License 必须由 ADR/现实 registry/provenance 审查决定。
26. V1 glTF Contract 不依赖 Bevy 0.19.1 明确未支持的 `KHR_mesh_quantization` / `KHR_texture_basisu` / `EXT_mesh_gpu_instancing`。
27. Runtime Resolver 只能依据实际 RenderDevice 已启用能力；需要额外 feature/limit 的路径必须在 RenderDevice 创建前配置。
28. `_NPR_*` glTF attributes 必须在 `GltfPlugin` loader 配置阶段注册；StylizedPlugin 不得假装能在事后自动补注册。
29. Golden 必须记录 canonical Blender render environment；extras/sidecar 冲突不得静默解析。
30. Schema namespace 是稳定数据 Contract，不跟 crates.io package name 隐式联动。

---

## PR Review Checklist

### Architecture

- 是否把 Instance Data 放进 Material？
- 是否把 View Data 放进 Material？
- 是否把 Profile-specific 数据放进 Core？
- 是否破坏单向依赖？

### GPU

- 是否增加 Binding？
- 是否增加 Texture Sample？
- 是否增加 Vertex Attribute？
- 是否增加 Pass？
- 是否超对应 baseline 的 Whole-Pipeline Budget？
- Disabled 状态是否仍执行无意义 GPU work？

### Platform

- 是否要求新 GPU Feature？
- fallback 是什么？
- WebGPU/GLES3.1/WebGL2 baseline（按支持范围）是否仍满足？
- GLES/D3D12/Vulkan/Metal 是否受影响？

### Validation

- Unit？
- Shader？
- Pipeline？
- Golden？
- Showcase？
- Benchmark？
- Docs？

---

## 主要风险

| 风险 | 后果 | 控制措施 |
|---|---|---|
| 过早重写 Reference | 丢失视觉正确性 | Reference Freeze + Golden |
| 过度 Crate 化 | API/依赖复杂 | V1 只保留 6 个主要 crate |
| Shader permutation 爆炸 | 编译时间/缓存膨胀 | Coarse Material Class |
| Portable 被 Hero Feature 污染 | Web/Mobile 失败 | Resource Budget Gate |
| 过度追求 bindless | 低端不兼容 | Fixed + Bindless 双路径 |
| Asset packing 破坏数据语义 | Material ID/Face 错误 | Texture Semantic |
| 跨 GPU Golden 过严 | CI 不稳定 | Canonical + Tolerance |
| “理论优化”无实际收益 | 增加复杂度 | Benchmark Evidence |
| Bevy API 快速变化 | 维护成本 | Compatibility Matrix + ADR |
| 未授权资产进入公开仓库 | 版权/发布问题 | Synthetic/Public Fixtures |
| V2 功能提前进入 V1 | 项目失控 | Feature Freeze |
| SurfaceClass 与 Tier 混用 | Resolver/LOD 语义错误 | 三轴模型 |
| Instance transport 破坏 batching | CPU/draw-call 回退 | Instance Transport ADR + batching test |
| Engine Reference 自证 | 翻译错误无法发现 | Upstream Golden 三层验证 |
| Cargo 包名/namespace 在首次发布前发生冲突 | 无法按既定名称发布 | ADR-0016 已选定 `bevy-stylized`；首次 crates.io publish 前重新 preflight |
| License provenance 不清 | 发布/合规风险 | License & Provenance ADR |
| Golden 依赖未记录的 Blender/GPU 环境 | Reference 漂移 | Canonical Reference Environment |
| extras/sidecar 双源冲突 | 同一资产行为不确定 | Metadata precedence contract |

---

## ADR Index

ADR 编号不得在治理文档中手工维护第二份列表。

唯一真相源：

[`docs/adr/ADR_INDEX.md`](../adr/ADR_INDEX.md)

规则：

- ADR ID 由文件名决定；
- `ADR_INDEX.md` 由文档检查/生成脚本验证；
- Traceability 只能引用实际存在的 ADR ID；
- `Accepted` ADR 必须包含非空 Decision、Evidence、Consequences；
- `V1 Blocker` 在 V1 Release 前必须全部关闭为 `Accepted`、`Rejected` 或明确移出 V1 scope；
- 不允许重用已经发布过的 ADR ID。

---

## Current reference source

The [source lock](../../assets/sources/gacha-authoring.json) owns the current
qualified release and digest. The active recipe and
[capture contract](../reference/09-static-character-capture.md) bound qualification.
Do not maintain another version/hash snapshot here. Reference lineage and
redistribution boundaries belong to [third-party notices](../../THIRD_PARTY_NOTICES.md).

## 本次深度检视的权威依据

关键事实必须以官方规范/上游源码为准。当前主要依据：

- Cargo registry naming restrictions: https://doc.rust-lang.org/cargo/reference/registry-index.html
- Existing `bevy_npr` crate: https://docs.rs/bevy_npr/latest/bevy_npr/
- Rust layout/ABI guarantees: https://doc.rust-lang.org/reference/type-layout.html
- glTF 2.0 application-specific attributes: https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html
- Blender glTF custom properties/attributes: https://docs.blender.org/manual/en/5.2/addons/scene_gltf2.html
- wgpu limits / GLES/WebGL2 baselines: https://docs.rs/wgpu-types/latest/wgpu_types/struct.Limits.html
- Bevy 0.19.1 automatic instancing and MeshTag: Bevy v0.19.1 `examples/shader/automatic_instancing.rs`
- Bevy 0.19.1 glTF custom attributes/extras/extensions: `crates/bevy_gltf/src/lib.rs`, `loader/mod.rs`, `assets.rs`
- Bevy DefaultPlugins/GltfPlugin integration: Bevy `DefaultPlugins` includes `GltfPlugin`; use PluginGroup configuration (`set`) for loader-time custom attributes
- Bevy 0.19.1 GPU array buffers: Bevy v0.19.1 `bevy_render::render_resource::GpuArrayBuffer`
- Bevy 0.19.1 WebGPU/WebGL examples/status: Bevy v0.19.1 examples README
- Bevy 0.19.1 renderer initialization/features/limits: `bevy_render::settings::WgpuSettings` and `WgpuSettingsPriority`
- Gacha Setup: https://github.com/PaoloESAN/gacha-setup and Releases
- Genshin reference lineage: https://github.com/festivities/Blender-miHoYo-Shaders

这些 URL/版本不是“永远不变的 truth”；正式 Release 时应将已验证版本写入 Toolchain Lock/Support Matrix。

---
