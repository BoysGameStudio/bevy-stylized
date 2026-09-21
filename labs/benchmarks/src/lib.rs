//! 固定 benchmark 场景（It-8；ADR-0029 证据格式）。
//!
//! Release gate 的 4 个固定场景（docs/release/02）：
//!
//! - `hero_1`：1 个 Hero 材质球（authored ramp + face SDF + matcap 全开）；
//! - `portable_100` / `portable_500` / `portable_1000`：StylizedCelMaterial
//!   实例（同 mesh+material，MeshTag 逐实例差异）。
//!
//! 测量：headless 离屏 1280×720，20 帧 warm-up + 60 帧测量，记录
//! 主线程 update 时间（mean/median/p95/min/max）。GPU 侧计时未启用
//! （不引入不稳定 API；如实记录为 None）。
//!
//! 输出 JSON 遵循 docs/performance/targets.md 的 target record 字段。

use std::time::Instant;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    color::LinearRgba,
    image::Image,
    math::{Vec3, primitives::Sphere},
    pbr::MeshMaterial3d,
    prelude::{Camera3d, Mesh, Mesh3d, Transform},
    render::{
        RenderPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        renderer::RenderAdapterInfo,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::capture;

/// 固定测量分辨率。
pub const WIDTH: u32 = 1280;
/// 固定测量分辨率。
pub const HEIGHT: u32 = 720;
/// Warm-up 帧数。
pub const WARMUP_FRAMES: usize = 20;
/// 测量帧数。
pub const MEASURED_FRAMES: usize = 60;

/// 帧时间统计（毫秒）。
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct FrameStats {
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

impl FrameStats {
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

/// 单个场景的 benchmark 记录（docs/performance/targets.md 的 target
/// record 字段子集 + 实测值；`Not qualified` 项显式置 null/标注）。
#[derive(Clone, Debug, serde::Serialize)]
pub struct BenchmarkRecord {
    /// Target ID。
    pub target_id: String,
    /// 场景名。
    pub scene: String,
    /// Surface Class。
    pub class: String,
    /// Execution Tier。
    pub tier: String,
    /// Backend。
    pub backend: String,
    /// 分辨率（固定，无动态分辨率）。
    pub resolution: [u32; 2],
    /// 帧统计（主线程 update 耗时）。
    pub frame_stats: FrameStats,
    /// GPU 侧计时（未启用——不引入不稳定 API）。
    pub gpu_timing: Option<String>,
    /// warm-up / 测量帧数。
    pub frames: [usize; 2],
}

/// 一次 benchmark 运行的完整报告。
#[derive(Clone, Debug, serde::Serialize)]
pub struct BenchmarkReport {
    /// ADR-0029 证据声明。
    pub evidence_policy: &'static str,
    /// 采集时间（Unix 秒）。
    pub captured_unix_seconds: u64,
    /// 引擎 commit。
    pub engine_git_commit: String,
    /// GPU（名称 + backend）。
    pub gpu: String,
    /// 驱动。
    pub driver: String,
    /// OS。
    pub os: String,
    /// 场景记录。
    pub records: Vec<BenchmarkRecord>,
}

fn headless_app(resources: RenderResources) -> App {
    let mut app = App::new();
    app.add_plugins(
        // headless：显式禁用 WinitPlugin（feature unification 下它可能被
        // 编译进来；事件循环必须在主线程创建，测试线程会 panic）。
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
    app
}

/// 固定光照（key light + 昼间 ambient；与 golden 场景同一 View 语义）。
fn apply_bench_view(app: &mut App) {
    *app.world_mut().resource_mut::<StylizedViewSettings>() =
        StylizedViewSettings(StylizedViewDescriptor {
            key_light_direction: Vec3::new(0.4, 0.8, 0.6).normalize(),
            key_light_color: LinearRgba::WHITE,
            exposure: 1.0,
            day_night_factor: 1.0,
            ..StylizedViewDescriptor::default()
        });
}

fn spawn_camera(app: &mut App) {
    // 固定 offscreen target（1280×720）保证测量分辨率恒定。
    let mut images = app.world_mut().resource_mut::<Assets<Image>>();
    let target = capture::create_capture_target(&mut images, WIDTH, HEIGHT);
    app.world_mut().spawn((
        Camera3d::default(),
        capture::capture_target_bundle(&target),
        Transform::from_xyz(0.0, 2.0, 18.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn ramp_image(rows: &[[u8; 4]]) -> Image {
    let mut data = Vec::new();
    for row in rows {
        for col in 0..64 {
            let px = if col >= 32 {
                [245, 245, 245, 255]
            } else {
                *row
            };
            data.extend_from_slice(&px);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: 64,
            height: rows.len() as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.data = Some(data);
    image
}

/// `hero_1`：1 个 Hero 材质球（全特性开）。
///
/// `pub`：showcase 的 stress/performance 用例复用（link, don't duplicate）。
pub fn build_hero_1(app: &mut App) {
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(1.2));
    let ramp = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(ramp_image(&[[120, 120, 200, 255]]));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(StylizedHeroMaterial {
            params: StylizedHeroParams::default().with_ramp_rows(1.0),
            ramp_texture: Some(ramp),
            ..StylizedHeroMaterial::default()
        });
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 1.2, 0.0),
    ));
}

/// portable_N：N 个 Cel 实例（同 mesh+material，MeshTag 差异）。
///
/// `pub`：showcase 的 stress/performance 用例复用。
pub fn build_portable_instances(app: &mut App, n: u32) {
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(0.3));
    let ramp = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(ramp_image(&[
            [80, 80, 220, 255],
            [220, 80, 80, 255],
            [80, 200, 80, 255],
            [220, 200, 80, 255],
        ]));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params: StylizedCelParams::default().with_ramp_rows(4.0),
            ramp_texture: Some(ramp),
            ..StylizedCelMaterial::default()
        });
    let cols = (n as f32).sqrt().ceil() as u32;
    for i in 0..n {
        let x = (i % cols) as f32 - cols as f32 * 0.5;
        // Diameter is 0.6: leave a gap so instanced draw ordering cannot
        // choose different surfaces at equal-depth sphere intersections.
        let y = (i / cols) as f32 * 0.65;
        app.world_mut().spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            StylizedInstanceData {
                profile_index: i % 4,
                ..StylizedInstanceData::default()
            },
            Transform::from_xyz(x * 0.85, y, 0.0),
        ));
    }
}

/// 运行单个场景：warm-up + 测量，返回帧统计。
fn measure_scene(
    resources: &RenderResources,
    scene: &str,
    class: &str,
    build: impl FnOnce(&mut App),
) -> (FrameStats, String, String) {
    let mut app = headless_app(resources.clone());
    apply_bench_view(&mut app);
    app.finish();
    app.cleanup();
    spawn_camera(&mut app);
    build(&mut app);
    for _ in 0..WARMUP_FRAMES {
        app.update();
    }
    let mut samples = Vec::with_capacity(MEASURED_FRAMES);
    for _ in 0..MEASURED_FRAMES {
        let start = Instant::now();
        app.update();
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let (gpu, driver) = app
        .world()
        .get_resource::<RenderAdapterInfo>()
        .map(|info| {
            (
                format!("{} ({:?})", info.name, info.backend),
                format!("{} {}", info.driver, info.driver_info),
            )
        })
        .unwrap_or_else(|| ("unknown".into(), "unknown".into()));
    let _ = scene;
    let _ = class;
    (FrameStats::from_samples(&mut samples), gpu, driver)
}

/// 4 个固定场景的 benchmark。返回 `None` = 无 GPU（graceful skip）。
pub fn run_benchmarks() -> Option<BenchmarkReport> {
    let resources = capture::probe_render_resources()?;

    let mut records = Vec::new();
    let (stats, gpu, driver) = measure_scene(&resources, "hero_1", "hero", build_hero_1);
    records.push(BenchmarkRecord {
        target_id: "linux-vulkan-rtx5090-hero-1-v1".into(),
        scene: "hero_1 (1 Hero material sphere, full features)".into(),
        class: "Hero".into(),
        tier: "Portable".into(),
        backend: "Vulkan".into(),
        resolution: [WIDTH, HEIGHT],
        frame_stats: stats,
        gpu_timing: None,
        frames: [WARMUP_FRAMES, MEASURED_FRAMES],
    });

    for n in [100_u32, 500, 1000] {
        let (stats, _, _) = measure_scene(&resources, "portable", "cel", |app| {
            build_portable_instances(app, n)
        });
        records.push(BenchmarkRecord {
            target_id: format!("linux-vulkan-rtx5090-portable-{n}-v1"),
            scene: format!(
                "portable_{n} ({n} Cel instances, shared mesh+material, MeshTag variation)"
            ),
            class: "Cel".into(),
            tier: "Portable".into(),
            backend: "Vulkan".into(),
            resolution: [WIDTH, HEIGHT],
            frame_stats: stats,
            gpu_timing: None,
            frames: [WARMUP_FRAMES, MEASURED_FRAMES],
        });
    }

    Some(BenchmarkReport {
        evidence_policy: "ADR-0029: fixed scenes, labeled class/tier/backend/resolution; \
                          improvement claims require before/after on the same device",
        captured_unix_seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
        engine_git_commit: golden_harness::engine_git_commit(),
        gpu,
        driver,
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        records,
    })
}
