//! Engine-side golden captures of the fixed legacy `bevy_cel` stack rendered
//! with real content: the Tenacity Odette character (third-party
//! HoYoverse-derived art, referenced by absolute path only, never copied).
//!
//! The material wiring mirrors Tenacity `view-ink::models` (`odette_material`,
//! `setup_character_npr_assets`) and its Showcase lighting conventions
//! (`Tonemapping::None`, ambient 250, Genshin Default palette, no physical
//! lights). All captures are local-only under `fixtures/golden-local/`.

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, AssetServer, Assets, LoadState},
    camera::{primitives::Aabb, visibility::Visibility},
    core_pipeline::tonemapping::Tonemapping,
    gltf::{GltfAssetLabel, GltfMaterialName},
    image::{Image, ImageAddressMode, ImageLoaderSettings, ImageSampler},
    math::{Quat, Vec3},
    pbr::{MeshMaterial3d, StandardMaterial},
    prelude::{
        AlphaMode, Camera3d, ClearColor, Color, Commands, Entity, GlobalTransform,
        IntoScheduleConfigs, LinearRgba, Name, Query, Res, ResMut, Resource, Transform, Update,
        With, Without,
    },
    render::{
        RenderPlugin,
        renderer::RenderAdapterInfo,
        settings::{RenderCreation, RenderResources},
    },
    window::{ExitCondition, WindowPlugin},
    world_serialization::WorldAssetRoot,
};
use bevy_cel::{
    CelLightingSettings, CharacterMaterial, CharacterMaterialPlugin, CharacterOutline,
    EyeEmissionSettings, FaceBasis, FaceSdfMaterialInstance, FaceSdfRig, FaceSdfSettings,
    FaceSdfTexturePair, GenshinLightSource, GenshinLightingPalette, GenshinMetallicSettings,
    HairHighlightSettings, IlmLayout, IlmSettings, OutlineCullMode, OutlineMaterial,
    OutlineSettings, ToonRampSettings, ToonSpecularSettings, character_material,
};
use golden_harness::{
    CapturedImage, DiffReport, GoldenPaths, GoldenProvenance, Tolerance, capture, diff_rgba8,
};

/// Default location of the sibling Tenacity checkout.
pub const DEFAULT_TENACITY_ROOT: &str = "/home/x/Projects/Tenacity";
/// Environment variable overriding the Tenacity checkout location.
pub const TENACITY_ROOT_ENV: &str = "TENACITY_ROOT";
/// Environment variable that rebaselines instead of diffing when set to `1`.
pub const GOLDEN_REBASELINE_ENV: &str = "GOLDEN_REBASELINE";

const ODETTE_GLB: &str = "models/odette.glb";
const ODETTE_NPR_ROOT: &str = "models/odette-npr";
const ODETTE_FACE_BONE: &str = "DEF-spine.006";
const CAPTURE_WIDTH: u32 = 512;
const CAPTURE_HEIGHT: u32 = 512;
const SETTLE_FRAMES: u32 = 20;
const MAX_WAIT_FRAMES: u32 = 600;

// Tenacity view-ink Odette calibration (models.rs §ODETTE_PROFILE).
const ODETTE_SCALE: f32 = 31.5;
const ODETTE_FEET_OFFSET_Y: f32 = 24.5;
const ODETTE_FORWARD_YAW_RADIANS: f32 = core::f32::consts::PI;

// Tenacity showcase lighting conventions (showcase/environment.rs).
const GACHA_REFERENCE_AMBIENT: f32 = 250.0;
const SHOWCASE_CLEAR_COLOR: Color = Color::srgb(0.105, 0.12, 0.13);
// GachaPeriod::Day direction-to-light (Blender -25 degree X tilt converted).
const DAY_DIRECTION_TO_LIGHT: Vec3 = Vec3::new(0.422_618_33, 0.906_307_8, 0.0);

// Tenacity view-ink Odette NPR constants (models.rs).
const ODETTE_SHADOW_RAMP_ROWS: u32 = 20;
const ODETTE_SHADOW_RAMP_COLUMNS: u32 = 256;
const ODETTE_RAMP_LIGHT_SCALE: f32 = 1.0;
const ODETTE_BODY_GLOBAL_SHADOW_COLOR: [f32; 3] = [1.0, 1.0, 1.0];
const ODETTE_FACE_GLOBAL_SHADOW_COLOR: [f32; 3] = [0.7, 0.7, 0.8];
const ODETTE_READABILITY_FLOOR: f32 = 0.22;
const ODETTE_FACE_READABILITY_FLOOR: f32 = 0.35;
const ODETTE_FACE_DAY_SHADOW_TINT: [f32; 3] = [0.899_384_5, 0.687_030_85, 0.632_042_65];
const ODETTE_FACE_SHADOW_SOFTNESS: f32 = 0.0001;
const ODETTE_SHADOW_RAMP_IS_SRGB: bool = true;
const ODETTE_OUTLINE_WIDTH: f32 = 0.125;
const ODETTE_SOURCE_OUTLINE_LOCAL_WIDTH: f32 = 0.0025;
const ODETTE_NEUTRAL_OUTLINE: LinearRgba = LinearRgba::new(0.0625, 0.085, 0.125, 1.0);
const ODETTE_HAIR_OUTLINE_COLORS: [LinearRgba; 5] = [
    LinearRgba::new(0.344_074_43, 0.419_952_84, 0.544_117_7, 1.0),
    LinearRgba::new(0.536_764_7, 0.276_275_9, 0.276_275_9, 1.0),
    LinearRgba::new(0.523_923_1, 0.376_297_56, 0.588_235_26, 1.0),
    LinearRgba::new(0.257_352_95, 0.164_251_85, 0.102_184_266, 1.0),
    LinearRgba::new(0.156_141_88, 0.325_545_9, 0.558_823_5, 1.0),
];
const ODETTE_BODY_OUTLINE_COLORS: [LinearRgba; 5] = [
    LinearRgba::new(0.280_276_72, 0.374_776_1, 0.529_411_8, 1.0),
    LinearRgba::new(0.691_176_35, 0.504_292_37, 0.472_642_66, 1.0),
    LinearRgba::new(0.345_588_1, 0.276_978_76, 0.231_239_08, 1.0),
    LinearRgba::new(0.437_445_82, 0.455_579_58, 0.639_705_9, 1.0),
    LinearRgba::new(0.053_525_023, 0.058_241_77, 0.080_882_3, 1.0),
];
const ODETTE_DRESS_OUTLINE_COLORS: [LinearRgba; 5] = [
    ODETTE_HAIR_OUTLINE_COLORS[0],
    ODETTE_HAIR_OUTLINE_COLORS[1],
    LinearRgba::new(0.523_923_1, 0.376_297_5, 0.588_235_2, 1.0),
    LinearRgba::new(0.257_352_92, 0.164_251_8, 0.102_184_24, 1.0),
    LinearRgba::new(0.400_627_1, 0.423_554_6, 0.477_941_1, 1.0),
];
const ODETTE_BODY01_OUTLINE_COLORS: [LinearRgba; 5] = [
    LinearRgba::new(0.231_833_85, 0.312_516_1, 0.470_588_15, 1.0),
    LinearRgba::new(0.536_764_7, 0.276_275_9, 0.276_275_9, 1.0),
    ODETTE_NEUTRAL_OUTLINE,
    ODETTE_NEUTRAL_OUTLINE,
    ODETTE_NEUTRAL_OUTLINE,
];
const ODETTE_DRESS01_OUTLINE_COLORS: [LinearRgba; 5] = [
    LinearRgba::new(0.409_169_52, 0.484_589_76, 0.632_352_95, 1.0),
    LinearRgba::new(0.536_764_7, 0.276_275_87, 0.276_275_87, 1.0),
    ODETTE_NEUTRAL_OUTLINE,
    ODETTE_NEUTRAL_OUTLINE,
    ODETTE_NEUTRAL_OUTLINE,
];
const ODETTE_FACE_OUTLINE_COLORS: [LinearRgba; 5] =
    [LinearRgba::new(0.691_176_4, 0.504_292_37, 0.472_642_66, 1.0); 5];
const ODETTE_NEUTRAL_RAMP: [u8; 12] = [255; 12];

/// Tenacity's per-primitive material classification (models.rs `odette_surface`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OdetteSurface {
    /// `HoYoverse - Odette Hair`.
    Hair,
    /// `HoYoverse - Odette Body`.
    Body,
    /// `HoYoverse - Odette Dress`.
    Dress,
    /// `HoYoverse - Odette Body01`.
    Body01,
    /// `HoYoverse - Odette Dress01`.
    Dress01,
    /// `HoYoverse - Odette Face`.
    Face,
    /// `HoYoverse - Odette Brow`.
    Brow,
    /// Pupil materials (eye readability path).
    Eye,
    /// Everything else.
    Neutral,
}

fn odette_surface(material: Option<&GltfMaterialName>) -> OdetteSurface {
    match material.map(|name| name.as_ref()) {
        Some("HoYoverse - Odette Hair") => OdetteSurface::Hair,
        Some("HoYoverse - Odette Body") => OdetteSurface::Body,
        Some("HoYoverse - Odette Dress") => OdetteSurface::Dress,
        Some("HoYoverse - Odette Body01") => OdetteSurface::Body01,
        Some("HoYoverse - Odette Dress01") => OdetteSurface::Dress01,
        Some("HoYoverse - Odette Face") => OdetteSurface::Face,
        Some("HoYoverse - Odette Brow") => OdetteSurface::Brow,
        Some(name) if name.contains("Pupil") || name.contains("pupil") => OdetteSurface::Eye,
        Some(_) | None => OdetteSurface::Neutral,
    }
}

/// Branches below these nodes never receive an outline (models.rs).
fn odette_outline_allowed(name: Option<&Name>, inherited: bool) -> bool {
    inherited && !name.is_some_and(|name| matches!(name.as_str(), "Pupil" | "EyeStar"))
}

/// One golden case of the capture matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabCase {
    /// Case name used for the PNG/sidecar file stem.
    pub name: &'static str,
    /// Camera framing strategy.
    pub framing: Framing,
    /// Whether inverted-hull outlines are attached.
    pub outlines: bool,
}

/// Deterministic camera framing for a case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Framing {
    /// Fit the union of mesh bounds with a fixed view direction.
    FullBody,
    /// Fixed offset from the `DEF-spine.006` head bone.
    FaceCloseUp,
}

/// The fixed reference capture matrix.
pub const CASES: &[LabCase] = &[
    LabCase {
        name: "full_body",
        framing: Framing::FullBody,
        outlines: true,
    },
    LabCase {
        name: "face_closeup",
        framing: Framing::FaceCloseUp,
        outlines: true,
    },
    LabCase {
        name: "full_body_no_outline",
        framing: Framing::FullBody,
        outlines: false,
    },
];

/// The outcome of running one case.
#[derive(Clone, Debug)]
pub enum CaseOutcome {
    /// A fresh baseline was written (first run or rebaseline).
    Baselined,
    /// The capture matched the stored baseline within tolerance.
    Matched(DiffReport),
    /// The capture exceeded the tolerance; a candidate PNG was written.
    Exceeded(DiffReport),
}

/// Errors produced while preparing or running a case.
#[derive(Debug, thiserror::Error)]
pub enum LabError {
    /// A harness or IO failure.
    #[error(transparent)]
    Golden(#[from] golden_harness::GoldenError),
    /// Plain filesystem failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The Odette scene never became ready.
    #[error("Odette scene did not become ready: {0}")]
    SceneNotReady(&'static str),
    /// Report serialization failure.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Resolves the Tenacity checkout root.
#[must_use]
pub fn tenacity_root() -> PathBuf {
    std::env::var(TENACITY_ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_TENACITY_ROOT))
}

/// The Tenacity asset root used as the Bevy `assets` directory.
#[must_use]
pub fn tenacity_asset_root() -> PathBuf {
    tenacity_root().join("crates/client-bevy/assets")
}

/// The local-only output directory for Odette golden captures.
#[must_use]
pub fn golden_local_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/golden-local/odette")
        .canonicalize()
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/golden-local/odette")
        })
}

/// Whether the Odette GLB and every NPR sidecar exist under the asset root.
#[must_use]
pub fn assets_available() -> bool {
    let root = tenacity_asset_root();
    let required = [
        ODETTE_GLB,
        "models/odette-npr/shadow_ramp.png",
        "models/odette-npr/hair_lightmap.png",
        "models/odette-npr/body_lightmap.png",
        "models/odette-npr/body01_lightmap.png",
        "models/odette-npr/face_lightmap.png",
        "models/odette-npr/face_shadow.png",
        "models/odette-npr/face02_shadow.png",
        "models/odette-npr/metal_map.png",
        "models/odette-npr/specular_ramp.png",
    ];
    required.iter().all(|path| root.join(path).is_file())
}

/// Sidecar image handles mirroring Tenacity's `CharacterNprAssets`.
#[derive(Resource, Clone)]
struct NprAssets {
    neutral_ramp: bevy::prelude::Handle<Image>,
    shadow_ramp: bevy::prelude::Handle<Image>,
    hair_ilm: bevy::prelude::Handle<Image>,
    body_ilm: bevy::prelude::Handle<Image>,
    body01_ilm: bevy::prelude::Handle<Image>,
    face_sdf: bevy::prelude::Handle<Image>,
    face_shadow: bevy::prelude::Handle<Image>,
    face02_shadow: bevy::prelude::Handle<Image>,
    metal_map: bevy::prelude::Handle<Image>,
    specular_ramp: bevy::prelude::Handle<Image>,
}

/// Case parameters consumed by the scene setup systems.
#[derive(Resource, Clone)]
struct CaseConfig {
    case: LabCase,
    target: capture::CaptureTarget,
}

#[derive(Resource, Default)]
struct LabState {
    adopted: bool,
}

fn odette_npr_sampler(name: &str) -> ImageSampler {
    // Match the source Blender nodes: the authored shadow ramp is Closest,
    // the packed ILM/face maps are Cubic (linear is the supported filter).
    if name == "shadow_ramp.png" {
        ImageSampler::nearest()
    } else {
        let mut descriptor = bevy::image::ImageSamplerDescriptor::linear();
        descriptor.address_mode_u = ImageAddressMode::Repeat;
        descriptor.address_mode_v = ImageAddressMode::Repeat;
        descriptor.address_mode_w = ImageAddressMode::Repeat;
        ImageSampler::Descriptor(descriptor)
    }
}

fn load_sidecar(
    asset_server: &AssetServer,
    name: &'static str,
    is_srgb: bool,
) -> bevy::prelude::Handle<Image> {
    asset_server
        .load_builder()
        .with_settings(move |settings: &mut ImageLoaderSettings| {
            settings.is_srgb = is_srgb;
            settings.sampler = odette_npr_sampler(name);
        })
        .load(format!("{ODETTE_NPR_ROOT}/{name}"))
}

fn sidecars_ready(asset_server: &AssetServer, images: &Assets<Image>, npr: &NprAssets) -> bool {
    [
        &npr.shadow_ramp,
        &npr.hair_ilm,
        &npr.body_ilm,
        &npr.body01_ilm,
        &npr.face_sdf,
        &npr.face_shadow,
        &npr.face02_shadow,
        &npr.metal_map,
        &npr.specular_ramp,
    ]
    .into_iter()
    .all(|handle| {
        matches!(asset_server.load_state(handle.id()), LoadState::Loaded) && images.contains(handle)
    })
}

fn spawn_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    let npr = NprAssets {
        neutral_ramp: images.add(golden_neutral_ramp()),
        shadow_ramp: load_sidecar(&asset_server, "shadow_ramp.png", ODETTE_SHADOW_RAMP_IS_SRGB),
        hair_ilm: load_sidecar(&asset_server, "hair_lightmap.png", false),
        body_ilm: load_sidecar(&asset_server, "body_lightmap.png", false),
        body01_ilm: load_sidecar(&asset_server, "body01_lightmap.png", false),
        face_sdf: load_sidecar(&asset_server, "face_lightmap.png", false),
        face_shadow: load_sidecar(&asset_server, "face_shadow.png", false),
        face02_shadow: load_sidecar(&asset_server, "face02_shadow.png", false),
        metal_map: load_sidecar(&asset_server, "metal_map.png", true),
        specular_ramp: load_sidecar(&asset_server, "specular_ramp.png", false),
    };
    commands.insert_resource(npr);
    let scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset(ODETTE_GLB));
    commands.spawn((
        WorldAssetRoot(scene),
        // Tenacity ModelAssetBasis: source-scale, feet offset, forward yaw.
        Transform {
            translation: Vec3::new(0.0, ODETTE_FEET_OFFSET_Y, 0.0),
            rotation: Quat::from_rotation_y(ODETTE_FORWARD_YAW_RADIANS),
            scale: Vec3::splat(ODETTE_SCALE),
        },
    ));
}

fn golden_neutral_ramp() -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let mut image = Image::new_fill(
        Extent3d {
            width: 3,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &ODETTE_NEUTRAL_RAMP,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

/// Port of Tenacity `CharacterNprAssets::odette_material` onto the frozen
/// b5b26ac-era `bevy_cel` API (env-var metallic toggle dropped: always on).
fn odette_material(
    mut source: StandardMaterial,
    surface: OdetteSurface,
    npr: &NprAssets,
    images: &Assets<Image>,
) -> CharacterMaterial {
    let packed_normal = source.normal_map_texture.clone();
    source.unlit = false;
    source.metallic = 0.0;
    source.perceptual_roughness = 1.0;
    source.reflectance = 0.0;
    // Gacha uses Diffuse alpha as shader control data, not transparency.
    source.alpha_mode = AlphaMode::Opaque;
    source.base_color = Color::WHITE;
    let mut material = character_material(source);
    material.extension = match surface {
        OdetteSurface::Hair
        | OdetteSurface::Body
        | OdetteSurface::Dress
        | OdetteSurface::Body01
        | OdetteSurface::Dress01 => {
            let ilm = match surface {
                OdetteSurface::Hair | OdetteSurface::Dress => &npr.hair_ilm,
                OdetteSurface::Body => &npr.body_ilm,
                OdetteSurface::Body01 | OdetteSurface::Dress01 => &npr.body01_ilm,
                _ => unreachable!(),
            };
            let image = images.get(ilm).expect("ILM readiness checked");
            let metal_map = images.get(&npr.metal_map).expect("metal map ready");
            let specular_ramp = images.get(&npr.specular_ramp).expect("specular ramp ready");
            let packed_normal = packed_normal.expect("body/hair keeps its packed normal");
            let packed_normal_image = images.get(&packed_normal).expect("packed normal loaded");
            let hair = HairHighlightSettings::default()
                .with_tint_shift([0.72, 0.84, 1.0], 0.08)
                .with_shape(24.0, 0.62)
                .with_softness_intensity(0.06, 0.08);
            let (specular_power, specular_intensity) = match surface {
                OdetteSurface::Hair => (4.0, 0.063),
                OdetteSurface::Body | OdetteSurface::Dress => (10.0, 0.1),
                OdetteSurface::Body01 | OdetteSurface::Dress01 => (50.0, 0.6),
                _ => unreachable!(),
            };
            let authored_emission_gain = match surface {
                OdetteSurface::Hair => 2.2,
                OdetteSurface::Body => 2.0,
                OdetteSurface::Dress => 1.5,
                OdetteSurface::Body01 | OdetteSurface::Dress01 => 3.5,
                _ => unreachable!(),
            };
            let material_id_mask = match surface {
                OdetteSurface::Body => 0b1111,
                OdetteSurface::Hair | OdetteSurface::Body01 | OdetteSurface::Dress01 => 0b1110,
                OdetteSurface::Dress => 0b0110,
                _ => unreachable!(),
            };
            let specular = ToonSpecularSettings::default()
                .with_tint_intensity([1.0; 3], specular_intensity)
                .with_shape(specular_power, 0.58)
                .with_softness(0.05);
            let lighting = CelLightingSettings::default()
                .with_hair(hair)
                .with_specular(specular)
                .with_material_masks(1.0, 0.0, 1.0)
                .with_readability_floor(ODETTE_READABILITY_FLOOR)
                .with_hair_enabled(false)
                .with_specular_enabled(true);
            material
                .extension
                .with_ramp_texture(npr.shadow_ramp.clone())
                .with_ramp_settings(
                    ToonRampSettings::default()
                        .with_atlas_row(0, ODETTE_SHADOW_RAMP_ROWS)
                        .with_genshin_source_scale(ODETTE_SCALE)
                        .with_genshin_material_id_mask(material_id_mask)
                        .with_bands(ODETTE_SHADOW_RAMP_COLUMNS, 0.0)
                        .with_transform(ODETTE_RAMP_LIGHT_SCALE, 0.0)
                        .with_genshin_body(
                            0.55,
                            1.0,
                            matches!(surface, OdetteSurface::Hair | OdetteSurface::Dress),
                        )
                        .with_genshin_global_shadow(ODETTE_BODY_GLOBAL_SHADOW_COLOR)
                        .with_genshin_exposure(1.0)
                        .with_authored_emission_gain(authored_emission_gain),
                )
                .with_ilm_texture(ilm.clone(), image)
                .with_ilm_settings(IlmSettings::default().with_layout(IlmLayout::GenshinCharacter))
                .with_genshin_metallic_textures(
                    (npr.metal_map.clone(), metal_map),
                    (npr.specular_ramp.clone(), specular_ramp),
                )
                .with_genshin_metallic_settings(
                    GenshinMetallicSettings::default().with_enabled(true),
                )
                .with_genshin_packed_normal_texture(packed_normal, packed_normal_image)
                .with_cel_lighting_settings(lighting)
        }
        OdetteSurface::Face => {
            let face = images.get(&npr.face_sdf).expect("face SDF ready");
            let face_shadow = images.get(&npr.face_shadow).expect("face shadow ready");
            material
                .extension
                .with_ramp_texture(npr.neutral_ramp.clone())
                .with_ramp_settings(
                    ToonRampSettings::default()
                        .with_bands(1, 0.0)
                        .with_genshin_global_shadow(ODETTE_FACE_GLOBAL_SHADOW_COLOR)
                        .with_genshin_exposure(1.0),
                )
                .with_ilm_settings(IlmSettings::default().with_layout(IlmLayout::GenshinCharacter))
                .with_face_sdf_textures(FaceSdfTexturePair::new(
                    (npr.face_sdf.clone(), face),
                    (npr.face_sdf.clone(), face),
                ))
                .with_face_shadow_texture(npr.face_shadow.clone(), face_shadow)
                .with_face_sdf_settings(
                    FaceSdfSettings::default()
                        .with_softness(ODETTE_FACE_SHADOW_SOFTNESS)
                        .with_shadow_tint(ODETTE_FACE_DAY_SHADOW_TINT)
                        .with_external_shadow_influence(0.0)
                        .with_right_uv_mirrored(true)
                        .with_genshin_direction(0.55),
                )
                .with_cel_lighting_settings(
                    CelLightingSettings::default()
                        .with_readability_floor(ODETTE_FACE_READABILITY_FLOOR),
                )
        }
        OdetteSurface::Eye => material
            .extension
            .with_ramp_texture(npr.neutral_ramp.clone())
            .with_ramp_settings(
                ToonRampSettings::default()
                    .with_bands(1, 0.0)
                    .with_transform(0.0, 1.0),
            )
            .with_eye_emission_settings(
                EyeEmissionSettings::default()
                    .with_eye_floor(0.82, 0.08)
                    .with_eye_class(1.0, 1.0)
                    .with_eye_enabled(true),
            ),
        OdetteSurface::Brow => {
            let face02 = images.get(&npr.face02_shadow).expect("brow lightmap ready");
            material
                .extension
                .with_ramp_texture(npr.neutral_ramp.clone())
                .with_ramp_settings(
                    ToonRampSettings::default()
                        .with_bands(1, 0.0)
                        .with_genshin_body(0.55, 1.0, true)
                        .with_genshin_global_shadow(ODETTE_FACE_GLOBAL_SHADOW_COLOR),
                )
                .with_ilm_texture(npr.face02_shadow.clone(), face02)
                .with_ilm_settings(IlmSettings::default().with_layout(IlmLayout::GenshinCharacter))
                .with_cel_lighting_settings(
                    CelLightingSettings::default()
                        .with_readability_floor(ODETTE_FACE_READABILITY_FLOOR),
                )
        }
        OdetteSurface::Neutral => material
            .extension
            .with_ramp_texture(npr.neutral_ramp.clone())
            .with_ramp_settings(ToonRampSettings::default().with_bands(1, 0.0))
            .with_cel_lighting_settings(
                CelLightingSettings::default().with_readability_floor(ODETTE_READABILITY_FLOOR),
            ),
    }
    .with_backface_base_color_uv_b(true)
    .with_control_only_vertex_color(true);
    // The source fabrics branch on Blender's Backfacing socket: render both sides.
    material.extension.outline_cull_mode = OutlineCullMode::None;
    material
}

fn odette_outline_material(
    outlines: &mut Assets<OutlineMaterial>,
    surface: OdetteSurface,
    npr: &NprAssets,
) -> Option<bevy::prelude::Handle<OutlineMaterial>> {
    fn flat(
        outlines: &mut Assets<OutlineMaterial>,
        color: LinearRgba,
    ) -> bevy::prelude::Handle<OutlineMaterial> {
        outlines.add(OutlineMaterial {
            settings: OutlineSettings::default()
                .with_color(color)
                .with_width(ODETTE_OUTLINE_WIDTH),
            ..Default::default()
        })
    }
    fn source(
        outlines: &mut Assets<OutlineMaterial>,
        colors: [LinearRgba; 5],
        enabled_mask: u32,
        lightmap_texture: Option<bevy::prelude::Handle<Image>>,
        force_color_1: bool,
    ) -> bevy::prelude::Handle<OutlineMaterial> {
        let has_lightmap = lightmap_texture.is_some();
        outlines.add(OutlineMaterial {
            settings: OutlineSettings::default().with_genshin_source(
                colors,
                enabled_mask,
                ODETTE_SOURCE_OUTLINE_LOCAL_WIDTH,
                force_color_1,
                has_lightmap,
            ),
            lightmap_texture,
        })
    }
    match surface {
        OdetteSurface::Hair => Some(source(
            outlines,
            ODETTE_HAIR_OUTLINE_COLORS,
            0b1110,
            Some(npr.hair_ilm.clone()),
            false,
        )),
        OdetteSurface::Body => Some(source(
            outlines,
            ODETTE_BODY_OUTLINE_COLORS,
            0b1111,
            Some(npr.body_ilm.clone()),
            false,
        )),
        OdetteSurface::Dress => Some(source(
            outlines,
            ODETTE_DRESS_OUTLINE_COLORS,
            0b0110,
            Some(npr.hair_ilm.clone()),
            false,
        )),
        OdetteSurface::Body01 => Some(source(
            outlines,
            ODETTE_BODY01_OUTLINE_COLORS,
            0b1110,
            Some(npr.body01_ilm.clone()),
            false,
        )),
        OdetteSurface::Dress01 => Some(source(
            outlines,
            ODETTE_DRESS01_OUTLINE_COLORS,
            0b1110,
            Some(npr.body01_ilm.clone()),
            false,
        )),
        OdetteSurface::Face => Some(source(outlines, ODETTE_FACE_OUTLINE_COLORS, 0, None, true)),
        // Brow, pupil, and EyeStar have no outline geometry in the source.
        OdetteSurface::Brow | OdetteSurface::Eye => None,
        OdetteSurface::Neutral => Some(flat(outlines, ODETTE_NEUTRAL_OUTLINE)),
    }
}

/// Applies the Tenacity Showcase lighting frame (Genshin Default palette,
/// strict reference, Day direction-to-light, no physical light influence).
fn apply_genshin_frame(material: &mut CharacterMaterial) {
    if material.extension.ilm_settings().layout() != IlmLayout::GenshinCharacter {
        return;
    }
    let palette = GenshinLightingPalette::default().with_strict_reference(true);
    let light_source = GenshinLightSource::explicit(DAY_DIRECTION_TO_LIGHT, [1.0; 3], 1.0);
    let ramp = material
        .extension
        .ramp_settings()
        .with_genshin_physical_light_influence(0.0)
        .with_atlas_blend(10, 0.0);
    material.extension = material
        .extension
        .clone()
        .with_ramp_settings(ramp)
        .with_genshin_lighting_palette(palette)
        .with_genshin_light_source(light_source);
}

fn adopt_odette_scene(
    mut commands: Commands,
    mut state: ResMut<LabState>,
    config: Res<CaseConfig>,
    npr: Option<Res<NprAssets>>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    standard: Res<Assets<StandardMaterial>>,
    mut characters: ResMut<Assets<CharacterMaterial>>,
    mut outlines: ResMut<Assets<OutlineMaterial>>,
    mesh_entities: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&GltfMaterialName>,
        Option<&Name>,
    )>,
    named: Query<(Entity, &Name)>,
) {
    if state.adopted {
        return;
    }
    let Some(npr) = npr else { return };
    if mesh_entities.is_empty() || !sidecars_ready(&asset_server, &images, &npr) {
        return;
    }

    let head = named
        .iter()
        .find(|(_, name)| name.as_str() == ODETTE_FACE_BONE)
        .map(|(entity, _)| entity)
        .expect("Odette GLB must expose its DEF-spine.006 head bone");
    let face_basis =
        FaceBasis::try_new(Vec3::Z, Vec3::NEG_X).expect("authored face basis must stay valid");

    for (entity, mesh_material, material_name, name) in &mesh_entities {
        let surface = odette_surface(material_name);
        let source = standard
            .get(&mesh_material.0)
            .expect("spawned scene materials are resident")
            .clone();
        let mut material = odette_material(source, surface, &npr, &images);
        apply_genshin_frame(&mut material);
        let handle = characters.add(material);
        let (handle, face_instance) = if surface == OdetteSurface::Face {
            let instance = FaceSdfMaterialInstance::try_new(
                FaceSdfRig::new(head, face_basis),
                &handle,
                &mut characters,
            )
            .expect("fresh face material must remain resident");
            (instance.material().clone(), Some(instance))
        } else {
            (handle, None)
        };
        let mut entity_commands = commands.entity(entity);
        entity_commands
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
        if let Some(instance) = face_instance {
            entity_commands.insert(instance);
        }
        if config.case.outlines
            && odette_outline_allowed(name, true)
            && let Some(outline) = odette_outline_material(&mut outlines, surface, &npr)
        {
            entity_commands.insert(CharacterOutline::new(outline));
        }
    }
    state.adopted = true;
}

/// Computes the union of mesh world bounds for deterministic framing.
fn scene_world_bounds(app: &mut App) -> Option<(Vec3, f32)> {
    let mut query = app
        .world_mut()
        .query::<(&Aabb, &GlobalTransform, &Visibility)>();
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut any = false;
    for (aabb, transform, visibility) in query.iter(app.world()) {
        if visibility == Visibility::Hidden {
            continue;
        }
        let center = aabb.center;
        let half = aabb.half_extents;
        for corner in 0..8 {
            let offset = bevy::math::Vec3A::new(
                if corner & 1 == 0 { -half.x } else { half.x },
                if corner & 2 == 0 { -half.y } else { half.y },
                if corner & 4 == 0 { -half.z } else { half.z },
            );
            let world = transform.transform_point(Vec3::from(center + offset));
            min = min.min(world);
            max = max.max(world);
            any = true;
        }
    }
    any.then(|| {
        let center = (min + max) * 0.5;
        let radius = (max - min).length() * 0.5;
        (center, radius)
    })
}

fn place_camera(app: &mut App, case: LabCase) {
    let (center, radius) = scene_world_bounds(app).expect("adopted scene has bounds");
    let (eye, look_at) = match case.framing {
        Framing::FullBody => {
            // The yaw-PI model basis puts Odette's authored front on -Z.
            let direction = Vec3::new(0.25, 0.12, -1.0).normalize();
            let distance = radius * 1.05 / (std::f32::consts::FRAC_PI_4 * 0.5).tan();
            (center + direction * distance, center)
        }
        Framing::FaceCloseUp => {
            let head = {
                let mut query = app
                    .world_mut()
                    .query_filtered::<(&Name, &GlobalTransform), Without<Camera3d>>();
                query
                    .iter(app.world())
                    .find(|(name, _)| name.as_str() == ODETTE_FACE_BONE)
                    .map(|(_, transform)| transform.translation())
                    .expect("head bone transform must exist")
            };
            let direction = Vec3::new(0.3, 0.1, -1.0).normalize();
            let height = radius * 2.0;
            (head + direction * height * 0.13, head)
        }
    };
    let mut cameras = app
        .world_mut()
        .query_filtered::<&mut Transform, With<Camera3d>>();
    for mut transform in cameras.iter_mut(app.world_mut()) {
        *transform = Transform::from_translation(eye).looking_at(look_at, Vec3::Y);
    }
}

fn spawn_camera(mut commands: Commands, config: Res<CaseConfig>) {
    commands.spawn((
        Camera3d::default(),
        capture::capture_target_bundle(&config.target),
        Tonemapping::None,
        bevy::prelude::AmbientLight {
            color: Color::WHITE,
            brightness: GACHA_REFERENCE_AMBIENT,
            ..Default::default()
        },
    ));
}

fn headless_app(resources: RenderResources, case: LabCase) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: tenacity_asset_root().to_string_lossy().into_owned(),
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
    app.add_plugins(CharacterMaterialPlugin);
    capture::install(&mut app);
    let target = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        capture::create_capture_target(&mut images, CAPTURE_WIDTH, CAPTURE_HEIGHT)
    };
    app.insert_resource(CaseConfig { case, target })
        .insert_resource(ClearColor(SHOWCASE_CLEAR_COLOR))
        .insert_resource(LabState::default())
        .add_systems(bevy::prelude::Startup, (spawn_scene, spawn_camera).chain());
    app.add_systems(Update, adopt_odette_scene);
    app.finish();
    app.cleanup();
    app
}

fn gpu_info(app: &App) -> (String, String) {
    app.world()
        .get_resource::<RenderAdapterInfo>()
        .map(|info| {
            (
                format!("{} ({:?})", info.name, info.backend),
                format!("{} {}", info.driver, info.driver_info),
            )
        })
        .unwrap_or_else(|| ("unknown".to_owned(), "unknown".to_owned()))
}

/// Builds the scene, settles it, and captures one frame for `case`.
///
/// Returns `Ok(None)` when no GPU adapter is available (graceful skip).
///
/// # Errors
///
/// Returns [`LabError`] when the scene never becomes ready or IO fails.
pub fn capture_case(case: LabCase) -> Result<Option<(CapturedImage, String, String)>, LabError> {
    let Some(resources) = capture::probe_render_resources() else {
        return Ok(None);
    };
    let mut app = headless_app(resources, case);

    // Wait for the GLB scene and NPR sidecars, then let adoption run.
    let mut waited = 0;
    loop {
        app.update();
        waited += 1;
        if app.world().resource::<LabState>().adopted {
            break;
        }
        if waited > MAX_WAIT_FRAMES {
            return Err(LabError::SceneNotReady("adoption timed out"));
        }
    }
    place_camera(&mut app, case);
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
    let target = app.world().resource::<CaseConfig>().target.clone();
    let request = capture::request_capture(app.world_mut(), target);
    let mut captured = None;
    for _ in 0..MAX_WAIT_FRAMES {
        app.update();
        if let Some(frame) = capture::try_take(app.world_mut(), request.clone()) {
            captured = Some(frame);
            break;
        }
    }
    let captured = captured.ok_or(LabError::SceneNotReady("readback timed out"))?;
    let (gpu, driver) = gpu_info(&app);
    Ok(Some((captured, gpu, driver)))
}

/// Runs one case against the stored baseline (or rebaselines).
///
/// Returns `Ok(None)` when assets or the GPU are missing (graceful skip).
///
/// # Errors
///
/// Returns [`LabError`] when capture or baseline IO fails.
pub fn run_case(
    case: LabCase,
    output_dir: &Path,
    rebaseline: bool,
    tolerance: Tolerance,
) -> Result<Option<CaseOutcome>, LabError> {
    let Some((captured, gpu, driver)) = capture_case(case)? else {
        return Ok(None);
    };
    let paths = GoldenPaths::new(output_dir, case.name);
    let mut provenance = GoldenProvenance::for_capture(
        "engine-golden-v1",
        None,
        "tenacity-odette-sidecars",
        None,
        "blender-gltf-npr-v1",
        "gacha/v1",
        &format!("odette/{}", case.name),
        "bevy_cel-0.1.0-legacy-oracle",
        &gpu,
        &driver,
        &captured,
    );
    if rebaseline || !paths.baseline_exists() {
        golden_harness::write_baseline(&paths, &captured, &mut provenance)?;
        return Ok(Some(CaseOutcome::Baselined));
    }
    let (baseline, _stored_provenance) = golden_harness::read_baseline(&paths)?;
    let stats = diff_rgba8(&baseline.data, &captured.data).map_err(|error| {
        LabError::SceneNotReady(match error {
            golden_harness::diff::DiffError::SizeMismatch { .. } => "baseline size mismatch",
        })
    })?;
    let report = DiffReport::evaluate(
        case.name.to_owned(),
        paths.png.to_string_lossy().into_owned(),
        stats,
        tolerance,
    );
    std::fs::create_dir_all(output_dir)?;
    std::fs::write(
        output_dir.join(format!("{}.diff.json", case.name)),
        serde_json::to_string_pretty(&report)?,
    )?;
    if report.passed {
        Ok(Some(CaseOutcome::Matched(report)))
    } else {
        let candidate = GoldenPaths::new(output_dir, &format!("{}.candidate", case.name));
        let mut candidate_provenance = provenance;
        golden_harness::write_baseline(&candidate, &captured, &mut candidate_provenance)?;
        Ok(Some(CaseOutcome::Exceeded(report)))
    }
}

/// Runs the full matrix. Returns the per-case outcomes; `None` when skipped.
///
/// # Errors
///
/// Returns [`LabError`] when any case fails to capture or write.
pub fn run_matrix() -> Result<Option<Vec<(LabCase, CaseOutcome)>>, LabError> {
    if !assets_available() {
        eprintln!(
            "odette-reference-lab: Odette assets not found under {} (set {TENACITY_ROOT_ENV}); skipping",
            tenacity_asset_root().display()
        );
        return Ok(None);
    }
    if capture::probe_render_resources().is_none() {
        eprintln!("odette-reference-lab: no GPU adapter available; skipping");
        return Ok(None);
    }
    let rebaseline = std::env::var(GOLDEN_REBASELINE_ENV).as_deref() == Ok("1");
    let output_dir = golden_local_dir();
    let mut outcomes = Vec::new();
    for case in CASES {
        let outcome = run_case(*case, &output_dir, rebaseline, Tolerance::default())?;
        outcomes.push((*case, outcome.expect("probed GPU remains available")));
    }
    Ok(Some(outcomes))
}

/// CLI entry shared by the binary and the integration test. Returns the
/// process exit code: 0 when every case matched (or was skipped/baselined).
#[must_use]
pub fn run_cli() -> ExitCode {
    match run_matrix() {
        Ok(None) => ExitCode::SUCCESS,
        Ok(Some(outcomes)) => {
            let mut failed = false;
            for (case, outcome) in outcomes {
                match outcome {
                    CaseOutcome::Baselined => {
                        println!("[baselined] {}", case.name);
                    }
                    CaseOutcome::Matched(report) => {
                        println!(
                            "[matched] {} max_delta={} mean_delta={:.4} psnr={:.2}dB differing={}/{}",
                            case.name,
                            report.stats.max_channel_delta,
                            report.stats.mean_absolute_delta,
                            report.stats.psnr_db,
                            report.stats.differing_pixels,
                            report.stats.total_pixels
                        );
                    }
                    CaseOutcome::Exceeded(report) => {
                        failed = true;
                        eprintln!(
                            "[exceeded] {} max_delta={} mean_delta={:.4} psnr={:.2}dB violations={:?}",
                            case.name,
                            report.stats.max_channel_delta,
                            report.stats.mean_absolute_delta,
                            report.stats.psnr_db,
                            report.violations
                        );
                    }
                }
            }
            if failed {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("odette-reference-lab failed: {error}");
            ExitCode::FAILURE
        }
    }
}
