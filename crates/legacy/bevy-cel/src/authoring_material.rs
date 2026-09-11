#![expect(
    missing_docs,
    reason = "serde field names are the versioned CEL wire contract"
)]

use serde::{Deserialize, Serialize};

use crate::{
    CelLightingSettings, EyeEmissionSettings, FaceSdfSettings, HairHighlightSettings,
    IlmMaterialIdSource, IlmSettings, OutlineSettings, RimLightingSettings, ToonRampSettings,
    ToonSpecularSettings,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureFilter {
    Nearest,
    #[default]
    Linear,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RampAuthoring {
    pub row: u32,
    pub row_count: u32,
    pub band_count: u32,
    pub smoothing: f32,
    pub scale: f32,
    pub offset: f32,
    pub image: Option<String>,
    pub filter: TextureFilter,
    pub mipmaps: bool,
}

impl RampAuthoring {
    #[must_use]
    pub fn to_settings(&self) -> ToonRampSettings {
        ToonRampSettings::default()
            .with_atlas_row(self.row, self.row_count)
            .with_bands(self.band_count, self.smoothing)
            .with_transform(self.scale, self.offset)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialIdSourceAuthoring {
    #[default]
    TextureThenVertex,
    VertexThenTexture,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IlmAuthoring {
    pub image: Option<String>,
    pub material_id_source: MaterialIdSourceAuthoring,
    pub filter: TextureFilter,
    pub mipmaps: bool,
}

impl IlmAuthoring {
    #[must_use]
    pub fn to_settings(&self) -> IlmSettings {
        let source = match self.material_id_source {
            MaterialIdSourceAuthoring::TextureThenVertex => IlmMaterialIdSource::TextureThenVertex,
            MaterialIdSourceAuthoring::VertexThenTexture => IlmMaterialIdSource::VertexThenTexture,
        };
        IlmSettings::default().with_material_id_source(source)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FaceAuthoring {
    pub left_image: Option<String>,
    pub right_image: Option<String>,
    pub bias: f32,
    pub rotation: f32,
    pub softness: f32,
    pub shadow_tint: [f32; 3],
    pub external_shadow_influence: f32,
    #[serde(default)]
    pub filter: TextureFilter,
    #[serde(default)]
    pub mipmaps: bool,
}

impl FaceAuthoring {
    #[must_use]
    pub fn to_settings(&self) -> FaceSdfSettings {
        FaceSdfSettings::default()
            .with_bias_rotation(self.bias, self.rotation)
            .with_softness(self.softness)
            .with_shadow_tint(self.shadow_tint)
            .with_external_shadow_influence(self.external_shadow_influence)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct HairAuthoring {
    pub enabled: bool,
    pub tint: [f32; 3],
    pub shift: f32,
    pub power: f32,
    pub threshold: f32,
    pub softness: f32,
    pub intensity: f32,
    pub material_mask: f32,
}

impl HairAuthoring {
    #[must_use]
    pub fn to_settings(&self) -> HairHighlightSettings {
        HairHighlightSettings::default()
            .with_tint_shift(self.tint, self.shift)
            .with_shape(self.power, self.threshold)
            .with_softness_intensity(self.softness, self.intensity)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SpecularAuthoring {
    pub enabled: bool,
    pub tint: [f32; 3],
    pub intensity: f32,
    pub power: f32,
    pub threshold: f32,
    pub softness: f32,
}

impl SpecularAuthoring {
    #[must_use]
    pub fn to_settings(&self) -> ToonSpecularSettings {
        ToonSpecularSettings::default()
            .with_tint_intensity(self.tint, self.intensity)
            .with_shape(self.power, self.threshold)
            .with_softness(self.softness)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RimAuthoring {
    pub enabled: bool,
    pub tint: [f32; 3],
    pub intensity: f32,
    pub power: f32,
    pub threshold: f32,
    pub softness: f32,
    pub material_mask: f32,
    pub special_fallback: f32,
}

impl RimAuthoring {
    #[must_use]
    pub fn to_settings(&self) -> RimLightingSettings {
        RimLightingSettings::default()
            .with_tint_intensity(self.tint, self.intensity)
            .with_shape(self.power, self.threshold)
            .with_softness(self.softness)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EyeEmissionAuthoring {
    pub eye_enabled: bool,
    pub shadow_floor: f32,
    pub ambient_floor: f32,
    pub eye_class: f32,
    pub highlight_preservation: f32,
    pub emission_enabled: bool,
    pub emission_tint: [f32; 3],
    pub emission_intensity: f32,
    pub emission_image: Option<String>,
    pub material_fallback: f32,
    pub outline_suppression: f32,
    pub filter: TextureFilter,
    pub mipmaps: bool,
}

impl EyeEmissionAuthoring {
    #[must_use]
    pub fn to_settings(&self) -> EyeEmissionSettings {
        EyeEmissionSettings::default()
            .with_eye_floor(self.shadow_floor, self.ambient_floor)
            .with_eye_class(self.eye_class, self.highlight_preservation)
            .with_emission(self.emission_tint, self.emission_intensity)
            .with_masks(self.material_fallback, self.outline_suppression)
            .with_eye_enabled(self.eye_enabled)
            .with_emission_enabled(self.emission_enabled)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutlineAlphaMode {
    Opaque,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OutlineAuthoring {
    pub enabled: bool,
    pub color: [f32; 3],
    pub width: f32,
    pub face_exclusion: f32,
    pub eye_exclusion: f32,
    pub alpha_mode: OutlineAlphaMode,
}

impl OutlineAuthoring {
    #[must_use]
    pub fn to_settings(&self) -> OutlineSettings {
        OutlineSettings::default()
            .with_color(bevy::color::LinearRgba::new(
                self.color[0],
                self.color[1],
                self.color[2],
                1.0,
            ))
            .with_width(self.width)
            .with_exclusion_masks(self.face_exclusion, self.eye_exclusion)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CelNprMaterialAuthoring {
    pub ramp: RampAuthoring,
    pub ilm: IlmAuthoring,
    pub face: FaceAuthoring,
    pub hair: HairAuthoring,
    pub specular: SpecularAuthoring,
    pub rim: RimAuthoring,
    pub eye_emission: EyeEmissionAuthoring,
    pub outline: OutlineAuthoring,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CelNprMaterialEntry {
    pub slot: u32,
    pub name: String,
    pub settings: CelNprMaterialAuthoring,
}

impl CelNprMaterialAuthoring {
    #[must_use]
    pub fn to_lighting_settings(&self) -> CelLightingSettings {
        CelLightingSettings::default()
            .with_hair(self.hair.to_settings())
            .with_specular(self.specular.to_settings())
            .with_rim(self.rim.to_settings())
            .with_material_masks(
                self.hair.material_mask,
                self.rim.material_mask,
                self.rim.special_fallback,
            )
            .with_hair_enabled(self.hair.enabled)
            .with_specular_enabled(self.specular.enabled)
            .with_rim_enabled(self.rim.enabled)
    }
}
