#define_import_path bevy_cel::character_vertex

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::{morph_position, morph_normal, morph_tangent},
    forward_io::Vertex,
    view_transformations::position_world_to_clip,
}

struct CharacterVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif
#ifdef CHARACTER_MATERIAL_ID_ATTRIBUTE
    @location(8) material_id: f32,
#endif
#ifdef CHARACTER_HAIR_TANGENT_ATTRIBUTE
    @location(9) hair_tangent: vec3<f32>,
#endif
#ifdef CHARACTER_HAIR_MASK_ATTRIBUTE
    @location(10) hair_mask: f32,
#endif
#ifdef CHARACTER_EMISSION_ATTRIBUTE
    @location(11) emission_mask: f32,
#endif
}

#ifdef MORPH_TARGETS
fn morph_character_vertex(vertex_in: Vertex, instance_index: u32) -> Vertex {
    var vertex = vertex_in;
    let first_vertex = mesh[instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;
    let weight_count = bevy_pbr::morph::layer_count(instance_index);
    for (var index: u32 = 0u; index < weight_count; index += 1u) {
        let weight = bevy_pbr::morph::weight_at(index, instance_index);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph_position(vertex_index, index, instance_index);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph_normal(vertex_index, index, instance_index);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4<f32>(weight * morph_tangent(vertex_index, index, instance_index), 0.0);
#endif
    }
    return vertex;
}
#endif

@vertex
fn vertex(
    vertex_no_morph: Vertex,
#ifdef CEL_SKIN_EXTENDED
    @location(8) extra_joint_indices: vec4<u32>,
    @location(9) extra_joint_weights: vec4<f32>,
#endif
#ifdef CHARACTER_MATERIAL_ID_ATTRIBUTE
    @location(11) material_id: f32,
#endif
#ifdef CHARACTER_HAIR_TANGENT_ATTRIBUTE
    @location(12) hair_tangent: vec3<f32>,
#endif
#ifdef CHARACTER_HAIR_MASK_ATTRIBUTE
    @location(13) hair_mask: f32,
#endif
#ifdef CHARACTER_EMISSION_ATTRIBUTE
    @location(14) emission_mask: f32,
#endif
) -> CharacterVertexOutput {
    var out: CharacterVertexOutput;
#ifdef MORPH_TARGETS
    var vertex = morph_character_vertex(vertex_no_morph, vertex_no_morph.instance_index);
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
#else
    var world_from_local = mesh_world_from_local;
#endif
#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index,
    );
#endif
#endif
#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.position = position_world_to_clip(out.world_position.xyz);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index,
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index,
        mesh_world_from_local[3],
    );
#endif
#ifdef CHARACTER_MATERIAL_ID_ATTRIBUTE
    out.material_id = material_id;
#endif
#ifdef CHARACTER_HAIR_TANGENT_ATTRIBUTE
    out.hair_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vec4<f32>(hair_tangent, 1.0),
        vertex_no_morph.instance_index,
    ).xyz;
#endif
#ifdef CHARACTER_HAIR_MASK_ATTRIBUTE
    out.hair_mask = hair_mask;
#endif
#ifdef CHARACTER_EMISSION_ATTRIBUTE
    out.emission_mask = emission_mask;
#endif
    return out;
}
