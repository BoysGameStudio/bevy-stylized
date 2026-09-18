//! Hull Outline（Inverted Hull）：独立材质 + 副本 pass（docs/rendering/03）。
//!
//! 顶点沿法线外扩 + front-face cull；无 geometry shader / tessellation /
//! compute 依赖，满足 Portable。宽度以物理像素为单位、视口尺寸无关。
//!
//! **Provisional（ADR-0008 草案）**：生命周期契约（bounds / LOD / despawn
//! 的完整语义）随 ADR-0008 关闭而定；当前实现为最小可用副本同步。

use bevy::prelude::{Or, With, error};
use bevy::{
    mesh::{
        Mesh3d, MeshVertexBufferLayoutRef,
        morph::{MeshMorphWeights, MorphWeights},
        skinning::SkinnedMesh,
    },
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MeshMaterial3d},
    prelude::{
        Asset, Commands, Component, DetectChanges, Entity, GlobalTransform, Handle, Query, Ref,
        Reflect, Transform, Visibility, Without,
    },
    render::render_resource::{AsBindGroup, Face, ShaderType, SpecializedMeshPipelineError},
    shader::ShaderRef,
};

/// Hull outline 着色器（vertex+fragment 单文件，Portable 编译单元）。
pub const OUTLINE_PORTABLE_SHADER_PATH: &str =
    "embedded://bevy_stylized_render/shaders/outline/portable_outline.wgsl";

/// 轮廓线 uniform（颜色 + 像素宽度；单 UBO）。
#[derive(Clone, Copy, Debug, PartialEq, Reflect, ShaderType)]
pub struct StylizedOutlineParams {
    /// 线性 rgb + 宽度（物理像素，0 = 关闭）。
    pub color_width: bevy::math::Vec4,
    /// Optional authored class colors, converted by `source_params.y` gamma.
    pub source_colors: [bevy::math::Vec4; 5],
    /// Local extrusion width, color exponent, suppression, reserved.
    pub source_params: bevy::math::Vec4,
    /// Local-space mode, enabled classes 2–5 mask, force first class, use map.
    pub source_state: bevy::math::UVec4,
}

#[cfg(test)]
mod lifecycle_tests {
    #[test]
    fn outline_mirrors_render_layers_and_removal() {
        use bevy::camera::visibility::RenderLayers;
        let mut app = App::new();
        app.add_systems(Update, sync_outline_replicas);
        let source = app
            .world_mut()
            .spawn((
                Mesh3d(Handle::default()),
                StylizedOutline::new(Handle::default()),
                RenderLayers::layer(3),
            ))
            .id();
        app.update();
        let replica = app
            .world_mut()
            .query_filtered::<Entity, With<StylizedOutlineReplica>>()
            .single(app.world())
            .unwrap();
        assert_eq!(
            app.world().get::<RenderLayers>(replica),
            Some(&RenderLayers::layer(3))
        );
        app.world_mut().entity_mut(source).remove::<RenderLayers>();
        app.update();
        assert!(app.world().get::<RenderLayers>(replica).is_none());
    }
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn outline_tracks_external_bounds_without_mesh_replacement() {
        use bevy::camera::primitives::Aabb;
        let mut app = App::new();
        app.add_systems(Update, sync_outline_replicas);
        let source = app
            .world_mut()
            .spawn((
                Mesh3d(Handle::default()),
                StylizedOutline::new(Handle::default()),
                Aabb::from_min_max(Vec3::ZERO, Vec3::ONE),
            ))
            .id();
        app.update();
        let replica = app
            .world_mut()
            .query_filtered::<Entity, With<StylizedOutlineReplica>>()
            .single(app.world())
            .unwrap();
        let next = Aabb::from_min_max(Vec3::new(-3., -1., 0.), Vec3::new(4., 5., 2.));
        app.world_mut().entity_mut(source).insert(next);
        app.update();
        let actual = app
            .world()
            .get::<Aabb>(replica)
            .expect("external bounds must propagate");
        assert_eq!(actual.center, next.center);
        assert_eq!(actual.half_extents, next.half_extents);
        app.world_mut().entity_mut(source).remove::<Aabb>();
        app.update();
        assert!(
            app.world().get::<Aabb>(replica).is_none(),
            "removed producer bounds must not remain on the outline"
        );
    }

    #[test]
    fn outline_tracks_live_inputs_and_removes_stale_deformation() {
        let mut app = App::new();
        app.add_systems(Update, sync_outline_replicas);
        let mut meshes = Assets::<Mesh>::default();
        let first = meshes.add(Cuboid::default());
        let next = meshes.add(Sphere::default());
        let mut materials = Assets::<StylizedOutlineMaterial>::default();
        let first_material = materials.add(StylizedOutlineMaterial::default());
        let next_material = materials.add(StylizedOutlineMaterial::default());
        let joint = app.world_mut().spawn_empty().id();
        let source = app
            .world_mut()
            .spawn((
                Mesh3d(first),
                StylizedOutline::new(first_material),
                SkinnedMesh {
                    inverse_bindposes: Handle::default(),
                    joints: vec![joint],
                },
                MorphWeights::new(vec![0.0], None).unwrap(),
                MeshMorphWeights::Value { weights: vec![0.0] },
            ))
            .id();
        app.update();
        let replica = app
            .world_mut()
            .query_filtered::<Entity, With<StylizedOutlineReplica>>()
            .single(app.world())
            .unwrap();
        app.world_mut().entity_mut(source).insert((
            Mesh3d(next.clone()),
            StylizedOutline::new(next_material.clone()),
            MeshMorphWeights::Value { weights: vec![1.0] },
        ));
        app.world_mut()
            .entity_mut(source)
            .get_mut::<MorphWeights>()
            .unwrap()
            .weights_mut()[0] = 1.0;
        app.update();
        let entity = app.world().entity(replica);
        assert_eq!(entity.get::<Mesh3d>().unwrap().0, next);
        assert_eq!(
            entity
                .get::<MeshMaterial3d<StylizedOutlineMaterial>>()
                .unwrap()
                .0,
            next_material
        );
        assert_eq!(entity.get::<MorphWeights>().unwrap().weights(), &[1.0]);
        assert!(
            matches!(entity.get::<MeshMorphWeights>(), Some(MeshMorphWeights::Value { weights }) if weights == &[1.0])
        );
        app.world_mut()
            .entity_mut(source)
            .remove::<(SkinnedMesh, MorphWeights, MeshMorphWeights)>();
        app.update();
        let entity = app.world().entity(replica);
        assert!(!entity.contains::<SkinnedMesh>());
        assert!(!entity.contains::<MorphWeights>());
        assert!(!entity.contains::<MeshMorphWeights>());
        app.world_mut().entity_mut(source).remove::<Mesh3d>();
        app.update();
        assert!(app.world().get_entity(replica).is_err());
    }
}

impl Default for StylizedOutlineParams {
    fn default() -> Self {
        Self {
            color_width: bevy::math::Vec4::new(0.02, 0.02, 0.03, Self::DEFAULT_WIDTH),
            source_colors: [bevy::math::Vec4::ONE; 5],
            source_params: bevy::math::Vec4::new(0.0, 1.0, 0.0, 0.0),
            source_state: bevy::math::UVec4::ZERO,
        }
    }
}

impl StylizedOutlineParams {
    /// 默认轮廓宽度（物理像素）。
    pub const DEFAULT_WIDTH: f32 = 2.0;
    /// 轮廓宽度上限（物理像素）。
    pub const MAX_WIDTH: f32 = 24.0;

    /// Select source-unit extrusion and five alpha-band-selected colors.
    /// The caller owns the source color encoding; `color_exponent=1` is linear.
    #[must_use]
    pub fn with_source_palette(
        mut self,
        local_width: f32,
        colors: [bevy::color::LinearRgba; 5],
        enabled_mask: u32,
        force_first: bool,
        color_exponent: f32,
    ) -> Self {
        self.source_colors = colors
            .map(|c| bevy::math::Vec4::new(c.red.max(0.0), c.green.max(0.0), c.blue.max(0.0), 1.0));
        self.source_params.x = if local_width.is_finite() {
            local_width.max(0.0)
        } else {
            0.0
        };
        self.source_params.y = if color_exponent.is_finite() {
            color_exponent.max(0.0)
        } else {
            1.0
        };
        self.source_state = bevy::math::UVec4::new(1, enabled_mask & 15, u32::from(force_first), 0);
        self
    }

    /// 设置线性颜色。
    #[must_use]
    pub fn with_color(mut self, color: bevy::color::LinearRgba) -> Self {
        self.color_width.x = color.red;
        self.color_width.y = color.green;
        self.color_width.z = color.blue;
        self
    }

    /// 设置像素宽度（clamp 到 [0, MAX_WIDTH]）。
    #[must_use]
    pub fn with_width(mut self, width: f32) -> Self {
        self.color_width.w = if width.is_finite() {
            width.clamp(0.0, Self::MAX_WIDTH)
        } else {
            Self::DEFAULT_WIDTH
        };
        self
    }
}

/// 反向外壳轮廓材质（front-cull，沿法线外扩）。
#[derive(Asset, AsBindGroup, Clone, Debug, Default, Reflect)]
pub struct StylizedOutlineMaterial {
    /// 轮廓 uniform。
    #[uniform(0)]
    pub params: StylizedOutlineParams,
    /// Optional authored class map (alpha bands); absent for plain pixel hulls.
    #[texture(1)]
    #[sampler(2)]
    pub class_texture: Option<Handle<bevy::prelude::Image>>,
}

impl Material for StylizedOutlineMaterial {
    fn vertex_shader() -> ShaderRef {
        OUTLINE_PORTABLE_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        OUTLINE_PORTABLE_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> bevy::material::AlphaMode {
        bevy::material::AlphaMode::Opaque
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        crate::skinning::specialize_secondary_skin(descriptor, layout)?;
        // Inverted hull：剔除正面。
        descriptor.primitive.cull_mode = Some(Face::Front);
        descriptor.label = Some("stylized opaque hull".into());
        Ok(())
    }
}

/// 挂在带 `Mesh3d` 实体上：为其生成一个轮廓副本实体。
///
/// 副本与源实体共享 mesh；作为子实体继承 transform/visibility。
/// 生命周期（provisional，ADR-0008）：[`sync_outline_replicas`] 每帧清理
/// 源组件已移除/源实体已销毁的孤儿副本；源实体 despawn 时副本作为子实体
/// 一并销毁。
#[derive(Component, Clone, Debug, Reflect)]
pub struct StylizedOutline {
    /// 轮廓材质。
    pub material: Handle<StylizedOutlineMaterial>,
}

impl StylizedOutline {
    /// 创建一个轮廓附着组件。
    #[must_use]
    pub const fn new(material: Handle<StylizedOutlineMaterial>) -> Self {
        Self { material }
    }
}

/// 标记轮廓副本实体（内部实现细节）。
#[derive(Component, Clone, Copy, Debug)]
#[doc(hidden)]
pub struct StylizedOutlineReplica {
    /// 源实体。
    pub source: Entity,
}

/// 为每个带 [`StylizedOutline`] 的 mesh 实体维护一个轮廓副本。
///
#[derive(Component)]
#[doc(hidden)]
pub struct StylizedOutlineSourceBounds;

/// 副本复制源实体的蒙皮/形态组件（`SkinnedMesh` / `MorphWeights`），
/// 否则蒙皮 mesh 的 outline pipeline 与实例 bind group 布局不匹配
/// （skinned vs model-only），渲染直接 validation error。
pub fn sync_outline_replicas(
    mut commands: Commands,
    sources: Query<
        (
            Entity,
            Ref<Mesh3d>,
            Option<Ref<StylizedOutline>>,
            Option<Ref<crate::host_material::HostOutline>>,
            Option<Ref<crate::host_material::HostInstanceInput>>,
            Option<Ref<SkinnedMesh>>,
            Option<Ref<MorphWeights>>,
            Option<Ref<MeshMorphWeights>>,
            Option<Ref<bevy::camera::primitives::Aabb>>,
            Option<Ref<bevy::camera::visibility::RenderLayers>>,
            Option<Ref<GlobalTransform>>,
        ),
        (
            Without<StylizedOutlineReplica>,
            Or<(
                With<StylizedOutline>,
                With<crate::host_material::HostOutline>,
            )>,
        ),
    >,
    replicas: Query<(
        Entity,
        &StylizedOutlineReplica,
        Option<&StylizedOutlineSourceBounds>,
    )>,
) {
    let mut existing = std::collections::HashMap::new();
    for (entity, replica, bounds) in &replicas {
        if sources.get(replica.source).is_err() {
            commands.entity(entity).despawn();
        } else {
            existing.insert(replica.source, (entity, bounds.is_some()));
        }
    }
    for (
        entity,
        mesh,
        outline,
        host_outline,
        host_input,
        skin,
        morph,
        morph_reference,
        bounds,
        layers,
        global,
    ) in &sources
    {
        if outline.is_some() && host_outline.is_some() {
            error!("A surface cannot have both static and host outline owners");
            continue;
        }
        let current = existing.get(&entity).copied();
        let new = current.is_none();
        let mut target = if let Some((replica, _)) = current {
            commands.entity(replica)
        } else {
            commands.spawn((
                StylizedOutlineReplica { source: entity },
                Transform::IDENTITY,
                GlobalTransform::IDENTITY,
                Visibility::Inherited,
                bevy::prelude::ChildOf(entity),
            ))
        };
        // The generated child has an identity local transform. An external
        // host has already finalized source globals; do not re-propagate its
        // hierarchy just to initialize renderer-owned replicas.
        if let Some(global) = global
            && (new || global.is_changed())
        {
            target.insert(*global);
        }
        if let Some(layers) = layers {
            if new || layers.is_changed() {
                target.insert((*layers).clone());
            }
        } else {
            target.remove::<bevy::camera::visibility::RenderLayers>();
        }
        if new || mesh.is_changed() {
            target.insert(Mesh3d(mesh.0.clone()));
            // Mesh replacement invalidates the old mesh's automatically computed bounds.
            target.remove::<bevy::camera::primitives::Aabb>();
        }
        if let Some(bounds) = bounds {
            if new || mesh.is_changed() || bounds.is_changed() {
                target.insert((*bounds, StylizedOutlineSourceBounds));
            }
        } else if current.is_some_and(|(_, mirrored)| mirrored) {
            target.remove::<(bevy::camera::primitives::Aabb, StylizedOutlineSourceBounds)>();
        }
        if let Some(outline) = outline
            && (new || outline.is_changed())
        {
            target.remove::<MeshMaterial3d<crate::host_material::HostMaterial<StylizedOutlineMaterial>>>();
            target.insert(MeshMaterial3d(outline.material.clone()));
        }
        if let Some(outline) = host_outline {
            if new || outline.is_changed() {
                target.remove::<MeshMaterial3d<StylizedOutlineMaterial>>();
                target.insert(MeshMaterial3d(outline.0.clone()));
            }
            if let Some(input) = host_input {
                if new || input.is_changed() {
                    target.insert((*input).clone());
                }
            } else {
                target.remove::<crate::host_material::HostInstanceInput>();
            }
        }
        if let Some(skin) = skin {
            if new || skin.is_changed() {
                target.insert((*skin).clone());
            }
        } else {
            target.remove::<SkinnedMesh>();
        }
        if let Some(morph) = morph {
            if new || morph.is_changed() {
                target.insert((*morph).clone());
            }
        } else {
            target.remove::<MorphWeights>();
        }
        if let Some(reference) = morph_reference {
            if new || reference.is_changed() {
                target.insert((*reference).clone());
            }
        } else {
            target.remove::<MeshMorphWeights>();
        }
    }
}
