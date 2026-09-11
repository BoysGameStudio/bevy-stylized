#define_import_path bevy_cel::internal_edge_vertex

// The internal-edge pass draws the source surface without hull widening, so
// it shares only the deformation logic with the inverted-hull vertex pass.

#import bevy_cel::outline_shared::{OutlineVertexOutput, deform_outline_vertex}
#import bevy_pbr::{
    mesh_functions,
    forward_io::Vertex,
    view_transformations::position_world_to_clip,
}

struct InternalEdgeUniform {
    color_thresholds: vec4<f32>,
    state: vec4<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> accent: InternalEdgeUniform;

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
    let world_position = mesh_functions::mesh_position_local_to_world(
        deformed.world_from_local,
        vec4<f32>(deformed.local_position, 1.0),
    ).xyz;
    var out: OutlineVertexOutput;
    out.position = position_world_to_clip(world_position);
    out.world_normal = normalize(deformed.world_normal);
#ifdef VERTEX_UVS_A
    out.uv = deformed.uv;
#endif
    return out;
}
