#![expect(
    missing_docs,
    reason = "serde field names are the versioned CEL wire contract"
)]

use serde::{Deserialize, Serialize};

use crate::{StylizedPostProcessProfile, StylizedPostProcessSettings};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FaceRigAuthoring {
    pub armature: Option<String>,
    pub head_bone: String,
    pub forward_bone: String,
    pub right_bone: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "schema-v1 stores seven independent runtime feature toggles"
)]
pub struct PostAuthoring {
    pub hdr: bool,
    pub bloom: bool,
    pub tonemapping: bool,
    pub vignette: bool,
    pub lens_distortion: bool,
    pub chromatic_aberration: bool,
    pub prepass_edges: bool,
    pub bloom_intensity: f32,
    pub vignette_strength: f32,
    pub vignette_radius: f32,
    pub vignette_softness: f32,
    pub lens_distortion_amount: f32,
    pub chromatic_aberration_amount: f32,
}

impl PostAuthoring {
    #[must_use]
    pub fn to_profile(&self) -> StylizedPostProcessProfile {
        let settings = StylizedPostProcessSettings::default()
            .with_bloom_intensity(self.bloom_intensity)
            .with_vignette(
                self.vignette_strength,
                self.vignette_radius,
                self.vignette_softness,
            )
            .with_lens_distortion(self.lens_distortion_amount)
            .with_chromatic_aberration(self.chromatic_aberration_amount);
        StylizedPostProcessProfile::default()
            .with_settings(settings)
            .with_hdr(self.hdr)
            .with_bloom(self.bloom)
            .with_tonemapping(self.tonemapping)
            .with_vignette(self.vignette)
            .with_lens_distortion(self.lens_distortion)
            .with_chromatic_aberration(self.chromatic_aberration)
            .with_prepass_edges(self.prepass_edges)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CelNprObjectAuthoring {
    pub face: FaceRigAuthoring,
    pub post: PostAuthoring,
}
