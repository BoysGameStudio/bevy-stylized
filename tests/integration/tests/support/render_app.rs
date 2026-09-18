//! Default headless fixture app.
#[path = "render_app_builder.rs"]
mod builder;
pub(crate) fn headless_app(
    resources: bevy::render::settings::RenderResources,
) -> bevy::prelude::App {
    builder::configured_app(resources, |_| {})
}
