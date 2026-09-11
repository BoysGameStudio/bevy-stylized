// allow: SIZE_OK - this forward entry point preserves Bevy helper and diagnostic ordering.
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{
        alpha_discard,
        apply_pbr_lighting,
        main_pass_post_lighting_processing,
    },
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}

#import bevy_cel::character_vertex::CharacterVertexOutput
#import bevy_pbr::mesh_view_bindings::{lights, view}

struct CharacterExtensionUniform {
    transform: vec4<f32>,
    atlas: vec4<u32>,
    atlas_blend: vec4<f32>,
    authored_emission_gain: vec4<f32>,
    genshin_body: vec4<f32>,
    genshin_global_shadow: vec4<f32>,
}

struct IlmUniform {
    fallback_channels: vec4<f32>,
    state: vec4<u32>,
    dimensions: vec4<u32>,
}

struct IlmChannels {
    specular_strength: f32,
    specular_threshold: f32,
    ao_shadow_influence: f32,
    material_id: u32,
    emission_special_mask: f32,
}

struct GenshinMetallicUniform {
    matcap_dark: vec4<f32>,
    matcap_light: vec4<f32>,
    matcap_shadow: vec4<f32>,
    specular_color: vec4<f32>,
    sharp_layer_color: vec4<f32>,
    matcap: vec4<f32>,
    specular: vec4<f32>,
    state: vec4<u32>,
}

struct GenshinLightingPaletteUniform {
    ambient: vec4<f32>,
    sharp_lit: vec4<f32>,
    soft_lit: vec4<f32>,
    sharp_shadow: vec4<f32>,
    soft_shadow: vec4<f32>,
    rim_lit: vec4<f32>,
    rim_shadow: vec4<f32>,
}

struct GenshinLightSourceUniform {
    direction_enabled: vec4<f32>,
    tint_energy: vec4<f32>,
}

struct FaceSdfUniform {
    basis_forward_bias: vec4<f32>,
    basis_right_rotation: vec4<f32>,
    shadow_tint_softness: vec4<f32>,
    state: vec4<f32>,
    debug_dimensions: vec4<u32>,
    genshin_direction: vec4<f32>,
}

struct FaceLightSample {
    selected_right: f32,
    threshold: f32,
    light_coordinate: vec2<f32>,
    valid: u32,
}

struct FaceLightingFactors {
    soft: f32,
    hard: f32,
}

struct HairHighlightUniform {
    tint_shift: vec4<f32>,
    shape: vec4<f32>,
}

struct ToonSpecularUniform {
    tint_intensity: vec4<f32>,
    shape: vec4<f32>,
}

struct RimLightingUniform {
    tint_intensity: vec4<f32>,
    shape: vec4<f32>,
}

struct CelLightingUniform {
    hair: HairHighlightUniform,
    specular: ToonSpecularUniform,
    rim: RimLightingUniform,
    material_masks: vec4<f32>,
    state: vec4<u32>,
}

struct EyeEmissionUniform {
    eye: vec4<f32>,
    emission: vec4<f32>,
    masks: vec4<f32>,
    state: vec4<u32>,
}

// Extension bindings start at 100. Bindings 0-99 are reserved for Bevy's base material.
@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> character_extension: CharacterExtensionUniform;

@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var toon_ramp_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var toon_ramp_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(103)
var<uniform> ilm_settings: IlmUniform;

@group(#{MATERIAL_BIND_GROUP}) @binding(104)
var ilm_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(105)
var ilm_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(106)
var<uniform> face_sdf_settings: FaceSdfUniform;

@group(#{MATERIAL_BIND_GROUP}) @binding(107)
var face_sdf_left_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(108)
var face_sdf_left_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(109)
var face_sdf_right_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(110)
var face_sdf_right_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(111)
var<uniform> cel_lighting: CelLightingUniform;

@group(#{MATERIAL_BIND_GROUP}) @binding(112)
var<uniform> eye_emission: EyeEmissionUniform;

@group(#{MATERIAL_BIND_GROUP}) @binding(113)
var emission_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(114)
var emission_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(115)
var<uniform> backface_base_color: vec4<u32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(116)
var face_shadow_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(117)
var face_shadow_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(118)
var genshin_metal_map_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(119)
var genshin_metal_map_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(120)
var genshin_specular_ramp_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(121)
var genshin_specular_ramp_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(122)
var<uniform> genshin_metallic: GenshinMetallicUniform;

@group(#{MATERIAL_BIND_GROUP}) @binding(123)
var genshin_packed_normal_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(124)
var genshin_packed_normal_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(125)
var<uniform> genshin_lighting_palette: GenshinLightingPaletteUniform;

@group(#{MATERIAL_BIND_GROUP}) @binding(126)
var<uniform> genshin_light_source: GenshinLightSourceUniform;

const TOON_RAMP_DEBUG_COORDINATE: u32 = 1u;
const TOON_RAMP_DEBUG_BAND: u32 = 2u;
const ILM_DEBUG_SPECULAR: u32 = 1u;
const ILM_DEBUG_AO_SHADOW: u32 = 2u;
const ILM_DEBUG_MATERIAL_ID: u32 = 3u;
const ILM_DEBUG_EMISSION_SPECIAL: u32 = 4u;
const ILM_DEBUG_RAMP_ROW: u32 = 5u;
const ILM_LAYOUT_GENSHIN_CHARACTER: u32 = 1u;
const FACE_SDF_DEBUG_SELECTED_MAP: u32 = 1u;
const FACE_SDF_DEBUG_THRESHOLD: u32 = 2u;
const FACE_SDF_DEBUG_LIGHT_COORDINATE: u32 = 3u;
const CEL_LIGHTING_DEBUG_HAIR_MASK: u32 = 1u;
const CEL_LIGHTING_DEBUG_HAIR_RESULT: u32 = 2u;
const CEL_LIGHTING_DEBUG_SPECULAR_MASK: u32 = 3u;
const CEL_LIGHTING_DEBUG_SPECULAR_RESULT: u32 = 4u;
const CEL_LIGHTING_DEBUG_RIM_MASK: u32 = 5u;
const CEL_LIGHTING_DEBUG_RIM_RESULT: u32 = 6u;
const EYE_EMISSION_DEBUG_EYE_CLASS: u32 = 1u;
const EYE_EMISSION_DEBUG_MASK: u32 = 2u;
const EYE_EMISSION_DEBUG_RESULT: u32 = 3u;
const EYE_EMISSION_DEBUG_OUTLINE_SUPPRESSION: u32 = 4u;
const GENSHIN_DEBUG_SOURCE_LIT: u32 = 1u;
const GENSHIN_DEBUG_RAMP_COORDINATE: u32 = 2u;
const GENSHIN_DEBUG_RAMP_BAND: u32 = 3u;
const GENSHIN_DEBUG_FACE_FACTOR: u32 = 4u;
const GENSHIN_DEBUG_PALETTE_MULTIPLIER: u32 = 5u;
const GENSHIN_DEBUG_SOURCE_NORMAL: u32 = 6u;
const FACE_LIGHT_EPSILON_SQUARED: f32 = 0.00001;
const DIRECTION_EPSILON_SQUARED: f32 = 0.00001;
const PI: f32 = 3.141592653589793;

fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn first_directional_light_active() -> bool {
    return lights.n_directional_lights > 0u
        && luminance(lights.directional_lights[0].color.rgb) > 0.000001;
}

fn raw_toon_light_proxy(lit_color: vec3<f32>, base_color: vec3<f32>) -> f32 {
    return max(luminance(lit_color) / max(luminance(base_color), 0.0001), 0.0);
}

fn toon_light_proxy(lit_color: vec3<f32>, base_color: vec3<f32>) -> f32 {
    return min(raw_toon_light_proxy(lit_color, base_color), 1.0);
}

fn safe_normalize(direction: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    let length_squared = dot(direction, direction);
    if length_squared > DIRECTION_EPSILON_SQUARED {
        return direction * inverseSqrt(length_squared);
    }
    let fallback_length_squared = dot(fallback, fallback);
    if fallback_length_squared > DIRECTION_EPSILON_SQUARED {
        return fallback * inverseSqrt(fallback_length_squared);
    }
    return vec3<f32>(1.0, 0.0, 0.0);
}

fn genshin_explicit_light_enabled() -> bool {
    return genshin_light_source.direction_enabled.w > 0.5;
}

fn genshin_source_light_active() -> bool {
    if genshin_explicit_light_enabled() {
        return genshin_light_source.tint_energy.w > 0.0001;
    }
    return first_directional_light_active();
}

// Body NdotL and metallic NdotH consume Blender's source `L`. Legacy
// materials retain their previously calibrated light-ray convention.
fn resolved_genshin_source_direction(fallback: vec3<f32>) -> vec3<f32> {
    if genshin_explicit_light_enabled() {
        return -safe_normalize(genshin_light_source.direction_enabled.xyz, fallback);
    }
    if first_directional_light_active() {
        return -safe_normalize(lights.directional_lights[0].direction_to_light, fallback);
    }
    return fallback;
}

// Face SDF and generic additive lobes historically consume Bevy's
// NdotL-ready direction. The explicit source uses its single declared vector
// for every branch, eliminating the old body/face carrier split.
fn resolved_genshin_direction_to_light(fallback: vec3<f32>) -> vec3<f32> {
    if genshin_explicit_light_enabled() {
        return safe_normalize(genshin_light_source.direction_enabled.xyz, fallback);
    }
    if first_directional_light_active() {
        return safe_normalize(lights.directional_lights[0].direction_to_light, fallback);
    }
    return fallback;
}

fn resolved_genshin_light_energy(physical_proxy: f32) -> f32 {
    return select(
        physical_proxy,
        max(genshin_light_source.tint_energy.w, 0.0),
        genshin_explicit_light_enabled(),
    );
}

fn resolved_genshin_light_tint(physical_tint: vec3<f32>) -> vec3<f32> {
    return select(
        physical_tint,
        clamp(
            genshin_light_source.tint_energy.rgb,
            vec3<f32>(0.0),
            vec3<f32>(1.0),
        ),
        genshin_explicit_light_enabled(),
    );
}

fn genshin_source_normal(
    in: CharacterVertexOutput,
    is_front: bool,
    fallback: vec3<f32>,
) -> vec3<f32> {
#ifdef VERTEX_UVS_A
    if backface_base_color.z != 0u {
        var uv = in.uv;
#ifdef VERTEX_UVS_B
        if backface_base_color.x != 0u && !is_front {
            uv = in.uv_b;
        }
#endif
        let packed = textureSample(
            genshin_packed_normal_texture,
            genshin_packed_normal_sampler,
            uv,
        ).rg;
        let tangent_x = packed.x * 2.0 - 1.0;
        let tangent_y = packed.y * 2.0 - 1.0;
        // The source texture packs no usable tangent Z in blue. Preserve its
        // authored, intentionally nonstandard +Y² reconstruction before
        // mapping through Bevy's stable Mikk tangent basis.
        let tangent_z = sqrt(max(
            1.0 - tangent_x * tangent_x + tangent_y * tangent_y,
            0.0,
        ));
        var tangent_normal = vec3<f32>(tangent_x, tangent_y, tangent_z);
        if !is_front {
            tangent_normal = -tangent_normal;
        }
        // Blender's `Optimized Normal Map` does not consume a mesh tangent.
        // It feeds UV.x and UV.y into two Bump nodes (distance 1 and 1000)
        // and uses those two resulting normals as a deliberately
        // non-orthogonal matrix beside Geometry.Normal. Rebuild the Bump
        // differential in fragment space so the same animated/deformed
        // surface drives both engines.
        let sigma_x = dpdx(in.world_position.xyz);
        let sigma_y = dpdy(in.world_position.xyz);
        let r1 = cross(sigma_y, in.world_normal);
        let r2 = cross(in.world_normal, sigma_x);
        let determinant = dot(sigma_x, r1);
        let determinant_sign = select(-1.0, 1.0, determinant >= 0.0);
        let base = abs(determinant) * in.world_normal;
        let source_scale = max(character_extension.atlas_blend.z, 1.0);
        let u_gradient = source_scale * determinant_sign
            * (dpdx(uv.x) * r1 + dpdy(uv.x) * r2);
        let v_gradient = source_scale * determinant_sign
            * (dpdx(uv.y) * r1 + dpdy(uv.y) * r2);
        // Both source Bump nodes have Invert enabled, so their signed surface
        // gradients add to the determinant-scaled normal.
        let tangent_u = safe_normalize(base + u_gradient, in.world_normal);
        let tangent_v = safe_normalize(base + v_gradient * 1000.0, in.world_normal);
        // There is intentionally no Normalize node after the source matrix
        // multiply. Preserve its magnitude because the following dot product
        // and half-Lambert clamp observe it.
        return tangent_normal.x * tangent_u
            + tangent_normal.y * tangent_v
            + tangent_normal.z * in.world_normal;
    }
#endif
    return fallback;
}

fn stable_tangent(normal: vec3<f32>) -> vec3<f32> {
    var axis = vec3<f32>(0.0, 1.0, 0.0);
    if abs(normal.y) > 0.95 {
        axis = vec3<f32>(1.0, 0.0, 0.0);
    }
    return safe_normalize(cross(normal, axis), vec3<f32>(1.0, 0.0, 0.0));
}

fn toon_step(value: f32, threshold: f32, softness: f32) -> f32 {
    if softness <= 0.0 {
        return select(0.0, 1.0, value >= threshold);
    }
    return smoothstep(max(threshold - softness, 0.0), min(threshold + softness, 1.0), value);
}

fn hair_highlight(
    tangent_input: vec3<f32>,
    normal: vec3<f32>,
    view: vec3<f32>,
    light: vec3<f32>,
) -> f32 {
    let matcap_fallback = safe_normalize(cross(normal, view), stable_tangent(normal));
    let tangent = safe_normalize(tangent_input, matcap_fallback);
    let shifted = safe_normalize(tangent + normal * cel_lighting.hair.tint_shift.w, matcap_fallback);
    let half_vector = safe_normalize(view + light, normal);
    let tangent_dot_half = clamp(dot(shifted, half_vector), -1.0, 1.0);
    let anisotropic = sqrt(max(1.0 - tangent_dot_half * tangent_dot_half, 0.0));
    let powered = pow(max(anisotropic, 0.00001), cel_lighting.hair.shape.x);
    return toon_step(powered, cel_lighting.hair.shape.y, cel_lighting.hair.shape.z);
}

fn toon_specular(normal: vec3<f32>, view: vec3<f32>, light: vec3<f32>) -> f32 {
    let half_vector = safe_normalize(view + light, normal);
    let powered = pow(max(dot(normal, half_vector), 0.00001), cel_lighting.specular.shape.x);
    return toon_step(powered, cel_lighting.specular.shape.y, cel_lighting.specular.shape.z);
}

fn genshin_nonmetal_specular(
    normal: vec3<f32>,
    view_direction: vec3<f32>,
    light_direction: vec3<f32>,
    ilm: IlmChannels,
) -> f32 {
    let material_id = min(ilm.material_id / 2u, 4u);
    var shininess = cel_lighting.specular.shape.x;
    var multiplier = cel_lighting.specular.tint_intensity.w;
    // Odette's five source materials reduce to three effective ID tables.
    if shininess < 5.0 {
        if material_id >= 4u {
            shininess = 12.0;
            multiplier = 0.3;
        } else if material_id >= 2u {
            shininess = 10.0;
            multiplier = 0.1;
        }
    } else if shininess > 40.0 && material_id >= 2u {
        shininess = 10.0;
        multiplier = 0.1;
    }
    let half_vector = safe_normalize(view_direction + light_direction, normal);
    let response = pow(max(dot(normal, half_vector), 0.001), shininess);
    let gate = select(0.0, 1.0, (1.03 - ilm.specular_threshold) < response);
    return ilm.specular_strength * multiplier * gate;
}

fn fresnel_rim(normal: vec3<f32>, view: vec3<f32>) -> f32 {
    let fresnel = clamp(1.0 - max(dot(normal, view), 0.0), 0.0, 1.0);
    let powered = pow(max(fresnel, 0.00001), cel_lighting.rim.shape.x);
    return toon_step(powered, cel_lighting.rim.shape.y, cel_lighting.rim.shape.z);
}

fn resolved_ramp_row(requested_row: u32) -> u32 {
    let row_count = max(character_extension.atlas.y, 1u);
    let base_row = min(character_extension.atlas.x, row_count - 1u);
    return min(base_row + requested_row, row_count - 1u);
}

fn sample_toon_ramp_row(coordinate: f32, row: u32) -> vec3<f32> {
    let row_count = max(character_extension.atlas.y, 1u);
    let band_count = max(character_extension.atlas.z, 1u);
    let scaled = min(coordinate, 0.999999) * f32(band_count);
    let band = min(u32(floor(scaled)), band_count - 1u);
    let next_band = min(band + 1u, band_count - 1u);
    let row_coordinate = (f32(row) + 0.5) / f32(row_count);
    let band_coordinate = (f32(band) + 0.5) / f32(band_count);
    let next_coordinate = (f32(next_band) + 0.5) / f32(band_count);
    let current = textureSampleLevel(
        toon_ramp_texture,
        toon_ramp_sampler,
        vec2<f32>(band_coordinate, row_coordinate),
        0.0,
    ).rgb;
    let next = textureSampleLevel(
        toon_ramp_texture,
        toon_ramp_sampler,
        vec2<f32>(next_coordinate, row_coordinate),
        0.0,
    ).rgb;
    let smoothing = clamp(character_extension.transform.z, 0.0, 0.5);
    if smoothing == 0.0 {
        return current;
    }
    let blend = smoothstep(1.0 - smoothing, 1.0, fract(scaled));
    return mix(current, next, blend);
}

fn sample_toon_ramp(coordinate: f32, requested_row: u32) -> vec3<f32> {
    let row_count = max(character_extension.atlas.y, 1u);
    let primary_row = resolved_ramp_row(requested_row);
    let secondary_offset = u32(round(max(character_extension.atlas_blend.x, 0.0)));
    let secondary_row = min(primary_row + secondary_offset, row_count - 1u);
    let page_blend = clamp(character_extension.atlas_blend.y, 0.0, 1.0);
    return mix(
        sample_toon_ramp_row(coordinate, primary_row),
        sample_toon_ramp_row(coordinate, secondary_row),
        page_blend,
    );
}

fn standard_vertex_output(in: CharacterVertexOutput, is_front: bool) -> VertexOutput {
    var out: VertexOutput;
    out.position = in.position;
    out.world_position = in.world_position;
    out.world_normal = in.world_normal;
#ifdef VERTEX_UVS_A
    out.uv = in.uv;
#ifdef VERTEX_UVS_B
    if backface_base_color.x != 0u && !is_front {
        // Blender's UV Lerp drives Diffuse, Normal, and Lightmap together.
        // Switching before Bevy builds PbrInput preserves its official normal
        // decoding instead of correcting only base color after the fact.
        out.uv = in.uv_b;
    }
#endif
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = in.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = in.world_tangent;
#endif
#ifdef VERTEX_COLORS
    // HoYoverse vertex colours are control data for AO and ramp width. The
    // Blender material never multiplies them into Diffuse, while Bevy's
    // StandardMaterial does so by default. Preserve the attribute separately
    // on CharacterVertexOutput, but feed neutral white into the PBR helper for
    // the explicit Genshin layout.
    out.color = select(
        in.color,
        vec4<f32>(1.0),
        backface_base_color.y != 0u,
    );
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = in.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = in.visibility_range_dither;
#endif
    return out;
}

fn decoded_ilm(in: CharacterVertexOutput, is_front: bool) -> IlmChannels {
    var sampled = ilm_settings.fallback_channels;
#ifdef VERTEX_UVS_A
    if ilm_settings.state.x != 0u {
        var ilm_uv = in.uv;
#ifdef VERTEX_UVS_B
        if backface_base_color.x != 0u && !is_front {
            ilm_uv = in.uv_b;
        }
#endif
        sampled = textureSampleLevel(ilm_texture, ilm_sampler, ilm_uv, 0.0);
    }
#endif
    var texture_id = u32(round(clamp(sampled.b, 0.0, 1.0) * 255.0));
    var special_mask = clamp(sampled.a, 0.0, 1.0);
    if ilm_settings.state.w == ILM_LAYOUT_GENSHIN_CHARACTER {
        // Gacha lightmaps encode five material classes in alpha at roughly
        // 0.0/0.3/0.5/0.7/1.0. The source 20-row ramps duplicate each class
        // across adjacent rows; the first ten rows are the authored daytime set.
        let alpha = clamp(sampled.a, 0.0, 1.0);
        var source_id = 0u;
        if alpha > 0.8 {
            source_id = 4u;
        } else if alpha > 0.6 {
            source_id = 3u;
        } else if alpha > 0.4 {
            source_id = 2u;
        } else if alpha > 0.2 {
            source_id = 1u;
        }
        let enabled_mask = u32(round(max(character_extension.atlas_blend.w, 0.0)));
        if source_id > 0u
            && enabled_mask != 0u
            && (enabled_mask & (1u << (source_id - 1u))) == 0u
        {
            source_id = 0u;
        }
        texture_id = source_id * 2u;
        special_mask = 1.0;
    }
    var material_id = texture_id;
#ifdef CHARACTER_MATERIAL_ID_ATTRIBUTE
    let vertex_id = u32(round(max(in.material_id, 0.0)));
    if ilm_settings.state.x == 0u || ilm_settings.state.y == 1u {
        material_id = vertex_id;
    }
#endif
    var ilm: IlmChannels;
    ilm.specular_strength = clamp(sampled.r, 0.0, 1.0);
    ilm.specular_threshold = clamp(sampled.b, 0.0, 1.0);
    ilm.ao_shadow_influence = clamp(sampled.g, 0.0, 1.0);
    ilm.material_id = material_id;
    ilm.emission_special_mask = special_mask;
    return ilm;
}

fn final_ao_multiplier(ilm: IlmChannels) -> f32 {
    // Canonical ILM defines G as a final color multiplier. Gacha lightmaps
    // instead feed G into their toon-shadow decision; multiplying it again
    // makes the authored diffuse and colored ramp uniformly dirty and dark.
    return select(
        ilm.ao_shadow_influence,
        1.0,
        ilm_settings.state.w == ILM_LAYOUT_GENSHIN_CHARACTER,
    );
}

fn genshin_light_tint(
    lit_color: vec3<f32>,
    base_color: vec3<f32>,
    raw_light_proxy: f32,
    influence: f32,
) -> vec3<f32> {
    if raw_light_proxy <= 0.0001 {
        return vec3<f32>(1.0);
    }
    // Divide away scalar exposure and the authored diffuse so a neutral
    // light remains neutral while colored Bevy lights still drive the source
    // shader's Sharp/Soft Lit Colour role. Bound and soften the ratio so an
    // absent base channel or a saturated boundary value cannot create NaN,
    // infinity, or an unreadable single-channel result.
    let reference = max(
        base_color * max(raw_light_proxy, 0.05),
        vec3<f32>(0.01),
    );
    let normalized = clamp(
        lit_color / reference,
        vec3<f32>(0.25),
        vec3<f32>(2.0),
    );
    return mix(
        vec3<f32>(1.0),
        mix(vec3<f32>(1.0), normalized, 0.35),
        clamp(influence, 0.0, 1.0),
    );
}

fn genshin_source_lit(
    normal: vec3<f32>,
    lightmap_ao: f32,
    vertex_ao: f32,
    use_vertex_ao: f32,
    fallback_proxy: f32,
) -> f32 {
    var half_lambert = fallback_proxy;
    if genshin_source_light_active() {
        let source_direction = resolved_genshin_source_direction(normal);
        half_lambert = clamp(dot(normal, source_direction) * 0.5 + 0.5, 0.0, 1.0);
    }
    // Blender's GI Body graph multiplies the Lightmap-G/vertex AO terms,
    // smoothsteps that result over 0.01..0.4, then gates its half-Lambert
    // value. G is a shadow-decision input, not a second final-color multiply.
    let ao = clamp(lightmap_ao, 0.0, 1.0)
        * mix(1.0, clamp(vertex_ao, 0.0, 1.0), clamp(use_vertex_ao, 0.0, 1.0));
    let ao_gate = smoothstep(0.01, 0.4, ao);
    return half_lambert * ao_gate;
}

fn genshin_gamma_22(color: vec3<f32>) -> vec3<f32> {
    return pow(max(color, vec3<f32>(0.0)), vec3<f32>(2.2));
}

fn genshin_vertex_control_color(color: vec3<f32>) -> vec3<f32> {
    // Exact Blender `sRGB -> Linear` node-group implementation. Despite its
    // label, its ShaderNodeGamma is 0.4545 and Blender evaluates Gamma as
    // `color ^ gamma`.
    return pow(
        clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)),
        vec3<f32>(0.4545),
    );
}

fn genshin_metallic_active(ilm: IlmChannels) -> bool {
    return genshin_metallic.state.x != 0u
        && genshin_metallic.state.y != 0u
        && ilm.specular_strength > 0.9;
}

fn genshin_metallic_lit_factor(coordinate: f32, ilm: IlmChannels) -> f32 {
    let high_g = select(0.0, 1.0, ilm.ao_shadow_influence > 0.95);
    return clamp(max(coordinate, high_g), 0.0, 1.0);
}

fn genshin_metallic_matcap(
    diffuse: vec3<f32>,
    normal: vec3<f32>,
    view_direction: vec3<f32>,
    lit_factor: f32,
) -> vec3<f32> {
    let camera_up = safe_normalize(view.world_from_view[1].xyz, vec3<f32>(0.0, 1.0, 0.0));
    // Blender transforms camera points (0,0,0) and (0,1,0) to world,
    // subtracts origin - upPoint, then crosses Incoming with that -Up axis.
    let matcap_x = cross(view_direction, -camera_up);
    let matcap_y = -cross(view_direction, matcap_x);
    let tile_scale = max(genshin_metallic.matcap.y, 0.0);
    let uv = vec2<f32>(
        dot(normal, matcap_x) * tile_scale,
        dot(normal, matcap_y),
    ) * 0.5 + vec2<f32>(0.5);
    let sampled = textureSampleLevel(
        genshin_metal_map_texture,
        genshin_metal_map_sampler,
        uv,
        0.0,
    ).rgb;
    let matcap_mix = clamp(
        sampled * genshin_metallic.matcap.x,
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    let dark = genshin_gamma_22(genshin_metallic.matcap_dark.rgb);
    let light = genshin_gamma_22(genshin_metallic.matcap_light.rgb);
    let shadow = genshin_gamma_22(genshin_metallic.matcap_shadow.rgb);
    let tint = mix(dark, light, matcap_mix);
    let shadow_tint = mix(vec3<f32>(1.0), shadow, clamp(1.0 - lit_factor, 0.0, 1.0));
    return diffuse * tint * shadow_tint;
}

fn genshin_metallic_specular(
    normal: vec3<f32>,
    view_direction: vec3<f32>,
    source_ray: vec3<f32>,
    lit_factor: f32,
    ilm: IlmChannels,
) -> vec3<f32> {
    let half_vector = safe_normalize(view_direction + source_ray, normal);
    let ndoth = max(dot(normal, half_vector), 0.001);
    let power = max(genshin_metallic.specular.x, 0.001);
    let scale = max(genshin_metallic.specular.y, 0.0);
    let response = scale * pow(ndoth, power);
    let ramp = textureSampleLevel(
        genshin_specular_ramp_texture,
        genshin_specular_ramp_sampler,
        vec2<f32>(response, 0.5),
        0.0,
    ).rgb;
    let soft = ramp
        * genshin_gamma_22(genshin_metallic.specular_color.rgb)
        * response;
    let sharp = select(
        soft,
        genshin_gamma_22(genshin_metallic.sharp_layer_color.rgb),
        response > genshin_metallic.specular.w,
    );
    let attenuation = clamp(genshin_metallic.specular.z, 0.0, 1.0);
    return sharp * (attenuation + (1.0 - attenuation) * lit_factor);
}

fn genshin_hero_energy(
    calibrated_proxy: f32,
    raw_proxy: f32,
    influence: f32,
) -> f32 {
    if genshin_explicit_light_enabled() {
        return max(genshin_light_source.tint_energy.w, 0.0);
    }
    if raw_proxy <= 0.0001 {
        return 0.0;
    }
    // Source-parity lighting is authored/emissive and therefore has unit
    // energy. Explicit interactive-lighting modes may blend back toward the
    // physical proxy without silently double-lighting the reference modes.
    let physical_energy = mix(0.72, 1.0, calibrated_proxy);
    return mix(1.0, physical_energy, clamp(influence, 0.0, 1.0));
}

fn genshin_global_lighting_multiplier(
    soft_factor: f32,
    sharp_factor: f32,
) -> vec3<f32> {
    if genshin_lighting_palette.ambient.w > 0.5 {
        let soft = mix(
            genshin_lighting_palette.soft_shadow.rgb,
            genshin_lighting_palette.soft_lit.rgb,
            clamp(soft_factor, 0.0, 1.0),
        );
        let sharp = mix(
            genshin_lighting_palette.sharp_shadow.rgb,
            genshin_lighting_palette.sharp_lit.rgb,
            clamp(sharp_factor, 0.0, 1.0),
        );
        return genshin_lighting_palette.ambient.rgb * soft * sharp;
    }
    let shadow = clamp(
        character_extension.genshin_global_shadow.rgb,
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    let soft = mix(shadow, vec3<f32>(1.0), clamp(soft_factor, 0.0, 1.0));
    let sharp = mix(shadow, vec3<f32>(1.0), clamp(sharp_factor, 0.0, 1.0));
    return soft * sharp;
}

fn genshin_lighting_palette_enabled() -> bool {
    return genshin_lighting_palette.ambient.w > 0.5;
}

fn genshin_strict_reference() -> bool {
    return genshin_lighting_palette.rim_lit.w > 0.5;
}

fn character_debug_active() -> bool {
    return character_extension.atlas.w != 0u
        || ilm_settings.state.z != 0u
        || face_sdf_settings.debug_dimensions.x != 0u
        || cel_lighting.state.w != 0u
        || genshin_lighting_palette.rim_shadow.w > 0.5;
}

fn apply_readability_bounds(color: vec3<f32>, base_color: vec3<f32>) -> vec3<f32> {
    let floor = clamp(cel_lighting.material_masks.w, 0.0, 1.0);
    var bounded = max(color, base_color * floor);
    let peak = max(max(bounded.r, bounded.g), bounded.b);
    // Only values that would clip are compressed. Normal authored values stay
    // byte-for-byte unchanged, while extreme light boundaries retain hue and
    // texture contrast instead of flattening to display white.
    bounded /= max(peak, 1.0);
    return bounded;
}

fn apply_authored_emission_gain(
    shaded_color: vec3<f32>,
    base_diffuse_color: vec3<f32>,
    base_diffuse_alpha: f32,
) -> vec3<f32> {
    let gain = character_extension.authored_emission_gain.x;
    if gain <= 0.0 {
        return shaded_color;
    }
    // Exact `Genshin Alpha` topology: subtract the authored 0.03 cutoff,
    // multiply the diffuse/gain emission branch by that mask, then use the
    // same mask again as the final shader mix factor. The resulting quadratic
    // response is intentionally not an additive boost of the shaded branch.
    let emission_mask = clamp(base_diffuse_alpha - 0.03, 0.0, 1.0);
    let emission_branch = base_diffuse_color * gain * emission_mask;
    return mix(shaded_color, emission_branch, emission_mask);
}

// ILM A overrides the material fallback. Present texture and vertex masks then multiply it;
// absent optional multipliers are neutral, while the default material fallback is zero.
fn resolved_emission_mask(in: CharacterVertexOutput, ilm: IlmChannels) -> f32 {
    var mask = select(
        eye_emission.masks.x,
        ilm.emission_special_mask,
        ilm_settings.state.x != 0u,
    );
#ifdef VERTEX_UVS_A
    if eye_emission.state.z != 0u {
        mask *= clamp(textureSampleLevel(emission_texture, emission_sampler, in.uv, 0.0).r, 0.0, 1.0);
    }
#endif
#ifdef CHARACTER_EMISSION_ATTRIBUTE
    mask *= clamp(in.emission_mask, 0.0, 1.0);
#endif
    return mask;
}

fn face_light_sample(direction_to_light: vec3<f32>) -> FaceLightSample {
    let projected_light = vec2<f32>(direction_to_light.x, direction_to_light.z);
    let projected_light_length_squared = dot(projected_light, projected_light);
    var sample: FaceLightSample;
    sample.selected_right = 1.0;
    sample.threshold = 0.5;
    sample.light_coordinate = vec2<f32>(0.5);
    sample.valid = 0u;
    if projected_light_length_squared <= FACE_LIGHT_EPSILON_SQUARED {
        return sample;
    }
    let light_xz = projected_light * inverseSqrt(projected_light_length_squared);
    let sine = sin(face_sdf_settings.basis_right_rotation.w);
    let cosine = cos(face_sdf_settings.basis_right_rotation.w);
    let rotated_light = vec2<f32>(
        light_xz.x * cosine - light_xz.y * sine,
        light_xz.x * sine + light_xz.y * cosine,
    );
    let forward = normalize(vec2<f32>(
        face_sdf_settings.basis_forward_bias.x,
        face_sdf_settings.basis_forward_bias.z,
    ));
    let right = normalize(vec2<f32>(
        face_sdf_settings.basis_right_rotation.x,
        face_sdf_settings.basis_right_rotation.z,
    ));
    let right_dot = clamp(dot(right, rotated_light), -1.0, 1.0);
    let forward_dot = clamp(dot(forward, rotated_light), -1.0, 1.0);
    sample.selected_right = select(0.0, 1.0, right_dot >= 0.0);
    if face_sdf_settings.genshin_direction.x > 0.5 {
        // The source uses its opposite-side mirrored threshold map for the
        // direction-to-light right-dot convention.
        sample.selected_right = select(0.0, 1.0, right_dot <= 0.0);
        // Blender evaluates the light-ray propagation direction, the negative
        // of Bevy's direction_to_light.
        let half_forward = clamp(forward_dot * 0.5 + 0.5, 0.0, 0.999);
        let shadow_position = clamp(face_sdf_settings.genshin_direction.y, 0.0, 1.0);
        let exponent = pow(clamp(1.5 - shadow_position, 0.0, 1.5), 3.0);
        sample.threshold = clamp(pow(half_forward, exponent), 0.0, 1.0);
    } else {
        sample.threshold = clamp(
            acos(forward_dot) / PI + face_sdf_settings.basis_forward_bias.w,
            0.0,
            1.0,
        );
    }
    sample.light_coordinate = vec2<f32>(right_dot, forward_dot) * 0.5 + 0.5;
    sample.valid = 1u;
    return sample;
}

#ifdef VERTEX_UVS_A
fn genshin_face_shadow_tint(in: CharacterVertexOutput) -> vec3<f32> {
    let auxiliary = textureSampleLevel(
        face_shadow_texture,
        face_shadow_sampler,
        in.uv,
        0.0,
    );
    let day_1 = vec3<f32>(0.899384519, 0.687030829, 0.632042637);
    let day_2 = vec3<f32>(0.816097567, 0.751542141, 0.767409141);
    let day_3 = vec3<f32>(0.787419178);
    let night_1 = vec3<f32>(0.883179744, 0.694080536, 0.638779475);
    let night_2 = vec3<f32>(0.650126524, 0.597168626, 0.731736764);
    let night_3 = day_3;
    var day = day_1;
    var night = night_1;
    if auxiliary.a > 0.2 && auxiliary.a <= 0.4 {
        day = day_2;
        night = night_2;
    } else if auxiliary.a > 0.4 && auxiliary.a <= 0.6 {
        day = day_3;
        night = night_3;
    }
    return mix(day, night, clamp(character_extension.atlas_blend.y, 0.0, 1.0));
}

fn face_sdf_lighting_factors(
    in: CharacterVertexOutput,
    sample: FaceLightSample,
) -> FaceLightingFactors {
    var right_uv = in.uv;
    if face_sdf_settings.debug_dimensions.w != 0u {
        right_uv.x = 1.0 - right_uv.x;
    }
    var threshold_value = textureSampleLevel(
        face_sdf_left_texture,
        face_sdf_left_sampler,
        in.uv,
        0.0,
    ).r;
    if sample.selected_right > 0.5 {
        threshold_value = textureSampleLevel(
            face_sdf_right_texture,
            face_sdf_right_sampler,
            right_uv,
            0.0,
        ).r;
    }
    let softness = face_sdf_settings.shadow_tint_softness.w;
    var factors: FaceLightingFactors;
    if face_sdf_settings.genshin_direction.x > 0.5
        && face_sdf_settings.genshin_direction.z > 0.5
    {
        let auxiliary = textureSampleLevel(
            face_shadow_texture,
            face_shadow_sampler,
            in.uv,
            0.0,
        );
        let wrapped_alpha = auxiliary.a - select(0.0, 1.0, 0.55 < auxiliary.a);
        let front_gate = clamp(((1.0 - sample.threshold) - 0.71) / 0.09, 0.0, 1.0);
        let correction = front_gate * wrapped_alpha * 5.0;
        let hard_width = max(softness, 0.000001);
        let hard_threshold = smoothstep(
            sample.threshold - hard_width,
            sample.threshold + hard_width,
            threshold_value,
        );
        // The source Face Factor owns a second, deliberately broader branch:
        // it adds 0.05 to both the sampled threshold value and the hard width
        // before the smoothstep. Genshin Face Lighting consumes this Soft
        // Factor independently from the hard Factor.
        let soft_width = hard_width + 0.05;
        let soft_threshold = smoothstep(
            sample.threshold - soft_width,
            sample.threshold + soft_width,
            threshold_value + 0.05,
        );
        factors.hard = clamp(hard_threshold * auxiliary.g + correction, 0.0, 1.0);
        factors.soft = clamp(soft_threshold * auxiliary.g + correction, 0.0, 1.0);
        return factors;
    }
    if softness == 0.0 {
        factors.hard = select(0.0, 1.0, threshold_value >= sample.threshold);
    } else {
        factors.hard = smoothstep(
            sample.threshold - softness,
            sample.threshold + softness,
            threshold_value,
        );
    }
    factors.soft = factors.hard;
    return factors;
}
#endif

@fragment
fn fragment(
    in: CharacterVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(
        standard_vertex_output(in, is_front),
        is_front,
    );
    let genshin_character = ilm_settings.state.w == ILM_LAYOUT_GENSHIN_CHARACTER;
    let source_normal = genshin_source_normal(in, is_front, pbr_input.N);
    if genshin_character {
        pbr_input.N = source_normal;
    }
    // Preserve the authored texture alpha before Bevy's opaque-material
    // contract intentionally replaces it with 1.0 in alpha_discard(). Genshin
    // uses this otherwise-hidden channel as an emission mask even though the
    // surface itself remains fully opaque.
    let authored_diffuse_alpha = pbr_input.material.base_color.a;
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material,
        pbr_input.material.base_color,
    );

    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
        if character_extension.transform.w > 0.5 {
            let emissive = pbr_input.material.emissive.rgb * pbr_input.material.base_color.a;
            let lit_without_emissive = max(out.color.rgb - emissive, vec3<f32>(0.0));
            let physical_raw_proxy = raw_toon_light_proxy(
                lit_without_emissive,
                pbr_input.material.base_color.rgb,
            );
            let raw_proxy = resolved_genshin_light_energy(physical_raw_proxy);
            let proxy = min(raw_proxy, 1.0);
            let physical_proxy = toon_light_proxy(
                lit_without_emissive, pbr_input.material.base_color.rgb,
            );
            let ilm = decoded_ilm(in, is_front);
            // Blender's Genshin body group consumes a normalized, thresholded
            // Lit Factor: its ordinary front-facing result is already near
            // one.  Bevy's PBR luminance proxy is in physical-lighting space
            // and measures roughly 0.1--0.15 on the same front-facing pixels
            // under the useful showcase studio rig.
            // Calibrate that domain before sampling the authored ramp; doing
            // this only for the declared Genshin layout keeps the framework's
            // canonical PBR-to-ramp contract unchanged.
            let calibrated_proxy = select(
                min(physical_proxy * 8.0, 1.0),
                proxy,
                genshin_explicit_light_enabled(),
            );
            let ramp_light_proxy = select(proxy, calibrated_proxy, genshin_character);
            var vertex_color_linear = vec3<f32>(1.0);
#ifdef VERTEX_COLORS
            // Named glTF export writes Blender's scene-linear `Col.color`.
            // The source graph subsequently applies its Gamma(0.4545) group,
            // so reproduce that explicit node rather than an IEC sRGB EOTF.
            vertex_color_linear = genshin_vertex_control_color(in.color.rgb);
#endif
            let source_lit = genshin_source_lit(
                source_normal,
                ilm.ao_shadow_influence,
                vertex_color_linear.r,
                character_extension.genshin_body.z,
                calibrated_proxy,
            );
            var coordinate = ramp_light_proxy;
            if genshin_character {
                let shadow_position = clamp(character_extension.genshin_body.x, 0.01, 1.0);
                let shadow_width = clamp(character_extension.genshin_body.y, 0.01, 4.0);
                let denominator = max(
                    shadow_position * 2.0 * vertex_color_linear.g * shadow_width,
                    0.0001,
                );
                coordinate = 1.0 - (shadow_position - source_lit) / denominator;
            }
            coordinate = clamp(
                coordinate * character_extension.transform.x
                    + character_extension.transform.y,
                0.0,
                1.0,
            );
            var face_lit = 1.0;
            var face_soft_lit = 1.0;
            var face_sample: FaceLightSample;
            var face_override = false;
#ifdef VERTEX_UVS_A
            if face_sdf_settings.state.y > 0.5
                && face_sdf_settings.state.z > 0.5
                && face_sdf_settings.state.w > 0.5
                && genshin_source_light_active()
            {
                face_sample = face_light_sample(
                    resolved_genshin_direction_to_light(pbr_input.N),
                );
                if face_sample.valid != 0u {
                    let face_factors = face_sdf_lighting_factors(in, face_sample);
                    face_lit = face_factors.hard;
                    face_soft_lit = face_factors.soft;
                    let external_shadow_influence = mix(
                        1.0,
                        proxy,
                        face_sdf_settings.state.x,
                    );
                    coordinate = clamp(
                        face_lit * external_shadow_influence * character_extension.transform.x
                            + character_extension.transform.y,
                        0.0,
                        1.0,
                    );
                    face_override = true;
                }
            }
#endif
            let ao_multiplier = final_ao_multiplier(ilm);
            let row_count = max(character_extension.atlas.y, 1u);
            let ramp_row = resolved_ramp_row(ilm.material_id);
            var band = sample_toon_ramp(coordinate, ilm.material_id);
            if face_override {
                var face_shadow_tint = face_sdf_settings.shadow_tint_softness.rgb;
#ifdef VERTEX_UVS_A
                if face_sdf_settings.genshin_direction.z > 0.5 {
                    face_shadow_tint = genshin_face_shadow_tint(in);
                }
#endif
                band *= mix(
                    face_shadow_tint,
                    vec3<f32>(1.0),
                    face_lit,
                );
            }
            let physical_light_influence = character_extension.genshin_global_shadow.w;
            let hero_energy = genshin_hero_energy(
                calibrated_proxy,
                raw_proxy,
                physical_light_influence,
            );
            let hero_light_tint = resolved_genshin_light_tint(
                genshin_light_tint(
                    lit_without_emissive,
                    pbr_input.material.base_color.rgb,
                    physical_raw_proxy,
                    physical_light_influence,
                ),
            );
            let direction_to_light = resolved_genshin_direction_to_light(source_normal);
            let source_ray = resolved_genshin_source_direction(-source_normal);
            let metallic_active = genshin_character
                && !face_override
                && genshin_metallic_active(ilm);
            let metallic_lit_factor = genshin_metallic_lit_factor(coordinate, ilm);
            var genshin_lighting_multiplier = vec3<f32>(1.0);
            if genshin_character {
                if face_override {
                    genshin_lighting_multiplier = genshin_global_lighting_multiplier(
                        face_soft_lit,
                        face_lit,
                    );
                } else {
                    let sharp_lit_factor = select(
                        0.0,
                        1.0,
                        metallic_lit_factor > 0.999,
                    );
                    genshin_lighting_multiplier = genshin_global_lighting_multiplier(
                        metallic_lit_factor,
                        sharp_lit_factor,
                    );
                }
            }
            var metallic_specular_color = vec3<f32>(0.0);
            if genshin_character && !face_override {
                // Exact `Light Mix -- Ramps 1`: shadow is a strict comparison
                // with the authored position, while ILM G > .95 forces the
                // original Diffuse even on the geometric shadow side.
                let shadow_position = clamp(character_extension.genshin_body.x, 0.01, 1.0);
                let use_ramp = source_lit < shadow_position
                    && ilm.ao_shadow_influence <= 0.95;
                let genshin_band = select(vec3<f32>(1.0), band, use_ramp);
                if metallic_active {
                    let matcap_color = genshin_metallic_matcap(
                        pbr_input.material.base_color.rgb,
                        source_normal,
                        pbr_input.V,
                        metallic_lit_factor,
                    );
                    metallic_specular_color = genshin_metallic_specular(
                        source_normal,
                        pbr_input.V,
                        source_ray,
                        metallic_lit_factor,
                        ilm,
                    );
                    // Source metallic shading is hero-authored rather than
                    // physically exposed. Only suppress its additive specular
                    // when every real light is absent so the all-zero boundary
                    // cannot produce a floating highlight.
                    let specular_light_gate = select(0.0, 1.0, raw_proxy > 0.0001);
                    out.color = vec4<f32>(
                        matcap_color
                            + metallic_specular_color * specular_light_gate
                            + emissive,
                        out.color.a,
                    );
                } else {
                    out.color = vec4<f32>(
                        pbr_input.material.base_color.rgb
                        * genshin_band
                        * hero_light_tint
                        * hero_energy
                        * ao_multiplier
                        + emissive,
                        out.color.a,
                    );
                }
            } else if face_override {
                out.color = vec4<f32>(
                    pbr_input.material.base_color.rgb
                        * band
                        * hero_light_tint
                        * hero_energy
                        * ao_multiplier
                        + emissive,
                    out.color.a,
                );
            } else {
                let remap = min(band / max(vec3<f32>(proxy), vec3<f32>(0.05)), vec3<f32>(4.0));
                out.color = vec4<f32>(
                    lit_without_emissive * remap * ao_multiplier + emissive,
                    out.color.a,
                );
            }
            let special_mask = select(
                cel_lighting.material_masks.z,
                ilm.emission_special_mask,
                ilm_settings.state.x != 0u,
            );
            var authored_hair_mask = cel_lighting.material_masks.x;
            var authored_hair_tangent = vec3<f32>(0.0);
#ifdef CHARACTER_HAIR_MASK_ATTRIBUTE
            authored_hair_mask = clamp(in.hair_mask, 0.0, 1.0);
#endif
#ifdef CHARACTER_HAIR_TANGENT_ATTRIBUTE
            authored_hair_tangent = in.hair_tangent;
#endif
            let hair_mask = authored_hair_mask * special_mask;
            let specular_mask = ilm.specular_strength;
            let rim_mask = cel_lighting.material_masks.y * special_mask;
            var hair_result = 0.0;
            var specular_result = 0.0;
            var rim_result = 0.0;
            if cel_lighting.state.x != 0u {
                hair_result = hair_highlight(
                    authored_hair_tangent,
                    source_normal,
                    pbr_input.V,
                    direction_to_light,
                ) * hair_mask * proxy;
                out.color = vec4<f32>(
                    out.color.rgb + cel_lighting.hair.tint_shift.rgb
                        * cel_lighting.hair.shape.w
                        * hair_result,
                    out.color.a,
                );
            }
            if cel_lighting.state.y != 0u {
                if metallic_active {
                    // The exact metallic branch above already includes its
                    // authored soft/sharp ramp; do not double-add the generic
                    // continuous-ILM specular approximation.
                    specular_result = luminance(metallic_specular_color);
                } else if genshin_character {
                    specular_result = genshin_nonmetal_specular(
                        source_normal,
                        pbr_input.V,
                        direction_to_light,
                        ilm,
                    ) * proxy;
                    out.color = vec4<f32>(
                        out.color.rgb
                            + cel_lighting.specular.tint_intensity.rgb * specular_result,
                        out.color.a,
                    );
                } else {
                    specular_result = toon_specular(source_normal, pbr_input.V, direction_to_light)
                        * specular_mask
                    * proxy;
                    out.color = vec4<f32>(
                        out.color.rgb + cel_lighting.specular.tint_intensity.rgb
                            * cel_lighting.specular.tint_intensity.w
                            * specular_result,
                        out.color.a,
                    );
                }
            }
            // The accepted Blender reference disables Fresnel and its
            // depth-rim curvature node is not a valid Blender 5.2 reference.
            // PoseTint's generic rim remains available to legacy physical
            // modes but must not contaminate an explicit source-light pass.
            if cel_lighting.state.z != 0u && !genshin_explicit_light_enabled() {
                rim_result = fresnel_rim(source_normal, pbr_input.V) * rim_mask * proxy;
                out.color = vec4<f32>(
                    out.color.rgb + cel_lighting.rim.tint_intensity.rgb
                        * cel_lighting.rim.tint_intensity.w
                        * rim_result,
                    out.color.a,
                );
            }
            if genshin_character && !genshin_lighting_palette_enabled() {
                out.color = vec4<f32>(
                    max(out.color.rgb - emissive, vec3<f32>(0.0))
                        * genshin_lighting_multiplier
                        + emissive,
                    out.color.a,
                );
            }
            let genshin_debug = u32(round(max(genshin_lighting_palette.rim_shadow.w, 0.0)));
            if genshin_debug == GENSHIN_DEBUG_SOURCE_LIT {
                let final_lit_factor = select(
                    metallic_lit_factor,
                    face_soft_lit,
                    face_override,
                );
                out.color = vec4<f32>(vec3<f32>(final_lit_factor), out.color.a);
            } else if genshin_debug == GENSHIN_DEBUG_RAMP_COORDINATE {
                out.color = vec4<f32>(vec3<f32>(coordinate), out.color.a);
            } else if genshin_debug == GENSHIN_DEBUG_RAMP_BAND {
                out.color = vec4<f32>(band, out.color.a);
            } else if genshin_debug == GENSHIN_DEBUG_FACE_FACTOR {
                out.color = vec4<f32>(
                    vec3<f32>(select(0.0, face_lit, face_override)),
                    out.color.a,
                );
            } else if genshin_debug == GENSHIN_DEBUG_PALETTE_MULTIPLIER {
                out.color = vec4<f32>(genshin_lighting_multiplier, out.color.a);
            } else if genshin_debug == GENSHIN_DEBUG_SOURCE_NORMAL {
                out.color = vec4<f32>(source_normal * 0.5 + vec3<f32>(0.5), out.color.a);
            } else if character_extension.atlas.w == TOON_RAMP_DEBUG_COORDINATE {
                out.color = vec4<f32>(vec3<f32>(coordinate), out.color.a);
            } else if character_extension.atlas.w == TOON_RAMP_DEBUG_BAND {
                out.color = vec4<f32>(band, out.color.a);
            } else if ilm_settings.state.z == ILM_DEBUG_SPECULAR {
                out.color = vec4<f32>(vec3<f32>(ilm.specular_strength), out.color.a);
            } else if ilm_settings.state.z == ILM_DEBUG_AO_SHADOW {
                out.color = vec4<f32>(vec3<f32>(ilm.ao_shadow_influence), out.color.a);
            } else if ilm_settings.state.z == ILM_DEBUG_MATERIAL_ID {
                let normalized_id = f32(ilm.material_id) / f32(max(row_count - 1u, 1u));
                out.color = vec4<f32>(vec3<f32>(normalized_id), out.color.a);
            } else if ilm_settings.state.z == ILM_DEBUG_EMISSION_SPECIAL {
                out.color = vec4<f32>(vec3<f32>(ilm.emission_special_mask), out.color.a);
            } else if ilm_settings.state.z == ILM_DEBUG_RAMP_ROW {
                let row_coordinate = (f32(ramp_row) + 0.5) / f32(row_count);
                out.color = vec4<f32>(vec3<f32>(row_coordinate), out.color.a);
            } else if face_override
                && face_sdf_settings.debug_dimensions.x == FACE_SDF_DEBUG_SELECTED_MAP
            {
                out.color = vec4<f32>(
                    mix(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), face_sample.selected_right),
                    out.color.a,
                );
            } else if face_override
                && face_sdf_settings.debug_dimensions.x == FACE_SDF_DEBUG_THRESHOLD
            {
                out.color = vec4<f32>(vec3<f32>(face_sample.threshold), out.color.a);
            } else if face_override
                && face_sdf_settings.debug_dimensions.x == FACE_SDF_DEBUG_LIGHT_COORDINATE
            {
                out.color = vec4<f32>(face_sample.light_coordinate, 0.0, out.color.a);
            } else if cel_lighting.state.w == CEL_LIGHTING_DEBUG_HAIR_MASK {
                out.color = vec4<f32>(vec3<f32>(hair_mask), out.color.a);
            } else if cel_lighting.state.w == CEL_LIGHTING_DEBUG_HAIR_RESULT {
                out.color = vec4<f32>(vec3<f32>(hair_result), out.color.a);
            } else if cel_lighting.state.w == CEL_LIGHTING_DEBUG_SPECULAR_MASK {
                out.color = vec4<f32>(vec3<f32>(specular_mask), out.color.a);
            } else if cel_lighting.state.w == CEL_LIGHTING_DEBUG_SPECULAR_RESULT {
                out.color = vec4<f32>(vec3<f32>(specular_result), out.color.a);
            } else if cel_lighting.state.w == CEL_LIGHTING_DEBUG_RIM_MASK {
                out.color = vec4<f32>(vec3<f32>(rim_mask), out.color.a);
            } else if cel_lighting.state.w == CEL_LIGHTING_DEBUG_RIM_RESULT {
                out.color = vec4<f32>(vec3<f32>(rim_result), out.color.a);
            }
            // Blender applies the global palette before `Genshin Alpha` so
            // the authored diffuse-alpha emission branch remains untinted.
            // Production intentionally retains its later safety ordering;
            // strict reference captures use the source order exactly.
            if genshin_lighting_palette_enabled()
                && genshin_strict_reference()
                && !character_debug_active()
            {
                out.color = vec4<f32>(
                    out.color.rgb * genshin_lighting_multiplier,
                    out.color.a,
                );
            }
            if cel_lighting.material_masks.w > 0.0
                && !genshin_strict_reference()
                && !character_debug_active()
            {
                out.color = vec4<f32>(
                    apply_readability_bounds(
                        out.color.rgb,
                        pbr_input.material.base_color.rgb,
                    ),
                    out.color.a,
                );
            }
            if genshin_character && !character_debug_active() {
                let source_exposure = clamp(character_extension.genshin_body.w, 0.25, 1.5);
                out.color = vec4<f32>(
                    apply_authored_emission_gain(
                        out.color.rgb,
                        pbr_input.material.base_color.rgb,
                        authored_diffuse_alpha,
                    ) * source_exposure,
                    out.color.a,
                );
                // The accepted Blender reference applies its global palette
                // after source alpha/emission composition. Applying it before
                // Bevy's opt-in readability floor or the authored alpha branch
                // lets those later stages restore untinted base color, turning
                // sunrise blue and night gray. Only the explicit palette mode
                // takes this late path; legacy Genshin materials retain the
                // established multiplier order above.
                if genshin_lighting_palette_enabled() && !genshin_strict_reference() {
                    out.color = vec4<f32>(
                        out.color.rgb * genshin_lighting_multiplier,
                        out.color.a,
                    );
                }
            }
        }
    } else {
        out.color = pbr_input.material.base_color;
    }
    let task18_ilm = decoded_ilm(in, is_front);
    let emission_mask = resolved_emission_mask(in, task18_ilm);
    let eye_class = eye_emission.eye.z;
    if eye_emission.state.x != 0u {
        let floor_strength = clamp(eye_emission.eye.x + eye_emission.eye.y, 0.0, 1.0);
        let floor_color = pbr_input.material.base_color.rgb * floor_strength;
        let authored_highlight = max(out.color.rgb - pbr_input.material.base_color.rgb, vec3<f32>(0.0));
        let preserved = max(out.color.rgb, floor_color)
            + authored_highlight * eye_emission.eye.w;
        out.color = vec4<f32>(mix(out.color.rgb, preserved, eye_class), out.color.a);
    }
    var emission_color = vec3<f32>(0.0);
    if eye_emission.state.y != 0u {
        emission_color = eye_emission.emission.rgb * eye_emission.emission.w * emission_mask;
        out.color = vec4<f32>(out.color.rgb + emission_color, out.color.a);
    }
    let outline_suppression = eye_emission.masks.y * max(eye_class, emission_mask);
    if eye_emission.state.w == EYE_EMISSION_DEBUG_EYE_CLASS {
        out.color = vec4<f32>(vec3<f32>(eye_class), out.color.a);
    } else if eye_emission.state.w == EYE_EMISSION_DEBUG_MASK {
        out.color = vec4<f32>(vec3<f32>(emission_mask), out.color.a);
    } else if eye_emission.state.w == EYE_EMISSION_DEBUG_RESULT {
        out.color = vec4<f32>(emission_color, out.color.a);
    } else if eye_emission.state.w == EYE_EMISSION_DEBUG_OUTLINE_SUPPRESSION {
        out.color = vec4<f32>(vec3<f32>(outline_suppression), out.color.a);
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
