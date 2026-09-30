//! Per-capture golden judgment for the courtyard capture matrix
//! (docs/showcase/05 §9；credential for `labs/scenario-model` coverage）。
//! 每条捕获 = `showcase::capture_spec` 的同一条执行路径（钉参数 → 机位 →
//! 捕获 profile → 判定），判定对象 = `fixtures/golden/showcase/` 的
//! canonical 基线（本平台 overlay 优先解析）。

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    render::{RenderPlugin, renderer::RenderAdapterInfo, settings::RenderCreation},
    window::{ExitCondition, WindowPlugin},
};
use golden_harness::{GoldenProvenance, capture};

fn golden_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/showcase")
}

fn adapter_info(resources: bevy::render::settings::RenderResources) -> (String, String, String) {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin {
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
    app.finish();
    app.cleanup();
    app.world()
        .get_resource::<RenderAdapterInfo>()
        .map(|info| {
            (
                format!("{} ({:?})", info.name, info.backend),
                format!("{} {}", info.driver, info.driver_info),
                format!("{:?}", info.backend),
            )
        })
        .unwrap_or_else(|| ("unknown".into(), "unknown".into(), "unknown".into()))
}

fn judge(capture_id: &str) {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("showcase-golden: no GPU adapter available; skipping");
        return;
    };
    let spec = scenario_model::courtyard_capture_specs()
        .into_iter()
        .find(|spec| spec.capture_id == capture_id)
        .expect("capture spec must be registered");
    let captured = showcase::capture_spec::capture_spec_frame(&spec, resources.clone());
    let regions = showcase::capture_spec::spec_regions(&spec);
    let (gpu, driver, backend) = adapter_info(resources);
    let platform = golden_harness::current_golden_platform(&backend);
    let paths = golden_harness::resolve_baseline_paths(&golden_dir(), capture_id, &platform);
    if !paths.baseline_exists() {
        // 基线缺席 = BLOCKED（不静默通过）；显式 GOLDEN_REBASELINE 才写
        // overlay 候选（canonical 只由参考宿主产出）。
        assert!(
            std::env::var("GOLDEN_REBASELINE").as_deref() == Ok("1"),
            "accepted baseline missing for {capture_id}; capture/accept explicitly (BLOCKED, not PASS)"
        );
        let mut provenance = GoldenProvenance::for_capture(
            "engine-golden-v1",
            None,
            "synthetic-fixtures",
            None,
            "blender-gltf-npr-v1",
            "bevy_stylized.material/v1",
            &format!("showcase/{capture_id}"),
            "bevy-stylized-render-0.1.0",
            &gpu,
            &driver,
            &captured,
        );
        provenance.regions = regions.clone();
        let target = golden_harness::resolve_capture_paths(&golden_dir(), capture_id, &platform);
        golden_harness::write_baseline(&target, &captured, &mut provenance)
            .expect("write baseline");
        eprintln!(
            "[baselined] showcase/{capture_id} -> {}",
            target.png.display()
        );
        return;
    }
    let (baseline, _) = golden_harness::read_baseline(&paths).expect("read baseline");
    let verdict =
        showcase::capture_spec::judge_against_baseline(&spec, &regions, &baseline, &captured);
    eprintln!(
        "[{}] {} | {}{}",
        if verdict.passed {
            "matched"
        } else {
            "exceeded"
        },
        verdict.capture_id,
        verdict.whole_frame,
        if verdict.regions.is_empty() {
            String::new()
        } else {
            format!(" | rois: {}", verdict.regions.join(", "))
        }
    );
    if !verdict.passed
        && std::env::var("GOLDEN_REBASELINE").as_deref() == Ok("1")
        && platform != golden_harness::CANONICAL_GOLDEN_PLATFORM
    {
        // 显式平台 overlay 候选捕获：canonical 基线绝不被覆写。
        let mut provenance = GoldenProvenance::for_capture(
            "engine-golden-v1",
            None,
            "synthetic-fixtures",
            None,
            "blender-gltf-npr-v1",
            "bevy_stylized.material/v1",
            &format!("showcase/{capture_id}"),
            "bevy-stylized-render-0.1.0",
            &gpu,
            &driver,
            &captured,
        );
        provenance.regions = regions;
        let overlay = golden_harness::resolve_capture_paths(&golden_dir(), capture_id, &platform);
        golden_harness::write_baseline(&overlay, &captured, &mut provenance)
            .expect("write overlay candidate");
        eprintln!(
            "[overlay-candidate] {platform}/showcase/{capture_id} (canonical baseline unchanged)"
        );
        return;
    }
    assert!(
        verdict.passed,
        "showcase/{capture_id} exceeded: {}",
        verdict.whole_frame
    );
}

macro_rules! capture_case {
    ($name:ident, $capture:literal) => {
        #[test]
        #[ignore = "requires GPU; run the explicit GPU lane"]
        fn $name() {
            judge($capture);
        }
    };
}

capture_case!(showcase_golden_courtyard_trio, "courtyard-trio");
capture_case!(showcase_golden_courtyard_ramp, "courtyard-ramp");
capture_case!(
    showcase_golden_courtyard_outline_hull,
    "courtyard-outline-hull"
);
// 显式展开（不经宏）：本测试是 tests/current-contracts.json 的
// required_tests 锚点（旧 outline_advanced_hull_screen_hybrid 的语义
// 由本捕获覆盖；presence 检查按 `fn <name>` 文本匹配）。
#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn showcase_golden_courtyard_outline_hybrid() {
    judge("courtyard-outline-hybrid");
}
capture_case!(showcase_golden_courtyard_lighting, "courtyard-lighting");
capture_case!(showcase_golden_courtyard_flags, "courtyard-flags");
capture_case!(showcase_golden_courtyard_stage, "courtyard-stage");
capture_case!(showcase_golden_courtyard_lod, "courtyard-lod");
capture_case!(showcase_golden_courtyard_dusk, "courtyard-dusk");
capture_case!(showcase_golden_courtyard_night, "courtyard-night");
capture_case!(showcase_golden_courtyard_post, "courtyard-post");
capture_case!(showcase_golden_courtyard_stress, "courtyard-stress");
// V4 cel 特性补全（05 §12 V4 行；隔离上下文捕获合同见验收记录 v3 条目）。
capture_case!(showcase_golden_courtyard_rim, "courtyard-rim");
capture_case!(showcase_golden_courtyard_shadow, "courtyard-shadow");
capture_case!(showcase_golden_courtyard_palette, "courtyard-palette");
capture_case!(showcase_golden_courtyard_metal, "courtyard-metal");
capture_case!(showcase_golden_courtyard_cutout, "courtyard-cutout");
