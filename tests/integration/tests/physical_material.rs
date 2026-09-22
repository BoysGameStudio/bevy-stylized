//! Physical material extension and nonzero host-uniform binding regression.
#[path = "support/render_app_builder.rs"]
mod builder;
use bevy::prelude::*;
use bevy_stylized_render::{
    host_inputs::{HostInstanceRecord, HostViewRecord},
    host_material::{
        HostAlphaMode, HostInstanceInput, HostMaterial, HostViewInput, publish_host_inputs,
    },
    physical::{PhysicalCelExtension, PhysicalCelMaterial, PhysicalCelPlugin},
};
use golden_harness::{CapturedImage, capture};

fn image(app: &mut App) -> CapturedImage {
    scene_runtime::capture_in_app(
        app,
        &scenario_model::ident::SessionId::new("physical-material").expect("session id"),
        &scenes::CameraRecipe::new(Vec3::new(0., 0., 5.), Vec3::ZERO),
        (256, 256),
        scene_runtime::Settle::Frames(32),
        false,
        640,
    )
    .expect("physical material capture timed out")
}

fn same(a: &CapturedImage, b: &CapturedImage) {
    assert_eq!(a.data.len(), b.data.len());
    let max = a
        .data
        .iter()
        .zip(&b.data)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    assert!(
        max <= 2,
        "unmodified physical lighting changed by {max}/255"
    );
}

#[test]
#[ignore = "requires GPU; run the physical_material suite"]
fn physical_extension_preserves_standard_lighting_and_host_binding() {
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = builder::configured_app(resources, |app| {
        app.add_plugins(PhysicalCelPlugin)
            .add_systems(PostUpdate, publish_host_inputs);
    });
    let source = StandardMaterial {
        base_color: Color::linear_rgb(0.3, 0.5, 0.7),
        metallic: 0.6,
        perceptual_roughness: 0.35,
        emissive: LinearRgba::rgb(0.01, 0.02, 0.03),
        ..default()
    };
    let standard = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(source.clone());
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(0.8));
    let entity = app
        .world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(standard), Transform::IDENTITY))
        .id();
    let before = image(&mut app);
    let material = PhysicalCelMaterial {
        base: source,
        extension: PhysicalCelExtension::default(),
    };
    let physical = app
        .world_mut()
        .resource_mut::<Assets<PhysicalCelMaterial>>()
        .add(material.clone());
    app.world_mut()
        .entity_mut(entity)
        .remove::<MeshMaterial3d<StandardMaterial>>()
        .insert(MeshMaterial3d(physical));
    let standalone = image(&mut app);
    same(&before, &standalone);
    let host = app
        .world_mut()
        .resource_mut::<Assets<HostMaterial<PhysicalCelMaterial>>>()
        .add(HostMaterial::new(material.clone(), HostAlphaMode::Opaque));
    let view = app
        .world_mut()
        .spawn(HostViewInput(HostViewRecord::default()))
        .id();
    app.world_mut()
        .entity_mut(entity)
        .remove::<MeshMaterial3d<PhysicalCelMaterial>>()
        .insert((
            MeshMaterial3d(host),
            HostInstanceInput {
                view,
                record: HostInstanceRecord::default(),
            },
        ));
    let neutral = image(&mut app);
    same(&before, &neutral);
    app.world_mut()
        .get_mut::<HostInstanceInput>(entity)
        .unwrap()
        .record
        .tint = Vec4::new(1., 0.1, 0.1, 1.);
    let tinted = image(&mut app);
    assert!(
        neutral
            .data
            .iter()
            .zip(&tinted.data)
            .filter(|(a, b)| a != b)
            .count()
            > 100,
        "host tint did not reach the shader"
    );
    let blend = app
        .world_mut()
        .resource_mut::<Assets<HostMaterial<PhysicalCelMaterial>>>()
        .add(HostMaterial::new(material, HostAlphaMode::Blend));
    app.world_mut()
        .entity_mut(entity)
        .insert(MeshMaterial3d(blend));
    app.world_mut()
        .get_mut::<HostInstanceInput>(entity)
        .unwrap()
        .record
        .tint
        .w = 0.;
    let transparent = image(&mut app);
    app.world_mut()
        .entity_mut(entity)
        .insert(Visibility::Hidden);
    let empty = image(&mut app);
    same(&transparent, &empty);
}

#[test]
#[ignore = "requires GPU; run the physical_material suite"]
fn physical_ramp_remaps_light_without_erasing_emission() {
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let mut app = builder::configured_app(resources, |app| {
        app.add_plugins(PhysicalCelPlugin);
    });
    let ramp = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width: 4,
                height: 1,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            &[0, 0, 0, 255],
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::default(),
        ));
    let base = StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::rgb(0.2, 0., 0.),
        ..default()
    };
    let control = app
        .world_mut()
        .resource_mut::<Assets<PhysicalCelMaterial>>()
        .add(PhysicalCelMaterial {
            base: base.clone(),
            extension: PhysicalCelExtension::default(),
        });
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(0.8));
    let entity = app
        .world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(control), Transform::IDENTITY))
        .id();
    let before = image(&mut app);
    let mut extension = PhysicalCelExtension {
        ramp: Some(ramp),
        ..default()
    };
    extension.params.transform.w = 1.;
    extension.params.atlas.z = 4;
    let remapped = app
        .world_mut()
        .resource_mut::<Assets<PhysicalCelMaterial>>()
        .add(PhysicalCelMaterial { base, extension });
    app.world_mut()
        .entity_mut(entity)
        .insert(MeshMaterial3d(remapped));
    let after = image(&mut app);
    let center = (128 * 256 + 128) * 4;
    assert!(
        after.data[center] > 20,
        "authored emission was erased with physical light"
    );
    assert!(
        after.data[center + 1] < 3 && after.data[center + 2] < 3,
        "black ramp must suppress physical diffuse/specular light"
    );
    assert!(
        before.data[center + 1] > after.data[center + 1] + 10,
        "the unmodified physical-light control must actually be lit"
    );
}

#[test]
#[ignore = "requires GPU; run the physical_material suite"]
fn physical_host_resource_layout_is_measured() {
    use bevy::pbr::{MeshPipelineViewLayoutKey, MeshPipelineViewLayouts};
    use bevy::render::{
        RenderApp,
        render_resource::{AsBindGroup, BindingType, BufferBindingType, ShaderStages},
    };
    use bevy_stylized_render::budget;
    let resources = capture::probe_render_resources().expect("required GPU unavailable");
    let device = resources.0.clone();
    let mut app = builder::configured_app(resources, |app| {
        app.add_plugins(PhysicalCelPlugin);
    });
    app.update();
    app.update();
    let entries = HostMaterial::<PhysicalCelMaterial>::bind_group_layout_entries(&device, true);
    let material = budget::count_layout_entries(&entries, ShaderStages::FRAGMENT);
    let layouts = app
        .sub_app(RenderApp)
        .world()
        .resource::<MeshPipelineViewLayouts>();
    let layout = layouts.get_view_layout(MeshPipelineViewLayoutKey::empty());
    let view = budget::count_layout_entries(&layout.main_layout.entries, ShaderStages::FRAGMENT);
    let uniforms = entries
        .iter()
        .filter_map(|e| match e.ty {
            BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                min_binding_size: Some(size),
                ..
            } => Some((e.binding, size.get())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        uniforms.iter().all(|(_, size)| *size <= 16_384),
        "a material uniform exceeds the shared binding limit"
    );
    assert_eq!(
        material.storage_buffers, 0,
        "fixed material layout must not silently select bindless storage"
    );
    let counts = |u: bevy_stylized::stylized_core::StylizedStageUsage| {
        serde_json::json!({
        "textures":u.textures,"samplers":u.samplers,"uniform_buffers":u.uniform_buffers,"storage_buffers":u.storage_buffers})
    };
    let value = serde_json::json!({"material_fragment":counts(material),"view_fragment":counts(view),"uniform_bindings":uniforms,
        "scope":"Measured fixed material and minimal view layout; platform routing and optional view features require separate qualification"});
    std::fs::create_dir_all("target/tmp/physical-material").unwrap();
    std::fs::write(
        "target/tmp/physical-material/layout.json",
        serde_json::to_string_pretty(&value).unwrap(),
    )
    .unwrap();
    println!("PHYSICAL_LAYOUT {value}");
}
