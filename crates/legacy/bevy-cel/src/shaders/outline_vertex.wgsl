#define_import_path bevy_cel::outline_vertex

#import bevy_cel::outline_shared::{OutlineVertexOutput, deform_outline_vertex}
#import bevy_pbr::{
    mesh_functions,
    forward_io::Vertex,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
}

struct OutlineUniform {
    color_width: vec4<f32>,
    masks: vec4<f32>,
    source_color_one: vec4<f32>,
    source_color_two: vec4<f32>,
    source_color_three: vec4<f32>,
    source_color_four: vec4<f32>,
    source_color_five: vec4<f32>,
    source_modulation: vec4<f32>,
    source_params: vec4<f32>,
    source_state: vec4<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> outline: OutlineUniform;

@vertex
fn vertex(vertex_no_morph: Vertex,
#ifdef CEL_SKIN_EXTENDED
    @location(8) extra_joint_indices: vec4<u32>,
    @location(9) extra_joint_weights: vec4<f32>,
#endif
) -> OutlineVertexOutput {
#ifdef CEL_SKIN_EXTENDED
    let deformed = deform_outline_vertex(vertex_no_morph, extra_joint_indices, extra_joint_weights);
#else
    let deformed = deform_outline_vertex(vertex_no_morph, vec4<u32>(0u), vec4<f32>(0.0));
#endif
    var local_position = deformed.local_position;
    if outline.source_state.x != 0u {
        local_position += normalize(deformed.local_normal)
            * max(outline.source_params.x, 0.0)
            * deformed.vertex_width;
    }
    let world_position = mesh_functions::mesh_position_local_to_world(
        deformed.world_from_local,
        vec4<f32>(local_position, 1.0),
    ).xyz;
    var clip_position = position_world_to_clip(world_position);
    let normal_clip = position_world_to_clip(world_position + deformed.world_normal);
    let normal_ndc = normal_clip.xy / max(abs(normal_clip.w), 0.0001)
        - clip_position.xy / max(abs(clip_position.w), 0.0001);
    let normal_length = length(normal_ndc);
    let suppression = clamp(max(outline.masks.x, outline.masks.y), 0.0, 1.0);
    let width_pixels = clamp(outline.color_width.w, 0.0, 24.0) * (1.0 - suppression);
    if outline.source_state.x == 0u && normal_length > 0.00001 && width_pixels > 0.0 {
        let pixel_to_ndc = vec2<f32>(2.0) / max(view.viewport.zw, vec2<f32>(1.0));
        let offset = normal_ndc / normal_length * pixel_to_ndc * width_pixels * clip_position.w;
        clip_position = vec4<f32>(clip_position.xy + offset, clip_position.zw);
    }
    var out: OutlineVertexOutput;
    out.position = clip_position;
    out.world_normal = normalize(deformed.world_normal);
#ifdef VERTEX_UVS_A
    out.uv = deformed.uv;
#endif
    return out;
}
