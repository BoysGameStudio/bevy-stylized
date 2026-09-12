//! V2-g：Regression/Performance Lab（docs/roadmap/02 §8；roadmap V2
//! §Full Regression/Performance Lab）。
//!
//! 自动 sweep：`Count × Resolution × Tier × Lighting × Pose`（Backend 轴
//! 本机只有 Vulkan 可测——其余 backend 如实记录为 `skipped-unavailable`，
//! 不伪造数据）。
//!
//! ## 矩阵与子采样（文档化）
//!
//! ```text
//! crowd（Cel 实例场景）:  Count{1,100,1000} × Res{512²,720p} × Tier{P,S}
//!                         × Lighting{day,night}            = 24 cells
//! strip（蒙皮条带场景）:  Pose{bind,animated} × Res × Tier × Lighting
//!                         （Count 固定 1）                  = 16 cells
//! high_probe：Tier=High 探测 2 cells（记录解析结果是否不同于 Standard）
//! 总计 42 cells；测量 15 warm-up + 40 帧（较 benchmarks 的 20+60 子采样，
//! sweep 规模下保持分钟级墙钟时间）。
//! ```
//!
//! Golden 子集：4 个代表 cell 在测量尾帧捕获，与
//! `fixtures/golden-local/regression-lab/` 基线 diff（首跑写基线）。
//!
//! 输出：单个 JSON 报告（docs/performance/targets.md 的 target record
//! 扩展 sweep 轴字段）+ 可选 `--gate` 回归门禁（mean 帧时回归 >10% 即
//! 失败，exit code 1）。

use std::time::Instant;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    color::LinearRgba,
    image::Image,
    math::{Mat4, Quat, Vec3},
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    pbr::MeshMaterial3d,
    prelude::{Camera3d, Component, Mesh, Mesh3d, Query, Transform, With},
    render::{
        RenderPlugin,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::capture;

/// sweep 报告 schema 标识。
pub const SWEEP_SCHEMA: &str = "bevy_stylized.sweep/v1";
/// warm-up 帧数（子采样：benchmarks 为 20）。
pub const WARMUP_FRAMES: usize = 15;
/// 测量帧数（子采样：benchmarks 为 60）。
pub const MEASURED_FRAMES: usize = 40;
/// 回归门禁阈值（mean 帧时劣化比例）。
pub const REGRESSION_THRESHOLD: f64 = 0.10;

/// sweep 分辨率轴。
pub const RESOLUTIONS: [[u32; 2]; 2] = [[512, 512], [1280, 720]];
/// sweep tier 轴（请求档；解析档如实记录）。
pub const TIERS: [StylizedExecutionTier; 2] = [
    StylizedExecutionTier::Portable,
    StylizedExecutionTier::Standard,
];
/// sweep 光照轴。
pub const LIGHTINGS: [&str; 2] = ["day", "night"];
/// crowd 场景的 Count 轴。
pub const COUNTS: [u32; 3] = [1, 100, 1000];
/// strip 场景的 Pose 轴。
pub const POSES: [&str; 2] = ["bind", "animated"];

fn tier_name(tier: StylizedExecutionTier) -> &'static str {
    match tier {
        StylizedExecutionTier::Portable => "portable",
        StylizedExecutionTier::Standard => "standard",
        StylizedExecutionTier::High => "high",
    }
}

/// 帧时间统计（毫秒；Serialize+Deserialize——gate 需要读基线）。
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SweepFrameStats {
    /// 平均。
    pub mean_ms: f64,
    /// 中位数。
    pub median_ms: f64,
    /// p95。
    pub p95_ms: f64,
    /// 最小。
    pub min_ms: f64,
    /// 最大。
    pub max_ms: f64,
}

impl SweepFrameStats {
    fn from_samples(samples: &mut [f64]) -> Self {
        samples.sort_by(f64::total_cmp);
        let n = samples.len();
        Self {
            mean_ms: samples.iter().sum::<f64>() / n as f64,
            median_ms: samples[n / 2],
            p95_ms: samples[(n as f64 * 0.95) as usize],
            min_ms: samples[0],
            max_ms: samples[n - 1],
        }
    }
}

/// 单个 sweep cell 的记录。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SweepCell {
    /// 稳定 cell ID（gate 按它对齐基线）。
    pub target_id: String,
    /// 场景族（crowd / strip）。
    pub scene: String,
    /// Count 轴（strip 固定 1）。
    pub count: u32,
    /// Pose 轴（crowd 为 "static"）。
    pub pose: String,
    /// 分辨率。
    pub resolution: [u32; 2],
    /// 请求档。
    pub requested_tier: String,
    /// 实际解析档（记录请求与解析的差异）。
    pub resolved_tier: String,
    /// 光照。
    pub lighting: String,
    /// Backend（本机恒 vulkan）。
    pub backend: String,
    /// Surface Class。
    pub class: String,
    /// 帧统计（主线程 update 耗时）。
    pub frame_stats: SweepFrameStats,
    /// golden 子集：捕获状态（None = 非子集 cell）。
    pub golden: Option<GoldenCellStatus>,
    /// GPU 侧计时（未启用）。
    pub gpu_timing: Option<String>,
}

/// golden 子集 cell 的状态。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GoldenCellStatus {
    /// baselined / matched / exceeded。
    pub status: String,
    /// 最大通道差（baselined 时为 0）。
    pub max_channel_delta: u8,
    /// PSNR dB。
    pub psnr_db: f64,
}

/// Backend 轴的诚实记录。
#[derive(Clone, Debug, serde::Serialize)]
pub struct BackendCoverage {
    /// vulkan：已运行。
    pub vulkan: &'static str,
    /// 其余 backend 一律 skipped-unavailable（单适配器机器）。
    pub others: &'static str,
}

/// 单个回归记录。
#[derive(Clone, Debug, serde::Serialize)]
pub struct GateFailure {
    /// cell ID。
    pub target_id: String,
    /// 基线 mean ms。
    pub baseline_mean_ms: f64,
    /// 本次 mean ms。
    pub current_mean_ms: f64,
    /// 劣化比例。
    pub delta_ratio: f64,
}

/// Gate 结果。
#[derive(Clone, Debug, serde::Serialize)]
pub struct GateOutcome {
    /// 基线文件路径。
    pub baseline: String,
    /// 阈值（比例）。
    pub threshold: f64,
    /// 回归的 cell。
    pub regressions: Vec<GateFailure>,
    /// 基线中存在但本次未运行的 cell（如实记录，不算回归）。
    pub missing_cells: Vec<String>,
}

/// 完整 sweep 报告。
#[derive(Clone, Debug, serde::Serialize)]
pub struct SweepReport {
    /// schema 标识。
    pub schema: &'static str,
    /// ADR-0029 证据声明。
    pub evidence_policy: &'static str,
    /// 采集时间（Unix 秒）。
    pub captured_unix_seconds: u64,
    /// 引擎 commit。
    pub engine_git_commit: String,
    /// GPU。
    pub gpu: String,
    /// 驱动。
    pub driver: String,
    /// OS。
    pub os: String,
    /// Backend 覆盖。
    pub backends: BackendCoverage,
    /// 帧数（warm-up, 测量）。
    pub frames: [usize; 2],
    /// cell 记录。
    pub cells: Vec<SweepCell>,
    /// gate 结果（--gate 时填充）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateOutcome>,
}

// ---------------------------------------------------------------------------
// 场景构建
// ---------------------------------------------------------------------------

/// 蒙皮条带的关节标记（animated pose 的摆动目标）。
#[derive(Component)]
struct StripRootJoint;

/// animated pose 的确定性帧计数（不用墙钟时间，保证 golden 可重复）。
#[derive(bevy::prelude::Resource)]
struct StripAnimFrame(u32);

/// strip 场景：程序化双骨骼蒙皮条带（与 showcase `animation` 用例同一
/// 构建逻辑；此处内联以控制相机/pose 轴——showcase 版本自带机位）。
fn build_strip(app: &mut App, animated: bool) {
    let (cols, rows) = (2usize, 16usize);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut joints: Vec<[u16; 4]> = Vec::new();
    let mut weights = Vec::new();
    for r in 0..=rows {
        let v = r as f32 / rows as f32;
        for c in 0..=cols {
            let u = c as f32 / cols as f32;
            positions.push([(u - 0.5) * 0.8, v * 2.4 - 1.2, 0.0]);
            normals.push([0.0, 0.0, 1.0]);
            uvs.push([u, v]);
            let j = u16::from(v > 0.5);
            joints.push([j, 0, 0, 0]);
            weights.push([1.0, 0.0, 0.0, 0.0]);
        }
    }
    let mut indices = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let i = (r * (cols + 1) + c) as u32;
            let w = cols as u32 + 1;
            indices.extend_from_slice(&[i, i + 1, i + w, i + 1, i + w + 1, i + w]);
        }
    }
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_JOINT_INDEX,
        bevy::mesh::VertexAttributeValues::Uint16x4(joints),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_JOINT_WEIGHT,
        bevy::mesh::VertexAttributeValues::Float32x4(weights),
    );
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);

    let root = app
        .world_mut()
        .spawn((Transform::from_xyz(0.0, -1.2, 0.0), StripRootJoint))
        .id();
    let tip = app
        .world_mut()
        .spawn((Transform::from_xyz(0.0, 0.0, 0.0),))
        .id();
    app.world_mut().entity_mut(root).add_child(tip);
    let inverse_bindposes = app
        .world_mut()
        .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
        .add(SkinnedMeshInverseBindposes::from(vec![
            Mat4::from_translation(Vec3::new(0.0, 1.2, 0.0)),
            Mat4::IDENTITY,
        ]));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial::default());
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        SkinnedMesh {
            inverse_bindposes,
            joints: vec![root, tip],
        },
        Transform::IDENTITY,
    ));
    if animated {
        // 帧计数驱动（不用 Time：捕获确定性 = golden 可重复）。
        app.insert_resource(StripAnimFrame(0));
        app.add_systems(
            bevy::prelude::Update,
            |mut frame: bevy::prelude::ResMut<StripAnimFrame>,
             mut joints: Query<&mut Transform, With<StripRootJoint>>| {
                frame.0 += 1;
                let angle = (frame.0 as f32 * 0.05).sin() * 0.5;
                for mut transform in &mut joints {
                    transform.rotation = Quat::from_rotation_z(angle);
                }
            },
        );
    }
}

// ---------------------------------------------------------------------------
// 测量
// ---------------------------------------------------------------------------

/// cell 的光照设置。
fn apply_lighting(app: &mut App, lighting: &str) {
    let descriptor = match lighting {
        "night" => StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.4, 0.8, 0.6).normalize(),
            key_light_color: LinearRgba::new(0.5, 0.6, 0.9, 1.0),
            exposure: 1.0,
            day_night_factor: 0.0,
            ..StylizedViewDescriptor::default()
        },
        _ => StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.4, 0.8, 0.6).normalize(),
            key_light_color: LinearRgba::WHITE,
            exposure: 1.0,
            day_night_factor: 1.0,
            ..StylizedViewDescriptor::default()
        },
    };
    *app.world_mut().resource_mut::<StylizedViewSettings>() = StylizedViewSettings(descriptor);
}

/// golden 子集的代表 cell（target_id）。
pub const GOLDEN_SUBSET: [&str; 4] = [
    "crowd/n1000/1280x720/portable/day",
    "crowd/n100/512x512/standard/night",
    "strip/animated/1280x720/portable/day",
    "strip/bind/512x512/standard/night",
];

/// cell ID（stable；gate 按它对齐基线）。
#[must_use]
pub fn cell_target_id(
    scene: &str,
    count: u32,
    pose: &str,
    resolution: [u32; 2],
    tier: StylizedExecutionTier,
    lighting: &str,
) -> String {
    let axis = if scene == "crowd" {
        format!("n{count}")
    } else {
        pose.to_string()
    };
    format!(
        "{scene}/{axis}/{}x{}/{}/{lighting}",
        resolution[0],
        resolution[1],
        tier_name(tier)
    )
}

/// 单个 cell 的测量产出。
pub struct CellOutcome {
    /// 帧统计。
    pub stats: SweepFrameStats,
    /// 实际解析档。
    pub resolved_tier: String,
    /// golden 子集状态（仅 [`GOLDEN_SUBSET`] cell）。
    pub golden: Option<GoldenCellStatus>,
    /// golden 捕获 PNG（子集 cell）。
    pub png: Option<Vec<u8>>,
}

/// 测量单个 cell（pub：smoke 测试与未来的单点复测复用；完整 sweep 见
/// [`run_sweep`]）。
#[allow(clippy::too_many_arguments)]
pub fn measure_cell(
    resources: &RenderResources,
    scene: &str,
    count: u32,
    pose: &str,
    resolution: [u32; 2],
    requested_tier: StylizedExecutionTier,
    lighting: &str,
    golden_dir: &std::path::Path,
) -> CellOutcome {
    let target_id = cell_target_id(scene, count, pose, resolution, requested_tier, lighting);
    let mut app = App::new();
    app.insert_resource(StylizedCapabilityConfig {
        requested_tier,
        ..StylizedCapabilityConfig::default()
    });
    app.add_plugins(
        // headless：显式禁用 WinitPlugin 之外的窗口（同 benchmarks 的约定）。
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "assets".to_string(),
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..Default::default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Manual(resources.clone()),
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
    capture::install(&mut app);
    app.finish();
    app.cleanup();

    let resolved_tier = app
        .world()
        .get_resource::<StylizedResolvedCapabilities>()
        .and_then(|caps| caps.resolved.as_ref().ok().map(|v| tier_name(v.tier)))
        .unwrap_or("unknown")
        .to_string();

    apply_lighting(&mut app, lighting);
    let target = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        capture::create_capture_target(&mut images, resolution[0], resolution[1])
    };
    app.world_mut().spawn((
        Camera3d::default(),
        capture::capture_target_bundle(&target),
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Transform::from_xyz(0.0, 2.0, 18.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    match scene {
        // crowd：复用 benchmarks 的实例场景构建器（link, don't duplicate）。
        "crowd" => benchmarks::build_portable_instances(&mut app, count),
        "strip" => build_strip(&mut app, pose == "animated"),
        other => panic!("unknown scene {other}"),
    }

    for _ in 0..WARMUP_FRAMES {
        app.update();
    }
    let mut samples = Vec::with_capacity(MEASURED_FRAMES);
    for _ in 0..MEASURED_FRAMES {
        let start = Instant::now();
        app.update();
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    // golden 子集：尾帧捕获 + 基线 diff（首跑写基线）。
    let (golden, png) = if GOLDEN_SUBSET.contains(&target_id.as_str()) {
        let request = capture::request_capture(app.world_mut(), target);
        let mut frame = None;
        for _ in 0..600 {
            app.update();
            if let Some(f) = capture::try_take(app.world_mut(), request.clone()) {
                frame = Some(f);
                break;
            }
        }
        let frame = frame.expect("golden capture timed out");
        let png = golden_harness::encode_png(&frame).expect("encode png");
        (
            Some(golden_subset_eval(golden_dir, &target_id, &png)),
            Some(png),
        )
    } else {
        (None, None)
    };

    CellOutcome {
        stats: SweepFrameStats::from_samples(&mut samples),
        resolved_tier,
        golden,
        png,
    }
}

/// 运行完整 sweep。返回 `None` = 无 GPU（graceful skip）。
///
/// `golden_dir`：golden 子集基线目录（存在则 diff，不存在则写基线）。
pub fn run_sweep(golden_dir: &std::path::Path) -> Option<(SweepReport, Vec<(String, Vec<u8>)>)> {
    let resources = capture::probe_render_resources()?;
    let mut cells = Vec::new();
    let mut golden_pngs: Vec<(String, Vec<u8>)> = Vec::new();

    let mut run_cell = |scene: &str,
                        count: u32,
                        pose: &str,
                        resolution: [u32; 2],
                        tier: StylizedExecutionTier,
                        lighting: &str| {
        let outcome = measure_cell(
            &resources, scene, count, pose, resolution, tier, lighting, golden_dir,
        );
        let target_id = cell_target_id(scene, count, pose, resolution, tier, lighting);
        if let Some(png) = &outcome.png {
            golden_pngs.push((target_id.clone(), png.clone()));
        }
        eprintln!(
            "[sweep] {target_id}: mean={:.3}ms p95={:.3}ms resolved={}",
            outcome.stats.mean_ms, outcome.stats.p95_ms, outcome.resolved_tier
        );
        cells.push(SweepCell {
            target_id,
            scene: scene.to_string(),
            count,
            pose: pose.to_string(),
            resolution,
            requested_tier: tier_name(tier).to_string(),
            resolved_tier: outcome.resolved_tier,
            lighting: lighting.to_string(),
            backend: "vulkan".to_string(),
            class: "Cel".to_string(),
            frame_stats: outcome.stats,
            golden: outcome.golden,
            gpu_timing: None,
        });
    };

    for &count in &COUNTS {
        for &resolution in &RESOLUTIONS {
            for &tier in &TIERS {
                for lighting in LIGHTINGS {
                    run_cell("crowd", count, "static", resolution, tier, lighting);
                }
            }
        }
    }
    for &pose in &POSES {
        for &resolution in &RESOLUTIONS {
            for &tier in &TIERS {
                for lighting in LIGHTINGS {
                    run_cell("strip", 1, pose, resolution, tier, lighting);
                }
            }
        }
    }
    // High 探测（子采样：两个代表 cell；记录解析档是否与 Standard 不同）。
    for (scene, count, pose) in [("crowd", 1000, "static"), ("strip", 1, "animated")] {
        run_cell(
            scene,
            count,
            pose,
            [1280, 720],
            StylizedExecutionTier::High,
            "day",
        );
    }

    let gpu = {
        let info = &resources.2;
        format!("{} ({:?})", info.name, info.backend)
    };
    let driver = {
        // RenderResources 不含 driver；从一次性 probe app 读
        // RenderAdapterInfo 代价高——driver 以空串如实记录（gpu 字段已含
        // backend；provenance 完备性见 docs/performance/targets.md）。
        String::new()
    };
    Some((
        SweepReport {
            schema: SWEEP_SCHEMA,
            evidence_policy: "ADR-0029: fixed sweep cells, labeled axes; regression gate = mean \
                              frame time +10% vs baseline on the same device",
            captured_unix_seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            engine_git_commit: golden_harness::engine_git_commit(),
            gpu,
            driver,
            os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
            backends: BackendCoverage {
                vulkan: "run",
                others: "skipped-unavailable (single-adapter machine; no dx12/metal/gl)",
            },
            frames: [WARMUP_FRAMES, MEASURED_FRAMES],
            cells,
            gate: None,
        },
        golden_pngs,
    ))
}

/// golden 子集评估：基线不存在则写入；存在则 diff。
fn golden_subset_eval(
    golden_dir: &std::path::Path,
    target_id: &str,
    png_bytes: &[u8],
) -> GoldenCellStatus {
    std::fs::create_dir_all(golden_dir).expect("create golden dir");
    let file = golden_dir.join(format!("{}.png", target_id.replace('/', "_")));
    if !file.exists() {
        std::fs::write(&file, png_bytes).expect("write golden baseline");
        return GoldenCellStatus {
            status: "baselined".into(),
            max_channel_delta: 0,
            psnr_db: 999.0, // JSON 无 inf；999 = 无 diff 的哨兵
        };
    }
    let baseline_bytes = std::fs::read(&file).expect("read golden baseline");
    let baseline = golden_harness::decode_png(&baseline_bytes).expect("decode baseline");
    let current = golden_harness::decode_png(png_bytes).expect("decode current");
    let stats = golden_harness::diff_rgba8(&baseline.data, &current.data).expect("same size");
    let report = golden_harness::DiffReport::evaluate(
        target_id.to_string(),
        file.to_string_lossy().into_owned(),
        stats,
        golden_harness::Tolerance::default(),
    );
    if !report.passed {
        // 诊断：超容差时把当前捕获落盘（golden 目录旁，不覆盖基线）。
        let current_path = golden_dir.join(format!("{}.current.png", target_id.replace('/', "_")));
        std::fs::write(current_path, png_bytes).expect("write current capture");
    }
    GoldenCellStatus {
        status: if report.passed { "matched" } else { "exceeded" }.into(),
        max_channel_delta: report.stats.max_channel_delta,
        // JSON 无 inf：同图时 psnr=inf，钳到哨兵 999。
        psnr_db: report.stats.psnr_db.min(999.0),
    }
}

// ---------------------------------------------------------------------------
// Gate
// ---------------------------------------------------------------------------

/// 基线文件的可读形态（gate 输入）。
#[derive(serde::Deserialize)]
pub struct BaselineReport {
    /// cell 记录。
    pub cells: Vec<SweepCell>,
}

/// 回归门禁：逐 cell 对齐 target_id，mean 帧时劣化超过阈值即记录。
/// 返回（是否有回归，结果）。
#[must_use]
pub fn gate_against(baseline: &BaselineReport, current: &SweepReport) -> (bool, GateOutcome) {
    let mut regressions = Vec::new();
    let mut missing = Vec::new();
    for base_cell in &baseline.cells {
        let Some(current_cell) = current
            .cells
            .iter()
            .find(|c| c.target_id == base_cell.target_id)
        else {
            missing.push(base_cell.target_id.clone());
            continue;
        };
        let base_mean = base_cell.frame_stats.mean_ms;
        let current_mean = current_cell.frame_stats.mean_ms;
        if base_mean > 0.0 {
            let delta = (current_mean - base_mean) / base_mean;
            if delta > REGRESSION_THRESHOLD {
                regressions.push(GateFailure {
                    target_id: base_cell.target_id.clone(),
                    baseline_mean_ms: base_mean,
                    current_mean_ms: current_mean,
                    delta_ratio: delta,
                });
            }
        }
    }
    let failed = !regressions.is_empty();
    (
        failed,
        GateOutcome {
            baseline: String::new(),
            threshold: REGRESSION_THRESHOLD,
            regressions,
            missing_cells: missing,
        },
    )
}

/// 由 Unix 秒推导 ISO 日期（civil-from-days；无 chrono 依赖）。
#[must_use]
pub fn civil_date_from_unix(unix_seconds: u64) -> String {
    let days = (unix_seconds / 86_400) as i64;
    // Howard Hinnant 的 civil_from_days。
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

// ---------------------------------------------------------------------------
// 测试辅助（纯逻辑，无 GPU）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(id: &str, mean_ms: f64) -> SweepCell {
        SweepCell {
            target_id: id.into(),
            scene: "crowd".into(),
            count: 100,
            pose: "static".into(),
            resolution: [512, 512],
            requested_tier: "portable".into(),
            resolved_tier: "portable".into(),
            lighting: "day".into(),
            backend: "vulkan".into(),
            class: "Cel".into(),
            frame_stats: SweepFrameStats {
                mean_ms,
                ..SweepFrameStats::default()
            },
            golden: None,
            gpu_timing: None,
        }
    }

    fn report(cells: Vec<SweepCell>) -> SweepReport {
        SweepReport {
            schema: SWEEP_SCHEMA,
            evidence_policy: "",
            captured_unix_seconds: 0,
            engine_git_commit: String::new(),
            gpu: String::new(),
            driver: String::new(),
            os: String::new(),
            backends: BackendCoverage {
                vulkan: "run",
                others: "skipped-unavailable",
            },
            frames: [WARMUP_FRAMES, MEASURED_FRAMES],
            cells,
            gate: None,
        }
    }

    #[test]
    fn gate_flags_regression_over_threshold() {
        let baseline = BaselineReport {
            cells: vec![cell("a", 10.0), cell("b", 5.0)],
        };
        // a: 10 → 11.2（+12% > 10%）回归；b: 5 → 5.4（+8%）通过。
        let current = report(vec![cell("a", 11.2), cell("b", 5.4)]);
        let (failed, outcome) = gate_against(&baseline, &current);
        assert!(failed);
        assert_eq!(outcome.regressions.len(), 1);
        assert_eq!(outcome.regressions[0].target_id, "a");
        assert!(outcome.missing_cells.is_empty());
    }

    #[test]
    fn gate_passes_within_threshold_and_records_missing() {
        let baseline = BaselineReport {
            cells: vec![cell("a", 10.0), cell("gone", 3.0)],
        };
        let current = report(vec![cell("a", 10.9)]);
        let (failed, outcome) = gate_against(&baseline, &current);
        assert!(!failed, "+9% 在阈值内");
        assert_eq!(outcome.missing_cells, vec!["gone"]);
    }

    #[test]
    fn cell_target_id_is_stable() {
        let id = cell_target_id(
            "crowd",
            1000,
            "static",
            [1280, 720],
            StylizedExecutionTier::Portable,
            "day",
        );
        assert_eq!(id, "crowd/n1000/1280x720/portable/day");
        let id = cell_target_id(
            "strip",
            1,
            "animated",
            [512, 512],
            StylizedExecutionTier::Standard,
            "night",
        );
        assert_eq!(id, "strip/animated/512x512/standard/night");
    }
}
