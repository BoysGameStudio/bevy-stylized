#import bevy_pbr::{
    forward_io::FragmentOutput,
    prepass_utils::{prepass_depth, prepass_normal},
}
#import bevy_cel::outline_shared::OutlineVertexOutput

struct InternalEdgeUniform {
    color_thresholds: vec4<f32>,
    state: vec4<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> accent: InternalEdgeUniform;

@fragment
fn fragment(in: OutlineVertexOutput, @builtin(sample_index) sample_index: u32) -> FragmentOutput {
#ifdef DEPTH_PREPASS
#ifdef NORMAL_PREPASS
    if accent.state.x == 0u {
        discard;
    }
    let center_depth = prepass_depth(in.position, sample_index);
    let center_normal = prepass_normal(in.position, sample_index);
    var edge = 0.0;
    let offsets = array<vec2<f32>, 8>(
        vec2<f32>(2.0, 0.0),
        vec2<f32>(-2.0, 0.0),
        vec2<f32>(0.0, 2.0),
        vec2<f32>(0.0, -2.0),
        vec2<f32>(1.5, 1.5),
        vec2<f32>(-1.5, 1.5),
        vec2<f32>(1.5, -1.5),
        vec2<f32>(-1.5, -1.5),
    );
    for (var index = 0u; index < 8u; index += 1u) {
        let neighbor_coord = vec4<f32>(in.position.xy + offsets[index], in.position.zw);
        let depth_delta = abs(prepass_depth(neighbor_coord, sample_index) - center_depth);
        let normal_delta = 1.0 - max(dot(prepass_normal(neighbor_coord, sample_index), center_normal), 0.0);
        edge = max(edge, max(depth_delta * 64.0, normal_delta));
    }
    if edge < accent.color_thresholds.w {
        discard;
    }
    var out: FragmentOutput;
    out.color = vec4<f32>(accent.color_thresholds.rgb, clamp(edge, 0.0, 1.0));
    return out;
#else
    discard;
#endif
#else
    discard;
#endif
}
