//! Deterministic task-13 directional face-SDF render fixture.

use std::{error::Error, fmt, path::PathBuf, time::Duration};

use bevy::{
    camera::RenderTarget,
    ecs::system::SystemParam,
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::PollType,
        renderer::RenderDevice,
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    window::{ExitCondition, WindowPlugin},
};
use bevy_cel::{
    CharacterExtension, CharacterMaterial, CharacterMaterialPlugin, FaceBasis, FaceSdfDebugMode,
    FaceSdfMaterialInstance, FaceSdfRig, FaceSdfSettings, FaceSdfTexturePair, ToonRampSettings,
};

include!("face_sdf_fixture/assets.rs");

const SIZE: u32 = 640;
const MAX_FRAMES: u32 = 300;

#[derive(Clone, Copy)]
enum FixtureMode {
    Light0,
    Light90,
    Light180,
    Vertical,
    Fallback,
    DebugSelected,
    DebugThreshold,
    DebugLight,
}

impl FixtureMode {
    fn parse(value: &str) -> Result<Self, FixtureError> {
        match value {
            "light-0" => Ok(Self::Light0),
            "light-90" => Ok(Self::Light90),
            "light-180" => Ok(Self::Light180),
            "vertical" => Ok(Self::Vertical),
            "fallback" => Ok(Self::Fallback),
            "debug-selected" => Ok(Self::DebugSelected),
            "debug-threshold" => Ok(Self::DebugThreshold),
            "debug-light" => Ok(Self::DebugLight),
            _ => Err(FixtureError::InvalidMode(value.to_owned())),
        }
    }

    const fn light_position(self) -> Vec3 {
        match self {
            Self::Light0 | Self::DebugSelected | Self::DebugThreshold | Self::DebugLight => {
                Vec3::new(4.0, 0.0, 0.0)
            }
            Self::Light90 | Self::Fallback => Vec3::new(0.0, 0.0, 4.0),
            Self::Light180 => Vec3::new(-4.0, 0.0, 0.0),
            Self::Vertical => Vec3::new(0.0, 4.0, 0.0),
        }
    }

    const fn light_up(self) -> Vec3 {
        match self {
            Self::Vertical => Vec3::Z,
            Self::Light0
            | Self::Light90
            | Self::Light180
            | Self::Fallback
            | Self::DebugSelected
            | Self::DebugThreshold
            | Self::DebugLight => Vec3::Y,
        }
    }

    const fn debug(self) -> FaceSdfDebugMode {
        match self {
            Self::DebugSelected => FaceSdfDebugMode::SelectedMap,
            Self::DebugThreshold => FaceSdfDebugMode::Threshold,
            Self::DebugLight => FaceSdfDebugMode::LightCoordinate,
            Self::Light0 | Self::Light90 | Self::Light180 | Self::Vertical | Self::Fallback => {
                FaceSdfDebugMode::Disabled
            }
        }
    }
}

#[derive(Debug)]
enum FixtureError {
    MissingOutput,
    InvalidMode(String),
    InvalidBasis,
    MissingMaterial,
    GpuPoll,
    CaptureTimeout,
    PixelValidation,
}

impl fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOutput => formatter.write_str(
                "usage: face_sdf_fixture <output.png> <light-0|light-90|light-180|vertical|fallback|debug-selected|debug-threshold|debug-light>",
            ),
            Self::InvalidMode(mode) => write!(formatter, "invalid fixture mode: {mode}"),
            Self::InvalidBasis => formatter.write_str("fixture face basis is invalid"),
            Self::MissingMaterial => formatter.write_str("fixture source material is unavailable"),
            Self::GpuPoll => formatter.write_str("GPU poll failed"),
            Self::CaptureTimeout => formatter.write_str("fixture capture timed out"),
            Self::PixelValidation => formatter.write_str("fixture cheek pixel validation failed"),
        }
    }
}

impl Error for FixtureError {}

#[derive(Resource)]
struct CaptureConfig {
    output: PathBuf,
    mode: FixtureMode,
}

#[derive(Resource)]
struct CaptureState {
    target: Handle<Image>,
    frames: u32,
    requested: bool,
    complete: bool,
    valid: bool,
}

#[derive(SystemParam)]
struct SetupParams<'w> {
    images: ResMut<'w, Assets<Image>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<CharacterMaterial>>,
}

fn main() -> Result<(), FixtureError> {
    let mut arguments = std::env::args().skip(1);
    let output = arguments
        .next()
        .map(PathBuf::from)
        .ok_or(FixtureError::MissingOutput)?;
    let mode = arguments
        .next()
        .ok_or(FixtureError::MissingOutput)
        .and_then(|value| FixtureMode::parse(&value))?;
    let mut app = App::new();
    app.insert_resource(CaptureConfig { output, mode })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                }),
        )
        .add_plugins(CharacterMaterialPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, request_capture);
    app.finish();
    app.cleanup();

    for _ in 0..MAX_FRAMES {
        app.update();
        if app
            .world()
            .resource::<RenderDevice>()
            .wgpu_device()
            .poll(PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(5)),
            })
            .is_err()
        {
            return Err(FixtureError::GpuPoll);
        }
        if app.world().resource::<CaptureState>().complete {
            return if app.world().resource::<CaptureState>().valid {
                Ok(())
            } else {
                Err(FixtureError::PixelValidation)
            };
        }
    }
    Err(FixtureError::CaptureTimeout)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy system parameters are values"
)]
fn setup(
    mut commands: Commands,
    config: Res<CaptureConfig>,
    mut params: SetupParams,
) -> Result<(), BevyError> {
    let target = params.images.add(render_target(SIZE));
    commands.insert_resource(CaptureState {
        target: target.clone(),
        frames: 0,
        requested: false,
        complete: false,
        valid: false,
    });

    let ramp = params.images.add(ramp_atlas());
    let left_image = face_threshold_map(false);
    let right_image = face_threshold_map(true);
    let left = params.images.add(left_image.clone());
    let right = params.images.add(right_image.clone());
    let settings = FaceSdfSettings::default()
        .with_softness(0.02)
        .with_shadow_tint([0.58, 0.36, 0.34])
        .with_external_shadow_influence(0.0)
        .with_debug(config.mode.debug());
    let extension = CharacterExtension::default()
        .with_ramp_texture(ramp)
        .with_ramp_settings(ToonRampSettings::default().with_bands(2, 0.0));
    let extension = match config.mode {
        FixtureMode::Fallback => extension.with_face_sdf_settings(settings),
        FixtureMode::Light0
        | FixtureMode::Light90
        | FixtureMode::Light180
        | FixtureMode::Vertical
        | FixtureMode::DebugSelected
        | FixtureMode::DebugThreshold
        | FixtureMode::DebugLight => extension
            .with_face_sdf_textures(FaceSdfTexturePair::new(
                (left, &left_image),
                (right, &right_image),
            ))
            .with_face_sdf_settings(settings),
    };
    let source = params.materials.add(CharacterMaterial {
        base: StandardMaterial {
            base_color: Color::srgb(0.96, 0.72, 0.62),
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        },
        extension,
    });
    let head = commands
        .spawn((Name::new("ProceduralHead"), Transform::default()))
        .id();
    let basis = FaceBasis::try_new(Vec3::Z, Vec3::X)
        .map_err(|_| BevyError::from(FixtureError::InvalidBasis))?;
    let instance = FaceSdfMaterialInstance::try_new(
        FaceSdfRig::new(head, basis),
        &source,
        &mut params.materials,
    )
    .map_err(|_| BevyError::from(FixtureError::MissingMaterial))?;
    let material = instance.material().clone();
    commands.spawn((
        Name::new("ProceduralFaceSdf"),
        Mesh3d(params.meshes.add(face_mesh())),
        MeshMaterial3d(material),
        instance,
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 14_000.0,
            ..default()
        },
        Transform::from_translation(config.mode.light_position())
            .looking_at(Vec3::ZERO, config.mode.light_up()),
    ));
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(target.into()),
        Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    Ok(())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy system parameters are values"
)]
fn request_capture(
    mut commands: Commands,
    config: Res<CaptureConfig>,
    mut state: ResMut<CaptureState>,
) {
    state.frames += 1;
    if state.frames < 8 || state.requested {
        return;
    }
    state.requested = true;
    commands
        .spawn(Screenshot::image(state.target.clone()))
        .observe(save_to_disk(config.output.clone()))
        .observe(mark_capture_complete);
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy observer parameters are values"
)]
fn mark_capture_complete(
    capture: On<ScreenshotCaptured>,
    config: Res<CaptureConfig>,
    mut state: ResMut<CaptureState>,
) {
    let left = capture.image.get_color_at(250, 320);
    let right = capture.image.get_color_at(390, 320);
    state.valid = match (left, right) {
        (Ok(left), Ok(right)) => validate_face_pixels(
            left.to_srgba().to_f32_array(),
            right.to_srgba().to_f32_array(),
            config.mode,
        ),
        (Err(_), _) | (_, Err(_)) => false,
    };
    state.complete = true;
}

#[cfg(test)]
mod tests {
    #[test]
    fn designated_cheek_pixels_swap_lit_shadow_order_when_light_rotates() {
        // Given
        let light_from_right = ([0.82, 0.65, 0.58, 1.0], [0.24, 0.12, 0.10, 1.0]);
        let light_from_left = ([0.23, 0.11, 0.09, 1.0], [0.81, 0.64, 0.57, 1.0]);

        // When
        let swaps = super::validate_cheek_swap(light_from_right, light_from_left);

        // Then
        assert!(swaps);
    }
}
