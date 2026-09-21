//! Test-only full-precision Hero table prototype. No shipping material/API is added.
#[path = "support/render_app_builder.rs"]
mod builder;
use bevy::{
    asset::uuid_handle,
    core_pipeline::{core_3d::Opaque3d, tonemapping::Tonemapping},
    mesh::MeshTag,
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::{
        RenderApp, render_phase::ViewBinnedRenderPhases, render_resource::*, storage::ShaderBuffer,
    },
    shader::{Shader, ShaderRef},
};
use bevy_stylized::prelude::*;
use golden_harness::{CapturedImage, capture};
use std::time::Instant;
const PROBE: Handle<Shader> = uuid_handle!("1b3cfac5-971c-40e8-97dc-8a53a2807030");
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
struct PrecisionTable {
    #[storage(100, read_only)]
    records: Handle<ShaderBuffer>,
}
impl MaterialExtension for PrecisionTable {
    fn fragment_shader() -> ShaderRef {
        PROBE.into()
    }
}
type TableHero = ExtendedMaterial<StylizedHeroMaterial, PrecisionTable>;

fn shader() -> String {
    // Derive only the input-load experiment from the actual shader. No copied
    // renderer is kept in fixtures, and the rendering equations stay identical.
    let source =
        include_str!("../../../crates/bevy-stylized-render/src/shaders/hero/hero_forward.wgsl");
    let original = "let tag_threshold = (tag >> TAG_FACE_THRESHOLD_SHIFT) & 0xFFu;\n        if tag_threshold != 0u {\n            threshold = f32(tag_threshold - 1u) / 254.0;\n        }";
    assert_eq!(
        source.matches(original).count(),
        1,
        "prototype must track its source boundary"
    );
    source
        .replace(
            "#define_import_path bevy_stylized_render::hero::hero_forward",
            "#define_import_path stylized_precision_probe",
        )
        .replace(
            original,
            "threshold = precision_records[tag & 0xFFFFFFu].x;",
        )
        + "\n@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<storage,read> precision_records: array<vec4<f32>>;\n"
}

fn run(
    resources: bevy::render::settings::RenderResources,
    count: u32,
    table: bool,
) -> (CapturedImage, serde_json::Value) {
    let mut app = builder::configured_app(resources, |app| {
        app.add_plugins(MaterialPlugin::<TableHero>::default());
        golden_harness::readiness::install(app);
        app.world_mut()
            .resource_mut::<Assets<Shader>>()
            .insert(
                PROBE.id(),
                Shader::from_wgsl(shader(), "hero_precision_probe.wgsl"),
            )
            .unwrap();
    });
    app.world_mut()
        .remove_resource::<bevy_stylized_render::lod::StylizedLodPolicyResource>();
    let sdf = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[128, 128, 128, 255],
            TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::default(),
        ));
    let mut base = StylizedHeroMaterial {
        face_sdf_texture: Some(sdf),
        debug_view: StylizedHeroDebugView::LightingFactors,
        ..default()
    };
    base.params.character = base
        .params
        .character
        .with_face_basis(Vec3::Z, Vec3::Y, 0.01);
    base.sync_flags();
    let thresholds: Vec<f32> = (0..count)
        .map(|i| 0.49 + 0.02 * (i as f32 + 0.37) / count as f32)
        .collect();
    let table_handle = if table {
        let buffer =
            app.world_mut()
                .resource_mut::<Assets<ShaderBuffer>>()
                .add(ShaderBuffer::from(
                    thresholds
                        .iter()
                        .map(|t| Vec4::new(*t, 0., 0., 0.))
                        .collect::<Vec<_>>(),
                ));
        Some(
            app.world_mut()
                .resource_mut::<Assets<TableHero>>()
                .add(TableHero {
                    base: base.clone(),
                    extension: PrecisionTable { records: buffer },
                }),
        )
    } else {
        None
    };
    let material = app
        .world_mut()
        .resource_mut::<Assets<StylizedHeroMaterial>>()
        .add(base);
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(0.8, 0.8));
    let cols = (count as f32).sqrt().ceil() as u32;
    for (i, threshold) in thresholds.iter().enumerate() {
        let mut entity = app.world_mut().spawn((
            Mesh3d(mesh.clone()),
            Transform::from_xyz(
                (i as u32 % cols) as f32 - (cols - 1) as f32 / 2.,
                (i as u32 / cols) as f32 - (cols - 1) as f32 / 2.,
                0.,
            ),
        ));
        if let Some(handle) = &table_handle {
            entity.insert((MeshMaterial3d(handle.clone()), MeshTag(i as u32)));
        } else {
            entity.insert((
                MeshMaterial3d(material.clone()),
                StylizedInstanceData {
                    face_threshold: Some(*threshold),
                    ..default()
                },
            ));
        }
    }
    let target = capture::create_capture_target(
        &mut app.world_mut().resource_mut::<Assets<Image>>(),
        512,
        512,
    );
    app.world_mut().spawn((
        Camera3d::default(),
        capture::capture_target_bundle(&target),
        Tonemapping::None,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::FixedVertical {
                viewport_height: cols as f32 + 1.,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0., 0., 10.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    golden_harness::readiness::wait(&mut app, 24);
    for _ in 0..60 {
        app.update();
    }
    let render = app.get_sub_app(RenderApp).unwrap();
    let phases = render
        .world()
        .resource::<ViewBinnedRenderPhases<Opaque3d>>();
    let batches: usize = phases
        .0
        .values()
        .map(|p| p.multidrawable_meshes.len() + p.batchable_meshes.len())
        .sum();
    assert_eq!(
        batches, 1,
        "same mesh/material must retain a single opaque phase batch"
    );
    let mut samples = Vec::new();
    for _ in 0..600 {
        let start = Instant::now();
        app.update();
        samples.push(start.elapsed().as_secs_f64() * 1000.);
    }
    let request = capture::request_capture(app.world_mut(), target);
    for _ in 0..600 {
        app.update();
        if let Some(image) = capture::try_take(app.world_mut(), request.clone()) {
            let report = serde_json::json!({"count":count,"table":table,"opaque_phase_batches":batches,"mean_update_ms":samples.iter().sum::<f64>()/samples.len() as f64,"raw_update_ms":samples,"payload_bytes_per_instance":if table{16}else{4},"optimized":!cfg!(debug_assertions)});
            return (image, report);
        }
    }
    panic!("precision probe capture timeout")
}

#[test]
#[ignore = "requires GPU; run the explicit GPU lane"]
fn hero_table_precision_prototype_preserves_batching() {
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let out = std::env::var_os("STYLIZED_PRECISION_OUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/stylized/hero-precision")
                .join(format!(
                    "{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ))
        });
    assert!(!out.exists(), "precision output must be new");
    std::fs::create_dir_all(&out).unwrap();
    eprintln!("PRECISION_REPORT {}", out.join("report.json").display());
    let mut cells = Vec::new();
    for (repeat, count) in (0..3).flat_map(|repeat| [100, 1000].map(|count| (repeat, count))) {
        let (tag, tag_report) = run(resources.clone(), count, false);
        let (table, table_report) = run(resources.clone(), count, true);
        let diff = golden_harness::diff_rgba8(&tag.data, &table.data).unwrap();
        assert!(
            diff.differing_pixels > 100,
            "probe must resolve the precision difference"
        );
        let distinct: std::collections::BTreeSet<_> =
            table.data.as_chunks::<4>().0.iter().map(|p| p[0]).collect();
        assert!(
            distinct.len() > 20,
            "full precision must reach independently shaded instances"
        );
        for (name, image) in [("tag", tag), ("table", table)] {
            std::fs::write(
                out.join(format!("{repeat}-{count}-{name}.png")),
                golden_harness::encode_png(&image).unwrap(),
            )
            .unwrap();
        }
        cells.push(serde_json::json!({"repeat":repeat,"tag":tag_report,"table":table_report,"differing_pixels":diff.differing_pixels}));
    }
    std::fs::write(out.join("report.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"PASS","device":format!("{:?}",resources.2.0),"cells":cells,"scope":"test-only Hero face-threshold table prototype; same shading equations, 0.01 softness; phase batch counts, not submitted draw counters; CPU app update only; no shipping API, table lifecycle or speedup acceptance"})).unwrap()).unwrap();
}
