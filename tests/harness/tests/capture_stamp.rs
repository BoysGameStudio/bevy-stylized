//! CaptureStamp 逐帧身份证据测试（GPU-gated，无 GPU graceful skip）。
//!
//! 对应 docs/showcase/01 R3/R13 的本地证据：跨提取/提交/读回的 stamp 身份、
//! 多在途/取消/场景编辑下的像素一致性，以及"等待中的额外 update 不改变
//! stamp↔帧绑定"。内容侧用相机 clear color 作为逐 update 可变的确定性
//! 场景状态（Tonemapping::None，无网格/材质依赖）。

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::{AssetMetaCheck, AssetPlugin},
    camera::ClearColorConfig,
    color::Color,
    core_pipeline::tonemapping::Tonemapping,
    prelude::{App, Camera, Camera3d, Transform, With},
    render::{
        RenderPlugin,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
};
use golden_harness::{CaptureRequest, CaptureTarget, CapturedImage, StampedCapture, capture};

fn stamp_app(resources: RenderResources, clear: Color) -> (App, CaptureTarget) {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin {
                file_path: "assets".to_string(),
                meta_check: AssetMetaCheck::Never,
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
    capture::install(&mut app);
    app.finish();
    app.cleanup();
    let target = {
        let mut images = app
            .world_mut()
            .resource_mut::<bevy::prelude::Assets<bevy::image::Image>>();
        capture::create_capture_target(&mut images, 64, 64)
    };
    // capture_target_bundle 自带 Camera；以 insert 覆盖 clear color（不重复组件）。
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            capture::capture_target_bundle(&target),
            Tonemapping::None,
            Transform::IDENTITY,
        ))
        .id();
    app.world_mut().entity_mut(camera).insert(Camera {
        clear_color: ClearColorConfig::Custom(clear),
        ..Camera::default()
    });
    (app, target)
}

fn set_clear(app: &mut App, clear: Color) {
    let mut cameras = app
        .world_mut()
        .query_filtered::<&mut Camera, With<Camera3d>>();
    for mut camera in cameras.iter_mut(app.world_mut()) {
        camera.clear_color = ClearColorConfig::Custom(clear);
    }
}

/// 主色均值（证据断言用；clear color 充满整个 target）。
fn mean_rgb(image: &CapturedImage) -> [f32; 3] {
    let mut sum = [0_f32; 3];
    for px in image.data.as_chunks::<4>().0 {
        for c in 0..3 {
            sum[c] += f32::from(px[c]);
        }
    }
    let n = (image.width * image.height) as f32;
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

fn take_stamped_until(
    app: &mut App,
    request: CaptureRequest,
    max_updates: usize,
) -> StampedCapture {
    for _ in 0..max_updates {
        if let Some(capture) = capture::try_take_stamped(app.world_mut(), request.clone()) {
            return capture;
        }
        app.update();
    }
    panic!("capture did not complete within {max_updates} updates");
}

const RED: Color = Color::srgb(1.0, 0.0, 0.0);
const GREEN: Color = Color::srgb(0.0, 1.0, 0.0);
const BLUE: Color = Color::srgb(0.0, 0.0, 1.0);

/// R3 证据一：两个在不同 update 发出的请求，其像素分别属于各自
/// 发行时点之后的首次提取（场景在中间发生变化时不能串帧），
/// stamp 的 update 序号与序列单调对应。
#[test]
#[ignore = "requires explicit GPU execution"]
fn two_in_flight_captures_bind_their_issuing_ticks() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("capture stamp: no GPU adapter available; skipping");
        return;
    };
    let (mut app, target) = stamp_app(resources, RED);
    app.update(); // K = 1
    let a = capture::request_capture(app.world_mut(), target.clone()); // issued = 1；复制 = update 2 的提取（仍 RED）
    app.update(); // K = 2：A 的复制提交（RED）
    set_clear(&mut app, GREEN);
    app.update(); // K = 3
    let b = capture::request_capture(app.world_mut(), target.clone()); // issued = 3；复制 = update 4 的提取（GREEN）
    let a = take_stamped_until(&mut app, a, 16);
    let b = take_stamped_until(&mut app, b, 16);

    assert!(a.stamp.sequence < b.stamp.sequence);
    assert_eq!(a.stamp.issued_update_index, 1);
    assert_eq!(b.stamp.issued_update_index, 3);
    assert!(a.stamp.completed_update_index >= 2);
    assert!(b.stamp.completed_update_index >= 4);
    assert!(b.stamp.completed_update_index >= a.stamp.completed_update_index);
    let [r, g, _] = mean_rgb(&a.image);
    assert!(r > 200.0 && g < 60.0, "A must be the RED frame: {a:?}");
    let [r, g, _] = mean_rgb(&b.image);
    assert!(g > 200.0 && r < 60.0, "B must be the GREEN frame: {b:?}");
    eprintln!(
        "[stamp] A seq={} issued={} completed={} render_frame={:?}; B seq={} issued={} completed={} render_frame={:?}",
        a.stamp.sequence,
        a.stamp.issued_update_index,
        a.stamp.completed_update_index,
        a.stamp.completed_render_frame,
        b.stamp.sequence,
        b.stamp.issued_update_index,
        b.stamp.completed_update_index,
        b.stamp.completed_render_frame
    );
}

/// R3 证据二：同一 update 同时发出的两个请求（不同相机/target），
/// 完成后各自关联到正确的内容；记录观察到的完成次序（FIFO 事实，
/// 关联正确性不依赖它——按请求实体关联）。
#[test]
#[ignore = "requires explicit GPU execution"]
fn simultaneous_requests_correlate_to_their_targets() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("capture stamp: no GPU adapter available; skipping");
        return;
    };
    let (mut app, red_target) = stamp_app(resources, RED);
    // 第二台相机 + target（BLUE）。
    let blue_target = {
        let mut images = app
            .world_mut()
            .resource_mut::<bevy::prelude::Assets<bevy::image::Image>>();
        capture::create_capture_target(&mut images, 64, 64)
    };
    let blue_camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            capture::capture_target_bundle(&blue_target),
            Tonemapping::None,
            Transform::IDENTITY,
        ))
        .id();
    app.world_mut().entity_mut(blue_camera).insert(Camera {
        clear_color: ClearColorConfig::Custom(BLUE),
        ..Camera::default()
    });
    app.update(); // K = 1
    let red = capture::request_capture(app.world_mut(), red_target.clone());
    let blue = capture::request_capture(app.world_mut(), blue_target.clone());
    assert_eq!(red.issued_update_index(), blue.issued_update_index());
    assert!(red.sequence() < blue.sequence());

    let mut red_capture = None;
    let mut blue_capture = None;
    let mut order = Vec::new();
    for _ in 0..16 {
        app.update();
        if red_capture.is_none()
            && let Some(capture) = capture::try_take_stamped(app.world_mut(), red.clone())
        {
            order.push("red");
            red_capture = Some(capture);
        }
        if blue_capture.is_none()
            && let Some(capture) = capture::try_take_stamped(app.world_mut(), blue.clone())
        {
            order.push("blue");
            blue_capture = Some(capture);
        }
        if red_capture.is_some() && blue_capture.is_some() {
            break;
        }
    }
    let red = red_capture.expect("red capture");
    let blue = blue_capture.expect("blue capture");
    let [r, _g, b] = mean_rgb(&red.image);
    assert!(
        r > 200.0 && b < 60.0,
        "red target must be RED: {:?}",
        red.stamp
    );
    let [r, _g, b] = mean_rgb(&blue.image);
    assert!(
        b > 200.0 && r < 60.0,
        "blue target must be BLUE: {:?}",
        blue.stamp
    );
    eprintln!("[stamp] completion order observed: {order:?} (correlation is per-request-entity)");
}

/// R13 证据三：等待中的额外 update 不改变 stamp↔帧绑定——像素仍属于
/// 发行后的首次提取，即使之后场景又变了、又过了多次 update。
#[test]
#[ignore = "requires explicit GPU execution"]
fn extra_updates_while_pending_keep_stamp_binding() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("capture stamp: no GPU adapter available; skipping");
        return;
    };
    let (mut app, target) = stamp_app(resources, RED);
    app.update(); // K = 1
    let request = capture::request_capture(app.world_mut(), target.clone()); // issued = 1 → 像素 = update 2 提取（RED）
    app.update(); // K = 2：复制提交（RED）
    set_clear(&mut app, GREEN);
    app.update();
    set_clear(&mut app, BLUE);
    app.update();
    app.update();
    let capture = take_stamped_until(&mut app, request, 16);
    assert_eq!(capture.stamp.issued_update_index, 1);
    assert!(capture.stamp.completed_update_index >= 2);
    let [r, g, b] = mean_rgb(&capture.image);
    assert!(
        r > 200.0 && g < 60.0 && b < 60.0,
        "extra updates must not shift the binding: {:?}",
        capture.stamp
    );
}

/// R3 证据四：取消的请求无可取结果；引擎晚到的完成被忽略；
/// 后续请求的序列/身份不回绕且内容正确。
#[test]
#[ignore = "requires explicit GPU execution"]
fn cancelled_request_leaves_no_result_and_keeps_later_identities() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("capture stamp: no GPU adapter available; skipping");
        return;
    };
    let (mut app, target) = stamp_app(resources, RED);
    app.update();
    let cancelled = capture::request_capture(app.world_mut(), target.clone());
    let cancelled_sequence = cancelled.sequence();
    capture::cancel_capture(app.world_mut(), cancelled.clone());
    app.update();
    app.update();
    assert!(capture::try_take(app.world_mut(), cancelled).is_none());

    app.update();
    let next = capture::request_capture(app.world_mut(), target.clone());
    assert_eq!(next.sequence(), cancelled_sequence + 1);
    let capture = take_stamped_until(&mut app, next, 16);
    assert_eq!(capture.stamp.sequence, cancelled_sequence + 1);
    let [r, g, _] = mean_rgb(&capture.image);
    assert!(r > 200.0 && g < 60.0, "later request must capture RED");
}

/// R3 证据五：场景编辑后发出的捕获返回新内容与全新 stamp（无过期复用）。
#[test]
#[ignore = "requires explicit GPU execution"]
fn capture_after_scene_edit_returns_fresh_content_and_stamp() {
    let Some(resources) = capture::probe_render_resources() else {
        eprintln!("capture stamp: no GPU adapter available; skipping");
        return;
    };
    let (mut app, target) = stamp_app(resources, RED);
    app.update();
    let before = capture::request_capture(app.world_mut(), target.clone());
    let before = take_stamped_until(&mut app, before, 16);
    let [r, g, _] = mean_rgb(&before.image);
    assert!(r > 200.0 && g < 60.0, "pre-edit frame must be RED");

    set_clear(&mut app, GREEN);
    app.update();
    let after = capture::request_capture(app.world_mut(), target.clone());
    let after = take_stamped_until(&mut app, after, 16);
    assert!(after.stamp.sequence > before.stamp.sequence);
    assert!(after.stamp.issued_update_index > before.stamp.issued_update_index);
    let [r, g, _] = mean_rgb(&after.image);
    assert!(g > 200.0 && r < 60.0, "post-edit frame must be GREEN");
}
