//! Optional camera-level display transform matching the Genshin Blender graph.

use bevy::prelude::{Component, Reflect};

/// Embedded shader used by [`GenshinDisplayTransform`].
pub const GENSHIN_DISPLAY_TRANSFORM_SHADER_PATH: &str =
    "embedded://bevy_cel/shaders/stylized_post_process.wgsl";

/// Applies the source Genshin display transform to one 3D camera.
///
/// The component is deliberately opt-in. It requires an HDR intermediate so
/// values above display white survive until the transform, but it does not
/// add bloom or change the camera's Bevy [`Tonemapping`](bevy::core_pipeline::tonemapping::Tonemapping)
/// component. Consumers replacing Bevy tonemapping should pair this with
/// `Tonemapping::None` and configure bloom independently.
///
/// Ownership: camera NPR components ([`Hdr`](bevy::camera::Hdr), bloom,
/// tonemapping, prepasses) are owned by `apply_profiles`, which enforces the
/// union of profile and display-transform requirements; manual insertion of
/// those components on a managed camera is overridden.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub struct GenshinDisplayTransform;

/// CPU reference for the fixed per-channel display transform.
///
/// This mirrors the shader formula for offline comparisons and contract tests.
#[must_use]
pub fn genshin_display_transform_channel(x: f32) -> f32 {
    (x * (1.36 * x + 0.047) / (x * (0.93 * x + 0.56) + 0.14)).clamp(0.0, 1.0)
}
