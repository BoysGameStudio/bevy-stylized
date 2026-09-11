#define_import_path bevy_cel::outline_shared

// Deformation logic shared by the inverted-hull and internal-edge vertex
// passes so both passes follow skinning and morph targets identically.

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::{morph_position, morph_normal},
    forward_io::Vertex,
}

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

// Deformed source-mesh state shared by both outline vertex passes.
struct DeformedOutlineVertex {
    world_from_local: mat4x4<f32>,
    local_position: vec3<f32>,
    local_normal: vec3<f32>,
    world_normal: vec3<f32>,
    vertex_width: f32,
#ifdef VERTEX_UVS_A
    uv: vec2<f32>,
#endif
}

// The extra influence set is only consumed when CEL_SKIN_EXTENDED is defined;
// callers pass zeroed arguments otherwise.
fn deform_outline_vertex(
    vertex_no_morph: Vertex,
    extra_joint_indices: vec4<u32>,
    extra_joint_weights: vec4<f32>,
) -> DeformedOutlineVertex {
#ifdef MORPH_TARGETS
    let vertex = morph_outline_vertex(vertex_no_morph, vertex_no_morph.instance_index);
#else
    let vertex = vertex_no_morph;
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
    var vertex_width = 1.0;
#ifdef VERTEX_COLORS
    vertex_width = clamp(vertex.color.a, 0.0, 1.0);
#endif
    var deformed: DeformedOutlineVertex;
    deformed.world_from_local = world_from_local;
    deformed.local_position = vertex.position;
    deformed.local_normal = vertex.normal;
    deformed.world_normal = world_normal;
    deformed.vertex_width = vertex_width;
#ifdef VERTEX_UVS_A
    deformed.uv = vertex.uv;
#endif
    return deformed;
}
