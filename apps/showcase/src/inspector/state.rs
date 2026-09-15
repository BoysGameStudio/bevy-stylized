use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugMode {
    Beauty,
    Lighting,
    BaseColor,
    PrePalette,
    HalfDot,
}
impl DebugMode {
    pub const ALL: [Self; 5] = [
        Self::Beauty,
        Self::Lighting,
        Self::BaseColor,
        Self::PrePalette,
        Self::HalfDot,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Beauty => "Beauty",
            Self::Lighting => "Lighting factors",
            Self::BaseColor => "Base color",
            Self::PrePalette => "Before palette",
            Self::HalfDot => "Normal / half vector",
        }
    }
    pub fn hero(self) -> bevy_stylized::prelude::StylizedHeroDebugView {
        use bevy_stylized::prelude::StylizedHeroDebugView as V;
        match self {
            Self::Beauty => V::Beauty,
            Self::Lighting => V::LightingFactors,
            Self::BaseColor => V::BaseColor,
            Self::PrePalette => V::PrePalette,
            Self::HalfDot => V::NormalHalfDot,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrbitCamera {
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}
impl OrbitCamera {
    pub fn transform(&self) -> Transform {
        let target = Vec3::from_array(self.target);
        let offset = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        ) * self.distance;
        Transform::from_translation(target + offset).looking_at(target, Vec3::Y)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self
            .target
            .iter()
            .chain([self.yaw, self.pitch, self.distance].iter())
            .all(|v| v.is_finite())
            || self.distance <= 0.001
            || self.pitch.abs() > 1.5
        {
            return Err("invalid camera: finite coordinates, positive distance and pitch within ±1.5 radians required".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub schema: String,
    pub files: BTreeMap<String, String>,
    pub cameras: [OrbitCamera; 2],
    pub linked: bool,
    pub debug: [DebugMode; 2],
    pub light: [f32; 3],
    pub exposure: f32,
    pub resolution: u32,
}
impl Recipe {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "bevy_stylized.inspector/v1" {
            return Err("unsupported inspector recipe".into());
        }
        for camera in &self.cameras {
            camera.validate()?;
        }
        if self.linked && self.cameras[0] != self.cameras[1] {
            return Err("linked cameras must have equal saved poses".into());
        }
        let light = Vec3::from_array(self.light);
        if !light.is_finite()
            || (light.length() - 1.0).abs() > 0.001
            || !self.exposure.is_finite()
            || !(0.01..=16.).contains(&self.exposure)
            || !(128..=2048).contains(&self.resolution)
        {
            return Err("invalid lighting, exposure or resolution".into());
        }
        Ok(())
    }
}
#[derive(Resource)]
pub struct State {
    pub recipe: Recipe,
    pub reset: Recipe,
    pub revision: u64,
    pub stable: u32,
    pub status: String,
    pub error: Option<String>,
    pub ready: bool,
    pub capture: bool,
    pub reload: bool,
    pub load_config: bool,
    pub captures: u32,
    pub retired: u32,
}
impl State {
    pub fn changed(&mut self) {
        self.revision += 1;
        self.stable = 0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn orbit_uses_metre_scale_and_stays_on_target() {
        let c = OrbitCamera {
            target: [0., 0.7, 0.],
            yaw: 0.,
            pitch: 0.,
            distance: 2.,
        };
        let t = c.transform();
        assert_eq!(t.translation, Vec3::new(0., 0.7, 2.));
        assert!((t.forward().as_vec3() - Vec3::NEG_Z).length() < 1e-6);
    }
    #[test]
    fn camera_rejects_degenerate_and_nonfinite_poses() {
        for distance in [0., -1., f32::NAN] {
            assert!(
                OrbitCamera {
                    target: [0.; 3],
                    yaw: 0.,
                    pitch: 0.,
                    distance
                }
                .validate()
                .is_err()
            );
        }
    }
}
