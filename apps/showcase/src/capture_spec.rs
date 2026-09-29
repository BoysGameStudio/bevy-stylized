//! 捕获规格驱动（docs/showcase/05 §9，阶段 3）：按 `CaptureSpec` 捕获
//! courtyard——钉参数（`CourtyardSettings`）→ 机位（`COURTYARD_CAMERA_PRESETS`
//! 经 driver 相机配方覆盖）→ 既有捕获 profile（fixed_step{22} → observe）
//! → 512² RGBA。
//!
//! 基线写出走既有 golden 机制：canonical 基线只由参考宿主（Linux/Vulkan）
//! 产出；本机（macos-metal 等）经 `GOLDEN_REBASELINE=1` 写平台 overlay
//! 候选。regions 的屏幕矩形由分区 AABB 经捕获相机投影自动计算（不手工
//! 标定）；ROI 阈值首轮锁 max_channel_delta=2、max_fail_ratio=0.005。
//!
//! 判定闭环：同机重捕 vs 基线——strict 整图走既有 Tolerance 判定，loose
//! 整图走 non-black + luma/chroma 包络（包络由基线统计派生），ROI 走
//! 区域双层门限。

use bevy::app::PluginGroup;
use bevy::math::{Mat4, Vec3};
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::settings::RenderResources;
use golden_harness::{
    GoldenProvenance, GoldenRegion, Tolerance, capture, current_golden_platform,
    project_aabb_to_screen, read_baseline, resolve_baseline_paths, resolve_capture_paths,
    validate_regions, write_baseline,
};
use scenario_model::capture_spec::{CaptureSpec, WholeFrameJudgment, courtyard_capture_specs};
use scene_runtime::{CaseDriver, DriverLimits, run_to_completion};
use scenes::metadata::{COURTYARD_CAMERA_PRESETS, COURTYARD_REGION_AABBS};
use scenes::{
    CameraAttachment, CourtyardFlagsPose, CourtyardOutlineMode, CourtyardSettings,
    CourtyardTimeOfDay,
};
use validation::diff::{RegionRect, RegionTolerance, diff_rgba8_region, evaluate_region};
use validation::frame::{FrameEnvelope, frame_stats};
use validation::image::ImageView;

/// ROI 首轮锁定阈值（验收记录复核用）。
const ROI_CHANNEL_DELTA: u8 = 2;
/// ROI 首轮锁定失败像素比。
const ROI_FAIL_RATIO: f64 = 0.005;

/// 捕获规格 → CourtyardSettings（钉值；未钉键保持 §0 默认）。
pub fn settings_from_spec(spec: &CaptureSpec) -> CourtyardSettings {
    let mut settings = CourtyardSettings::default();
    for (key, value) in &spec.parameters {
        match key.as_str() {
            "time_of_day" => {
                settings.time_of_day = match value.as_str().expect("validated") {
                    "day" => CourtyardTimeOfDay::Day,
                    "dusk" => CourtyardTimeOfDay::Dusk,
                    _ => CourtyardTimeOfDay::Night,
                };
            }
            "outline_mode" => {
                settings.outline_mode = match value.as_str().expect("validated") {
                    "off" => CourtyardOutlineMode::Off,
                    "hybrid" => CourtyardOutlineMode::Hybrid,
                    _ => CourtyardOutlineMode::Hull,
                };
            }
            "crowd" => settings.crowd = value.as_u64().expect("validated") as u32,
            "post" => settings.post = value.as_bool().expect("validated"),
            "flags" => {
                settings.flags = match value.as_str().expect("validated") {
                    "posed" => CourtyardFlagsPose::Posed,
                    _ => CourtyardFlagsPose::Rest,
                };
            }
            _ if key.starts_with("trio.face.") => {
                settings.face_threshold = if value.is_null() {
                    None
                } else {
                    Some(value.as_f64().expect("validated") as f32)
                };
            }
            _ => {}
        }
    }
    settings
}

/// 捕获相机配方：机位表 eye/look + 参数驱动的附件（与 builders::courtyard
/// 的附件规则同源：hybrid → ScreenOutline 优先，否则 post → PostProcess）。
/// 捕获规格的相机配方（机位表 eye/look + 参数驱动附件）。
pub fn camera_recipe_for_spec(
    spec: &CaptureSpec,
    settings: &CourtyardSettings,
) -> scenes::CameraRecipe {
    let preset = COURTYARD_CAMERA_PRESETS
        .iter()
        .find(|preset| preset.id == spec.camera)
        .expect("capture spec camera must resolve");
    let attachment = match (settings.outline_mode, settings.post) {
        (CourtyardOutlineMode::Hybrid, _) => CameraAttachment::ScreenOutline,
        (_, true) => CameraAttachment::PostProcess(
            bevy_stylized::stylized_render::post::StylizedPostProcess {
                vignette_strength: 0.5,
                chromatic_aberration: 0.004,
                display_transform:
                    bevy_stylized::stylized_render::post::StylizedDisplayTransform::SimpleCompress,
                ..Default::default()
            },
        ),
        (_, false) => CameraAttachment::None,
    };
    scenes::CameraRecipe::with_attachment(preset.eye, preset.look, attachment)
}

/// 捕获相机的 view-projection（Camera3d 默认投影：fov 45° 垂直，1:1；
/// x/y NDC 与 reversed-Z 实现一致，深度映射不影响屏幕矩形）。
fn capture_view_proj(spec: &CaptureSpec) -> Mat4 {
    let preset = COURTYARD_CAMERA_PRESETS
        .iter()
        .find(|preset| preset.id == spec.camera)
        .expect("capture spec camera must resolve");
    Mat4::perspective_rh(45.0_f32.to_radians(), 1.0, 0.1, 200.0)
        * Mat4::look_at_rh(preset.eye, preset.look, Vec3::Y)
}

/// 按捕获规格捕获一帧（512² RGBA）。
pub fn capture_spec_frame(
    spec: &CaptureSpec,
    resources: RenderResources,
) -> golden_harness::CapturedImage {
    let case = crate::all_cases()
        .into_iter()
        .find(|case| case.name == spec.scene.as_str())
        .expect("capture spec scene must be registered");
    let settings = settings_from_spec(spec);
    let recipe = camera_recipe_for_spec(spec, &settings);
    let mut app = crate::headless_app(
        resources,
        case.asset_root.as_deref().unwrap_or("assets"),
        spec.tier.or(case.tier),
    );
    crate::default_view(app.world_mut());
    app.insert_resource(settings);
    let binding = scenes::registry::bindings()
        .into_iter()
        .find(|binding| binding.id == case.name)
        .expect("binding");
    let mut driver = CaseDriver::new(
        app.world(),
        scenario_model::ident::SessionId::new("courtyard-capture-spec").expect("session id"),
        binding,
        crate::capture_profile_graph(),
        DriverLimits {
            implicit_readiness: false,
            ..DriverLimits::default()
        },
    )
    .expect("capture profile must be a valid plan")
    .with_camera_recipe(recipe);
    if let Err(error) = run_to_completion(&mut app, &mut driver, 700) {
        panic!("capture {} failed: {error}", spec.capture_id);
    }
    let mut receipts = driver.drain_observations();
    receipts.pop().expect("one observation").image
}

/// 规格的 ROI 区域（AABB 投影自动计算 + 首轮锁定阈值）。
pub fn spec_regions(spec: &CaptureSpec) -> Vec<GoldenRegion> {
    let view_proj = capture_view_proj(spec);
    let regions: Vec<GoldenRegion> = spec
        .rois
        .iter()
        .map(|id| {
            let aabb = COURTYARD_REGION_AABBS
                .iter()
                .find(|region| region.id == id)
                .expect("capture spec roi must resolve");
            let rect = project_aabb_to_screen(aabb.min, aabb.max, view_proj, 512, 512)
                .unwrap_or_else(|| panic!("region {id} not visible in {}", spec.camera));
            GoldenRegion {
                id: id.clone(),
                rect,
                max_channel_delta: ROI_CHANNEL_DELTA,
                max_fail_ratio: ROI_FAIL_RATIO,
            }
        })
        .collect();
    assert_eq!(
        validate_regions(&regions, 512, 512),
        Ok(()),
        "{} rois must be pairwise non-overlapping",
        spec.capture_id
    );
    regions
}

/// 整图宽松包络（由基线统计派生；数值记录进验收文档）。
fn loose_envelope(
    baseline: &golden_harness::CapturedImage,
) -> (validation::frame::FrameStats, FrameEnvelope) {
    let view = ImageView::new(baseline.width, baseline.height, &baseline.data).expect("view");
    let stats = frame_stats(view);
    let envelope = FrameEnvelope {
        min_mean_luma: (stats.mean_luma * 0.6).max(1.0),
        max_mean_luma: stats.mean_luma * 1.4 + 10.0,
        min_stddev_luma: (stats.stddev_luma * 0.6).max(5.0),
        min_mean_chroma: 0.0,
        max_mean_chroma: stats.mean_chroma * 1.5 + 20.0,
    };
    (stats, envelope)
}

/// 单条捕获的判定结果（验收记录的机器可读行）。
#[derive(Clone, Debug)]
pub struct CaptureVerdict {
    /// 捕获 ID。
    pub capture_id: String,
    /// 整图判定描述。
    pub whole_frame: String,
    /// ROI 判定描述（逐区域）。
    pub regions: Vec<String>,
    /// 是否通过。
    pub passed: bool,
    /// 整图统计（包络钉值来源）。
    pub frame_stats: Option<validation::frame::FrameStats>,
}

/// 同机重捕 vs 基线的判定（整图 loose/strict + ROI 双层门限）。
pub fn judge_against_baseline(
    spec: &CaptureSpec,
    regions: &[GoldenRegion],
    baseline: &golden_harness::CapturedImage,
    candidate: &golden_harness::CapturedImage,
) -> CaptureVerdict {
    let mut passed = true;
    let mut whole_frame = String::new();
    match spec.whole_frame {
        WholeFrameJudgment::Strict => {
            let stats =
                golden_harness::diff_rgba8(&baseline.data, &candidate.data).expect("same size");
            let report = golden_harness::DiffReport::evaluate(
                spec.capture_id.clone(),
                spec.capture_id.clone(),
                stats,
                Tolerance::default(),
            );
            whole_frame = format!(
                "strict: max_delta={} mean_delta={:.4} psnr={:.2}dB -> {}",
                stats.max_channel_delta,
                stats.mean_absolute_delta,
                stats.psnr_db,
                if report.passed { "PASS" } else { "FAIL" }
            );
            passed &= report.passed;
        }
        WholeFrameJudgment::Loose => {
            let (baseline_stats, envelope) = loose_envelope(baseline);
            let candidate_view =
                ImageView::new(candidate.width, candidate.height, &candidate.data).expect("view");
            let candidate_stats = frame_stats(candidate_view);
            let verdict = validation::frame::evaluate_frame_envelope(candidate_stats, envelope);
            let verdict_ok = verdict.is_ok();
            whole_frame = format!(
                "loose: luma {:.1}±env / stddev {:.1} / chroma {:.1} -> {}",
                candidate_stats.mean_luma,
                candidate_stats.stddev_luma,
                candidate_stats.mean_chroma,
                if verdict_ok { "PASS" } else { "FAIL" }
            );
            if let Err(violations) = verdict {
                whole_frame.push_str(&format!(" {violations:?}"));
            }
            passed &= verdict_ok;
            let _ = baseline_stats;
        }
    }
    let mut region_lines = Vec::new();
    let baseline_view =
        ImageView::new(baseline.width, baseline.height, &baseline.data).expect("view");
    let candidate_view =
        ImageView::new(candidate.width, candidate.height, &candidate.data).expect("view");
    for region in regions {
        let rect = RegionRect {
            x: region.rect.x,
            y: region.rect.y,
            width: region.rect.width,
            height: region.rect.height,
        };
        let tolerance = RegionTolerance {
            max_channel_delta: region.max_channel_delta,
            max_fail_ratio: region.max_fail_ratio,
        };
        let stats =
            diff_rgba8_region(baseline_view, candidate_view, rect, tolerance).expect("region diff");
        let verdict = evaluate_region(stats, tolerance);
        passed &= verdict.is_ok();
        region_lines.push(format!(
            "{}: max_delta={} fail_ratio={:.5} -> {}",
            region.id,
            stats.max_channel_delta,
            stats.fail_ratio,
            if verdict.is_ok() { "PASS" } else { "FAIL" }
        ));
    }
    CaptureVerdict {
        capture_id: spec.capture_id.clone(),
        whole_frame,
        regions: region_lines,
        passed,
        frame_stats: Some(frame_stats(baseline_view)),
    }
}

/// 运行捕获规格（`all` = 全部 10 条，串行）：捕获 →（GOLDEN_REBASELINE=1
/// 且基线缺席时写 overlay 候选）→ 同机重捕 → 判定。返回进程退出码；
/// 捕获图同时写 `target/stylized/showcase/captures/<id>.png` 供检视。
pub fn run_capture_specs(filter: &str) -> i32 {
    let specs = courtyard_capture_specs();
    let selected: Vec<&CaptureSpec> = if filter == "all" {
        specs.iter().collect()
    } else {
        match specs.iter().find(|spec| spec.capture_id == filter) {
            Some(spec) => vec![spec],
            None => {
                eprintln!("unknown capture spec: {filter}");
                eprintln!(
                    "available: all, {}",
                    specs
                        .iter()
                        .map(|spec| spec.capture_id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                return 2;
            }
        }
    };
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("capture-spec: no GPU adapter available; BLOCKED (not PASS)");
        return 3;
    };
    // 平台身份必须来自实际渲染后端（绝不猜）。
    let backend = {
        let mut probe = bevy::prelude::App::new();
        probe.add_plugins(
            bevy::DefaultPlugins
                .build()
                .set(bevy::asset::AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..Default::default()
                })
                .set(bevy::render::RenderPlugin {
                    render_creation: bevy::render::settings::RenderCreation::Manual(
                        resources.clone(),
                    ),
                    synchronous_pipeline_compilation: true,
                    ..Default::default()
                })
                .set(bevy::window::WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    ..Default::default()
                }),
        );
        probe.finish();
        probe.cleanup();
        probe
            .world()
            .get_resource::<RenderAdapterInfo>()
            .map(|info| {
                (
                    format!("{} ({:?})", info.name, info.backend),
                    format!("{} {}", info.driver, info.driver_info),
                    format!("{:?}", info.backend),
                )
            })
            .unwrap_or_else(|| ("unknown".into(), "unknown".into(), "unknown".into()))
    };
    let (gpu, driver, backend) = backend;
    let platform = current_golden_platform(&backend);
    let golden_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/showcase");
    let staging_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/stylized/showcase/captures");
    std::fs::create_dir_all(&staging_dir).expect("create captures dir");
    let rebase = std::env::var("GOLDEN_REBASELINE").as_deref() == Ok("1");
    let mut failed = 0;
    let mut verdicts = Vec::new();
    for spec in selected {
        let regions = spec_regions(spec);
        let captured = capture_spec_frame(spec, resources.clone());
        std::fs::write(
            staging_dir.join(format!("{}.png", spec.capture_id)),
            golden_harness::encode_png(&captured).expect("encode png"),
        )
        .expect("write staging png");
        let mut provenance = GoldenProvenance::for_capture(
            "engine-golden-v1",
            None,
            "synthetic-fixtures",
            None,
            "blender-gltf-npr-v1",
            "bevy_stylized.material/v1",
            &format!("showcase/{}", spec.capture_id),
            "bevy-stylized-render-0.1.0",
            &gpu,
            &driver,
            &captured,
        );
        provenance.regions = regions.clone();
        if !resolve_baseline_paths(&golden_dir, &spec.capture_id, &platform).baseline_exists() {
            if !rebase {
                eprintln!(
                    "[{}] baseline missing; rerun with GOLDEN_REBASELINE=1 (BLOCKED)",
                    spec.capture_id
                );
                failed += 1;
                continue;
            }
            let target = resolve_capture_paths(&golden_dir, &spec.capture_id, &platform);
            write_baseline(&target, &captured, &mut provenance).expect("write baseline");
            eprintln!(
                "[baselined] {}/showcase/{} -> {}",
                platform,
                spec.capture_id,
                target.png.display()
            );
        }
        // 判定闭环：同机重捕 vs 基线（路径在写出后重解析——刚写入的
        // overlay 在写出前对 resolve_baseline_paths 不可见）。
        let paths = resolve_baseline_paths(&golden_dir, &spec.capture_id, &platform);
        let recapture = capture_spec_frame(spec, resources.clone());
        let (baseline, _) = read_baseline(&paths).expect("read baseline");
        let verdict = judge_against_baseline(spec, &regions, &baseline, &recapture);
        println!(
            "[{}] {} | {}{}",
            if verdict.passed { "PASS" } else { "FAIL" },
            verdict.capture_id,
            verdict.whole_frame,
            if verdict.regions.is_empty() {
                String::new()
            } else {
                format!(" | rois: {}", verdict.regions.join(", "))
            }
        );
        if !verdict.passed {
            failed += 1;
        }
        verdicts.push(verdict);
    }
    if failed > 0 {
        eprintln!("capture-spec: {failed} capture(s) failed judgment");
        1
    } else {
        0
    }
}
