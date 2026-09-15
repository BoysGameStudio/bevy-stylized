//! `bevy-stylized`：对外稳定 facade（docs/architecture/03）。
//!
//! Facade 只负责：Re-export、Plugin 装配、编译能力路由（ADR-0024）、
//! 稳定公共 API；不复制 core 的任何逻辑。
//!
//! - [`StylizedPlugin`]：装配 core 领域类型与能力管线的入口插件；
//! - [`capability`]：ADR-0022 两阶段能力生命周期（Stage A 配置 +
//!   Stage B 只读解析）；
//! - `gltf` feature（默认开启）：`_NPR_*` glTF 导入层；
//! - `render` feature（默认开启）：Cel@Portable 渲染切片（It-3b）。
//!
//! 最小出图路径（≤10 行 setup，`render` feature）：
//!
//! ```no_run
//! # #[cfg(feature = "render")] {
//! use bevy::prelude::*;
//! use bevy_stylized::prelude::*;
//!
//! App::new()
//!     .add_plugins((DefaultPlugins, StylizedPlugin::default()))
//!     .add_systems(Startup, |mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StylizedCelMaterial>>| {
//!         commands.spawn((Mesh3d(meshes.add(Sphere::new(1.0))), MeshMaterial3d(materials.add(StylizedCelMaterial::default()))));
//!         commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 4.0)));
//!     })
//!     .run();
//! # }
//! ```
//!
//! **注意（ADR-0028，provisional）**：DefaultPlugins 自带的 `GltfPlugin`
//! 不含 `_NPR_*` 注册，加完 DefaultPlugins 再装 `StylizedPlugin` **不能**
//! retroactively 补注册；请用 [`stylized_gltf_plugin`] 替换默认
//! `GltfPlugin`（见该函数文档）。

mod capability;
#[cfg(all(feature = "gltf", feature = "render"))]
pub mod integration;
#[cfg(all(feature = "gltf", feature = "render"))]
pub use integration::{
    StylizedAssetIntegrationPlugin, StylizedMaterialAdoption, StylizedMaterialSet,
};
// `dynamic_quality` 依赖 render crate 的 `StylizedTierOverride`（V2-b 起
// tier override 由 LOD 系统与 governor 共同消费，单一真源在 render）。
#[cfg(feature = "render")]
mod dynamic_quality;

pub mod prelude {
    //! 常用类型的一站式导出。
    #[cfg(feature = "render")]
    pub use crate::dynamic_quality::*;
    #[cfg(feature = "render")]
    pub use crate::stylized_render::*;
    pub use crate::{StylizedPlugin, capability::*, stylized_core::prelude::*};
    #[cfg(feature = "gltf")]
    pub use crate::{stylized_gltf::*, stylized_gltf_plugin};
}

pub use capability::{
    StylizedCapabilityConfig, StylizedResolvedCapabilities, requested_wgpu_features,
    stylized_capabilities_from_wgpu,
};
#[cfg(feature = "render")]
pub use dynamic_quality::{
    StylizedDynamicQuality, StylizedDynamicQualityPlugin, StylizedQualityDecisionEvent,
    StylizedTierOverride, dynamic_quality_system,
};

/// core crate 的整体 re-export（facade 不复制逻辑）。
pub use bevy_stylized_core as stylized_core;
pub use bevy_stylized_core::{
    StylizedCapabilitySet, StylizedExecutionTier, StylizedFallbackPolicy, StylizedGpuFeature,
    StylizedGpuLimits, StylizedInstanceDescriptor, StylizedMaterialDescriptor, StylizedProfile,
    StylizedProfileMetadata, StylizedQualityPolicy, StylizedResolveError, StylizedResolvedVariant,
    StylizedSchemaVersion, StylizedSurfaceClass, StylizedValidationMode, StylizedViewDescriptor,
    resolve_variant, resolve_with_report,
};

/// glTF 导入层的整体 re-export（`gltf` feature）。
#[cfg(feature = "gltf")]
pub use bevy_stylized_gltf as stylized_gltf;
#[cfg(feature = "gltf")]
pub use bevy_stylized_gltf::{
    NPR_VERTEX_ATTRIBUTES, StylizedGltfConfigError, StylizedGltfPluginExt, StylizedImportError,
    StylizedMaterialImport, StylizedSidecar, check_gltf_plugin_not_added, npr_gltf_plugin,
};

/// Genshin reference family profile（`profile-genshin` feature，默认关闭）。
#[cfg(feature = "profile-genshin")]
pub use bevy_stylized_profile_genshin as profile_genshin;
#[cfg(feature = "profile-genshin")]
pub use bevy_stylized_profile_genshin::GenshinProfile;

/// Cel@Portable 渲染切片的整体 re-export（`render` feature）。
#[cfg(feature = "render")]
pub use bevy_stylized_render as stylized_render;
#[cfg(feature = "render")]
pub use bevy_stylized_render::{
    StylizedCelMaterial, StylizedCelParams, StylizedOutline, StylizedOutlineMaterial,
    StylizedOutlineParams, StylizedRenderCapabilities, StylizedRenderPlugin, StylizedViewSettings,
};

use bevy::{prelude::*, render::renderer::RenderDevice};

/// bevy-stylized 入口插件（programmatic / no-glTF 路径的最小装配）。
///
/// - `build`：登记 core 领域类型、写入 [`StylizedCapabilityConfig`] 与
///   `StylizedQualityPolicy` 资源；
/// - `finish`：ADR-0022 Stage B——若 RenderPlugin 已创建 `RenderDevice`，
///   读取其实际 features/limits 并解析 variant；否则写入 Portable 基线的
///   CPU 兜底。It-3b 的渲染切片将消费
///   [`StylizedResolvedCapabilities`](capability::StylizedResolvedCapabilities)。
#[derive(Default)]
pub struct StylizedPlugin {
    /// Stage A 能力配置（也可在 `App` 构建前以同名 resource 覆盖）。
    pub capability_config: StylizedCapabilityConfig,
}

impl Plugin for StylizedPlugin {
    fn build(&self, app: &mut App) {
        // 用户可在 add_plugins 之前 insert_resource 覆盖插件内配置。
        if !app.world().contains_resource::<StylizedCapabilityConfig>() {
            app.insert_resource(self.capability_config.clone());
        }
        app.init_resource::<StylizedQualityPolicyResource>();
        app.register_type::<StylizedMaterialDescriptor>()
            .register_type::<StylizedInstanceDescriptor>()
            .register_type::<StylizedViewDescriptor>()
            .register_type::<StylizedSurfaceClass>()
            .register_type::<StylizedExecutionTier>()
            .register_type::<StylizedValidationMode>();
        #[cfg(feature = "render")]
        app.add_plugins(StylizedRenderPlugin);
    }

    fn finish(&self, app: &mut App) {
        let config = app
            .world()
            .get_resource::<StylizedCapabilityConfig>()
            .cloned()
            .unwrap_or_default();

        // Stage B（ADR-0022）：只读已创建的 RenderDevice，绝不尝试事后启用特性。
        let actual = app
            .get_sub_app_mut(bevy::render::RenderApp)
            .and_then(|render_app| render_app.world().get_resource::<RenderDevice>().cloned())
            .map(|device| stylized_capabilities_from_wgpu(device.features(), &device.limits()))
            .unwrap_or_default(); // headless / 无 RenderPlugin：Portable 基线 CPU 兜底

        let (resolved, report) = match config.resolve(&actual) {
            Ok((variant, report)) => (Ok(variant), Some(report)),
            Err(error) => (Err(error), None),
        };
        // render 切片消费同一份 Stage B 结果（ADR-0022；数据拷贝，不复制逻辑）。
        #[cfg(feature = "render")]
        app.insert_resource(StylizedRenderCapabilities::new(&actual, resolved.clone()));
        app.insert_resource(capability::StylizedResolvedCapabilities {
            actual,
            resolved,
            report,
        });
    }
}

/// Quality policy 的 ECS resource 包装（core 类型保持与 ECS 解耦）。
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct StylizedQualityPolicyResource(pub StylizedQualityPolicy);

/// 返回已注册全部 `_NPR_*` 顶点属性的 `GltfPlugin`。
///
/// **Provisional（ADR-0028 未决）**：当前机制为 builder 配置 +
/// fail-closed 顺序守卫（[`check_gltf_plugin_not_added`]）。用法：
///
/// ```ignore
/// App::new()
///     .add_plugins(DefaultPlugins.build().set(stylized_gltf_plugin()))
///     .add_plugins(StylizedPlugin::default());
/// ```
#[cfg(feature = "gltf")]
#[must_use]
pub fn stylized_gltf_plugin() -> bevy::gltf::GltfPlugin {
    npr_gltf_plugin()
}
