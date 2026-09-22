//! Masked（alpha-cutout）表面与描边的外观政策 GPU 证据（ADR-0023）。
//!
//! 场景：球体经 `StylizedCelMaterial`（cutoff 0.5）挂 alpha 纹理——v 向
//! 赤道带 alpha=0（一周镂空）；绿色背景板在后。两条描边路径分别钉死：
//!
//! - **hull 外扩**（Portable）：轮廓副本是纯几何外扩壳，不采样 base
//!   alpha → 遵循 pre-discard 几何轮廓；镂空带被壳体填充为描边色
//!   （实心覆盖，不是沿镂空边界的悬空细环）；
//! - **screen-space 内边缘**（Standard+）：prepass 共享主材质 alpha
//!   输入并 discard → 边缘遵循 post-discard 可见轮廓；镂空带露出背景，
//!   带边界出现正确的边缘描边。

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    color::LinearRgba,
    image::Image,
    math::Vec3,
    mesh::Mesh,
    pbr::MeshMaterial3d,
    prelude::{Mesh3d, Transform},
    render::{
        RenderPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        settings::RenderCreation,
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_stylized::prelude::*;
use golden_harness::capture;

/// 三种语义像素的分类（纯色设计使分类对 backend 噪声稳健）。
/// `dark_outline`：screen-edge 覆盖层的默认描边色为近黑；hull 测试用纯蓝。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Px {
    /// 绿色背景。
    Background,
    /// 描边（hull=纯蓝 / screen=近黑）。
    Outline,
    /// 球体表面（纹理红）或其他。
    Surface,
}

fn classify(p: [u8; 3], dark_outline: bool) -> Px {
    let [r, g, b] = p;
    if g > 150 && r < 100 && b < 100 {
        Px::Background
    } else if dark_outline {
        if r < 80 && g < 80 && b < 80 {
            Px::Outline
        } else {
            Px::Surface
        }
    } else if b > 150 && r < 100 && g < 100 {
        Px::Outline
    } else {
        Px::Surface
    }
}

fn classify_frame(
    frame: &golden_harness::CapturedImage,
    size: usize,
    dark_outline: bool,
) -> Vec<Vec<Px>> {
    (0..size)
        .map(|y| {
            (0..size)
                .map(|x| {
                    let o = (y * size + x) * 4;
                    classify(
                        [frame.data[o], frame.data[o + 1], frame.data[o + 2]],
                        dark_outline,
                    )
                })
                .collect()
        })
        .collect()
}

/// 诊断输出（16x16 降采样 ASCII 图）；调试几何/方向用。
fn dump_map(map: &[Vec<Px>]) {
    let size = map.len();
    for y in (0..size).step_by(size / 16) {
        let row: String = (0..size)
            .step_by(size / 16)
            .map(|x| match map[y][x] {
                Px::Background => '.',
                Px::Outline => 'O',
                Px::Surface => '#',
            })
            .collect();
        eprintln!("  {row}");
    }
}

/// 非背景像素的外接圆（中心 + 半径）。screen-edge 覆盖层在视口边框有
/// 边缘伪影，计算剪影时排除最外 4px 帧。
fn silhouette_circle(map: &[Vec<Px>]) -> (isize, isize, f32) {
    let size = map.len();
    let mut bounds = [isize::MAX, isize::MAX, 0isize, 0isize];
    for (y, row) in map.iter().enumerate().take(size - 4).skip(4) {
        for (x, px) in row.iter().enumerate().take(size - 4).skip(4) {
            if *px != Px::Background {
                bounds[0] = bounds[0].min(x as isize);
                bounds[1] = bounds[1].min(y as isize);
                bounds[2] = bounds[2].max(x as isize);
                bounds[3] = bounds[3].max(y as isize);
            }
        }
    }
    let [min_x, min_y, max_x, max_y] = bounds;
    assert!(
        max_x > min_x + 50 && max_y > min_y + 50,
        "sphere visible: {bounds:?}"
    );
    (
        (min_x + max_x) / 2,
        (min_y + max_y) / 2,
        ((max_x - min_x) + (max_y - min_y)) as f32 / 4.0,
    )
}

/// 对圆内每个像素调用 `f`。
fn for_each_in_circle(map: &[Vec<Px>], cx: isize, cy: isize, radius: f32, mut f: impl FnMut(Px)) {
    let r = radius as isize;
    for dy in -r..=r {
        let y = (cy + dy) as usize;
        let half = ((r * r - dy * dy) as f32).sqrt() as isize;
        for dx in -half..=half {
            f(map[y][(cx + dx) as usize]);
        }
    }
}

/// 装配镂空球场景（赤道带 alpha=0），返回捕获帧（256²）。
///
/// `outline`：hull 外扩（蓝色，6px）；`screen_edges`：Standard 会话 +
/// 相机 `StylizedScreenOutline`（经 CameraAttachment::ScreenOutline）。
fn render_masked_sphere(outline: bool, screen_edges: bool) -> golden_harness::CapturedImage {
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "assets".into(),
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
    if screen_edges {
        // Standard 会话（Stage A 请求；screen edges 是 Standard+ 路径）。
        app.insert_resource(StylizedCapabilityConfig {
            requested_tier: StylizedExecutionTier::Standard,
            ..StylizedCapabilityConfig::default()
        });
    }
    app.add_plugins(StylizedPlugin::default());
    capture::install(&mut app);
    golden_harness::readiness::install(&mut app);
    app.finish();
    app.cleanup();
    {
        let mut view = app.world_mut().resource_mut::<StylizedViewSettings>();
        view.0.key_light_direction = Vec3::Z;
        view.0.ambient_override = Some(LinearRgba::WHITE);
    }

    // 64x64 纹理：红底；v∈[0.4,0.6] 赤道带 alpha=0（一周镂空）。
    let mut data = Vec::new();
    for y in 0..64 {
        for _x in 0..64 {
            let v = y as f32 / 63.0;
            let a = if (0.4..=0.6).contains(&v) { 0u8 } else { 255 };
            data.extend_from_slice(&[200, 100, 100, a]);
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.data = Some(data);
    let base = app.world_mut().resource_mut::<Assets<Image>>().add(image);

    let sphere = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(bevy::math::primitives::Sphere::new(0.8));
    let masked = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params: StylizedCelParams::default().with_alpha_cutoff(0.5),
            base_color_texture: Some(base),
            ..Default::default()
        });
    let entity = app
        .world_mut()
        .spawn((Mesh3d(sphere), MeshMaterial3d(masked), Transform::IDENTITY))
        .id();
    if outline {
        let hull = app
            .world_mut()
            .resource_mut::<Assets<StylizedOutlineMaterial>>()
            .add(StylizedOutlineMaterial {
                params: StylizedOutlineParams::default()
                    .with_color(LinearRgba::new(0.0, 0.0, 1.0, 1.0))
                    .with_width(6.0),
                class_texture: None,
            });
        app.world_mut()
            .entity_mut(entity)
            .insert(StylizedOutline::new(hull));
    }

    // 绿色背景板（镂空带的参照物）。
    let background_params = StylizedCelParams {
        base_color_factor: bevy::math::Vec4::new(0.0, 1.0, 0.0, 1.0),
        ..StylizedCelParams::default()
    };
    let background_material = app
        .world_mut()
        .resource_mut::<Assets<StylizedCelMaterial>>()
        .add(StylizedCelMaterial {
            params: background_params,
            ..Default::default()
        });
    {
        let quad = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(bevy::math::primitives::Rectangle::new(8.0, 8.0));
        app.world_mut().spawn((
            Mesh3d(quad),
            MeshMaterial3d(background_material),
            Transform::from_xyz(0.0, 0.0, -1.5),
        ));
    }

    let attachment = if screen_edges {
        scenes::CameraAttachment::ScreenOutline
    } else {
        scenes::CameraAttachment::None
    };
    scene_runtime::capture_in_app(
        &mut app,
        &scenario_model::ident::SessionId::new("masked-outline").expect("session id"),
        &scenes::CameraRecipe::with_attachment(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO, attachment),
        (256, 256),
        scene_runtime::Settle::ReadinessThenFrames(24),
        true,
        660,
    )
    .expect("capture")
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn masked_hull_follows_geometric_silhouette() {
    let frame = render_masked_sphere(true, false);
    let map = classify_frame(&frame, 256, false);
    dump_map(&map);

    // 球体剪影（hull 外扩后）：非背景像素的外接圆。
    let (cx, cy, radius) = silhouette_circle(&map);
    eprintln!("hull: center=({cx},{cy}) r={radius}");

    // pre-discard 政策：剪影圆内部（半径再缩 12px 排除 rim/AA）不允许出现
    // 背景色——镂空带被 hull 壳体实心填充。
    let mut interior_background = 0usize;
    let mut interior_outline = 0usize;
    let mut interior_total = 0usize;
    for_each_in_circle(&map, cx, cy, radius - 12.0, |px| {
        interior_total += 1;
        match px {
            Px::Background => interior_background += 1,
            Px::Outline => interior_outline += 1,
            Px::Surface => {}
        }
    });
    eprintln!(
        "hull: interior total={interior_total} background={interior_background} outline={interior_outline}"
    );
    assert_eq!(
        interior_background, 0,
        "hull follows pre-discard geometry: the cutout band is hull-filled, no background inside the silhouette"
    );
    // 实心填充而非"沿镂空边界的悬空细环"：赤道带约占内部 20-25% 面积；
    // 细环只会有 ~3-5%。
    let fill_percent = interior_outline * 100 / interior_total;
    assert!(
        fill_percent > 12,
        "cutout band must be solid hull fill, not a floating rim: {fill_percent}% of the interior"
    );
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn masked_screen_edges_follow_post_discard_silhouette() {
    let frame = render_masked_sphere(false, true);
    let map = classify_frame(&frame, 256, true);
    dump_map(&map);

    let (cx, cy, radius) = silhouette_circle(&map);
    eprintln!("screen: center=({cx},{cy}) r={radius}");

    // post-discard 政策：镂空带露出绿色背景（prepass discard）。
    let mut interior_background = 0usize;
    let mut interior_outline = 0usize;
    let mut interior_total = 0usize;
    for_each_in_circle(&map, cx, cy, radius - 12.0, |px| {
        interior_total += 1;
        match px {
            Px::Background => interior_background += 1,
            Px::Outline => interior_outline += 1,
            Px::Surface => {}
        }
    });
    eprintln!(
        "screen: interior total={interior_total} background={interior_background} outline={interior_outline}"
    );
    let background_percent = interior_background * 100 / interior_total;
    assert!(
        background_percent > 10,
        "prepass discards the cutout band: background must show through ({background_percent}% of the interior)"
    );
    // 镂空深处干净：背景占比 >60% 的"洞内部行"内不允许描边像素
    // （无悬空填充/环；边界强调带只出现在洞的边界行之外）。
    let mut hole_rows_outline = 0usize;
    let mut hole_rows_total = 0usize;
    let inner = (radius - 12.0) as isize;
    for dy in -inner..=inner {
        let y = (cy + dy) as usize;
        let half = ((inner * inner - dy * dy) as f32).sqrt() as isize;
        let mut row_background = 0usize;
        for dx in -half..=half {
            if map[y][(cx + dx) as usize] == Px::Background {
                row_background += 1;
            }
        }
        let row_total = (2 * half + 1) as usize;
        if row_background * 5 > row_total * 3 {
            // 深洞行（>60% 背景）：中段（两端排除剪影 rim 通道）。
            let margin = (half as f32 * 0.25).max(12.0) as isize;
            for dx in (-half + margin)..=(half - margin) {
                if map[y][(cx + dx) as usize] == Px::Outline {
                    hole_rows_outline += 1;
                }
                hole_rows_total += 1;
            }
        }
    }
    eprintln!("screen: deep hole rows outline={hole_rows_outline}/{hole_rows_total}");
    assert!(hole_rows_total > 0, "hole band rows detected");
    assert_eq!(
        hole_rows_outline, 0,
        "deep hole interior must be clean background: no floating outline or fill"
    );
    // 边界强调带存在（镂空边界的边缘描边；见 ADR-0023：其宽度异常为
    // 未合格项记录在案，此处只钉死"存在且内容遵循 post-discard"）。
    assert!(
        interior_outline > 0,
        "edge emphasis around the cutout boundary exists"
    );
}
