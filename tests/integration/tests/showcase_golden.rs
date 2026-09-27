//! Per-scene golden judgment for the previously demonstration-only PC showcase
//! scenes (credential for `labs/scenario-model` coverage; scenes render through
//! the scenes registry → driver capture path, judged against the accepted
//! baselines in `fixtures/golden/showcase/`).

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    render::{RenderPlugin, renderer::RenderAdapterInfo, settings::RenderCreation},
    window::{ExitCondition, WindowPlugin},
};
use golden_harness::{GoldenProvenance, Tolerance, capture, diff_rgba8};

fn golden_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/showcase")
}

fn adapter_info(resources: bevy::render::settings::RenderResources) -> (String, String, String) {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
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

fn judge(scene: &str) {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("showcase-golden: no GPU adapter available; skipping");
        return;
    };
    let case = showcase::all_cases()
        .into_iter()
        .find(|case| case.name == scene)
        .expect("showcase case must be registered");
    let captured = showcase::capture_case(&case, resources.clone());
    let (gpu, driver, backend) = adapter_info(resources);
    let platform = golden_harness::current_golden_platform(&backend);
    let paths = golden_harness::resolve_baseline_paths(&golden_dir(), scene, &platform);
    let mut provenance = GoldenProvenance::for_capture(
        "engine-golden-v1",
        None,
        "synthetic-fixtures",
        None,
        "blender-gltf-npr-v1",
        "bevy_stylized.material/v1",
        &format!("showcase/{scene}"),
        "bevy-stylized-render-0.1.0",
        &gpu,
        &driver,
        &captured,
    );
    if !paths.baseline_exists() {
        assert!(
            std::env::var("GOLDEN_REBASELINE").as_deref() == Ok("1"),
            "accepted baseline missing; capture/accept explicitly"
        );
        let target = golden_harness::resolve_capture_paths(&golden_dir(), scene, &platform);
        golden_harness::write_baseline(&target, &captured, &mut provenance)
            .expect("write baseline");
        eprintln!("[baselined] showcase/{scene} -> {}", target.png.display());
        return;
    }
    let (baseline, _) = golden_harness::read_baseline(&paths).expect("read baseline");
    let stats = diff_rgba8(&baseline.data, &captured.data).expect("same size");
    let report = golden_harness::DiffReport::evaluate(
        scene.to_owned(),
        paths.png.to_string_lossy().into_owned(),
        stats,
        Tolerance::default(),
    );
    eprintln!(
        "[{}] showcase/{scene} max_delta={} mean_delta={:.4} psnr={:.2}dB",
        if report.passed { "matched" } else { "exceeded" },
        report.stats.max_channel_delta,
        report.stats.mean_absolute_delta,
        report.stats.psnr_db,
    );
    if !report.passed
        && std::env::var("GOLDEN_REBASELINE").as_deref() == Ok("1")
        && platform != golden_harness::CANONICAL_GOLDEN_PLATFORM
    {
        // Explicit platform-overlay capture: the canonical baseline stays
        // untouched; the overlay image is a candidate until reviewed and
        // accepted (fixtures/README.md).
        let overlay = golden_harness::resolve_capture_paths(&golden_dir(), scene, &platform);
        golden_harness::write_baseline(&overlay, &captured, &mut provenance)
            .expect("write overlay candidate");
        eprintln!("[overlay-candidate] {platform}/showcase/{scene} (canonical baseline unchanged)");
        return;
    }
    assert!(
        report.passed,
        "showcase/{scene} exceeded: {:?}",
        report.violations
    );
}

macro_rules! scene_case {
    ($name:ident, $scene:literal) => {
        #[test]
        #[ignore = "requires GPU; run the explicit GPU lane"]
        fn $name() {
            judge($scene);
        }
    };
}

scene_case!(showcase_golden_overview, "overview");
scene_case!(showcase_golden_lighting, "lighting");
scene_case!(showcase_golden_hair, "hair");
scene_case!(showcase_golden_metal, "metal");
scene_case!(showcase_golden_reference, "reference");
scene_case!(showcase_golden_performance, "performance");
scene_case!(showcase_golden_lod, "lod");
scene_case!(showcase_golden_environment_day, "environment-day");
scene_case!(showcase_golden_environment_dusk, "environment-dusk");
scene_case!(showcase_golden_environment_night, "environment-night");
scene_case!(showcase_golden_effects, "effects");
// The remaining catalog scenes also get showcase-path baselines: the browser
// lane judges against this set uniformly (their own suites keep their
// specialized capture setups and coverage credentials).
scene_case!(showcase_golden_ramp, "ramp");
scene_case!(showcase_golden_face, "face");
scene_case!(showcase_golden_outline, "outline");
scene_case!(showcase_golden_animation, "animation");
scene_case!(showcase_golden_post, "post");
scene_case!(showcase_golden_stress, "stress");
scene_case!(showcase_golden_outline_advanced, "outline-advanced");
