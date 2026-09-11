//! Directional face-SDF material and runtime contracts.

use bevy::{
    asset::{AssetPlugin, Assets, RenderAssetUsages},
    image::Image,
    prelude::{
        App, GlobalTransform, Handle, MeshMaterial3d, MinimalPlugins, Quat, Transform,
        TransformPlugin, Vec3,
    },
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_cel::{
    CharacterExtension, CharacterMaterial, CharacterMaterialPlugin, FACE_SDF_LEFT_SAMPLER_BINDING,
    FACE_SDF_LEFT_TEXTURE_BINDING, FACE_SDF_RIGHT_SAMPLER_BINDING, FACE_SDF_RIGHT_TEXTURE_BINDING,
    FACE_SDF_UNIFORM_BINDING, FACE_SHADOW_SAMPLER_BINDING, FACE_SHADOW_TEXTURE_BINDING, FaceBasis,
    FaceBasisError, FaceSdfDebugMode, FaceSdfMapSide, FaceSdfMaterialInstance, FaceSdfRig,
    FaceSdfSettings, FaceSdfTexturePair,
};

fn assert_coordinates(actual: [f32; 2], expected: [f32; 2]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(actual, expected)| (actual - expected).abs() < 1.0e-5)
    );
}

fn linear_image(width: u32, height: u32) -> Image {
    Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[128, 128, 128, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    )
}

#[test]
fn face_basis_orthonormalizes_finite_non_degenerate_inputs()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let forward = Vec3::new(0.0, 0.2, 2.0);
    let right = Vec3::new(3.0, 0.1, 0.4);

    // When
    let basis = FaceBasis::try_new(forward, right)?;

    // Then
    assert!((basis.forward().length() - 1.0).abs() < 1.0e-5);
    assert!((basis.right().length() - 1.0).abs() < 1.0e-5);
    assert!(basis.forward().dot(basis.right()).abs() < 1.0e-5);
    assert!(basis.forward().y.abs() < f32::EPSILON);
    assert!(basis.right().y.abs() < f32::EPSILON);
    Ok(())
}

#[test]
fn face_basis_rejects_non_finite_degenerate_and_parallel_inputs() {
    // Given
    let cases = [
        (Vec3::NAN, Vec3::X, FaceBasisError::NonFinite),
        (Vec3::Y, Vec3::X, FaceBasisError::Degenerate),
        (Vec3::Z, Vec3::Z, FaceBasisError::Parallel),
    ];

    // When / Then
    for (forward, right, expected) in cases {
        assert_eq!(FaceBasis::try_new(forward, right), Err(expected));
    }
}

#[test]
fn directional_threshold_selects_side_and_coordinates_at_cardinal_angles()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let settings = FaceSdfSettings::from_basis(FaceBasis::try_new(Vec3::Z, Vec3::X)?)
        .with_bias_rotation(0.0, 0.0);

    // When
    let right = settings
        .light_sample(Vec3::X)
        .ok_or_else(|| std::io::Error::other("right light did not project"))?;
    let front = settings
        .light_sample(Vec3::Z)
        .ok_or_else(|| std::io::Error::other("front light did not project"))?;
    let left = settings
        .light_sample(Vec3::NEG_X)
        .ok_or_else(|| std::io::Error::other("left light did not project"))?;

    // Then
    assert_eq!(right.selected_map(), FaceSdfMapSide::Right);
    assert_eq!(front.selected_map(), FaceSdfMapSide::Right);
    assert_eq!(left.selected_map(), FaceSdfMapSide::Left);
    assert!((right.threshold() - 0.5).abs() < 1.0e-5);
    assert!(front.threshold().abs() < 1.0e-5);
    assert!((left.threshold() - 0.5).abs() < 1.0e-5);
    assert_coordinates(right.light_coordinate(), [1.0, 0.5]);
    assert_coordinates(front.light_coordinate(), [0.5, 1.0]);
    assert_coordinates(left.light_coordinate(), [0.0, 0.5]);
    Ok(())
}

#[test]
fn genshin_direction_uses_the_authored_half_dot_power_threshold()
-> Result<(), Box<dyn std::error::Error>> {
    let settings = FaceSdfSettings::from_basis(FaceBasis::try_new(Vec3::Z, Vec3::NEG_X)?)
        .with_genshin_direction(0.55);
    assert!(settings.uses_genshin_direction());
    assert_eq!(settings.genshin_shadow_position(), 0.55);

    for (direction, expected) in [
        (Vec3::Z, 0.999_142),
        (Vec3::new(1.0, 0.0, 0.0), 0.551_956),
        (Vec3::new(0.0, 0.0, -1.0), 0.0),
    ] {
        let actual = settings
            .light_sample(direction)
            .expect("projected light")
            .threshold();
        assert!(
            (actual - expected).abs() < 0.000_01,
            "{direction:?}: {actual}"
        );
    }
    Ok(())
}

#[test]
fn face_controls_clamp_invalid_values_and_preserve_typed_debug_state() {
    // Given
    let settings = FaceSdfSettings::default()
        .with_bias_rotation(f32::NAN, f32::INFINITY)
        .with_softness(f32::NAN)
        .with_shadow_tint([f32::NAN, -1.0, 2.0])
        .with_external_shadow_influence(f32::INFINITY)
        .with_debug(FaceSdfDebugMode::LightCoordinate);

    // When
    let controls = (
        settings.bias(),
        settings.rotation(),
        settings.softness(),
        settings.shadow_tint(),
        settings.external_shadow_influence(),
        settings.debug_mode(),
    );

    // Then
    assert_eq!(
        controls,
        (
            0.0,
            0.0,
            0.0,
            [1.0, 1.0, 1.0],
            0.0,
            FaceSdfDebugMode::LightCoordinate,
        )
    );
}

#[test]
fn shared_gacha_face_map_can_mirror_only_the_right_facing_lookup() {
    let defaults = FaceSdfSettings::default();
    let mirrored = defaults.with_right_uv_mirrored(true);

    assert!(!defaults.right_uv_mirrored());
    assert!(mirrored.right_uv_mirrored());

    let source = include_str!("../src/shaders/character_forward.wgsl");
    assert!(source.contains("right_uv.x = 1.0 - right_uv.x"));
    assert!(source.contains("&& genshin_source_light_active()"));
    assert!(source.contains("resolved_genshin_direction_to_light(pbr_input.N)"));
}

#[test]
fn face_defaults_use_white_tint_and_crisp_valid_softness() {
    // Given
    let defaults = FaceSdfSettings::default();

    // When
    let softened = defaults.with_softness(0.25);

    // Then
    assert!(
        defaults
            .shadow_tint()
            .into_iter()
            .all(|channel| (channel - 1.0).abs() < f32::EPSILON)
    );
    assert!(defaults.softness().abs() < f32::EPSILON);
    assert!((softened.softness() - 0.25).abs() < f32::EPSILON);
}

#[test]
fn vertical_and_near_vertical_light_have_no_directional_face_sample()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let settings = FaceSdfSettings::from_basis(FaceBasis::try_new(Vec3::Z, Vec3::X)?);

    // When
    let vertical = settings.light_sample(Vec3::Y);
    let near_vertical = settings.light_sample(Vec3::new(0.001, 1.0, 0.001));

    // Then
    assert!(vertical.is_none());
    assert!(near_vertical.is_none());
    Ok(())
}

#[test]
fn face_textures_require_matching_linear_nonzero_images() {
    // Given
    let left_handle = Handle::default();
    let right_handle = Handle::default();
    let linear = linear_image(8, 8);
    let mismatched = linear_image(4, 8);
    let srgb = Image::new_fill(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[128, 128, 128, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    // When
    let valid = CharacterExtension::default().with_face_sdf_textures(FaceSdfTexturePair::new(
        (left_handle.clone(), &linear),
        (right_handle.clone(), &linear),
    ));
    let invalid_size =
        CharacterExtension::default().with_face_sdf_textures(FaceSdfTexturePair::new(
            (left_handle.clone(), &linear),
            (right_handle.clone(), &mismatched),
        ));
    let invalid_srgb = CharacterExtension::default().with_face_sdf_textures(
        FaceSdfTexturePair::new((left_handle, &srgb), (right_handle, &linear)),
    );

    // Then
    assert!(valid.face_sdf_settings().uses_textures());
    assert!(valid.face_sdf_textures().is_some());
    assert!(!invalid_size.face_sdf_settings().uses_textures());
    assert!(invalid_size.face_sdf_textures().is_none());
    assert!(!invalid_srgb.face_sdf_settings().uses_textures());
    assert!(invalid_srgb.face_sdf_textures().is_none());

    let auxiliary =
        CharacterExtension::default().with_face_shadow_texture(Handle::default(), &linear);
    let invalid_auxiliary =
        CharacterExtension::default().with_face_shadow_texture(Handle::default(), &srgb);
    assert!(auxiliary.face_shadow_texture().is_some());
    assert!(
        auxiliary
            .face_sdf_settings()
            .uses_genshin_face_shadow_texture()
    );
    assert!(invalid_auxiliary.face_shadow_texture().is_none());
}

#[test]
fn face_bindings_follow_ramp_and_ilm_without_renumbering() {
    // Given
    let face_bindings = [
        FACE_SDF_UNIFORM_BINDING,
        FACE_SDF_LEFT_TEXTURE_BINDING,
        FACE_SDF_LEFT_SAMPLER_BINDING,
        FACE_SDF_RIGHT_TEXTURE_BINDING,
        FACE_SDF_RIGHT_SAMPLER_BINDING,
    ];

    // When
    let source = include_str!("../src/shaders/character_forward.wgsl");

    // Then
    assert_eq!(face_bindings, [106, 107, 108, 109, 110]);
    assert_eq!(
        [FACE_SHADOW_TEXTURE_BINDING, FACE_SHADOW_SAMPLER_BINDING],
        [116, 117]
    );
    assert!(
        [
            "@binding(100)",
            "@binding(101)",
            "@binding(102)",
            "@binding(103)",
            "@binding(104)",
            "@binding(105)",
            "@binding(106)",
            "@binding(107)",
            "@binding(108)",
            "@binding(109)",
            "@binding(110)",
            "@binding(116)",
            "@binding(117)",
        ]
        .into_iter()
        .all(|token| source.contains(token))
    );
}

#[test]
fn shader_uses_directional_sdf_and_exposes_all_debug_outputs() {
    // Given
    let source = include_str!("../src/shaders/character_forward.wgsl");

    // When
    let required = [
        "direction_to_light",
        "face_light_sample",
        "face_sdf_lighting_factors",
        "let soft_width = hard_width + 0.05",
        "face_factors.soft",
        "FACE_SDF_DEBUG_SELECTED_MAP",
        "FACE_SDF_DEBUG_THRESHOLD",
        "FACE_SDF_DEBUG_LIGHT_COORDINATE",
        "FACE_LIGHT_EPSILON_SQUARED",
        "projected_light_length_squared",
        "sample.valid",
        "external_shadow_influence",
        "face_shadow_texture",
        "base_diffuse_alpha - 0.03",
        "pow(clamp(1.5 - shadow_position, 0.0, 1.5), 3.0)",
    ];

    // Then
    assert!(required.into_iter().all(|token| source.contains(token)));
    assert!(!source.contains("face_ndotl"));
}

#[test]
fn material_instances_clone_the_source_and_update_from_current_head_global_transform()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        CharacterMaterialPlugin,
    ));
    let source = app
        .world_mut()
        .resource_mut::<Assets<CharacterMaterial>>()
        .add(CharacterMaterial::default());
    let head = app
        .world_mut()
        .spawn((
            Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
        ))
        .id();
    let basis = FaceBasis::try_new(Vec3::Z, Vec3::X)?;
    let first = FaceSdfMaterialInstance::try_new(
        FaceSdfRig::new(head, basis),
        &source,
        &mut app.world_mut().resource_mut::<Assets<CharacterMaterial>>(),
    )?;
    let second = FaceSdfMaterialInstance::try_new(
        FaceSdfRig::new(head, basis),
        &source,
        &mut app.world_mut().resource_mut::<Assets<CharacterMaterial>>(),
    )?;
    let first_handle = first.material().clone();
    let second_handle = second.material().clone();
    app.world_mut()
        .spawn((MeshMaterial3d(first_handle.clone()), first));

    // When
    app.update();

    // Then
    assert_ne!(first_handle.id(), source.id());
    assert_ne!(second_handle.id(), source.id());
    assert_ne!(first_handle.id(), second_handle.id());
    let materials = app.world().resource::<Assets<CharacterMaterial>>();
    let source_material = materials
        .get(&source)
        .ok_or_else(|| std::io::Error::other("source material is missing"))?;
    assert!(
        source_material
            .extension
            .face_sdf_settings()
            .basis()
            .is_none()
    );
    let world_basis = materials
        .get(&first_handle)
        .ok_or_else(|| std::io::Error::other("unique material is missing"))?
        .extension
        .face_sdf_settings()
        .basis()
        .ok_or_else(|| std::io::Error::other("head basis did not update"))?;
    assert!(
        world_basis.forward().distance(Vec3::X) < 1.0e-5,
        "world basis was {world_basis:?}"
    );
    assert!(world_basis.right().distance(Vec3::NEG_Z) < 1.0e-5);
    Ok(())
}

#[test]
fn missing_head_disables_only_face_override_on_the_unique_material()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        CharacterMaterialPlugin,
    ));
    let source = app
        .world_mut()
        .resource_mut::<Assets<CharacterMaterial>>()
        .add(CharacterMaterial::default());
    let missing_head = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(missing_head);
    let basis = FaceBasis::try_new(Vec3::Z, Vec3::X)?;
    let instance = FaceSdfMaterialInstance::try_new(
        FaceSdfRig::new(missing_head, basis),
        &source,
        &mut app.world_mut().resource_mut::<Assets<CharacterMaterial>>(),
    )?;
    let handle = instance.material().clone();
    app.world_mut()
        .spawn((MeshMaterial3d(handle.clone()), instance));

    // When
    app.update();

    // Then
    let material = app
        .world()
        .resource::<Assets<CharacterMaterial>>()
        .get(&handle)
        .ok_or_else(|| std::io::Error::other("unique material is missing"))?;
    assert!(material.extension.face_sdf_settings().basis().is_none());
    assert_eq!(
        material.base.base_color,
        CharacterMaterial::default().base.base_color
    );
    assert!(!material.extension.ramp_settings().uses_texture());
    assert!(!material.extension.ilm_settings().uses_texture());
    Ok(())
}
