//! `bevy-stylized-render`：Cel/Character/Hero 材质、实例传输、view、
//! LOD、描边和后处理的运行时实现。
//!
//! **Provisional（ADR-0001 未决）**：本 crate 是建立在 Bevy `Material`
//! 组合路径上的薄层临时 backend；公共语义全部挂在 `bevy-stylized-core`
//! 的 descriptor 上，ADR-0001 三方对比（It-6）结论引发的返工被限制在
//! 本 crate 内部。
//!
//! 内容：
//!
//! - [`material`]：[`StylizedCelMaterial`]——Base Color / Ramp（纹理或解析
//!   fallback）/ ILM(AO) / Toon Lighting（half-lambert + ramp）/ Rim；
//! - [`character`]：[`StylizedCharacterMaterial`]——Cel + Face SDF /
//!   Hair 高光 / Rim / Emission（Character class，docs/rendering/01）；
//! - [`hero`]：Hero 材质与显式家族光照参数；
//! - [`post`] / [`screen_outline`]：后处理与屏幕描边；
//! - [`outline`]：[`StylizedOutlineMaterial`] + [`StylizedOutline`]——
//!   inverted hull（顶点外扩 + front-face cull）；
//! - [`instance`]：[`StylizedInstanceData`]——Instance 层数据经 MeshTag
//!   位打包传输（provisional pending ADR-0004），不破坏自动合批；
//! - [`instance_table`]：GPU Instance Database（V2-e）——StorageTable
//!   通道（Standard+ 能力门控）：MeshTag 携带 24-bit 槽位索引，
//!   `ShaderBuffer` 表携带全精度记录；
//! - [`lod`]：per-entity NPR LOD（V2-b）——距离分带 → tier 上限/feature
//!   削减（经 MeshTag flags 位，零材质变异）+ 可选几何换 mesh；
//! - [`view`]：[`StylizedViewSettings`]——core `StylizedViewDescriptor`
//!   → Bevy view-layer 对象（key light / ambient / exposure）的薄同步；
//! - [`capability`]：ADR-0022 Stage B 结果的消费与 fail-closed 检查。
//!
//! Portable 不变量：不请求任何可选 GPU 特性；不依赖 storage buffer /
//! compute / bindless；固定 binding；shader 按 Class×Tier 分编译单元
//! （`cel/portable_forward.wgsl`、`character/character_forward.wgsl`、
//! `outline/portable_outline.wgsl` + 共享 `common/npr_common.wgsl`）。

pub mod budget;
pub mod capability;
pub mod character;
pub mod face_input;
pub mod hero;
pub mod host_inputs;
pub mod host_material;
pub mod instance;
pub mod instance_table;
pub mod lod;
pub mod material;
pub mod outline;
pub mod outline_order;
pub mod physical;
pub mod post;
pub mod presentation;
pub mod screen_outline;
pub mod skinning;
pub mod view;
pub use face_input::{StylizedFaceInput, StylizedFaceInputStatus};

pub use budget::{
    count_layout_entries, count_material_usage, instance_usage, mesh_usage_for_model,
};
pub use capability::StylizedRenderCapabilities;
pub use character::{
    CHARACTER_PORTABLE_FORWARD_SHADER_PATH, StylizedCharacterMaterial, StylizedCharacterParams,
    StylizedCharacterTableMaterial, apply_character_portable_fallback,
    character_params_for_variant,
};
pub use hero::{
    HERO_FORWARD_SHADER_PATH, StylizedGenshinLightingParams, StylizedHeroDebugView,
    StylizedHeroMaterial, StylizedHeroParams, StylizedHeroSurfaceControl,
    apply_hero_portable_fallback, hero_params_for_variant,
};
pub use instance::{
    StylizedInstanceData, StylizedInstanceSlot, StylizedInstanceTransport,
    StylizedInstanceTransportMode, select_transport,
};
pub use instance_table::{
    INSTANCE_RECORD_STRIDE, StylizedInstanceTable, StylizedInstanceTableEntry,
    pack_instance_record, sync_instance_table,
};
pub use lod::{
    INSTANCE_FLAG_DROP_HERO_EXTRAS, INSTANCE_FLAG_FACE_HARD_STEP, INSTANCE_FLAG_HAIR_SPECULAR_OFF,
    StylizedLodMeshes, StylizedLodPolicyResource, StylizedTierOverride, lod_flags_for, sync_lod,
};
pub use material::{
    CEL_FLAG_ILM_TEXTURE, CEL_FLAG_RAMP_TEXTURE, CEL_FLAG_RIM, CEL_PORTABLE_FORWARD_SHADER_PATH,
    StylizedCelMaterial, StylizedCelParams,
};
pub use outline::{
    OUTLINE_PORTABLE_SHADER_PATH, StylizedOutline, StylizedOutlineMaterial, StylizedOutlineParams,
    StylizedOutlineReplica,
};
pub use post::{
    STYLIZED_FULLSCREEN_SHADER_PATH, StylizedDisplayTransform, StylizedFullscreenEffect,
    StylizedPostProcess,
};
pub use screen_outline::{
    SCREEN_OUTLINE_SHADER_PATH, StylizedOutlineMode, StylizedScreenOutline,
    StylizedScreenOutlineMaterial, StylizedScreenOutlineParams, resolve_outline_mode,
};
pub use view::{
    StylizedKeyLight, StylizedViewCamera, StylizedViewScope, StylizedViewSettings,
    environment_distance_fog, exposure_component, resolve_ambient_light,
};

use bevy::{
    app::{App, Plugin, PostUpdate},
    asset::{Assets, embedded_asset},
    pbr::MaterialPlugin,
    prelude::IntoScheduleConfigs,
};

/// 渲染插件：注册 Cel/Character/Hero、描边、后处理 shader 与同步系统。
///
/// 由 facade 的 `StylizedPlugin`（`render` feature）自动装配；也可单独
/// 加入。需要宿主 App 已配置 RenderPlugin 才能真正出图（headless 无
/// RenderPlugin 时 MaterialPlugin 自动退化为仅资产注册）。
#[derive(Default)]
pub struct StylizedRenderPlugin;

impl Plugin for StylizedRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<presentation::PresentationSchedule>()
            .configure_sets(
                PostUpdate,
                presentation::NativePresentationSystems.run_if(presentation::native_schedule),
            );
        embedded_asset!(app, "shaders/cel/portable_forward.wgsl");
        embedded_asset!(app, "shaders/character/character_forward.wgsl");
        embedded_asset!(app, "shaders/outline/portable_outline.wgsl");
        embedded_asset!(app, "shaders/common/npr_common.wgsl");
        embedded_asset!(app, "shaders/common/host_inputs.wgsl");
        embedded_asset!(app, "shaders/common/complete_skin_prepass.wgsl");
        embedded_asset!(app, "shaders/hero/hero_forward.wgsl");
        embedded_asset!(app, "shaders/post/stylized_fullscreen.wgsl");
        embedded_asset!(app, "shaders/outline/screen_outline.wgsl");
        app.add_plugins((
            MaterialPlugin::<StylizedCelMaterial>::default(),
            MaterialPlugin::<StylizedOutlineMaterial>::default(),
            MaterialPlugin::<StylizedCharacterMaterial>::default(),
            MaterialPlugin::<character::StylizedCharacterTableMaterial>::default(),
            MaterialPlugin::<StylizedHeroMaterial>::default(),
            MaterialPlugin::<screen_outline::StylizedScreenOutlineMaterial>::default(),
        ));
        post::register(app);
        outline_order::register(app);
        app.register_type::<StylizedCelMaterial>()
            .register_type::<StylizedOutlineMaterial>()
            .register_type::<StylizedCharacterMaterial>()
            .register_type::<StylizedHeroMaterial>()
            .register_type::<screen_outline::StylizedScreenOutline>()
            .register_type::<outline::StylizedOutline>()
            .register_type::<instance::StylizedInstanceData>()
            .register_type::<face_input::StylizedFaceInput>()
            .register_type::<face_input::StylizedFaceInputStatus>();
        // Instance 传输通道：依 Stage B 解析结果选择（Standard+ →
        // StorageTable，见 instance::select_transport；表通道落地见
        // instance_table 模块）。
        let mode = instance::select_transport(
            app.world()
                .get_resource::<StylizedRenderCapabilities>()
                .and_then(|c| c.resolved.as_ref()),
        );
        app.insert_resource(instance::StylizedInstanceTransportMode(mode));
        view::register(app);
        lod::register(app);
        instance_table::register(app);
        app.add_systems(
            PostUpdate,
            (
                material::sync_cel_material_flags.after(bevy::asset::AssetEventSystems),
                skinning::update_complete_skin_bounds
                    .after(bevy::asset::AssetEventSystems)
                    .before(bevy::camera::visibility::VisibilitySystems::CalculateBounds),
                character::sync_character_material_flags.after(bevy::asset::AssetEventSystems),
                character::sync_character_table_material_flags
                    .after(bevy::asset::AssetEventSystems),
                hero::sync_hero_material_flags.after(bevy::asset::AssetEventSystems),
                outline::sync_outline_replicas
                    .in_set(presentation::NativePresentationSystems)
                    .after(bevy::app::AnimationSystems)
                    .before(bevy::transform::TransformSystems::Propagate)
                    .before(bevy::camera::visibility::VisibilitySystems::CalculateBounds),
                screen_outline::sync_screen_outline,
                face_input::sync_face_inputs
                    .in_set(presentation::NativePresentationSystems)
                    .after(hero::sync_hero_material_flags)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .before(instance::sync_instance_tags),
                face_input::restore_removed_face_inputs
                    .in_set(presentation::NativePresentationSystems)
                    .after(face_input::sync_face_inputs)
                    .before(instance::sync_instance_tags),
                instance::sync_instance_tags.in_set(presentation::NativePresentationSystems),
            ),
        );
    }

    fn finish(&self, app: &mut App) {
        // ADR-0022 Stage B 消费：fail-closed（debug 断言 + error 日志）。
        capability::debug_assert_capabilities_satisfied(
            app.world().get_resource::<StylizedRenderCapabilities>(),
        );
        // `npr_common.wgsl` 是纯 WGSL import 模块：没有 pipeline 经
        // `ShaderRef` 引用它，必须主动加载才能让 shader composer 解析
        // import 图（无 render 插件时仅注册 embedded asset）。
        if app
            .world()
            .contains_resource::<Assets<bevy::shader::Shader>>()
        {
            let handle: bevy::prelude::Handle<bevy::shader::Shader> = bevy::asset::load_embedded_asset!(
                app.world_mut(),
                "shaders/common/npr_common.wgsl"
            );
            core::mem::forget(handle);
            let handle: bevy::prelude::Handle<bevy::shader::Shader> = bevy::asset::load_embedded_asset!(
                app.world_mut(),
                "shaders/common/host_inputs.wgsl"
            );
            core::mem::forget(handle);
        }
    }
}
