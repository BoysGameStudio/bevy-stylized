use bevy::prelude::{Component, Reflect};

/// Ordered stages in the reusable 3D camera stack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostProcessStage {
    /// Opaque and transparent scene rendering.
    MainPass,
    /// Built-in linear HDR bloom.
    Bloom,
    /// Distortion, chromatic separation, and vignette.
    StylizedFullscreen,
    /// Final display transform.
    Tonemapping,
}

/// Rendering order enforced by the stack's Bevy render systems.
pub const STYLIZED_POST_PROCESS_ORDER: [PostProcessStage; 4] = [
    PostProcessStage::MainPass,
    PostProcessStage::Bloom,
    PostProcessStage::StylizedFullscreen,
    PostProcessStage::Tonemapping,
];

/// Finite, bounded parameters shared by stylized camera profiles.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct StylizedPostProcessSettings {
    bloom_intensity: f32,
    vignette_strength: f32,
    vignette_radius: f32,
    vignette_softness: f32,
    lens_distortion: f32,
    chromatic_aberration: f32,
}

impl Default for StylizedPostProcessSettings {
    fn default() -> Self {
        Self {
            bloom_intensity: 0.18,
            vignette_strength: 0.38,
            vignette_radius: 0.86,
            vignette_softness: 0.3,
            lens_distortion: 0.035,
            chromatic_aberration: 0.004,
        }
    }
}

impl StylizedPostProcessSettings {
    /// Sets built-in bloom scattering intensity.
    #[must_use]
    pub const fn with_bloom_intensity(mut self, value: f32) -> Self {
        self.bloom_intensity = finite_clamped(value, 0.18, 0.0, 1.0);
        self
    }

    /// Sets vignette strength, normalized radius, and falloff softness.
    #[must_use]
    pub const fn with_vignette(mut self, strength: f32, radius: f32, softness: f32) -> Self {
        self.vignette_strength = finite_clamped(strength, 0.38, 0.0, 1.0);
        self.vignette_radius = finite_clamped(radius, 0.86, 0.1, 1.5);
        self.vignette_softness = finite_clamped(softness, 0.3, 0.001, 1.0);
        self
    }

    /// Sets radial barrel-distortion strength.
    #[must_use]
    pub const fn with_lens_distortion(mut self, value: f32) -> Self {
        self.lens_distortion = finite_clamped(value, 0.035, 0.0, 0.25);
        self
    }

    /// Sets radial RGB sample separation in normalized UV units.
    #[must_use]
    pub const fn with_chromatic_aberration(mut self, value: f32) -> Self {
        self.chromatic_aberration = finite_clamped(value, 0.004, 0.0, 0.05);
        self
    }

    /// Returns built-in bloom scattering intensity.
    #[must_use]
    pub const fn bloom_intensity(self) -> f32 {
        self.bloom_intensity
    }

    /// Returns vignette strength, radius, and softness.
    #[must_use]
    pub const fn vignette(self) -> [f32; 3] {
        [
            self.vignette_strength,
            self.vignette_radius,
            self.vignette_softness,
        ]
    }

    /// Returns radial barrel-distortion strength.
    #[must_use]
    pub const fn lens_distortion(self) -> f32 {
        self.lens_distortion
    }

    /// Returns radial RGB sample separation.
    #[must_use]
    pub const fn chromatic_aberration(self) -> f32 {
        self.chromatic_aberration
    }

    pub(crate) const fn fullscreen_values(self) -> [f32; 5] {
        [
            self.vignette_strength,
            self.vignette_radius,
            self.vignette_softness,
            self.lens_distortion,
            self.chromatic_aberration,
        ]
    }

    /// Returns whether every stored scalar is finite.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.fullscreen_values().into_iter().all(f32::is_finite) && self.bloom_intensity.is_finite()
    }

    /// Returns whether every stored scalar is inside its public bound.
    #[must_use]
    pub fn is_bounded(self) -> bool {
        (0.0..=1.0).contains(&self.bloom_intensity)
            && (0.0..=1.0).contains(&self.vignette_strength)
            && (0.1..=1.5).contains(&self.vignette_radius)
            && (0.001..=1.0).contains(&self.vignette_softness)
            && (0.0..=0.25).contains(&self.lens_distortion)
            && (0.0..=0.05).contains(&self.chromatic_aberration)
    }
}

/// Independent feature switches and parameters for one 3D camera.
///
/// While this component is present, the camera's [`Hdr`](bevy::camera::Hdr),
/// [`Bloom`](bevy::post_process::bloom::Bloom),
/// [`Tonemapping`](bevy::core_pipeline::tonemapping::Tonemapping), and prepass
/// components are owned by the plugin's `apply_profiles` system: it writes the
/// union of this profile, an active [`GenshinDisplayTransform`](crate::GenshinDisplayTransform), and enabled
/// internal-edge materials every frame. Manual insertion or removal of those
/// components on the camera is overridden.
#[derive(Clone, Copy, Component, Debug, PartialEq, Reflect)]
pub struct StylizedPostProcessProfile {
    settings: StylizedPostProcessSettings,
    features: u32,
}

const HDR: u32 = 1 << 0;
const BLOOM: u32 = 1 << 1;
const TONEMAPPING: u32 = 1 << 2;
const VIGNETTE: u32 = 1 << 3;
const LENS_DISTORTION: u32 = 1 << 4;
const CHROMATIC_ABERRATION: u32 = 1 << 5;
const PREPASS_EDGES: u32 = 1 << 6;

impl Default for StylizedPostProcessProfile {
    fn default() -> Self {
        Self {
            settings: StylizedPostProcessSettings::default(),
            features: HDR | BLOOM | TONEMAPPING | VIGNETTE | LENS_DISTORTION | CHROMATIC_ABERRATION,
        }
    }
}

macro_rules! profile_toggle {
    ($builder:ident, $getter:ident, $flag:ident, $builder_doc:literal, $getter_doc:literal) => {
        #[doc = $builder_doc]
        #[must_use]
        pub const fn $builder(mut self, enabled: bool) -> Self {
            if enabled {
                self.features |= $flag;
            } else {
                self.features &= !$flag;
            }
            self
        }

        #[doc = $getter_doc]
        #[must_use]
        pub const fn $getter(self) -> bool {
            self.features & $flag != 0
        }
    };
}

impl StylizedPostProcessProfile {
    /// Replaces all bounded effect parameters.
    #[must_use]
    pub const fn with_settings(mut self, settings: StylizedPostProcessSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Returns all bounded effect parameters.
    #[must_use]
    pub const fn settings(self) -> StylizedPostProcessSettings {
        self.settings
    }

    profile_toggle!(
        with_hdr,
        hdr_enabled,
        HDR,
        "Toggles the HDR camera target.",
        "Returns whether HDR is enabled."
    );
    profile_toggle!(
        with_bloom,
        bloom_enabled,
        BLOOM,
        "Toggles built-in linear bloom.",
        "Returns whether bloom is enabled."
    );
    profile_toggle!(
        with_tonemapping,
        tonemapping_enabled,
        TONEMAPPING,
        "Toggles `TonyMcMapface` tonemapping.",
        "Returns whether tonemapping is enabled."
    );
    profile_toggle!(
        with_vignette,
        vignette_enabled,
        VIGNETTE,
        "Toggles vignette falloff.",
        "Returns whether vignette is enabled."
    );
    profile_toggle!(
        with_lens_distortion,
        lens_distortion_enabled,
        LENS_DISTORTION,
        "Toggles radial lens distortion.",
        "Returns whether lens distortion is enabled."
    );
    profile_toggle!(
        with_chromatic_aberration,
        chromatic_aberration_enabled,
        CHROMATIC_ABERRATION,
        "Toggles radial chromatic aberration.",
        "Returns whether chromatic aberration is enabled."
    );
    profile_toggle!(
        with_prepass_edges,
        prepass_edges_enabled,
        PREPASS_EDGES,
        "Toggles depth/normal textures for optional edge effects.",
        "Returns whether optional edge prepasses are enabled."
    );
}

const fn finite_clamped(value: f32, fallback: f32, minimum: f32, maximum: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        fallback
    }
}
