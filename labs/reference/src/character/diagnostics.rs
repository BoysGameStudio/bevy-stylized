//! Diagnostic material passes and sharp lighting-factor qualification.
use super::{
    frames::{frame, pipelines_ready},
    io::read,
    metrics::coverage_iou,
};
use bevy::prelude::*;
use bevy_stylized::prelude::*;
use golden_harness::{CapturedImage, capture};
use serde_json::{Value, json};
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(super) fn coverage(
    app: &mut App,
    target: &capture::CaptureTarget,
) -> Result<CapturedImage, String> {
    let mut q = app.world_mut().query::<(
        Entity,
        Option<&MeshMaterial3d<StylizedCelMaterial>>,
        Option<&MeshMaterial3d<StylizedCharacterMaterial>>,
        Option<&MeshMaterial3d<StylizedHeroMaterial>>,
    )>();
    let originals: Vec<_> = q
        .iter(app.world())
        .filter(|(_, a, b, c)| a.is_some() || b.is_some() || c.is_some())
        .map(|(e, a, b, c)| (e, a.cloned(), b.cloned(), c.cloned()))
        .collect();
    // Vertex colors carry NPR controls, not albedo. A white coverage pass
    // must ignore them while preserving positions/indices and all deformation.
    let mut meshes = Vec::new();
    for (e, _, _, _) in &originals {
        let original = app
            .world()
            .get::<Mesh3d>(*e)
            .ok_or("coverage mesh")?
            .0
            .clone();
        let mut copy = app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&original)
            .ok_or("coverage mesh asset")?
            .clone();
        copy.remove_attribute(Mesh::ATTRIBUTE_COLOR);
        let diagnostic = app.world_mut().resource_mut::<Assets<Mesh>>().add(copy);
        app.world_mut().entity_mut(*e).insert(Mesh3d(diagnostic));
        meshes.push((*e, original));
    }
    let white = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
    for (e, _, _, _) in &originals {
        app.world_mut()
            .entity_mut(*e)
            .remove::<(
                MeshMaterial3d<StylizedCelMaterial>,
                MeshMaterial3d<StylizedCharacterMaterial>,
                MeshMaterial3d<StylizedHeroMaterial>,
            )>()
            .insert(MeshMaterial3d(white.clone()));
    }
    let start = Instant::now();
    let mut stable = 0;
    loop {
        app.update();
        if pipelines_ready(app)? {
            stable += 1
        } else {
            stable = 0
        }
        if stable >= 24 {
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            return Err("coverage pipeline timeout".into());
        }
    }
    let image = frame(app, target)?;
    for (e, a, b, c) in originals {
        let mut entity = app.world_mut().entity_mut(e);
        entity.remove::<MeshMaterial3d<StandardMaterial>>();
        if let Some(m) = a {
            entity.insert(m);
        }
        if let Some(m) = b {
            entity.insert(m);
        }
        if let Some(m) = c {
            entity.insert(m);
        }
    }
    for (entity, mesh) in meshes {
        app.world_mut().entity_mut(entity).insert(Mesh3d(mesh));
    }
    Ok(image)
}
#[allow(clippy::too_many_arguments)]
pub(super) fn lighting_diagnostics(
    app: &mut App,
    target: &capture::CaptureTarget,
    camera: Entity,
    root: &Path,
    out: &Path,
    id: &str,
    factor: &str,
    center: &str,
) -> Result<Value, String> {
    let background = app.world().resource::<ClearColor>().clone();
    let msaa = *app.world().get::<Msaa>(camera).ok_or("camera MSAA")?;
    app.world_mut().insert_resource(ClearColor(Color::BLACK));
    app.world_mut().entity_mut(camera).insert(Msaa::Off);
    let mask = coverage(app, target)?;
    let reference_mask =
        golden_harness::decode_png(&read(root.join(center))?).map_err(|e| e.to_string())?;
    let center_report = coverage_iou(&reference_mask, &mask)?;
    std::fs::write(
        out.join(format!("{id}-center-coverage-bevy.png")),
        golden_harness::encode_png(&mask).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::copy(
        root.join(center),
        out.join(format!("{id}-center-coverage-blender.png")),
    )
    .map_err(|e| e.to_string())?;
    let debug_mode = app.world().resource::<DiagnosticMode>().0;
    let modes: Vec<_> = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .iter_mut()
        .map(|(id, m)| {
            let old = m.debug_view;
            m.debug_view = debug_mode;
            (id, old)
        })
        .collect();
    let start = Instant::now();
    let mut stable = 0;
    loop {
        app.update();
        if pipelines_ready(app)? {
            stable += 1
        } else {
            stable = 0
        }
        if stable >= 24 {
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            return Err("lighting diagnostic pipeline timeout".into());
        }
    }
    let rendered = frame(app, target)?;
    let reference =
        golden_harness::decode_png(&read(root.join(factor))?).map_err(|e| e.to_string())?;
    std::fs::write(
        out.join(format!("{id}-factors-bevy.png")),
        golden_harness::encode_png(&rendered).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::copy(
        root.join(factor),
        out.join(format!("{id}-factors-blender.png")),
    )
    .map_err(|e| e.to_string())?;
    for (id, mode) in modes {
        if let Some(mut m) = app
            .world_mut()
            .resource_mut::<Assets<StylizedHeroMaterial>>()
            .get_mut(id)
        {
            m.debug_view = mode
        }
    }
    app.world_mut().entity_mut(camera).insert(msaa);
    app.world_mut().insert_resource(background);
    if rendered.width != reference.width || rendered.height != reference.height {
        return Err("factor dimensions mismatch".into());
    }
    if debug_mode != StylizedHeroDebugView::LightingFactors {
        return Ok(
            json!({"status":"UNQUALIFIED","reason":"base-color diagnostic, not lighting factors","center_coverage":center_report}),
        );
    }
    let mut intersection = 0usize;
    let mut union = 0usize;
    let mut count_a = 0usize;
    let mut count_b = 0usize;
    for (a, b) in reference
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(rendered.data.as_chunks::<4>().0)
    {
        let x = a[2] > 200 && a[1] < 188;
        let y = b[2] > 200 && b[1] < 188;
        count_a += usize::from(x);
        count_b += usize::from(y);
        intersection += usize::from(x && y);
        union += usize::from(x || y);
    }
    if count_a == 0 || count_b == 0 {
        return Ok(
            json!({"status":"UNQUALIFIED","reason":"empty lighting-factor shadow mask","center_coverage":center_report}),
        );
    }
    let iou = intersection as f64 / union as f64;
    Ok(
        json!({"status":if iou>=0.95&&center_report["passed"]==true{"PASS"}else{"FAIL"},"algorithm":"sharp-light-factor/v1","iou":iou,"shadow_pixels_blender":count_a,"shadow_pixels_bevy":count_b,"center_coverage":center_report,"scope":"NPR sharp lighting classification; not physical cast shadows"}),
    )
}

#[derive(Resource)]
pub(super) struct DiagnosticMode(pub(super) StylizedHeroDebugView);
