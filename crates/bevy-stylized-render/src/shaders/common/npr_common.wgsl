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

// Face SDF 光位计算：head basis（headright = normalize(cross(forward, up))）
// + 归一化光向量的点积（docs/reference/07 §3.3 已核实：上游用头部
// forward/up 场景标记传递，3D 基向量直接点积，非平面投影）。
// 返回 (forward_dot, right_dot)；输入退化时返回 (1, 0)。
fn face_light_dots(
    direction_to_light: vec3<f32>,
    face_forward_world: vec3<f32>,
    face_up_world: vec3<f32>,
) -> vec2<f32> {
    let light_len_sq = dot(direction_to_light, direction_to_light);
    if light_len_sq <= 1e-8 {
        return vec2<f32>(1.0, 0.0);
    }
    let light = direction_to_light * inverseSqrt(light_len_sq);
    let forward_len_sq = dot(face_forward_world, face_forward_world);
    if forward_len_sq <= 1e-8 {
        return vec2<f32>(1.0, 0.0);
    }
    let forward = face_forward_world * inverseSqrt(forward_len_sq);
    let up = normalize(face_up_world);
    let right = normalize(cross(forward, up));
    return vec2<f32>(
        clamp(dot(forward, light), -1.0, 1.0),
        clamp(dot(right, light), -1.0, 1.0),
    );
}

// 经典 face SDF 阈值：acos(forward_dot)/PI（0 = 正前方受光，1 = 背光）。
fn face_threshold_from_dots(forward_dot: f32) -> f32 {
    return clamp(acos(forward_dot) / 3.14159265, 0.0, 1.0);
}

// ---------------------------------------------------------------------------
// 环境 NPR（V2-d，provisional pending ADR-0009）
//
// 数据通道：Bevy `DistanceFog` uniform（view bind group binding 13，仅当
// view 挂 DistanceFog 组件时存在，故全部读取必须 `#ifdef DISTANCE_FOG`）。
// 字段语义（本框架约定，非 Bevy 原生散射语义）：
//   base_color               = 雾色（rgb）+ 整体强度（a）
//   be.x / be.y              = linear 雾 start / end
//   directional_light_color  = 风格化阴影染色（rgb）+ 强度（a）
// 不新增任何 binding/纹理：Portable 合法、零材质资产写。
// ---------------------------------------------------------------------------

#ifdef DISTANCE_FOG
#import bevy_pbr::mesh_view_bindings::fog
#endif

// 线性雾因子（与 core `stylized_fog_factor` 逐字一致的镜像）。
fn stylized_fog_factor(distance: f32, start: f32, end: f32) -> f32 {
    if end <= start {
        return 0.0; // 非法参数 fail-closed（无雾）
    }
    return clamp((distance - start) / (end - start), 0.0, 1.0);
}

// 距离雾：颜色向雾色混合（无 DistanceFog 组件或强度 0 时逐位恒等）。
fn environment_fog_apply(color: vec3<f32>, world_position: vec3<f32>, camera_position: vec3<f32>) -> vec3<f32> {
#ifdef DISTANCE_FOG
    if fog.mode == 1u { // FOG_MODE_LINEAR
        let alpha = clamp(fog.base_color.a, 0.0, 1.0);
        if alpha > 0.0 {
            let dist = distance(world_position, camera_position);
            let factor = stylized_fog_factor(dist, fog.be.x, fog.be.y) * alpha;
            if factor > 0.0 {
                return mix(color, fog.base_color.rgb, factor);
            }
        }
    }
#endif
    return color;
}

// 阴影染色强度（调用方据此 skip，保证逐位恒等）。
fn environment_shadow_tint_strength() -> f32 {
#ifdef DISTANCE_FOG
    return clamp(fog.directional_light_color.a, 0.0, 1.0);
#else
    return 0.0;
#endif
}

// 风格化阴影染色：乘法 tint（strength 0 → 逐位恒等；无组件时恒等）。
fn environment_shadow_tint(shadow_color: vec3<f32>) -> vec3<f32> {
#ifdef DISTANCE_FOG
    let strength = clamp(fog.directional_light_color.a, 0.0, 1.0);
    if strength > 0.0 {
        return shadow_color * mix(vec3<f32>(1.0), fog.directional_light_color.rgb, strength);
    }
#endif
    return shadow_color;
}
