#define_import_path bevy_stylized_render::character::portable_forward

// Character@Portable 前向（It-4，provisional pending ADR-0001/0009）。
//
// = Cel 基础（base color / analytic ramp / rim）+ Character 扩展：
//   - Face SDF：水平面 face basis + 阈值比较（侧选择后 UV 镜像采单张
//     SDF）；per-instance 阈值/侧选择经 MeshTag（instance 层）；
//   - Hair 高光：cheap specular（Blinn 半角 + ILM.G mask）；
//   - Emission：ILM.A × emission 颜色；
//   - Portable fallback（CHAR_FLAG_PORTABLE_FALLBACK）：face → 硬 step，
//     hair 高光 → 关闭（由 rim 补偿，材质层已换入）。
//
// 预算：3 纹理 / 3 sampler / 1 UBO（base + ILM + face SDF）。

#import bevy_stylized_render::common::npr_common::{
    NprDeformedVertex, deform_vertex, analytic_ramp, face_light_dots, face_threshold_from_dots,
    environment_fog_apply, environment_shadow_tint,
}
#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    mesh_functions,
    mesh_view_bindings::{view, lights},
    view_transformations::position_world_to_clip,
}

struct CharacterCelParams {
    base_color_factor: vec4<f32>,
    // x: 阴影阈值, y: 软过渡宽度, z: 阴影下限亮度, w: cel 标志位（bitcast u32）
    shading: vec4<f32>,
    // x: rim 幂次, y: rim 强度, z: 预留, w: alpha mask cutoff
    rim: vec4<f32>,
    rim_color: vec4<f32>,
}

struct CharacterParams {
    cel: CharacterCelParams,
    // xyz: 局部空间 face forward, w: SDF 软过渡宽度
    face_forward_softness: vec4<f32>,
    // x: 材质级 face 阈值基准, y: 位标志（bitcast u32）, z: side 强制模式,
    // w: day/night 因子（上游为材质属性，docs/reference/07 §3.1）
    face_misc: vec4<f32>,
    // x: hair 高光幂次, y: hair 高光强度
    hair: vec4<f32>,
    // rgb: emission 颜色, w: 强度
    emission: vec4<f32>,
    // palette（docs/reference/07 §3.7 已核实：sharp/soft 双档 × lit/shadow）
    palette_soft_lit: vec4<f32>,
    palette_soft_shadow: vec4<f32>,
    palette_sharp_lit: vec4<f32>,
    palette_sharp_shadow: vec4<f32>,
    // face basis 的 up（headright = normalize(cross(forward, up))）
    face_up: vec4<f32>,
}

const CEL_FLAG_ALPHA_MASK: u32 = 8u;
const CHAR_FLAG_ILM_TEXTURE: u32 = 1u;
const CHAR_FLAG_FACE_SDF_TEXTURE: u32 = 2u;
const CHAR_FLAG_HAIR_SPECULAR: u32 = 4u;
const CHAR_FLAG_EMISSION: u32 = 8u;
const CHAR_FLAG_PORTABLE_FALLBACK: u32 = 16u;

// MeshTag 位段（与 instance.rs 一致；provisional pending ADR-0004）。
const TAG_PROFILE_INDEX_SHIFT: u32 = 0u;
const TAG_FACE_THRESHOLD_SHIFT: u32 = 8u;
const TAG_FACE_SIDE_SHIFT: u32 = 16u;

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> params: CharacterParams;
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
    let cel_flags = bitcast<u32>(params.cel.shading.w);
    let flags = bitcast<u32>(params.face_misc.y);

    var base = params.cel.base_color_factor;
#ifdef VERTEX_UVS_A
    base *= textureSample(base_color_texture, base_color_sampler, in.uv);
#endif
    if (cel_flags & CEL_FLAG_ALPHA_MASK) != 0u {
        if base.a < params.cel.rim.w {
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
    let normal = normalize(in.world_normal);
    let half_lambert = clamp(dot(normal, light_direction) * 0.5 + 0.5, 0.0, 1.0);

    // ILM decode once（docs/reference/07 §3.2 已核实布局）：
    // R=metal mask(>0.9) / G=AO / B=高光抑制(1.03−B) / A=材质分带 ID。
    // AO 通道带 readability floor（provisional pending ADR-0009）。
    var ao = 1.0;
    var spec_suppress = 1.03;
    var band_id = 0.0;
#ifdef VERTEX_UVS_A
    if (flags & CHAR_FLAG_ILM_TEXTURE) != 0u {
        let packed = textureSample(ilm_texture, ilm_sampler, in.uv);
        ao = mix(0.5, 1.0, packed.g);
        spec_suppress = 1.03 - packed.b;
        band_id = packed.a;
    }
#endif

    // LitFactor：half-lambert 经 AO 门控（docs/reference/07 §3.1 已核实：
    // lerp(0, half_lambert, smoothstep(0.01, 0.4, ao))）。
    var lit_factor = half_lambert * smoothstep(0.01, 0.4, ao);

    // Face SDF：有纹理时替换 cel 阴影；per-instance 阈值/侧选择经 MeshTag。
#ifdef VERTEX_UVS_A
    if (flags & CHAR_FLAG_FACE_SDF_TEXTURE) != 0u {
        var tag: u32 = 0u;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
        tag = mesh_functions::get_tag(in.instance_index);
#endif
        // v1 近似：face forward 只随 model matrix 变换（不随蒙皮）；
        // 逐骨骼 face rig 属后续迭代（rendering/02 注记）。
        let face_forward_world = normalize(
            (mesh_functions::get_world_from_local(in.instance_index)
                * vec4<f32>(params.face_forward_softness.xyz, 0.0)).xyz
        );
        let face_up_world = normalize(
            (mesh_functions::get_world_from_local(in.instance_index)
                * vec4<f32>(params.face_up.xyz, 0.0)).xyz
        );
        let dots = face_light_dots(light_direction, face_forward_world, face_up_world);
        // 阈值：材质基准；MeshTag 携带时逐实例覆盖（量化 1..=255 → 0..=1）。
        var threshold = face_threshold_from_dots(dots.x);
        let tag_threshold = (tag >> TAG_FACE_THRESHOLD_SHIFT) & 0xFFu;
        if tag_threshold != 0u {
            threshold = f32(tag_threshold - 1u) / 254.0;
        }
        // 侧选择：自动按光源方向；bit16 / face_misc.z 强制翻转。
        var flip = dots.y < 0.0;
        let force_flip = ((tag >> TAG_FACE_SIDE_SHIFT) & 1u) != 0u || params.face_misc.z > 0.5;
        if force_flip {
            flip = !flip;
        }
        var uv_x = in.uv.x;
        if flip {
            uv_x = 1.0 - uv_x;
        }
        let sdf = textureSample(face_sdf_texture, face_sdf_sampler, vec2<f32>(uv_x, in.uv.y)).r;
        let softness = params.face_forward_softness.w;
        if (flags & CHAR_FLAG_PORTABLE_FALLBACK) != 0u {
            // Portable fallback：硬 step（少一次 smoothstep 精度开销）。
            lit_factor = step(threshold, sdf);
        } else {
            lit_factor = smoothstep(threshold - softness, threshold + softness, sdf);
        }
    }
#endif

    // 双档光照（docs/reference/07 §3.7 已核实：sharp/soft 串联相乘 +
    // ambient，全 MULTIPLY；palette 为材质静态 authored 数据）。
    // 阴影色经环境阴影染色（V2-d；强度 0 时逐位恒等）。
    let soft_mix = mix(
        environment_shadow_tint(params.palette_soft_shadow.rgb),
        params.palette_soft_lit.rgb,
        lit_factor,
    );
    let sharp_mix = select(
        environment_shadow_tint(params.palette_sharp_shadow.rgb),
        params.palette_sharp_lit.rgb,
        lit_factor > 0.5,
    );
    let ambient = lights.ambient_color.rgb;
    var color = base.rgb * soft_mix * sharp_mix * max(ambient, vec3<f32>(0.05));

    // Hair 高光（cheap Blinn 半角 × ILM.B 高光抑制；Portable fallback 下
    // 由 rim 补偿）。
    if (flags & CHAR_FLAG_HAIR_SPECULAR) != 0u {
        let view_direction = normalize(view.world_position - in.world_position.xyz);
        let half_vector = normalize(light_direction + view_direction);
        let specular = pow(clamp(dot(normal, half_vector), 0.0, 1.0), params.hair.x);
        color += specular * params.hair.y * spec_suppress * light_color;
    }

    // Rim。
    if (cel_flags & 4u) != 0u {
        let view_direction = normalize(view.world_position - in.world_position.xyz);
        let fresnel = pow(clamp(1.0 - dot(normal, view_direction), 0.0, 1.0), params.cel.rim.x);
        color += fresnel * params.cel.rim.y * params.cel.rim_color.rgb;
    }

    // Emission（mask = base color alpha；上游经 alpha switch 路由
    // emission/transparency——无 ILM emission 通道，此决策 provisional
    // pending ADR-0023）。
    if (flags & CHAR_FLAG_EMISSION) != 0u {
        color += params.emission.rgb * params.emission.w * base.a;
    }

    // 环境 NPR（V2-d）：距离雾（无 DistanceFog 组件时逐位恒等）。
    color = environment_fog_apply(color, in.world_position.xyz, view.world_position);

    color *= view.exposure;

    var out: FragmentOutput;
    // 与 cel 一致：输出 alpha 恒为 1（控制数据不得泄漏到帧缓冲 alpha）。
    out.color = vec4<f32>(color, 1.0);
    return out;
}
