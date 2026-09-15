use super::{Runtime, State, fit, state::DebugMode};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};

pub(super) fn ui(
    mut contexts: EguiContexts,
    mut state: ResMut<State>,
    runtime: Res<Runtime>,
) -> Result {
    let textures = runtime
        .targets
        .each_ref()
        .map(|t| contexts.add_image(EguiTextureHandle::Strong(t.image.clone())));
    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "inspector".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    ctx.set_visuals(egui::Visuals::dark());
    egui::Panel::top("header").show(&mut viewport_ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Bevy Stylized");
            ui.separator();
            ui.label("Character inspector");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label("LIVE A/B  ·  MSAA 4x");
            });
        });
    });
    egui::Panel::left("controls")
        .resizable(false)
        .default_size(250.)
        .show(&mut viewport_ui, |ui| {
            ui.add_space(12.);
            ui.heading("Asset");
            ui.label(&runtime.input.name);
            ui.label(format!("{} material primitives", runtime.input.meshes));
            if ui.button("Reload scene").clicked() {
                state.reload = true;
            }
            ui.add_space(16.);
            ui.heading("Camera");
            ui.horizontal(|ui| {
                for (label, face) in [("Full body", false), ("Face focus", true)] {
                    if ui
                        .add_enabled(runtime.bounds.is_some(), egui::Button::new(label))
                        .clicked()
                        && let Some((min, max)) = runtime.bounds
                    {
                        let c = fit(min, max, face, runtime.face_focus);
                        state.recipe.cameras = [c.clone(), c];
                    }
                }
            });
            ui.small(if runtime.input.focus.is_some() {
                "Face focus: explicit asset coordinates"
            } else if runtime.face_focus.is_some() {
                "Face focus: face-SDF surface bounds"
            } else {
                "Face focus: approximate height-based framing"
            });
            let linked_before = state.recipe.linked;
            ui.checkbox(&mut state.recipe.linked, "Link cameras");
            if !linked_before && state.recipe.linked {
                state.recipe.cameras[1] = state.recipe.cameras[0].clone();
            }
            ui.small("Drag: orbit · Right-drag: pan\nScroll: zoom · Double-click: reset");
            ui.add_space(16.);
            ui.heading("Shared lighting");
            let light = Vec3::from_array(state.recipe.light);
            let mut yaw = light.x.atan2(light.z).to_degrees();
            let mut pitch = light.y.clamp(-1., 1.).asin().to_degrees();
            let changed = ui
                .add(egui::Slider::new(&mut yaw, -180. ..=180.).text("Azimuth"))
                .changed()
                | ui.add(egui::Slider::new(&mut pitch, -85. ..=85.).text("Elevation"))
                    .changed();
            if changed {
                let (y, p) = (yaw.to_radians(), pitch.to_radians());
                state.recipe.light = [y.sin() * p.cos(), p.sin(), y.cos() * p.cos()];
            }
            ui.add(
                egui::Slider::new(&mut state.recipe.exposure, 0.1..=4.)
                    .logarithmic(true)
                    .text("Exposure"),
            );
            if ui.button("Reset light").clicked() {
                state.recipe.light = state.reset.light;
                state.recipe.exposure = state.reset.exposure;
            }
            ui.add_space(16.);
            ui.heading("Capture & recipe");
            ui.label(format!(
                "{} × {} per view",
                state.recipe.resolution, state.recipe.resolution
            ));
            if ui
                .add_enabled(
                    state.ready && runtime.pending.is_none(),
                    egui::Button::new("Save A/B + recipe"),
                )
                .clicked()
            {
                state.capture = true;
            }
            if ui
                .add_enabled(
                    runtime.options.config.is_some(),
                    egui::Button::new("Reload launch recipe"),
                )
                .clicked()
            {
                state.load_config = true;
            }
            if ui.button("Reset inspection").clicked() {
                state.recipe = state.reset.clone();
            }
            ui.add_space(12.);
            ui.small("Both views are live runtime output.\nNeither is an upstream reference.");
        });
    egui::Panel::bottom("status").show(&mut viewport_ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                if state.error.is_some() {
                    egui::Color32::LIGHT_RED
                } else if state.ready {
                    egui::Color32::LIGHT_GREEN
                } else {
                    egui::Color32::YELLOW
                },
                &state.status,
            );
            ui.separator();
            ui.label(format!(
                "Generation {} · {} saved",
                runtime.generation, state.captures
            ));
        });
    });
    egui::CentralPanel::default().show(&mut viewport_ui, |ui| {
        ui.columns(2, |columns| {
            for side in 0..2 {
                let ui = &mut columns[side];
                ui.horizontal(|ui| {
                    ui.heading(if side == 0 { "Live A" } else { "Live B" });
                    egui::ComboBox::from_id_salt(("debug", side))
                        .selected_text(state.recipe.debug[side].label())
                        .show_ui(ui, |ui| {
                            for mode in DebugMode::ALL.into_iter().filter(|mode| {
                                *mode == DebugMode::Beauty || runtime.input.supports_debug()
                            }) {
                                ui.selectable_value(
                                    &mut state.recipe.debug[side],
                                    mode,
                                    mode.label(),
                                );
                            }
                        });
                });
                let size = ui
                    .available_width()
                    .min(ui.available_height() - 22.)
                    .max(32.);
                let response = ui.add(
                    egui::Image::new((textures[side], egui::vec2(size, size)))
                        .sense(egui::Sense::click_and_drag()),
                );
                if response.double_clicked() {
                    state.recipe.cameras[side] = state.reset.cameras[side].clone();
                }
                let mut changed = response.double_clicked();
                if response.hovered() || response.dragged() {
                    let delta = ui.input(|i| i.pointer.delta());
                    let c = &mut state.recipe.cameras[side];
                    if response.dragged_by(egui::PointerButton::Primary) {
                        c.yaw -= delta.x * 0.006;
                        c.pitch = (c.pitch + delta.y * 0.006).clamp(-1.45, 1.45);
                        changed = true;
                    }
                    if response.dragged_by(egui::PointerButton::Secondary) {
                        let t = c.transform();
                        let shift = (-t.right() * delta.x + t.up() * delta.y) * (c.distance / size);
                        c.target = (Vec3::from_array(c.target) + shift).to_array();
                        changed = true;
                    }
                    let scroll = if response.hovered() {
                        ui.input(|i| i.smooth_scroll_delta.y)
                    } else {
                        0.
                    };
                    if scroll != 0. {
                        c.distance = (c.distance * (-scroll * 0.003).exp()).clamp(0.01, 10000.);
                        changed = true;
                    }
                }
                if changed && state.recipe.linked {
                    state.recipe.cameras[1 - side] = state.recipe.cameras[side].clone();
                }
            }
        });
    });
    Ok(())
}
