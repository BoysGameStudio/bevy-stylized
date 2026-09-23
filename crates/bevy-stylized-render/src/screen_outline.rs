//! Screen-space 描边（depth/normal 边缘检测）+ hull 混合（It-V2a，
//! ADR-0008 Accepted 2026-09-23）。
//!
//! ## 档位门控
//!
//! Screen-space 边缘需要 `DepthPrepass` + `NormalPrepass`——**Standard+
//! 特性**（Portable 基线保持既有 hull 描边；不引入任何可选 GPU feature，
//! 但 prepass 是显式成本）。选择经 [`resolve_outline_mode`]：resolved tier
//! < Standard 时 screen 模式回落 hull（对应 core 的
//! `StylizedFallbackStep::ScreenOutlineToHull`）。
//!
//! ## 结构
//!
//! - 相机组件 [`StylizedScreenOutline`]（per-camera opt-in）：sync 系统
//!   为其插入 `DepthPrepass`/`NormalPrepass` 并生成一个屏幕覆盖三角形
//!   （[`StylizedScreenOutlineMaterial`]，Transparent3d + Blend）；
//! - 默认成本为零：不带该组件的相机不产生任何 prepass/overlay；
//! - 混合模式：hull（轮廓/剪影）照旧走 opaque 主 pass，screen 描边负责
//!   内部褶皱/接触边缘——两者独立开关、可叠加。

use bevy::{
    core_pipeline::prepass::{DepthPrepass, NormalPrepass},
    mesh::{Mesh, MeshVertexBufferLayoutRef},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MeshMaterial3d},
    prelude::{
        Asset, Assets, Camera3d, Commands, Component, Entity, GlobalTransform, Has, Query, Reflect,
        RemovedComponents, Res, ResMut, Transform, With,
    },
    render::render_resource::{
        AsBindGroup, CompareFunction, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};
use bevy_stylized_core::{StylizedExecutionTier, StylizedResolvedVariant};

use crate::StylizedRenderCapabilities;

/// Screen-space 描边 shader 路径。
pub const SCREEN_OUTLINE_SHADER_PATH: &str =
    "embedded://bevy_stylized_render/shaders/outline/screen_outline.wgsl";

/// 描边模式（ADR-0008；masked 外观见 ADR-0023 的未合格格）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum StylizedOutlineMode {
    /// 仅 hull（Portable 基线）。
    #[default]
    Hull,
    /// 仅 screen-space（内部边缘）。
    ScreenSpace,
    /// 混合：hull 剪影 + screen 内部边缘。
    Hybrid,
}

/// 依 resolved tier 解析描边模式（Standard+ 才允许 screen-space；
/// 降级返回是否回落）。
#[must_use]
pub fn resolve_outline_mode(
    requested: StylizedOutlineMode,
    resolved: Option<&StylizedResolvedVariant>,
) -> (StylizedOutlineMode, bool) {
    let tier = resolved.map(|v| v.tier).unwrap_or_default();
    let screen_allowed = tier >= StylizedExecutionTier::Standard;
    if screen_allowed {
        return (requested, false);
    }
    match requested {
        StylizedOutlineMode::Hull => (StylizedOutlineMode::Hull, false),
        StylizedOutlineMode::ScreenSpace | StylizedOutlineMode::Hybrid => {
            (StylizedOutlineMode::Hull, true)
        }
    }
}

/// 相机组件：启用 screen-space 描边（Standard+；per-camera opt-in）。
#[derive(Component, Clone, Debug, Reflect)]
pub struct StylizedScreenOutline {
    /// 边缘颜色（线性）。
    pub color: bevy::color::LinearRgba,
    /// 深度不连续阈值（NDC 深度差）。
    pub depth_threshold: f32,
    /// 法线不连续阈值（1 − dot）。
    pub normal_threshold: f32,
    /// 采样邻域半径（像素）。
    pub width: f32,
    /// 整体强度（alpha 乘子）。
    pub intensity: f32,
}

impl Default for StylizedScreenOutline {
    fn default() -> Self {
        Self {
            color: bevy::color::LinearRgba::new(0.02, 0.02, 0.035, 1.0),
            depth_threshold: 0.01,
            normal_threshold: 0.25,
            width: 1.5,
            intensity: 1.0,
        }
    }
}

/// Screen 描边 uniform。
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect, ShaderType)]
pub struct StylizedScreenOutlineParams {
    /// rgb = 边缘颜色，w = 强度。
    pub color_intensity: bevy::math::Vec4,
    /// x = 深度阈值，y = 法线阈值，z = 采样半径（像素）。
    pub thresholds: bevy::math::Vec4,
}

/// 屏幕覆盖三角形的 overlay 材质（Transparent3d + Blend；无深度写）。
#[derive(Asset, AsBindGroup, Clone, Debug, Default, Reflect)]
pub struct StylizedScreenOutlineMaterial {
    /// 参数 uniform。
    #[uniform(0)]
    pub params: StylizedScreenOutlineParams,
}

impl Material for StylizedScreenOutlineMaterial {
    fn vertex_shader() -> ShaderRef {
        SCREEN_OUTLINE_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SCREEN_OUTLINE_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> bevy::material::AlphaMode {
        bevy::material::AlphaMode::Blend
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
        // overlay：不写深度、恒过深度测试（边缘可见性由 prepass 内容决定）。
        descriptor.primitive.cull_mode = None;
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.depth_write_enabled = Some(false);
            depth.depth_compare = Some(CompareFunction::Always);
        }
        Ok(())
    }
}

/// overlay 三角形的源相机（内部实现细节）。
#[derive(Component, Clone, Copy, Debug)]
#[doc(hidden)]
pub struct StylizedScreenOutlineOverlay {
    /// 相机实体。
    pub camera: Entity,
}

/// 同步系统：带 [`StylizedScreenOutline`] 的相机获得 prepass 组件 +
/// overlay 三角形；组件移除或运行时降级（ADR-0030：重配置后解析档低于
/// Standard）时撤销本系统授予的 prepass 与 overlay，回落 hull-only。
///
/// 所有权：本系统独占带 [`StylizedScreenOutline`] 相机的
/// `DepthPrepass`/`NormalPrepass`；宿主不得为同一相机另行添加 prepass。
pub fn sync_screen_outline(
    mut commands: Commands,
    capabilities: Option<Res<StylizedRenderCapabilities>>,
    cameras: Query<(Entity, &StylizedScreenOutline, Has<DepthPrepass>), With<Camera3d>>,
    mut removed: RemovedComponents<StylizedScreenOutline>,
    overlays: Query<(Entity, &StylizedScreenOutlineOverlay)>,
    // Option：资产资源缺失（无 AssetPlugin 的 headless 测试）时零工作。
    mut materials: Option<ResMut<Assets<StylizedScreenOutlineMaterial>>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
) {
    let resolved = capabilities.and_then(|c| c.resolved.clone());
    for (camera, settings, had_prepass) in &cameras {
        let (mode, fell_back) =
            resolve_outline_mode(StylizedOutlineMode::ScreenSpace, resolved.as_ref());
        if fell_back {
            // Standard 以下：回落 hull-only，不产生 prepass 成本；运行时已
            // 授予的 prepass/overlay 撤销（ADR-0030 降档路径）。
            if had_prepass {
                commands
                    .entity(camera)
                    .remove::<(DepthPrepass, NormalPrepass)>();
            }
            if let Some((overlay, _)) = overlays.iter().find(|(_, o)| o.camera == camera) {
                commands.entity(overlay).despawn();
            }
            continue;
        }
        let _ = mode;
        // prepass opt-in
        let mut camera_commands = commands.entity(camera);
        camera_commands.insert((DepthPrepass, NormalPrepass));
        // overlay 三角形
        let exists = overlays
            .iter()
            .any(|(_e, overlay)| overlay.camera == camera);
        if exists {
            continue;
        }
        let (Some(materials), Some(meshes)) = (materials.as_mut(), meshes.as_mut()) else {
            bevy::log::warn!(
                "StylizedScreenOutline: asset resources missing; skipping overlay spawn"
            );
            continue;
        };
        let params = StylizedScreenOutlineParams {
            color_intensity: bevy::math::Vec4::new(
                settings.color.red,
                settings.color.green,
                settings.color.blue,
                settings.intensity,
            ),
            thresholds: bevy::math::Vec4::new(
                settings.depth_threshold,
                settings.normal_threshold,
                settings.width,
                0.0,
            ),
        };
        let material = materials.add(StylizedScreenOutlineMaterial { params });
        let mesh = meshes.add(screen_triangle_mesh());
        commands.spawn((
            StylizedScreenOutlineOverlay { camera },
            bevy::prelude::Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
            bevy::prelude::Visibility::Inherited,
        ));
    }
    // StylizedScreenOutline 组件移除：回收本系统授予的 overlay 与 prepass
    // （相机可能已 despawn，移除只对仍存在的实体生效）。
    for camera in removed.read() {
        if let Some((overlay, _)) = overlays.iter().find(|(_, o)| o.camera == camera) {
            commands.entity(overlay).despawn();
        }
        if let Ok(mut entity) = commands.get_entity(camera) {
            entity.remove::<(DepthPrepass, NormalPrepass)>();
        }
    }
}

/// 屏幕覆盖三角形（vertex shader 直接把 position 当 clip 坐标输出，
/// 与相机无关）。
fn screen_triangle_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[-1.0, -1.0, 0.0], [3.0, -1.0, 0.0], [-1.0, 3.0, 0.0]],
    );
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_stylized_core::StylizedSurfaceClass;

    fn resolved(tier: StylizedExecutionTier) -> StylizedResolvedVariant {
        StylizedResolvedVariant {
            class: StylizedSurfaceClass::Cel,
            requested_tier: tier,
            tier,
            tier_reduced: false,
            missing_features: Vec::new(),
        }
    }

    #[test]
    fn outline_mode_gating() {
        // Standard+：请求即所得。
        assert_eq!(
            resolve_outline_mode(
                StylizedOutlineMode::Hybrid,
                Some(&resolved(StylizedExecutionTier::Standard))
            ),
            (StylizedOutlineMode::Hybrid, false)
        );
        // Portable：screen/hybrid 回落 hull 并报告回落。
        assert_eq!(
            resolve_outline_mode(
                StylizedOutlineMode::ScreenSpace,
                Some(&resolved(StylizedExecutionTier::Portable))
            ),
            (StylizedOutlineMode::Hull, true)
        );
        assert_eq!(
            resolve_outline_mode(
                StylizedOutlineMode::Hull,
                Some(&resolved(StylizedExecutionTier::Portable))
            ),
            (StylizedOutlineMode::Hull, false)
        );
        // 无 resolver 结果（headless 默认）→ Portable 基线 → 回落。
        assert_eq!(
            resolve_outline_mode(StylizedOutlineMode::ScreenSpace, None),
            (StylizedOutlineMode::Hull, true)
        );
    }
}
