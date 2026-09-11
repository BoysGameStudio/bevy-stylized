#define_import_path bevy_stylized_render::hero::hero_forward

// Hero@Portable 前向（It-5，provisional pending ADR-0001/0005/0006）。
//
// = Character（cel 基础 + face SDF + hair 高光 + rim + emission）
//   + authored ramp（atlas 行 = MeshTag profile_index）
//   + packed normal（xy 存储 z 重建）
//   + metal matcap（view-space normal 查找 × ILM.B metal mask）
//   + spec ramp（半角高光坐标采样 × 强度）
//
// Portable 合法：全部 fixed binding + filterable float 纹理；无 storage /
// compute / bindless。fallback（HERO_FLAG_PORTABLE_FALLBACK）关闭
// matcap / spec ramp / packed normal。
//
// 结构名在本模块内唯一（naga-oil 跨模块同名 struct 冲突规避）。

#import bevy_stylized_render::common::npr_common::{
    deform_vertex, analytic_ramp, face_light_dots, face_threshold_from_dots,
}
#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    mesh_functions,
    mesh_view_bindings::{view, lights},
    view_transformations::position_world_to_clip,
}

struct HeroCelParams {
    base_color_factor: vec4<f32>,
    shading: vec4<f32>,
    // x: rim 幂次, y: rim 强度, z: 预留, w: alpha mask cutoff
    rim: vec4<f32>,
    rim_color: vec4<f32>,
}

struct HeroCharacterParams {
    cel: HeroCelParams,
    face_forward_softness: vec4<f32>,
    face_misc: vec4<f32>,
    hair: vec4<f32>,
    emission: vec4<f32>,
}

struct HeroParams {
    character: HeroCharacterParams,
    // x: matcap 强度, y: spec ramp 强度, z: ramp atlas 行数, w: 预留
    hero_a: vec4<f32>,
    // rgb: matcap 染色, w: 位标志（bitcast u32）
    hero_b: vec4<f32>,
}

const CEL_FLAG_ALPHA_MASK: u32 = 8u;
const CHAR_FLAG_ILM_TEXTURE: u32 = 1u;
const CHAR_FLAG_FACE_SDF_TEXTURE: u32 = 2u;
const CHAR_FLAG_HAIR_SPECULAR: u32 = 4u;
const CHAR_FLAG_EMISSION: u32 = 8u;
const HERO_FLAG_RAMP_TEXTURE: u32 = 1u;
const HERO_FLAG_PACKED_NORMAL: u32 = 2u;
const HERO_FLAG_MATCAP: u32 = 4u;
const HERO_FLAG_SPEC_RAMP: u32 = 8u;
const HERO_FLAG_PORTABLE_FALLBACK: u32 = 16u;

// MeshTag 位段（与 instance.rs 一致；provisional pending ADR-0004）。
const TAG_FACE_THRESHOLD_SHIFT: u32 = 8u;
const TAG_FACE_SIDE_SHIFT: u32 = 16u;

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> params: HeroParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var base_color_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var ilm_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var ilm_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5)
var face_sdf_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6)
var face_sdf_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(7)
var ramp_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8)
var ramp_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(9)
var packed_normal_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10)
var packed_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(11)
var matcap_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(12)
var matcap_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(13)
var spec_ramp_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(14)
var spec_ramp_sampler: sampler;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    let deformed = deform_vertex(vertex_in);
    var out: VertexOutput;
#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = normalize(
        (deformed.world_from_local * vec4<f32>(deformed.local_normal, 0.0)).xyz
    );
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        deformed.local_normal,
        deformed.instance_index,
    );
#endif
#endif
#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(
        deformed.world_from_local,
        vec4<f32>(deformed.local_position, 1.0),
    );
    out.position = position_world_to_clip(out.world_position.xyz);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex_in.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex_in.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        deformed.world_from_local,
        vertex_in.tangent,
        deformed.instance_index,
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex_in.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = deformed.instance_index;
#endif
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    let cel_flags = bitcast<u32>(params.character.cel.shading.w);
    let char_flags = bitcast<u32>(params.character.face_misc.y);
    let hero_flags = bitcast<u32>(params.hero_b.w);

    var base = params.character.cel.base_color_factor;
#ifdef VERTEX_UVS_A
    base *= textureSample(base_color_texture, base_color_sampler, in.uv);
#endif
    if (cel_flags & CEL_FLAG_ALPHA_MASK) != 0u {
        if base.a < params.character.cel.rim.w {
            discard;
        }
    }

    // View 层 key light。
    var light_direction = vec3<f32>(0.0, 1.0, 0.0);
    var light_color = vec3<f32>(1.0);
    if lights.n_directional_lights > 0u {
        light_direction = lights.directional_lights[0].direction_to_light;
        light_color = lights.directional_lights[0].color.rgb;
    }

    // Packed normal（xy 存储，z 重建；通用解码，未锁定上游的非常规
    // 重建式——profile 特化属 profile crate）。
    var normal = normalize(in.world_normal);
#ifdef VERTEX_UVS_A
    if (hero_flags & HERO_FLAG_PACKED_NORMAL) != 0u {
        let packed = textureSample(packed_normal_texture, packed_normal_sampler, in.uv).rg;
        let tangent_xy = packed * 2.0 - vec2<f32>(1.0);
        let tangent_z = sqrt(max(1.0 - dot(tangent_xy, tangent_xy), 0.0));
        // 无切线基时的保守近似：把解码法线视作物体空间扰动叠加到几何法线。
        let perturb = vec3<f32>(tangent_xy.x, tangent_xy.y, tangent_z);
        normal = normalize(normal + (perturb - vec3<f32>(0.0, 0.0, 1.0)) * 0.5);
    }
#endif

    let half_lambert = clamp(dot(normal, light_direction) * 0.5 + 0.5, 0.0, 1.0);

    // ILM decode once（R=AO[带 floor], G=hair mask, B=metal mask, A=emission）。
    var ilm = vec4<f32>(1.0, 0.0, 0.0, 0.0);
#ifdef VERTEX_UVS_A
    if (char_flags & CHAR_FLAG_ILM_TEXTURE) != 0u {
        let packed = textureSample(ilm_texture, ilm_sampler, in.uv);
        ilm = vec4<f32>(mix(0.5, 1.0, packed.r), packed.g, packed.b, packed.a);
    }
#endif

    // 阴影：authored ramp（行 = MeshTag profile_index）优先，缺失时解析 ramp。
    var lit = vec3<f32>(analytic_ramp(
        half_lambert,
        params.character.cel.shading.x,
        params.character.cel.shading.y,
        params.character.cel.shading.z,
    ));
    if (hero_flags & HERO_FLAG_RAMP_TEXTURE) != 0u {
        var row = 0.0;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
        row = f32(mesh_functions::get_tag(in.instance_index) & 0xFFu);
#endif
        let ramp_v = (row + 0.5) / max(params.hero_a.z, 1.0);
        lit = textureSample(ramp_texture, ramp_sampler, vec2<f32>(half_lambert, ramp_v)).rgb;
    }

    // Face SDF（与 character 一致；共享材质 + MeshTag 实例数据）。
#ifdef VERTEX_UVS_A
    if (char_flags & CHAR_FLAG_FACE_SDF_TEXTURE) != 0u {
        var tag: u32 = 0u;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
        tag = mesh_functions::get_tag(in.instance_index);
#endif
        let face_forward_world = normalize(
            (mesh_functions::get_world_from_local(in.instance_index)
                * vec4<f32>(params.character.face_forward_softness.xyz, 0.0)).xyz
        );
        let dots = face_light_dots(light_direction, face_forward_world);
        var threshold = face_threshold_from_dots(dots.x);
        let tag_threshold = (tag >> TAG_FACE_THRESHOLD_SHIFT) & 0xFFu;
        if tag_threshold != 0u {
            threshold = f32(tag_threshold - 1u) / 254.0;
        }
        var flip = dots.y < 0.0;
        let force_flip = ((tag >> TAG_FACE_SIDE_SHIFT) & 1u) != 0u
            || params.character.face_misc.z > 0.5;
        if force_flip {
            flip = !flip;
        }
        var uv_x = in.uv.x;
        if flip {
            uv_x = 1.0 - uv_x;
        }
        let sdf = textureSample(face_sdf_texture, face_sdf_sampler, vec2<f32>(uv_x, in.uv.y)).r;
        let softness = params.character.face_forward_softness.w;
        if (hero_flags & HERO_FLAG_PORTABLE_FALLBACK) != 0u {
            lit = vec3<f32>(step(threshold, sdf));
        } else {
            lit = vec3<f32>(smoothstep(threshold - softness, threshold + softness, sdf));
        }
    }
#endif

    let ambient = lights.ambient_color.rgb;
    var color = base.rgb * (ambient + light_color * lit) * ilm.r;

    // Hair 高光（cheap Blinn × ILM.G）。
    if (char_flags & CHAR_FLAG_HAIR_SPECULAR) != 0u {
        let view_direction = normalize(view.world_position - in.world_position.xyz);
        let half_vector = normalize(light_direction + view_direction);
        let specular = pow(clamp(dot(normal, half_vector), 0.0, 1.0), params.character.hair.x);
        color += specular * params.character.hair.y * ilm.g * light_color;
    }

    // Spec ramp（半角高光强度为坐标采样 ramp；Portable fallback 下关闭）。
    if (hero_flags & HERO_FLAG_SPEC_RAMP) != 0u {
        let view_direction = normalize(view.world_position - in.world_position.xyz);
        let half_vector = normalize(light_direction + view_direction);
        let spec_coord = clamp(dot(normal, half_vector), 0.0, 1.0);
        let spec = textureSample(spec_ramp_texture, spec_ramp_sampler, vec2<f32>(spec_coord, 0.5)).rgb;
        color += spec * params.hero_a.y * light_color;
    }

    // Metal matcap（view-space normal 查找 × ILM.B metal mask；fallback 关闭）。
    if (hero_flags & HERO_FLAG_MATCAP) != 0u {
        let view_normal = normalize(
            (view.view_from_world * vec4<f32>(normal, 0.0)).xyz
        );
        let matcap_uv = view_normal.xy * 0.5 + vec2<f32>(0.5);
        let matcap = textureSample(matcap_texture, matcap_sampler, matcap_uv).rgb;
        color = mix(color, color * params.hero_b.rgb + matcap * params.hero_a.x, ilm.b);
    }

    // Rim。
    if (cel_flags & 4u) != 0u {
        let view_direction = normalize(view.world_position - in.world_position.xyz);
        let fresnel = pow(clamp(1.0 - dot(normal, view_direction), 0.0, 1.0), params.character.cel.rim.x);
        color += fresnel * params.character.cel.rim.y * params.character.cel.rim_color.rgb;
    }

    // Emission（ILM.A mask）。
    if (char_flags & CHAR_FLAG_EMISSION) != 0u {
        color += params.character.emission.rgb * params.character.emission.w * ilm.a;
    }

    color *= view.exposure;

    var out: FragmentOutput;
    out.color = vec4<f32>(color, 1.0);
    return out;
}
