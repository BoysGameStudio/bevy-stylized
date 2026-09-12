#define_import_path bevy_stylized_render::cel::portable_forward

// Cel@Portable 前向主材质（It-3b 切片，provisional pending ADR-0001）。
//
// Portable 约束：不依赖 bindless/FP16/subgroup/compute/HDR；无 storage
// buffer；固定 binding（1 UBO + 3 texture + 3 sampler）。
//
// 分层数据流（docs/rendering/02）：
//   Material 层 → 本文件的 params/texture binding（资产级、帧间不变）
//   View 层     → bevy_pbr::mesh_view_bindings（key light 经
//                 DirectionalLight，ambient 经 AmbientLight，exposure 经
//                 相机 Exposure）
//   Instance 层 → mesh_functions::get_tag（MeshTag → ramp 行索引，
//                 provisional ADR-0004 传输方案 A）

#import bevy_stylized_render::common::npr_common::{deform_vertex, analytic_ramp}
#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    mesh_functions,
    mesh_view_bindings::{view, lights},
    view_transformations::position_world_to_clip,
}

struct StylizedCelParams {
    base_color_factor: vec4<f32>,
    // x: 阴影阈值, y: 软过渡宽度, z: 阴影下限亮度, w: 标志位（bitcast u32）
    shading: vec4<f32>,
    // x: rim 幂次, y: rim 强度, z: ramp 纹理行数, w: alpha mask cutoff
    rim: vec4<f32>,
    rim_color: vec4<f32>,
}

const FLAG_RAMP_TEXTURE: u32 = 1u;
const FLAG_ILM_TEXTURE: u32 = 2u;
const FLAG_RIM: u32 = 4u;
const FLAG_ALPHA_MASK: u32 = 8u;
// MeshTag 低 8 位作为 ramp 行索引（provisional ADR-0004 方案 A）；
// 行数由 params.rim.z 提供。

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> params: StylizedCelParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var base_color_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var ramp_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var ramp_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5)
var ilm_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6)
var ilm_sampler: sampler;

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
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        deformed.instance_index,
        deformed.world_from_local[3],
    );
#endif
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    let flags = bitcast<u32>(params.shading.w);

    var base = params.base_color_factor;
#ifdef VERTEX_UVS_A
    base *= textureSample(base_color_texture, base_color_sampler, in.uv);
#endif
    // Alpha mask（gacha 资产的 diffuse alpha 是控制数据；Cel 切片按
    // cutoff discard，完整 alpha 矩阵 pending ADR-0023）。
    if (flags & FLAG_ALPHA_MASK) != 0u {
        if base.a < params.rim.w {
            discard;
        }
    }

    // View 层：NPR key light（经 DirectionalLight 同步）。
    var light_direction = vec3<f32>(0.0, 1.0, 0.0);
    var light_color = vec3<f32>(1.0);
    if lights.n_directional_lights > 0u {
        light_direction = lights.directional_lights[0].direction_to_light;
        light_color = lights.directional_lights[0].color.rgb;
    }

    let normal = normalize(in.world_normal);
    let half_lambert = clamp(dot(normal, light_direction) * 0.5 + 0.5, 0.0, 1.0);

    // Material 层：ILM G 通道 AO（decode once；布局 docs/reference/07
    // §3.2 已核实：R=metal/G=AO/B=高光抑制/A=材质分带 ID）。
    var ao = 1.0;
#ifdef VERTEX_UVS_A
    if (flags & FLAG_ILM_TEXTURE) != 0u {
        ao = textureSample(ilm_texture, ilm_sampler, in.uv).g;
    }
#endif

    // Ramp：纹理路径按 MeshTag 选择行；缺失时解析 ramp。
    var lit = vec3<f32>(analytic_ramp(
        half_lambert,
        params.shading.x,
        params.shading.y,
        params.shading.z,
    ));
    if (flags & FLAG_RAMP_TEXTURE) != 0u {
        var row = 0.0;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
        row = f32(mesh_functions::get_tag(in.instance_index) & 0xFFu);
#endif
        let ramp_v = (row + 0.5) / max(params.rim.z, 1.0);
        lit = textureSample(ramp_texture, ramp_sampler, vec2<f32>(half_lambert, ramp_v)).rgb;
    }

    // 环境项（View 层 AmbientLight）。
    let ambient = lights.ambient_color.rgb;
    var color = base.rgb * (ambient + light_color * lit) * ao;

    // Rim（材质静态参数控制强度；默认关闭）。
    if (flags & FLAG_RIM) != 0u {
        let view_direction = normalize(view.world_position - in.world_position.xyz);
        let fresnel = pow(clamp(1.0 - dot(normal, view_direction), 0.0, 1.0), params.rim.x);
        color += fresnel * params.rim.y * params.rim_color.rgb;
    }

    // View 层曝光。
    color *= view.exposure;

    var out: FragmentOutput;
    // Cel@Portable 只输出 Opaque/Mask：diffuse alpha 在 gacha 资产里是
    // 控制数据，不得泄漏到帧缓冲 alpha（会把捕获/合成结果染黑）。
    out.color = vec4<f32>(color, 1.0);
    return out;
}
