#define_import_path bevy_stylized_render::outline::portable_outline

// Hull Outline（Inverted Hull）Portable 编译单元（It-3b 切片，
// provisional pending ADR-0008）。
//
// 顶点沿法线在裁剪空间外扩（物理像素宽度，视口无关）+ 正面剔除
// （由材质的 specialize 设置 cull_mode = Front）。无 geometry shader /
// tessellation / compute 依赖。

#import bevy_stylized_render::common::npr_common::deform_vertex
#import bevy_pbr::{
    forward_io::{Vertex, FragmentOutput},
    mesh_functions,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
}

struct StylizedOutlineParams {
    // 线性 rgb + 宽度（物理像素，0 = 关闭）
    color_width: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> outline: StylizedOutlineParams;

struct OutlineVertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex(vertex_in: Vertex) -> OutlineVertexOutput {
    let deformed = deform_vertex(vertex_in);
    let world_position = mesh_functions::mesh_position_local_to_world(
        deformed.world_from_local,
        vec4<f32>(deformed.local_position, 1.0),
    ).xyz;
    var clip_position = position_world_to_clip(world_position);
#ifdef VERTEX_NORMALS
    let world_normal = normalize(
        (deformed.world_from_local * vec4<f32>(deformed.local_normal, 0.0)).xyz
    );
    // 顶点色 alpha 作为外扩宽度掩码（上游 outline GN 的
    // "Use Vertex Colors" 语义；抑制发卡等零厚度面片的 hull 自遮挡）。
    var vertex_width = 1.0;
#ifdef VERTEX_COLORS
    vertex_width = clamp(vertex_in.color.a, 0.0, 1.0);
#endif
    // 把世界空间法线投到裁剪空间，沿其方向做像素宽度外扩。
    let normal_clip = position_world_to_clip(world_position + world_normal);
    let clip_w = max(abs(clip_position.w), 0.0001);
    let normal_w = max(abs(normal_clip.w), 0.0001);
    let normal_ndc = normal_clip.xy / normal_w - clip_position.xy / clip_w;
    let normal_length = length(normal_ndc);
    let width_pixels = outline.color_width.w * vertex_width;
    if normal_length > 0.00001 && width_pixels > 0.0 {
        let pixel_to_ndc = vec2<f32>(2.0) / max(view.viewport.zw, vec2<f32>(1.0));
        let offset = normal_ndc / normal_length * pixel_to_ndc * width_pixels * clip_position.w;
        clip_position = vec4<f32>(clip_position.xy + offset, clip_position.zw);
    }
#endif
    var out: OutlineVertexOutput;
    out.position = clip_position;
    return out;
}

@fragment
fn fragment(in: OutlineVertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = vec4<f32>(outline.color_width.rgb * view.exposure, 1.0);
    return out;
}
