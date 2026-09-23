//! Whole-pipeline 预算测试（docs/rendering/01；ADR-0002 Accepted 2026-09-23）。
//!
//! 对每条 (Class × Tier) 变体统计 per-stage 总用量：
//!
//! ```text
//! View（真实 view layout 反射） + Mesh + Instance + Material（AsBindGroup
//! 反射） + Profile + Safety Margin
//! ```
//!
//! 对照 core 的三个 baseline 限制模型（WebGPU Core / GLES3.1 / WebGL2）。
//!
//! 门禁规则：
//!
//! - Cel@Portable / Character@Portable 必须 fit WebGL2 模型（硬断言）；
//! - Hero@Portable 全量如实记录；若超限则在报告中标注（不隐藏），并给出
//!   fallback 变体的逻辑用量（hero fallback 关闭 matcap/spec ramp/packed
//!   normal——注意：fixed layout 下纹理槽位仍被绑定，真正减绑需要 per-tier
//!   材质变体，见报告 note）。
//!
//! 输出：`target/tmp/interim-budget/pipeline_budget.json`（gitignored）。
//! 需要 GPU（RenderDevice + 真实 view layout）；无 GPU graceful skip。

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    pbr::{MeshPipelineViewLayoutKey, MeshPipelineViewLayouts},
    render::{
        RenderApp, RenderPlugin,
        render_resource::ShaderStages,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use bevy_stylized::stylized_core::budget::{
    INTERIM_SAFETY_MARGIN, STYLIZED_LIMIT_MODELS, StylizedBudgetLimits, StylizedStageUsage,
};
use bevy_stylized::stylized_render::{
    budget, character::StylizedCharacterMaterial, hero::StylizedHeroMaterial,
};

#[derive(serde::Serialize)]
struct VariantBudget {
    class: &'static str,
    tier: &'static str,
    material: MaterialCounts,
    view_fragment: MaterialCounts,
    total_fragment: MaterialCounts,
    fits: Vec<(String, bool)>,
}

#[derive(Clone, Copy, serde::Serialize)]
struct MaterialCounts {
    textures: u32,
    samplers: u32,
    uniform_buffers: u32,
    storage_buffers: u32,
}

impl From<StylizedStageUsage> for MaterialCounts {
    fn from(u: StylizedStageUsage) -> Self {
        Self {
            textures: u.textures,
            samplers: u.samplers,
            uniform_buffers: u.uniform_buffers,
            storage_buffers: u.storage_buffers,
        }
    }
}

#[derive(serde::Serialize)]
struct PipelineBudgetReport {
    note: &'static str,
    safety_margin: MaterialCounts,
    variants: Vec<VariantBudget>,
}

fn headless_render_app(resources: RenderResources) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "assets".to_string(),
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..Default::default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Manual(resources),
                synchronous_pipeline_compilation: true,
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..Default::default()
            }),
    );
    app.add_plugins(StylizedPlugin::default());
    app.finish();
    app.cleanup();
    app.update();
    app.update();
    app
}

/// 把（实测设备的）view 用量按目标模型换算：storage 不可用的模型上
/// clustered lights 等退回 uniform（Bevy 的真实 downlevel 行为，
/// `AVAILABLE_STORAGE_BUFFER_BINDINGS` 分支）。
fn view_usage_for_model(
    measured: StylizedStageUsage,
    model: &StylizedBudgetLimits,
) -> StylizedStageUsage {
    if model.max_storage_buffers_per_stage > 0 {
        measured
    } else {
        StylizedStageUsage {
            textures: measured.textures,
            samplers: measured.samplers,
            uniform_buffers: measured.uniform_buffers + measured.storage_buffers,
            storage_buffers: 0,
        }
    }
}

fn variant_budget(
    class: &'static str,
    tier: &'static str,
    material: StylizedStageUsage,
    view_fragment: StylizedStageUsage,
) -> VariantBudget {
    let fits = STYLIZED_LIMIT_MODELS
        .iter()
        .map(|model| {
            // mesh / view 绑定均按目标模型评估（storage 可用性）。
            let mesh = budget::mesh_usage_for_model(model, false);
            let view = view_usage_for_model(view_fragment, model);
            let total = view + mesh + material + budget::instance_usage() + INTERIM_SAFETY_MARGIN;
            (model.name.to_string(), total.fits_within(model))
        })
        .collect();
    let reference_total = view_usage_for_model(view_fragment, &STYLIZED_LIMIT_MODELS[0])
        + material
        + INTERIM_SAFETY_MARGIN;
    VariantBudget {
        class,
        tier,
        material: material.into(),
        view_fragment: view_fragment.into(),
        total_fragment: reference_total.into(),
        fits,
    }
}

/// Hero@Portable fallback 变体的逻辑用量（fallback 关闭 matcap / spec ramp /
/// packed normal 三条路径；注意 fixed layout 仍绑定槽位——见测试文档）。
fn hero_fallback_usage() -> StylizedStageUsage {
    StylizedStageUsage {
        textures: 5, // base + ilm + face_sdf + ramp + optional face shadow control
        samplers: 5,
        uniform_buffers: 1,
        storage_buffers: 0,
    }
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn whole_pipeline_budget_fits_baseline_models() {
    let Some(resources) = golden_harness::probe_render_resources() else {
        eprintln!("pipeline-budget: no GPU adapter available; skipping");
        return;
    };
    let mut app = headless_render_app(resources.clone());
    let render_device = resources.0.clone();

    // View layout：真实反射（空 key = 无 prepass/MSAA/OIT 的最小 view layout）。
    let view_fragment = {
        let render_app = app.get_sub_app_mut(RenderApp).expect("render app");
        let layouts = render_app
            .world()
            .get_resource::<MeshPipelineViewLayouts>()
            .expect("MeshPipelineViewLayouts registered by RenderPlugin");
        let layout = layouts.get_view_layout(MeshPipelineViewLayoutKey::empty());
        budget::count_layout_entries(&layout.main_layout.entries, ShaderStages::FRAGMENT)
    };
    eprintln!(
        "[budget] view fragment stage: {}T/{}S/{}U/{}SB",
        view_fragment.textures,
        view_fragment.samplers,
        view_fragment.uniform_buffers,
        view_fragment.storage_buffers,
    );

    let variants = vec![
        variant_budget(
            "cel",
            "portable",
            budget::count_material_usage::<StylizedCelMaterial>(
                &render_device,
                ShaderStages::FRAGMENT,
            ),
            view_fragment,
        ),
        variant_budget(
            "character",
            "portable",
            budget::count_material_usage::<StylizedCharacterMaterial>(
                &render_device,
                ShaderStages::FRAGMENT,
            ),
            view_fragment,
        ),
        variant_budget(
            "hero",
            "portable",
            budget::count_material_usage::<StylizedHeroMaterial>(
                &render_device,
                ShaderStages::FRAGMENT,
            ),
            view_fragment,
        ),
        variant_budget(
            "hero-fallback",
            "portable",
            hero_fallback_usage(),
            view_fragment,
        ),
        variant_budget(
            "outline",
            "portable",
            budget::count_material_usage::<StylizedOutlineMaterial>(
                &render_device,
                ShaderStages::FRAGMENT,
            ),
            view_fragment,
        ),
    ];

    for variant in &variants {
        eprintln!(
            "[budget] {}@{}: material {}T/{}S/{}U; total-fragment {}T/{}S/{}U/{}SB; fits: {:?}",
            variant.class,
            variant.tier,
            variant.material.textures,
            variant.material.samplers,
            variant.material.uniform_buffers,
            variant.total_fragment.textures,
            variant.total_fragment.samplers,
            variant.total_fragment.uniform_buffers,
            variant.total_fragment.storage_buffers,
            variant.fits,
        );
    }

    let report = PipelineBudgetReport {
        note: "whole-pipeline budget (ADR-0002 Accepted); \
               view = reflected mesh view layout (empty key); mesh evaluated \
               per target model; hero-fallback 为逻辑用量（fixed layout 仍绑定 \
               全部槽位，真正减绑需 per-tier 材质变体）",
        safety_margin: INTERIM_SAFETY_MARGIN.into(),
        variants,
    };
    let out_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/interim-budget");
    std::fs::create_dir_all(&out_dir).expect("create budget dir");
    std::fs::write(
        out_dir.join("pipeline_budget.json"),
        serde_json::to_string_pretty(&report).expect("serialize"),
    )
    .expect("write pipeline budget report");

    // 硬门禁：Cel/Character/Outline/Hero-fallback @ Portable 必须 fit WebGL2。
    for variant in &report.variants {
        let webgl2_fit = variant
            .fits
            .iter()
            .find(|(name, _)| name == "webgl2")
            .map(|(_, fits)| *fits)
            .expect("webgl2 model present");
        match variant.class {
            "hero" => {
                // 全量 Hero@Portable：如实记录（可能超限），不作为硬门禁；
                // 其 fallback 变体（hero-fallback）必须 fit。
                if !webgl2_fit {
                    eprintln!(
                        "[budget] NOTE: hero@portable exceeds the WebGL2 model — \
                         记录在案（fallback 变体见 hero-fallback 行）"
                    );
                }
            }
            _ => assert!(
                webgl2_fit,
                "{}@{} must fit the WebGL2 model (fits: {:?})",
                variant.class, variant.tier, variant.fits
            ),
        }
    }
}

/// UBO packing / sampler dedup 审计（docs/rendering/01 "Uniform Packing /
/// Sampler Deduplication"）。
///
/// - 每个材质必须恰好 1 个 packed UBO（禁止每 feature 一个 UBO）；
/// - sampler：当前每纹理槽位 1 个 sampler 绑定（AsBindGroup derive 不支持
///   共享绑定）——按 state 共享的 dedup 是已记录的优化机会（本测试把
///   1:1 现状钉死为审计基线，dedup 落地时应更新此处并附测量）。
#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn uniform_packing_and_sampler_audit() {
    let Some(resources) = golden_harness::probe_render_resources() else {
        eprintln!("packing-audit: no GPU adapter available; skipping");
        return;
    };
    let render_device = resources.0.clone();
    let audit = [
        (
            "StylizedCelMaterial",
            budget::count_material_usage::<StylizedCelMaterial>(
                &render_device,
                ShaderStages::FRAGMENT,
            ),
        ),
        (
            "StylizedCharacterMaterial",
            budget::count_material_usage::<StylizedCharacterMaterial>(
                &render_device,
                ShaderStages::FRAGMENT,
            ),
        ),
        (
            "StylizedHeroMaterial",
            budget::count_material_usage::<StylizedHeroMaterial>(
                &render_device,
                ShaderStages::FRAGMENT,
            ),
        ),
        (
            "StylizedOutlineMaterial",
            budget::count_material_usage::<StylizedOutlineMaterial>(
                &render_device,
                ShaderStages::FRAGMENT,
            ),
        ),
    ];
    for (name, usage) in audit {
        assert_eq!(
            usage.uniform_buffers, 1,
            "{name} must pack into exactly one UBO (never one-UBO-per-feature)"
        );
        assert_eq!(
            usage.samplers, usage.textures,
            "{name}: sampler dedup not implemented — 1 sampler per texture slot              (audit baseline; sharing by state is a recorded optimization)"
        );
    }
}
