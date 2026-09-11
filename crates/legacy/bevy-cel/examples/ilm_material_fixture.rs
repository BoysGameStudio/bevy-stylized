//! Deterministic task-12 two-region ILM render fixture.

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
    CharacterExtension, CharacterMaterial, CharacterMaterialPlugin, IlmDebugMode, IlmSettings,
    ToonRampSettings,
};

include!("ilm_material_fixture/assets.rs");

const SIZE: u32 = 640;

#[derive(Clone, Copy)]
enum FixtureMode {
    Final,
    Specular,
    Ao,
    MaterialId,
    Emission,
    RampRow,
    Absent,
}

impl FixtureMode {
    fn parse(value: &str) -> Result<Self, FixtureError> {
        match value {
            "final" => Ok(Self::Final),
            "specular" => Ok(Self::Specular),
            "ao" => Ok(Self::Ao),
            "material-id" => Ok(Self::MaterialId),
            "emission" => Ok(Self::Emission),
            "ramp-row" => Ok(Self::RampRow),
            "absent" => Ok(Self::Absent),
            _ => Err(FixtureError::InvalidMode(value.to_owned())),
        }
    }

    const fn debug(self) -> IlmDebugMode {
        match self {
            Self::Final | Self::Absent => IlmDebugMode::Disabled,
            Self::Specular => IlmDebugMode::SpecularStrength,
            Self::Ao => IlmDebugMode::AoShadowInfluence,
            Self::MaterialId => IlmDebugMode::MaterialId,
            Self::Emission => IlmDebugMode::EmissionSpecialMask,
            Self::RampRow => IlmDebugMode::RampRow,
        }
    }
}

#[derive(Debug)]
enum FixtureError {
    MissingOutput,
    InvalidMode(String),
    GpuPoll,
    CaptureTimeout,
    PixelValidation,
}

impl fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOutput => formatter.write_str(
                "usage: ilm_material_fixture <output.png> <final|specular|ao|material-id|emission|ramp-row|absent>",
            ),
            Self::InvalidMode(mode) => write!(formatter, "invalid fixture mode: {mode}"),
            Self::GpuPoll => formatter.write_str("GPU poll failed"),
            Self::CaptureTimeout => formatter.write_str("fixture capture timed out"),
            Self::PixelValidation => formatter.write_str("fixture pixel validation failed"),
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

    for _ in 0..300 {
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
fn setup(mut commands: Commands, config: Res<CaptureConfig>, mut params: SetupParams) {
    let target = params.images.add(render_target(SIZE));
    commands.insert_resource(CaptureState {
        target: target.clone(),
        frames: 0,
        requested: false,
        complete: false,
        valid: false,
    });

    let ramp = params.images.add(ramp_atlas());
    let ilm = ilm_texture();
    let ilm_handle = params.images.add(ilm.clone());
    let settings = IlmSettings::default().with_debug(config.mode.debug());
    let extension = CharacterExtension::default()
        .with_ramp_texture(ramp)
        .with_ramp_settings(
            ToonRampSettings::default()
                .with_atlas_row(0, 2)
                .with_bands(1, 0.0),
        );
    let extension = match config.mode {
        FixtureMode::Absent => extension.with_ilm_settings(settings),
        FixtureMode::Final
        | FixtureMode::Specular
        | FixtureMode::Ao
        | FixtureMode::MaterialId
        | FixtureMode::Emission
        | FixtureMode::RampRow => extension
            .with_ilm_texture(ilm_handle, &ilm)
            .with_ilm_settings(settings),
    };
    let material = params.materials.add(CharacterMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        },
        extension,
    });
    commands.spawn((
        Mesh3d(params.meshes.add(two_region_mesh())),
        MeshMaterial3d(material),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(target.into()),
        Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
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
    let left = capture.image.get_color_at(220, 320);
    let right = capture.image.get_color_at(420, 320);
    state.valid = match (left, right) {
        (Ok(left), Ok(right)) => validate_regions(
            left.to_srgba().to_f32_array(),
            right.to_srgba().to_f32_array(),
            config.mode,
        ),
        (Err(_), _) | (_, Err(_)) => false,
    };
    state.complete = true;
}
