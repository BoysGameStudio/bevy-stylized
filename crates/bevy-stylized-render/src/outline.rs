//! Hull Outline（Inverted Hull）：独立材质 + 副本 pass（docs/rendering/03）。
//!
//! 顶点沿法线外扩 + front-face cull；无 geometry shader / tessellation /
//! compute 依赖，满足 Portable。宽度以物理像素为单位、视口尺寸无关。
//!
//! **Provisional（ADR-0008 草案）**：生命周期契约（bounds / LOD / despawn
//! 的完整语义）随 ADR-0008 关闭而定；当前实现为最小可用副本同步。

use bevy::{
    mesh::{
        Mesh3d, MeshVertexBufferLayoutRef,
        morph::{MeshMorphWeights, MorphWeights},
        skinning::SkinnedMesh,
    },
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MeshMaterial3d},
    prelude::{
        Asset, Commands, Component, Entity, GlobalTransform, Handle, Query, Reflect, Transform,
        Visibility, With,
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
}

impl Default for StylizedOutlineParams {
    fn default() -> Self {
        Self {
            color_width: bevy::math::Vec4::new(0.02, 0.02, 0.03, Self::DEFAULT_WIDTH),
        }
    }
}

impl StylizedOutlineParams {
    /// 默认轮廓宽度（物理像素）。
    pub const DEFAULT_WIDTH: f32 = 2.0;
    /// 轮廓宽度上限（物理像素）。
    pub const MAX_WIDTH: f32 = 24.0;

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
    // 预留：lightmap 多色轮廓属于 Character/Hero 语义，不进 Portable
    // Cel 切片；加入时会以独立字段扩展而不是塞进本 uniform。
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
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Inverted hull：剔除正面。
        descriptor.primitive.cull_mode = Some(Face::Front);
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
/// 副本复制源实体的蒙皮/形态组件（`SkinnedMesh` / `MorphWeights`），
/// 否则蒙皮 mesh 的 outline pipeline 与实例 bind group 布局不匹配
/// （skinned vs model-only），渲染直接 validation error。
pub fn sync_outline_replicas(
    mut commands: Commands,
    sources: Query<(
        Entity,
        &Mesh3d,
        &StylizedOutline,
        Option<&SkinnedMesh>,
        Option<&MorphWeights>,
        Option<&MeshMorphWeights>,
    )>,
    replicas: Query<(Entity, &StylizedOutlineReplica)>,
    source_alive: Query<(), With<StylizedOutline>>,
) {
    // 清理孤儿副本（源组件移除或源实体销毁；子副本可能已随源实体 despawn）。
    for (entity, replica) in &replicas {
        if source_alive.get(replica.source).is_err()
            && let Ok(mut replica_commands) = commands.get_entity(entity)
        {
            replica_commands.despawn();
        }
    }
    // 补齐缺失副本。
    for (entity, mesh, outline, skin, morph, morph_reference) in &sources {
        let alive = replicas.iter().any(|(_, replica)| replica.source == entity);
        if alive {
            continue;
        }
        let mut replica_commands = commands.spawn((
            StylizedOutlineReplica { source: entity },
            Mesh3d(mesh.0.clone()),
            MeshMaterial3d(outline.material.clone()),
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
            Visibility::Inherited,
        ));
        // ChildOf 语义：副本跟随源实体 transform。
        replica_commands.insert(bevy::prelude::ChildOf(entity));
        if let Some(skin) = skin {
            replica_commands.insert(skin.clone());
        }
        if let Some(morph) = morph {
            replica_commands.insert(morph.clone());
        }
        if let Some(morph_reference) = morph_reference {
            replica_commands.insert(morph_reference.clone());
        }
    }
}
