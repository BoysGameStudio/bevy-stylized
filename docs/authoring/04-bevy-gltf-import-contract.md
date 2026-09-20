# Bevy 0.19.1 glTF Import Contract

> **Status:** Normative architecture specification


这条工作流已经能落到 Bevy 官方 API，但必须显式配置。

### Custom Vertex Attributes

Bevy 0.19.1 `GltfPlugin` 提供：

```text
add_custom_vertex_attribute(name, MeshVertexAttribute)
```

并要求 glTF attribute name 以 `_` 开头。

因此 `StylizedPlugin`/glTF integration 必须在 glTF loader 注册前把所有 V1 `_NPR_*` attributes 注册到 `GltfPlugin`。不能假定未知 custom semantic 会被默认保存。

### extras

Bevy 0.19.1 暴露：

```text
GltfExtras
GltfMaterialExtras
GltfMeshExtras
GltfSceneExtras
```

其内容是未经类型化的 JSON string。

因此 `bevy_stylized` convention 必须自己定义稳定 JSON root，例如：

```json
{
  "bevy_stylized": {
    "schema": 1,
    "profile": "genshin/v1"
  }
}
```

注意：glTF `extras` 本身没有规范化 namespace；`bevy_stylized` 只是本项目 application convention。

Importer 必须：

```text
parse
→ validate schema/version
→ reject unknown required fields/profile version
→ resolve extras/sidecar precedence
→ detect conflicting duplicate metadata
→ produce typed `Stylized*` descriptors
```

V1 必须选择并文档化唯一的 metadata precedence policy。推荐原则是“一个 canonical source，另一个只允许补充或必须一致”；禁止 extras 与 sidecar 对同一 required field 给出不同值后静默择一。

正式 Runtime Contract 不允许直接在 shader/runtime 系统里到处解析 arbitrary JSON。

### 不支持的 glTF extension 不得成为 V1 前提

Bevy 0.19.1 明确未支持：

```text
KHR_mesh_quantization
KHR_texture_basisu extension syntax
EXT_mesh_gpu_instancing
```

所以 V1 importer/fixtures/CI 必须覆盖“纯 vanilla glTF + registered custom attributes + extras/sidecar”的标准路径。

若未来 Bevy 新版本新增支持，应通过 Bevy Compatibility ADR/测试升级，不应隐式改变 V1 schema。

---

## glTF Plugin Initialization Ordering Contract（V1 必需）

Bevy `DefaultPlugins` 在启用 `bevy_gltf` feature 时已经包含 `GltfPlugin`。而 `_NPR_*` custom vertex attributes 必须通过：

```rust
GltfPlugin::add_custom_vertex_attribute(...)
```

在 glTF loader 注册配置阶段提供。

因此不能把下面这段伪装成“完整 glTF 集成必然成立”：

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(StylizedPlugin::default());
```

因为 `StylizedPlugin` 如果只在 `DefaultPlugins` 之后加入，不能假定自己还能 retroactively 改写已经配置好的 `GltfPlugin` custom attribute registry。

### V1 Facade 必须提供初始化前 Helper

目标 API 形态可以是以下之一（最终由 API ADR 决定）：

```rust
App::new()
    .add_plugins(DefaultPlugins.set(stylized_gltf_plugin()))
    .add_plugins(StylizedPlugin::default());
```

或等价的：

```text
StylizedDefaultPlugins / StylizedGltfIntegration helper
```

要求：

1. `_NPR_*` registry 在任何 NPR glTF asset load 前完成；
2. helper 与最终 Package Name 解耦；
3. 不重复添加第二个 GltfPlugin；
4. no-gltf 用户仍可只使用 `StylizedPlugin`；
5. 初始化顺序有 compile/run integration test；
6. 文档不能承诺比 Bevy 真实 Plugin 初始化模型更“魔法”的行为。

### V1 接入体验目标

“基础接入 ≤10 行”仍保留，但要分成：

```text
Programmatic/No-glTF
→ DefaultPlugins + StylizedPlugin

glTF + _NPR_* Authoring
→ DefaultPlugins configured with Stylized-aware GltfPlugin + StylizedPlugin
```

两条路径都必须有可复制运行的 Example。

---
