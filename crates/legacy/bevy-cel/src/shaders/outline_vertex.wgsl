#define_import_path bevy_cel::outline_vertex

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::{morph_position, morph_normal},
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

struct OutlineVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(1) uv: vec2<f32>,
#endif
}

#ifdef MORPH_TARGETS
fn morph_outline_vertex(vertex_in: Vertex, instance_index: u32) -> Vertex {
    var vertex = vertex_in;
    let vertex_index = vertex.index - mesh[instance_index].first_vertex_index;
    let layer_count = bevy_pbr::morph::layer_count(instance_index);
    for (var index: u32 = 0u; index < layer_count; index += 1u) {
        let weight = bevy_pbr::morph::weight_at(index, instance_index);
        vertex.position += weight * morph_position(vertex_index, index, instance_index);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph_normal(vertex_index, index, instance_index);
#endif
    }
    return vertex;
}
#endif

@vertex
fn vertex(vertex_no_morph: Vertex,
#ifdef CEL_SKIN_EXTENDED
    @location(8) extra_joint_indices: vec4<u32>,
    @location(9) extra_joint_weights: vec4<f32>,
#endif
) -> OutlineVertexOutput {
#ifdef MORPH_TARGETS
    var vertex = morph_outline_vertex(vertex_no_morph, vertex_no_morph.instance_index);
#else
    var vertex = vertex_no_morph;
#endif
    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
#ifdef SKINNED
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index,
    );
#ifdef CEL_SKIN_EXTENDED
    world_from_local += skinning::skin_model(extra_joint_indices, extra_joint_weights, vertex_no_morph.instance_index);
#endif
    let world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    let world_from_local = mesh_world_from_local;
    let world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index,
    );
#endif
    var local_position = vertex.position;
    if outline.source_state.x != 0u {
        var vertex_width = 1.0;
#ifdef VERTEX_COLORS
        vertex_width = clamp(vertex.color.a, 0.0, 1.0);
#endif
        local_position += normalize(vertex.normal)
            * max(outline.source_params.x, 0.0)
            * vertex_width;
    }
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(local_position, 1.0),
    ).xyz;
    var clip_position = position_world_to_clip(world_position);
    let normal_clip = position_world_to_clip(world_position + world_normal);
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
    out.world_normal = normalize(world_normal);
#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
    return out;
}
