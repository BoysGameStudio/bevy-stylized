#define_import_path bevy_stylized_render::common::npr_common

// 共享模块（docs/rendering/03）：Cel/Character 编译单元共用的顶点形变与
// 纯函数工具。无 binding、无材质参数——只有 mesh/view 层输入。

#import bevy_pbr::{
    forward_io::Vertex,
    mesh_bindings::mesh,
    mesh_functions,
    morph::{morph_position, morph_normal},
    skinning,
}

// 形变后的顶点状态（蒙皮/形态统一在此处理）。
struct NprDeformedVertex {
    world_from_local: mat4x4<f32>,
    local_position: vec3<f32>,
    local_normal: vec3<f32>,
    instance_index: u32,
}

fn deform_vertex(vertex_in: Vertex) -> NprDeformedVertex {
    var vertex = vertex_in;
#ifdef MORPH_TARGETS
    // 形态目标：权重 × 位置/法线 delta（legacy 验证过的通用形态）。
    let first_vertex = mesh[vertex_in.instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;
    let layer_count = bevy_pbr::morph::layer_count(vertex_in.instance_index);
    for (var index: u32 = 0u; index < layer_count; index += 1u) {
        let weight = bevy_pbr::morph::weight_at(index, vertex_in.instance_index);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph_position(vertex_index, index, vertex_in.instance_index);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph_normal(vertex_index, index, vertex_in.instance_index);
#endif
    }
#endif
    var out: NprDeformedVertex;
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
#ifdef SKINNED
    out.world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_in.instance_index,
    );
#else
    out.world_from_local = world_from_local;
#endif
#ifdef VERTEX_POSITIONS
    out.local_position = vertex.position;
#else
    out.local_position = vec3<f32>(0.0);
#endif
#ifdef VERTEX_NORMALS
    out.local_normal = vertex.normal;
#else
    out.local_normal = vec3<f32>(0.0, 1.0, 0.0);
#endif
    out.instance_index = vertex_in.instance_index;
    return out;
}

// 解析 ramp fallback：smoothstep 阈值化 half-lambert。
fn analytic_ramp(half_lambert: f32, threshold: f32, softness: f32, floor_: f32) -> f32 {
    let t = smoothstep(threshold - softness, threshold + softness, half_lambert);
    return mix(floor_, 1.0, t);
}

// Face SDF 通用光位计算（v1 假定角色直立，face basis 取水平面 XZ 投影）。
// 返回 (forward_dot, right_dot)；light 退化（近似垂直）时返回 (1, 0)。
fn face_light_dots(direction_to_light: vec3<f32>, face_forward_world: vec3<f32>) -> vec2<f32> {
    let light_xz = direction_to_light.xz;
    let light_len_sq = dot(light_xz, light_xz);
    if light_len_sq <= 1e-8 {
        return vec2<f32>(1.0, 0.0);
    }
    let light_dir = light_xz * inverseSqrt(light_len_sq);
    let forward_xz = face_forward_world.xz;
    let forward_len_sq = dot(forward_xz, forward_xz);
    if forward_len_sq <= 1e-8 {
        return vec2<f32>(1.0, 0.0);
    }
    let forward = forward_xz * inverseSqrt(forward_len_sq);
    // right = forward 在水平面上逆时针 90°（Y-up 右手系）。
    let right = vec2<f32>(forward.y, -forward.x);
    return vec2<f32>(
        clamp(dot(forward, light_dir), -1.0, 1.0),
        clamp(dot(right, light_dir), -1.0, 1.0),
    );
}

// 经典 face SDF 阈值：acos(forward_dot)/PI（0 = 正前方受光，1 = 背光）。
fn face_threshold_from_dots(forward_dot: f32) -> f32 {
    return clamp(acos(forward_dot) / 3.14159265, 0.0, 1.0);
}
