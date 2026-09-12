#define_import_path bevy_stylized_render::outline::screen_outline

// Screen-space 边缘描边（Standard+；provisional pending ADR-0008）。
//
// 屏幕覆盖三角形（vertex 直出 clip 坐标），fragment 读 depth/normal
// prepass 纹理做 8 邻域不连续检测。Transparent3d + Blend + 无深度写。

#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    prepass_utils::{prepass_depth, prepass_normal},
}

struct ScreenOutlineParams {
    // rgb = 边缘颜色, w = 强度
    color_intensity: vec4<f32>,
    // x = 深度阈值, y = 法线阈值, z = 采样半径（像素）
    thresholds: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> params: ScreenOutlineParams;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // position 直接当 clip 坐标（屏幕覆盖三角形，与相机/场景无关）。
    out.position = vec4<f32>(vertex_in.position.xy, 0.5, 1.0);
#ifdef VERTEX_POSITIONS
    out.world_position = vec4<f32>(vertex_in.position, 1.0);
#endif
#ifdef VERTEX_NORMALS
    out.world_normal = vec3<f32>(0.0, 0.0, 1.0);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex_in.uv;
#endif
    return out;
}

@fragment
fn fragment(in: VertexOutput, @builtin(sample_index) sample_index: u32) -> FragmentOutput {
#ifdef DEPTH_PREPASS
#ifdef NORMAL_PREPASS
    let center_depth = prepass_depth(in.position, sample_index);
    let center_normal = prepass_normal(in.position, sample_index);
    let radius = max(params.thresholds.z, 0.5);
    var edge = 0.0;
    let offsets = array<vec2<f32>, 8>(
        vec2<f32>(radius, 0.0),
        vec2<f32>(-radius, 0.0),
        vec2<f32>(0.0, radius),
        vec2<f32>(0.0, -radius),
        vec2<f32>(radius, radius),
        vec2<f32>(-radius, radius),
        vec2<f32>(radius, -radius),
        vec2<f32>(-radius, -radius),
    );
    for (var index = 0u; index < 8u; index += 1u) {
        let neighbor_coord = vec4<f32>(in.position.xy + offsets[index], in.position.zw);
        let depth_delta = abs(prepass_depth(neighbor_coord, sample_index) - center_depth);
        let normal_delta = 1.0 - max(dot(prepass_normal(neighbor_coord, sample_index), center_normal), 0.0);
        edge = max(edge, max(depth_delta / max(params.thresholds.x, 1e-5), normal_delta / max(params.thresholds.y, 1e-3)));
    }
    if edge < 1.0 {
        discard;
    }
    var out: FragmentOutput;
    let strength = clamp(edge * params.color_intensity.w, 0.0, 1.0);
    out.color = vec4<f32>(params.color_intensity.rgb, strength);
    return out;
#else
    discard;
#endif
#else
    discard;
#endif
}
