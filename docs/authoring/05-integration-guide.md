# 集成指南（bevy-stylized）

> **Status:** Working note（随代码走；规范边界见 docs/architecture/03）。

面向接入方的最短路径。当前版本覆盖 Cel/Character/Hero @ Portable
（其余档位由 capability resolver 决定，见下文 fallback）。

## 1. 程序化接入（无 glTF，≤10 行）

```rust
use bevy::prelude::*;
use bevy_stylized::prelude::*;

App::new()
    .add_plugins((DefaultPlugins, StylizedPlugin::default()))
    .add_systems(Startup, |mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StylizedCelMaterial>>| {
        commands.spawn((Mesh3d(meshes.add(Sphere::new(1.0))), MeshMaterial3d(materials.add(StylizedCelMaterial::default()))));
        commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 4.0)));
    })
    .run();
```

`StylizedPlugin`（默认 features `gltf` + `render`）装配：能力管线
（ADR-0022 两阶段）+ Cel/Character/Hero 材质 + outline + 后处理 +
view 同步。

## 2. glTF 接入（`_NPR_*` 属性）

**Provisional（ADR-0028 未决）**：DefaultPlugins 自带的 `GltfPlugin` 不含
`_NPR_*` 注册，必须替换：

```rust
use bevy::gltf::GltfPlugin;

App::new()
    .add_plugins(DefaultPlugins.build().disable::<GltfPlugin>())
    .add_plugins(bevy_stylized::stylized_gltf_plugin()) // 已注册全部 _NPR_* 属性
    .add_plugins(bevy_stylized::StylizedPlugin::default());
```

顺序错误（先加 GltfPlugin 再配置）会 fail-closed 报明确错误，而不是静默
丢属性（见 `check_gltf_plugin_not_added` 与插件顺序集成测试）。

## 3. CLI

```text
bevy-stylized validate <file.glb> [--json]   # _NPR_* 契约校验；exit 0/1/2
bevy-stylized inspect  <file.glb> [--json]   # 内容摘要 + 可驱动 class 启发式
```

exit code：0 = 通过/正常报告；1 = 契约违规（如已存在的 `_NPR_*` accessor
形态与契约不符，fail-closed）；2 = 用法/IO 错误。

## 4. 三层数据与 fallback 行为

- **Material 层**：纹理/ramp/静态参数（资产级，帧间不变）；
- **Instance 层**：`StylizedInstanceData`（face 阈值/侧、profile 索引、
  outline 缩放）经 MeshTag 传输，不破坏自动合批（provisional，
  pending ADR-0004）；
- **View 层**：`StylizedViewSettings`（key light、palette[0]→环境色、
  day/night、exposure）经 Bevy 相机/灯光对象进入渲染。

设备能力不满足请求档位时：resolver 确定性降档（Class 不变），class 级
fallback 链削减特性（Hero 先关 matcap/spec ramp/packed normal），并产出
`StylizedFallbackReport`（`StylizedResolvedCapabilities.report`）。永不
静默跨 Class 降级。

## 5. 自定义 Profile（ADR-0025）

第三方 profile 只依赖 facade re-export 的 core 契约，实现
`StylizedProfile` trait 即可，不接触 render internals。完整可运行示例：

```text
cargo run -p bevy-stylized --example custom_profile
```

（`crates/bevy-stylized/examples/custom_profile.rs`；注册契约的测试证明
见 `crates/bevy-stylized/tests/custom_profile.rs`。）

## 6. Showcase / 调试入口

12 个固定用例见 docs/showcase/02；golden/benchmark 证据设施见
fixtures/README.md 与 `.github/README.md`（CI 两层政策）。

## 7. Alpha / Shadow / Prepass / Outline 支持矩阵（provisional pending ADR-0023）

| 能力 | Cel | Character | Hero | Hull Outline |
|---|---|---|---|---|
| Opaque | ✅ | ✅ | ✅ | ✅ |
| Mask（alpha cutoff） | ✅ | ✅ | ✅ | ⛔ |
| Blend / Premultiplied / Add / Multiply | ⛔ | ⛔ | ⛔ | ⛔ |
| Shadow 投射/接收 | ⛔ | ⛔ | ⛔ | ⛔ |
| Depth/Normal Prepass | ⛔（不需要） | ⛔ | ⛔ | ⛔（不需要） |

规则（fail-closed，由 `tests/semantics_matrix.rs` 钉死）：

- **Blend 在 V1 类型层面不可达**：四个材质的 `alpha_mode()` 只会返回
  `Opaque` 或 `Mask(cutoff)`，不存在 Blend 代码路径——不是"静默渲染错"，
  而是"无法表达"。需要半透明的发卡/特效属 V2（见 docs/rendering/03 矩阵）；
- prepass/shadow 显式关闭（`enable_prepass/enable_shadows == false`），
  hull outline 不依赖任何 prepass；
- gacha 资产的 diffuse alpha 是控制数据而非透明度：接入层应强制 Opaque
  （alpha 不泄漏到帧缓冲，见 odette golden 用例的排障记录）。

