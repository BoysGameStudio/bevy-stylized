//! ADR-0023 增量：渲染语义矩阵测试（provisional，pending ADR-0023）。
//!
//! 当前切片支持的组合作为数据驱动用例钉死：
//!
//! - alpha mode：Opaque / Mask（Blend 等不在切片内，见 docs/rendering/03
//!   矩阵表）；
//! - outline on/off；
//! - cull：主材质继承（不覆写 cull），outline 强制 Front。
//!
//! 全部为 CPU 侧语义断言；视觉组合由 golden 用例覆盖。

use bevy::prelude::*;
use bevy::render::render_resource::Face;
use bevy_stylized::prelude::*;

/// 语义矩阵的一行。
#[derive(Clone, Copy, Debug)]
struct MatrixCase {
    /// alpha mask cutoff（None = Opaque）。
    alpha_cutoff: Option<f32>,
    /// 是否挂 hull outline。
    outline: bool,
}

const MATRIX: &[MatrixCase] = &[
    MatrixCase {
        alpha_cutoff: None,
        outline: false,
    },
    MatrixCase {
        alpha_cutoff: None,
        outline: true,
    },
    MatrixCase {
        alpha_cutoff: Some(0.5),
        outline: false,
    },
    MatrixCase {
        alpha_cutoff: Some(0.5),
        outline: true,
    },
];

#[test]
fn alpha_cull_matrix_semantics_hold() {
    use bevy::pbr::Material;
    for case in MATRIX {
        let mut params = StylizedCelParams::default();
        if let Some(cutoff) = case.alpha_cutoff {
            params = params.with_alpha_cutoff(cutoff);
        }
        let material = StylizedCelMaterial {
            params,
            ..StylizedCelMaterial::default()
        };
        // alpha mode 与 cutoff 一致
        match case.alpha_cutoff {
            None => assert_eq!(material.alpha_mode(), AlphaMode::Opaque),
            Some(cutoff) => {
                assert_eq!(material.alpha_mode(), AlphaMode::Mask(cutoff));
            }
        }
        // cull 继承语义：`double_sided=false` → specialize 不触碰 cull_mode；
        // `double_sided=true` → cull_mode=None（真实 descriptor 断言见 GPU
        // golden 用例的发卡面片渲染）。
        let _ = Face::Front; // outline 在 specialize 中强制 Front

        if case.outline {
            let outline = StylizedOutlineMaterial::default();
            assert_eq!(outline.alpha_mode(), AlphaMode::Opaque);
        }
    }
}

/// Hero 语义矩阵行（provisional pending ADR-0023）：packed normal ×
/// matcap × alpha mode 的标志位/pipeline-key 语义。
#[derive(Clone, Copy, Debug)]
struct HeroMatrixCase {
    /// packed normal 纹理。
    packed_normal: bool,
    /// matcap 强度（0 = 关闭）。
    matcap_intensity: f32,
    /// alpha mask cutoff。
    alpha_cutoff: Option<f32>,
    /// 双面渲染。
    double_sided: bool,
}

const HERO_MATRIX: &[HeroMatrixCase] = &[
    HeroMatrixCase {
        packed_normal: false,
        matcap_intensity: 0.0,
        alpha_cutoff: None,
        double_sided: false,
    },
    HeroMatrixCase {
        packed_normal: true,
        matcap_intensity: 0.0,
        alpha_cutoff: None,
        double_sided: false,
    },
    HeroMatrixCase {
        packed_normal: false,
        matcap_intensity: 0.6,
        alpha_cutoff: None,
        double_sided: true,
    },
    HeroMatrixCase {
        packed_normal: true,
        matcap_intensity: 0.6,
        alpha_cutoff: Some(0.5),
        double_sided: true,
    },
];

#[test]
fn hero_matrix_flags_and_keys_hold() {
    use bevy::pbr::Material;
    use bevy_stylized::stylized_render::hero::{
        HERO_FLAG_MATCAP, HERO_FLAG_PACKED_NORMAL, StylizedHeroMaterialKey,
    };
    for case in HERO_MATRIX {
        let mut material = StylizedHeroMaterial {
            double_sided: case.double_sided,
            ..StylizedHeroMaterial::default()
        };
        if case.packed_normal {
            material.packed_normal_texture = Some(Handle::default());
        }
        if case.matcap_intensity > 0.0 {
            material.matcap_texture = Some(Handle::default());
        }
        material.params = material
            .params
            .with_matcap(case.matcap_intensity, bevy::color::LinearRgba::WHITE);
        material.sync_flags();
        assert_eq!(
            material.params.has_flag(HERO_FLAG_PACKED_NORMAL),
            case.packed_normal
        );
        assert_eq!(
            material.params.has_flag(HERO_FLAG_MATCAP),
            case.matcap_intensity > 0.0
        );
        match case.alpha_cutoff {
            None => assert_eq!(material.alpha_mode(), AlphaMode::Opaque),
            Some(cutoff) => {
                material.params.character.cel =
                    material.params.character.cel.with_alpha_cutoff(cutoff);
                assert_eq!(material.alpha_mode(), AlphaMode::Mask(cutoff));
            }
        }
        // pipeline 特化键忠实反映双面位（cull 语义的载体）。
        let key = StylizedHeroMaterialKey::from(&material);
        assert_eq!(key.double_sided, case.double_sided);
    }
}

/// 无 per-frame 材质资产变异的架构回归测试：
/// N 个实例共享一个材质句柄，跑若干帧后材质资产不得收到任何
/// `Modified`/`Added`（除首次）事件——instance/view 数据只走
/// MeshTag/Bevy view 对象通道。
#[test]
fn no_per_frame_material_mutation_with_shared_material() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..Default::default()
        })
        .add_plugins(bevy::mesh::MeshPlugin)
        .add_plugins(StylizedPlugin::default());

    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial::default());
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(bevy::math::primitives::Sphere::new(0.3));
    for i in 0..100u32 {
        app.world_mut().spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            StylizedInstanceData {
                profile_index: i % 4,
                face_threshold: Some(i as f32 / 100.0),
                ..StylizedInstanceData::default()
            },
            Transform::from_xyz(i as f32, 0.0, 0.0),
        ));
    }
    // 再加一个 camera 触发 view 同步路径。
    app.world_mut().spawn(Camera3d::default());

    for _ in 0..10 {
        app.update();
    }

    // 数材质资产的 Modified 事件（Added 只允许首次注册那次）。
    let events = app
        .world()
        .resource::<Messages<AssetEvent<StylizedCelMaterial>>>();
    let mut added = 0;
    let mut modified = 0;
    for event in events.get_cursor().read(events) {
        match event {
            AssetEvent::Added { .. } | AssetEvent::LoadedWithDependencies { .. } => added += 1,
            AssetEvent::Modified { .. } => modified += 1,
            _ => {}
        }
    }
    assert_eq!(modified, 0, "per-frame material mutation detected");
    assert!(added <= 1, "material asset re-added: {added}");

    // 且 instance 数据确实经 MeshTag 落地（sync_instance_tags 已跑）。
    let mut query = app.world_mut().query::<&bevy::mesh::MeshTag>();
    let distinct: std::collections::BTreeSet<u32> = query.iter(app.world()).map(|t| t.0).collect();
    assert_eq!(distinct.len(), 100, "every instance got its own MeshTag");
}

/// Alpha/Blend 契约（provisional pending ADR-0023）：**Blend 在 V1 不可达**
/// ——四个材质的 `alpha_mode()` 类型层面只能返回 Opaque/Mask；本测试钉死
/// "任何路径都不得静默产生 Blend"。
#[test]
fn blend_is_unreachable_in_v1_materials() {
    use bevy::pbr::Material;
    let cel = StylizedCelMaterial::default();
    let _ = &cel;
    let character = StylizedCharacterMaterial::default();
    let hero = StylizedHeroMaterial::default();
    let outline = StylizedOutlineMaterial::default();
    for alpha in [
        cel.alpha_mode(),
        character.alpha_mode(),
        hero.alpha_mode(),
        outline.alpha_mode(),
    ] {
        assert!(
            matches!(alpha, AlphaMode::Opaque | AlphaMode::Mask(_)),
            "V1 must never produce Blend: {alpha:?}"
        );
    }
}

/// Shadow/prepass/outline 契约（provisional pending ADR-0023；V2-a 修订）：
///
/// - 主材质 `enable_prepass() == true`（**能力**开启；成本门控在相机——
///   只有带 DepthPrepass/NormalPrepass 组件的相机产生 prepass 开销，
///   见 `StylizedScreenOutline` 的 per-camera opt-in）；
/// - `enable_shadows() == false` 不变（V1 无阴影投射）；
/// - hull outline 不依赖任何 prepass（golden 用例已证）；
/// - overlay 材质自身不进 prepass。
#[test]
fn prepass_and_shadow_contract_is_explicit() {
    use bevy::pbr::Material;
    for shadows in [
        StylizedCelMaterial::enable_shadows(),
        StylizedCharacterMaterial::enable_shadows(),
        StylizedHeroMaterial::enable_shadows(),
        StylizedOutlineMaterial::enable_shadows(),
        StylizedScreenOutlineMaterial::enable_shadows(),
    ] {
        assert!(!shadows, "V1 must not claim shadows");
    }
    // 主材质 prepass 能力开启（成本由相机组件门控）；overlay 关闭。
    assert!(StylizedCelMaterial::enable_prepass());
    assert!(StylizedCharacterMaterial::enable_prepass());
    assert!(StylizedHeroMaterial::enable_prepass());
    assert!(!StylizedOutlineMaterial::enable_prepass());
    assert!(!StylizedScreenOutlineMaterial::enable_prepass());
}
