/// Borrowed validated inputs for a matching left/right face-SDF texture pair.
pub struct FaceSdfTexturePair<'a> {
    left: (Handle<Image>, &'a Image),
    right: (Handle<Image>, &'a Image),
}

impl<'a> FaceSdfTexturePair<'a> {
    /// Creates a pair whose format and dimensions are validated when bound.
    #[must_use]
    pub const fn new(
        left: (Handle<Image>, &'a Image),
        right: (Handle<Image>, &'a Image),
    ) -> Self {
        Self { left, right }
    }
}

impl Default for CharacterExtension {
    fn default() -> Self {
        Self {
            ramp_settings: ToonRampSettings::default(),
            ramp_texture: None,
            ilm_settings: IlmSettings::default(),
            ilm_texture: None,
            face_sdf_settings: FaceSdfSettings::default(),
            face_sdf_left_texture: None,
            face_sdf_right_texture: None,
            cel_lighting_settings: CelLightingSettings::default(),
            eye_emission_settings: EyeEmissionSettings::default(),
            emission_texture: None,
            backface_base_color: UVec4::ZERO,
            face_shadow_texture: None,
            genshin_metal_map: None,
            genshin_specular_ramp: None,
            genshin_metallic_settings: GenshinMetallicSettings::default(),
            genshin_packed_normal: None,
            genshin_lighting_palette: GenshinLightingPalette::default(),
            genshin_light_source: GenshinLightSource::default(),
            outline_cull_mode: OutlineCullMode::Inherit,
        }
    }
}

impl CharacterExtension {
    /// Replaces the optional world-space source light for Genshin materials.
    #[must_use]
    pub const fn with_genshin_light_source(mut self, source: GenshinLightSource) -> Self {
        self.genshin_light_source = source;
        self
    }

    /// Returns the optional world-space source light.
    #[must_use]
    pub const fn genshin_light_source(&self) -> &GenshinLightSource {
        &self.genshin_light_source
    }

    /// Replaces the optional source-authored Genshin global lighting palette.
    #[must_use]
    pub const fn with_genshin_lighting_palette(
        mut self,
        palette: GenshinLightingPalette,
    ) -> Self {
        self.genshin_lighting_palette = palette;
        self
    }

    /// Returns the source-authored Genshin global lighting palette.
    #[must_use]
    pub const fn genshin_lighting_palette(&self) -> &GenshinLightingPalette {
        &self.genshin_lighting_palette
    }

    /// Selects UV1 for standard material textures and ILM on back-facing
    /// fragments while retaining UV0 on front-facing fragments. This mirrors
    /// Blender Gacha materials whose Diffuse/Normal/Lightmap `UV Lerp` nodes
    /// are all driven by the Backfacing socket. The legacy method name is
    /// retained for API compatibility.
    #[must_use]
    pub const fn with_backface_base_color_uv_b(mut self, enabled: bool) -> Self {
        self.backface_base_color.x = enabled as u32;
        self
    }

    /// Keeps mesh vertex colour available to NPR control logic without
    /// letting `StandardMaterial` multiply it into the authored diffuse map.
    #[must_use]
    pub const fn with_control_only_vertex_color(mut self, enabled: bool) -> Self {
        self.backface_base_color.y = enabled as u32;
        self
    }

    /// Reports whether back-facing fragments sample the base texture on UV1.
    #[must_use]
    pub const fn uses_backface_base_color_uv_b(&self) -> bool {
        self.backface_base_color.x != 0
    }

    /// Reports whether vertex colour is reserved for shader control data.
    #[must_use]
    pub const fn uses_control_only_vertex_color(&self) -> bool {
        self.backface_base_color.y != 0
    }

    /// Replaces validated eye readability, HDR emission, and outline seam controls.
    #[must_use]
    pub fn with_eye_emission_settings(mut self, settings: EyeEmissionSettings) -> Self {
        self.eye_emission_settings = settings.with_texture(self.emission_texture.is_some());
        self
    }

    /// Enables a valid authored linear emission mask texture.
    #[must_use]
    pub fn with_emission_texture(mut self, texture: Handle<Image>, image: &Image) -> Self {
        let size = image.texture_descriptor.size;
        let valid = size.width > 0
            && size.height > 0
            && size.depth_or_array_layers == 1
            && image.texture_descriptor.dimension
                == bevy::render::render_resource::TextureDimension::D2
            && EyeEmissionSettings::supports_texture_format(image.texture_descriptor.format);
        self.eye_emission_settings = self.eye_emission_settings.with_texture(valid);
        self.emission_texture = valid.then_some(texture);
        self
    }

    /// Returns validated eye readability, HDR emission, and outline seam controls.
    #[must_use]
    pub const fn eye_emission_settings(&self) -> &EyeEmissionSettings {
        &self.eye_emission_settings
    }

    /// Returns the valid linear emission mask texture, when present.
    #[must_use]
    pub const fn emission_texture(&self) -> Option<&Handle<Image>> {
        self.emission_texture.as_ref()
    }

    /// Replaces validated hair, specular, rim, mask, and diagnostic controls.
    #[must_use]
    pub const fn with_cel_lighting_settings(mut self, settings: CelLightingSettings) -> Self {
        self.cel_lighting_settings = settings;
        self
    }

    /// Returns validated additive cel-lighting controls.
    #[must_use]
    pub const fn cel_lighting_settings(&self) -> &CelLightingSettings {
        &self.cel_lighting_settings
    }

    /// Enables an authored linear ramp atlas.
    #[must_use]
    pub fn with_ramp_texture(mut self, texture: Handle<Image>) -> Self {
        self.ramp_settings = self.ramp_settings.with_texture(true);
        self.ramp_texture = Some(texture);
        self
    }

    /// Replaces the validated ramp controls.
    #[must_use]
    pub fn with_ramp_settings(mut self, settings: ToonRampSettings) -> Self {
        self.ramp_settings = settings.with_texture(self.ramp_texture.is_some());
        self
    }

    /// Returns the active ramp controls.
    #[must_use]
    pub const fn ramp_settings(&self) -> &ToonRampSettings {
        &self.ramp_settings
    }

    /// Returns the authored ramp atlas, or `None` for unchanged forward lighting.
    #[must_use]
    pub const fn ramp_texture(&self) -> Option<&Handle<Image>> {
        self.ramp_texture.as_ref()
    }

    /// Clears the optional toon-ramp texture while retaining scalar settings.
    #[must_use]
    pub fn without_ramp_texture(mut self) -> Self {
        self.ramp_texture = None;
        self.ramp_settings = self.ramp_settings.with_texture(false);
        self
    }

    /// Enables an authored linear-data ILM texture when its dimensions are valid.
    #[must_use]
    pub fn with_ilm_texture(mut self, texture: Handle<Image>, image: &Image) -> Self {
        let size = image.texture_descriptor.size;
        self.ilm_settings = self
            .ilm_settings
            .with_texture_dimensions(size.width, size.height)
            .with_texture(is_linear_ilm_format(image.texture_descriptor.format));
        if self.ilm_settings.uses_texture() {
            self.ilm_texture = Some(texture);
        }
        self
    }

    /// Replaces the validated ILM decode and diagnostic controls.
    #[must_use]
    pub fn with_ilm_settings(mut self, settings: IlmSettings) -> Self {
        let settings = match self.ilm_settings.texture_dimensions() {
            Some((width, height)) => settings.with_texture_dimensions(width, height),
            None => settings,
        };
        self.ilm_settings = settings.with_texture(self.ilm_texture.is_some());
        self
    }

    /// Returns the validated ILM controls.
    #[must_use]
    pub const fn ilm_settings(&self) -> &IlmSettings {
        &self.ilm_settings
    }

    /// Returns the authored ILM texture, or `None` for neutral fallback channels.
    #[must_use]
    pub const fn ilm_texture(&self) -> Option<&Handle<Image>> {
        self.ilm_texture.as_ref()
    }

    /// Clears the optional ILM texture while retaining scalar settings.
    #[must_use]
    pub fn without_ilm_texture(mut self) -> Self {
        self.ilm_texture = None;
        self.ilm_settings = self.ilm_settings.with_texture(false);
        self
    }

    /// Enables matching linear left/right threshold maps after validating both images.
    #[must_use]
    pub fn with_face_sdf_textures(mut self, textures: FaceSdfTexturePair<'_>) -> Self {
        let (left_texture, left_image) = textures.left;
        let (right_texture, right_image) = textures.right;
        let left_size = left_image.texture_descriptor.size;
        let right_size = right_image.texture_descriptor.size;
        let valid = valid_face_image(left_image)
            && valid_face_image(right_image)
            && left_size.width == right_size.width
            && left_size.height == right_size.height;
        if valid {
            self.face_sdf_settings = self
                .face_sdf_settings
                .with_texture_dimensions(left_size.width, left_size.height)
                .with_textures(true);
            self.face_sdf_left_texture = Some(left_texture);
            self.face_sdf_right_texture = Some(right_texture);
        } else {
            self.face_sdf_settings = self
                .face_sdf_settings
                .with_texture_dimensions(0, 0)
                .with_textures(false);
            self.face_sdf_left_texture = None;
            self.face_sdf_right_texture = None;
        }
        self
    }

    /// Replaces controls while preserving validated texture state and dimensions.
    #[must_use]
    pub fn with_face_sdf_settings(mut self, settings: FaceSdfSettings) -> Self {
        let settings = match self.face_sdf_settings.texture_dimensions() {
            Some((width, height)) => settings.with_texture_dimensions(width, height),
            None => settings,
        };
        self.face_sdf_settings = settings
            .with_textures(
                self.face_sdf_left_texture.is_some() && self.face_sdf_right_texture.is_some(),
            )
            .with_genshin_face_shadow_texture(self.face_shadow_texture.is_some());
        self
    }

    /// Binds the source packed HoYoverse face-shadow correction map.
    #[must_use]
    pub fn with_face_shadow_texture(mut self, texture: Handle<Image>, image: &Image) -> Self {
        let valid = valid_face_image(image);
        self.face_shadow_texture = valid.then_some(texture);
        self.face_sdf_settings = self
            .face_sdf_settings
            .with_genshin_face_shadow_texture(valid);
        self
    }

    /// Returns the packed source face-shadow correction map when valid.
    #[must_use]
    pub const fn face_shadow_texture(&self) -> Option<&Handle<Image>> {
        self.face_shadow_texture.as_ref()
    }

    /// Binds the source sRGB metallic matcap and linear specular ramp.
    #[must_use]
    pub fn with_genshin_metallic_textures(
        mut self,
        metal_map: (Handle<Image>, &Image),
        specular_ramp: (Handle<Image>, &Image),
    ) -> Self {
        let valid = valid_genshin_metal_map(metal_map.1)
            && valid_genshin_specular_ramp(specular_ramp.1);
        self.genshin_metal_map = valid.then_some(metal_map.0);
        self.genshin_specular_ramp = valid.then_some(specular_ramp.0);
        self.genshin_metallic_settings = self.genshin_metallic_settings.with_textures(valid);
        self
    }

    /// Replaces metallic controls while preserving texture validation state.
    #[must_use]
    pub fn with_genshin_metallic_settings(
        mut self,
        settings: GenshinMetallicSettings,
    ) -> Self {
        self.genshin_metallic_settings = settings.with_textures(
            self.genshin_metal_map.is_some() && self.genshin_specular_ramp.is_some(),
        );
        self
    }

    /// Returns the source metallic controls.
    #[must_use]
    pub const fn genshin_metallic_settings(&self) -> &GenshinMetallicSettings {
        &self.genshin_metallic_settings
    }

    /// Returns both validated source textures.
    #[must_use]
    pub fn genshin_metallic_textures(&self) -> Option<(&Handle<Image>, &Handle<Image>)> {
        self.genshin_metal_map
            .as_ref()
            .zip(self.genshin_specular_ramp.as_ref())
    }

    /// Binds the source packed normal whose RG channels require Z
    /// reconstruction instead of StandardMaterial's ordinary RGB decode.
    #[must_use]
    pub fn with_genshin_packed_normal_texture(
        mut self,
        texture: Handle<Image>,
        image: &Image,
    ) -> Self {
        let valid = valid_genshin_packed_normal(image);
        self.genshin_packed_normal = valid.then_some(texture);
        self.backface_base_color.z = valid as u32;
        self
    }

    /// Returns the validated packed source normal texture.
    #[must_use]
    pub const fn genshin_packed_normal_texture(&self) -> Option<&Handle<Image>> {
        self.genshin_packed_normal.as_ref()
    }



    /// Clears the optional packed face-shadow correction map.
    #[must_use]
    pub fn without_face_shadow_texture(mut self) -> Self {
        self.face_shadow_texture = None;
        self.face_sdf_settings = self
            .face_sdf_settings
            .with_genshin_face_shadow_texture(false);
        self
    }

    /// Returns active directional face controls and world basis.
    #[must_use]
    pub const fn face_sdf_settings(&self) -> &FaceSdfSettings {
        &self.face_sdf_settings
    }

    /// Returns both threshold maps, or `None` when either map failed validation.
    #[must_use]
    pub fn face_sdf_textures(&self) -> Option<(&Handle<Image>, &Handle<Image>)> {
        self.face_sdf_left_texture
            .as_ref()
            .zip(self.face_sdf_right_texture.as_ref())
    }

    /// Clears both face-SDF textures while retaining scalar settings.
    #[must_use]
    pub fn without_face_sdf_textures(mut self) -> Self {
        self.face_sdf_left_texture = None;
        self.face_sdf_right_texture = None;
        self.face_sdf_settings = self
            .face_sdf_settings
            .with_texture_dimensions(0, 0)
            .with_textures(false);
        self
    }

    /// Clears the optional emission texture while retaining scalar settings.
    #[must_use]
    pub fn without_emission_texture(mut self) -> Self {
        self.emission_texture = None;
        self.eye_emission_settings = self.eye_emission_settings.with_texture(false);
        self
    }

    pub(crate) const fn set_face_sdf_settings(&mut self, settings: FaceSdfSettings) {
        self.face_sdf_settings = settings;
    }
}

const fn is_linear_ilm_format(format: TextureFormat) -> bool {
    matches!(
        format,
        TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba16Float
            | TextureFormat::Rgba32Float
            | TextureFormat::Rgba16Unorm
    )
}

fn valid_face_image(image: &Image) -> bool {
    let size = image.texture_descriptor.size;
    size.width > 0
        && size.height > 0
        && size.depth_or_array_layers == 1
        && image.texture_descriptor.dimension
            == bevy::render::render_resource::TextureDimension::D2
        && matches!(
            image.texture_descriptor.format,
            TextureFormat::R8Unorm
                | TextureFormat::Rg8Unorm
                | TextureFormat::Rgba8Unorm
                | TextureFormat::R16Float
                | TextureFormat::Rg16Float
                | TextureFormat::Rgba16Float
                | TextureFormat::R32Float
                | TextureFormat::Rg32Float
                | TextureFormat::Rgba32Float
        )
}

fn valid_genshin_metal_map(image: &Image) -> bool {
    let size = image.texture_descriptor.size;
    size.width > 0
        && size.height > 0
        && size.depth_or_array_layers == 1
        && image.texture_descriptor.dimension
            == bevy::render::render_resource::TextureDimension::D2
        && matches!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb | TextureFormat::Bgra8UnormSrgb
        )
}

fn valid_genshin_specular_ramp(image: &Image) -> bool {
    let size = image.texture_descriptor.size;
    size.width > 0
        && size.height > 0
        && size.depth_or_array_layers == 1
        && image.texture_descriptor.dimension
            == bevy::render::render_resource::TextureDimension::D2
        && is_linear_ilm_format(image.texture_descriptor.format)
}

fn valid_genshin_packed_normal(image: &Image) -> bool {
    let size = image.texture_descriptor.size;
    size.width > 0
        && size.height > 0
        && size.depth_or_array_layers == 1
        && image.texture_descriptor.dimension
            == bevy::render::render_resource::TextureDimension::D2
        && is_linear_ilm_format(image.texture_descriptor.format)
}
