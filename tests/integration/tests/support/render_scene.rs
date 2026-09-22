//! Shared deterministic scene/capture setup for semantic GPU test binaries.
//!
//! 捕获生命周期经 `labs/scene-runtime` 的 [`CaptureCycle`]（settle→observe
//! 周期；帧语义与旧手写"24 次 update 后请求、下一帧提取"逐帧一致，
//! 见 scene-runtime cycle 模块文档）。断言与期望留在各测试；本模块不生成
//! 任何期望值。
use bevy::prelude::*;
#[path = "render_app.rs"]
mod render_app;
use golden_harness::CapturedImage;
pub(super) use render_app::headless_app;
use scenario_model::ident::SessionId;
use scene_runtime::{CycleResource, Settle, capture_in_app, set_cycle_projection};
use scenes::CameraRecipe;

pub(super) const SIZE: u32 = 256;

pub(super) fn capture_frame(app: &mut App) -> CapturedImage {
    capture_frame_with_projection(app, bevy::camera::Projection::default())
}

pub(super) fn capture_frame_with_projection(
    app: &mut App,
    projection: bevy::camera::Projection,
) -> CapturedImage {
    if app.world().contains_resource::<CycleResource>() {
        set_cycle_projection(app, projection.clone());
    }
    capture_in_app(
        app,
        &SessionId::new("integration-capture").expect("session id"),
        &CameraRecipe::new(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO).with_projection(projection),
        (SIZE, SIZE),
        Settle::Frames(24),
        false,
        700,
    )
    .expect("capture timed out")
}

pub(super) fn quad_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.8, -0.8, 0.0],
            [0.8, -0.8, 0.0],
            [0.8, 0.8, 0.0],
            [-0.8, 0.8, 0.0],
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}
