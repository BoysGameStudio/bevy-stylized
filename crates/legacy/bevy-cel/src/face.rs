use std::{error::Error, fmt};

use bevy::{math::Vec3, prelude::Reflect};

const BASIS_EPSILON: f32 = 1.0e-5;

/// Invalid world or local face basis supplied at an API boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Reflect)]
pub enum FaceBasisError {
    /// At least one component is NaN or infinite.
    NonFinite,
    /// XZ projection removes at least one direction.
    Degenerate,
    /// Forward and right cannot form an orthogonal pair.
    Parallel,
}

impl fmt::Display for FaceBasisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => formatter.write_str("face basis contains a non-finite value"),
            Self::Degenerate => formatter.write_str("face basis has no usable XZ direction"),
            Self::Parallel => formatter.write_str("face basis directions are parallel"),
        }
    }
}

impl Error for FaceBasisError {}

/// Validated orthonormal XZ basis used by directional face shading.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct FaceBasis {
    forward: Vec3,
    right: Vec3,
}

impl FaceBasis {
    /// Projects to XZ and builds an orthonormal forward/right pair.
    ///
    /// # Errors
    /// Returns a typed error for non-finite, degenerate, or parallel inputs.
    pub fn try_new(forward: Vec3, right: Vec3) -> Result<Self, FaceBasisError> {
        if !forward.is_finite() || !right.is_finite() {
            return Err(FaceBasisError::NonFinite);
        }
        let forward = Vec3::new(forward.x, 0.0, forward.z);
        let right = Vec3::new(right.x, 0.0, right.z);
        if forward.length_squared() <= BASIS_EPSILON || right.length_squared() <= BASIS_EPSILON {
            return Err(FaceBasisError::Degenerate);
        }
        let forward = forward.normalize();
        let orthogonal_right = right - forward * right.dot(forward);
        if orthogonal_right.length_squared() <= BASIS_EPSILON {
            return Err(FaceBasisError::Parallel);
        }
        Ok(Self {
            forward,
            right: orthogonal_right.normalize(),
        })
    }

    #[must_use]
    /// Returns normalized world or local face-forward.
    pub const fn forward(self) -> Vec3 {
        self.forward
    }

    #[must_use]
    /// Returns normalized world or local face-right.
    pub const fn right(self) -> Vec3 {
        self.right
    }
}

/// Shader-visible directional face diagnostic.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Reflect)]
pub enum FaceSdfDebugMode {
    /// Render normal face shading.
    #[default]
    Disabled,
    /// Render left as red and right as blue.
    SelectedMap,
    /// Render the normalized directional threshold.
    Threshold,
    /// Render right/forward light coordinates in red/green.
    LightCoordinate,
}

impl FaceSdfDebugMode {
    pub(crate) const fn shader_value(self) -> u32 {
        match self {
            Self::Disabled => 0,
            Self::SelectedMap => 1,
            Self::Threshold => 2,
            Self::LightCoordinate => 3,
        }
    }

    pub(crate) const fn from_shader_value(value: u32) -> Self {
        match value {
            1 => Self::SelectedMap,
            2 => Self::Threshold,
            3 => Self::LightCoordinate,
            0 | 4..=u32::MAX => Self::Disabled,
        }
    }
}

/// Directional threshold texture selected for one light direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceSdfMapSide {
    /// Left-facing threshold texture.
    Left,
    /// Right-facing threshold texture.
    Right,
}

/// CPU mirror of the shader's directional selection and threshold values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceSdfLightSample {
    selected_map: FaceSdfMapSide,
    threshold: f32,
    light_coordinate: [f32; 2],
}

impl FaceSdfLightSample {
    pub(crate) const fn new(
        selected_map: FaceSdfMapSide,
        threshold: f32,
        light_coordinate: [f32; 2],
    ) -> Self {
        Self {
            selected_map,
            threshold,
            light_coordinate,
        }
    }

    #[must_use]
    /// Returns the selected left/right map.
    pub const fn selected_map(self) -> FaceSdfMapSide {
        self.selected_map
    }

    #[must_use]
    /// Returns the normalized threshold after bias and rotation.
    pub const fn threshold(self) -> f32 {
        self.threshold
    }

    #[must_use]
    /// Returns right/forward dot values remapped to `[0, 1]`.
    pub const fn light_coordinate(self) -> [f32; 2] {
        self.light_coordinate
    }
}
