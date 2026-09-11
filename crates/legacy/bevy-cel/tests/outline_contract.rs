//! Inverted-hull and optional prepass-outline behavioral contracts.

use bevy::{
    app::App,
    asset::{AssetPlugin, Assets},
    color::LinearRgba,
    material::AlphaMode,
    mesh::{Mesh, Mesh3d},
    pbr::MeshMaterial3d,
    prelude::{ChildOf, Handle, MinimalPlugins},
};
use bevy_cel::{
    CharacterMaterialPlugin, CharacterOutline, EyeEmissionSettings, InternalEdgeAccentSettings,
    OUTLINE_FRAGMENT_SHADER_PATH, OUTLINE_VERTEX_SHADER_PATH, OutlineMaterial, OutlineSettings,
};

#[test]
fn outline_width_rejects_malformed_values_and_stays_bounded() {
    // Given
    let malformed = [
        0.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        10_000.0,
    ];

    // When
    let widths = malformed.map(|width| OutlineSettings::default().with_width(width).width());

    // Then
    assert!(widths.into_iter().all(f32::is_finite));
    assert!(
        widths
            .into_iter()
            .all(|width| (0.0..=OutlineSettings::MAX_WIDTH).contains(&width))
    );
    assert!(widths[0].abs() < f32::EPSILON);
    assert!(widths[1].abs() < f32::EPSILON);
    assert!((widths[2] - OutlineSettings::DEFAULT_WIDTH).abs() < f32::EPSILON);
    assert!((widths[5] - OutlineSettings::MAX_WIDTH).abs() < f32::EPSILON);
}

#[test]
fn outline_width_scales_linearly_until_the_depth_aware_bound() {
    // Given
    let narrow = OutlineSettings::default().with_width(2.0);
    let wide = OutlineSettings::default().with_width(4.0);

    // When
    let ratio = wide.width() / narrow.width();

    // Then
    assert!((ratio - 2.0).abs() <= 0.01);
}

#[test]
fn material_color_width_and_task18_suppression_are_independent() {
    // Given
    let color = LinearRgba::new(0.08, 0.12, 0.2, 1.0);

    // When
    let settings = OutlineSettings::default()
        .with_color(color)
        .with_width(3.5)
        .with_exclusion_masks(0.75, 0.25)
        .with_task18_suppression(
            &EyeEmissionSettings::default()
                .with_eye_class(1.0, 1.0)
                .with_masks(0.0, 0.8),
        );

    // Then
    assert_eq!(settings.color(), color);
    assert!((settings.width() - 3.5).abs() < f32::EPSILON);
    assert_eq!(settings.exclusion_masks(), (0.75, 0.8));
}

#[test]
fn genshin_outline_uses_local_vertex_width_and_five_lightmap_classes() {
    let colors = [
        LinearRgba::new(0.1, 0.2, 0.3, 1.0),
        LinearRgba::new(0.2, 0.3, 0.4, 1.0),
        LinearRgba::new(0.3, 0.4, 0.5, 1.0),
        LinearRgba::new(0.4, 0.5, 0.6, 1.0),
        LinearRgba::new(0.5, 0.6, 0.7, 1.0),
    ];
    let settings =
        OutlineSettings::default().with_genshin_source(colors, 0b1110, 0.0025, false, true);
    assert!(settings.uses_genshin_source());
    assert_eq!(settings.genshin_local_width(), 0.0025);

    let vertex = include_str!("../src/shaders/outline_vertex.wgsl");
    let fragment = include_str!("../src/shaders/outline_fragment.wgsl");
    assert!(vertex.contains("vertex.color.a"));
    assert!(vertex.contains("local_position += normalize(vertex.normal)"));
    for threshold in ["alpha > 0.2", "alpha > 0.4", "alpha > 0.6", "alpha > 0.8"] {
        assert!(fragment.contains(threshold));
    }
    assert!(fragment.contains("vec3<f32>(2.2)"));
    assert!(fragment.contains("outline.source_modulation.rgb"));
}

#[test]
fn character_outline_spawns_a_separate_shared_mesh_pass() {
    // Given
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        CharacterMaterialPlugin,
    ));
    let mesh = Handle::<Mesh>::default();
    let material = app
        .world_mut()
        .resource_mut::<Assets<OutlineMaterial>>()
        .add(OutlineMaterial::default());
    let source = app
        .world_mut()
        .spawn((Mesh3d(mesh.clone()), CharacterOutline::new(material)))
        .id();

    // When
    app.update();

    // Then
    let mut replicas = app
        .world_mut()
        .query::<(&Mesh3d, &MeshMaterial3d<OutlineMaterial>, &ChildOf)>();
    let matches = replicas
        .iter(app.world())
        .filter(|(replica_mesh, _, parent)| replica_mesh.0 == mesh && parent.parent() == source)
        .count();
    assert_eq!(matches, 1);
}

#[test]
fn outline_material_is_an_opaque_separate_pass() {
    // Given
    let material = OutlineMaterial::default();

    // When
    let alpha_mode = material.alpha_mode();

    // Then
    assert_eq!(alpha_mode, AlphaMode::Opaque);
    assert_eq!(
        OutlineMaterial::vertex_shader_path(),
        OUTLINE_VERTEX_SHADER_PATH
    );
    assert_eq!(
        OutlineMaterial::fragment_shader_path(),
        OUTLINE_FRAGMENT_SHADER_PATH
    );
}

#[test]
fn internal_edge_accent_disables_cleanly_without_both_prepasses() {
    // Given
    let settings = InternalEdgeAccentSettings::default().with_enabled(true);

    // When
    let states = [
        settings.is_active(false, false),
        settings.is_active(true, false),
        settings.is_active(false, true),
        settings.is_active(true, true),
    ];

    // Then
    assert_eq!(states, [false, false, false, true]);
}

#[test]
fn outline_shaders_preserve_deformation_and_keep_prepass_accent_optional() {
    // Given
    let vertex = include_str!("../src/shaders/outline_vertex.wgsl");
    let accent = include_str!("../src/shaders/internal_edge.wgsl");

    // When
    let deformation_contract = [
        "morph_position",
        "morph_normal",
        "skin_model",
        "skin_normals",
    ];

    // Then
    assert!(
        deformation_contract
            .into_iter()
            .all(|token| vertex.contains(token))
    );
    assert!(vertex.contains("position_world_to_clip"));
    assert!(accent.contains("DEPTH_PREPASS"));
    assert!(accent.contains("NORMAL_PREPASS"));
    assert!(accent.contains("discard"));
}

#[test]
fn alpha_blend_limitation_is_explicit_in_public_settings() {
    // Given
    let settings = OutlineSettings::default();

    // When
    let supported = settings.supports_alpha_mode(AlphaMode::Blend);

    // Then
    assert!(!supported);
    assert!(settings.supports_alpha_mode(AlphaMode::Opaque));
    assert!(!settings.supports_alpha_mode(AlphaMode::Mask(0.5)));
}

#[test]
fn outline_tracks_replaced_source_mesh_and_dynamic_bounds() {
    use bevy::{
        asset::RenderAssetUsages, camera::primitives::Aabb, math::Vec3, mesh::PrimitiveTopology,
    };
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        CharacterMaterialPlugin,
    ));
    app.insert_resource(Assets::<Mesh>::default());
    let make_mesh = || {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD,
        )
    };
    let first = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(make_mesh());
    let second = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(make_mesh());
    let material = app
        .world_mut()
        .resource_mut::<Assets<OutlineMaterial>>()
        .add(OutlineMaterial::default());
    let source = app
        .world_mut()
        .spawn((
            Mesh3d(first),
            CharacterOutline::new(material),
            Aabb::from_min_max(Vec3::ZERO, Vec3::ONE),
        ))
        .id();
    app.update();
    let replacement = Aabb::from_min_max(Vec3::splat(-2.), Vec3::splat(3.));
    app.world_mut()
        .entity_mut(source)
        .insert((Mesh3d(second.clone()), replacement));
    app.update();
    let mut query = app
        .world_mut()
        .query::<(&Mesh3d, &Aabb, &ChildOf, &MeshMaterial3d<OutlineMaterial>)>();
    let rows = query
        .iter(app.world())
        .filter(|(_, _, parent, _)| parent.parent() == source)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0.0, second);
    assert_eq!(rows[0].1.center, replacement.center);
    assert_eq!(rows[0].1.half_extents, replacement.half_extents);
}
