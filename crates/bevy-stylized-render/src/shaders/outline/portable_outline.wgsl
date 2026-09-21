#import bevy_stylized_render::common::npr_common::{deformed_world_normal, sample_cubic}
#define_import_path bevy_stylized_render::outline::portable_outline

// Hull Outline（Inverted Hull）Portable 编译单元（It-3b 切片，
// provisional pending ADR-0008）。
//
// 顶点沿法线在裁剪空间外扩（物理像素宽度，视口无关）+ 正面剔除
// （由材质的 specialize 设置 cull_mode = Front）。无 geometry shader /
// tessellation / compute 依赖。

#import bevy_stylized_render::common::npr_common::deform_vertex
#import bevy_stylized_render::common::npr_common::add_secondary_skin
#import bevy_pbr::{
    forward_io::{Vertex, FragmentOutput},
    mesh_functions,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
}

struct StylizedOutlineParams {
    // 线性 rgb + 宽度（物理像素，0 = 关闭）
    color_width: vec4<f32>,
    source_colors: array<vec4<f32>,5>,
    source_params: vec4<f32>,
    source_state: vec4<u32>,
}

#ifdef NPR_HOST_INPUTS
#import bevy_stylized_render::common::host_inputs::{HostInstance, HostView}
struct HostUniform { material: StylizedOutlineParams, instances: array<HostInstance,128>, views: array<HostView,16>, }
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> host_uniform: HostUniform;
fn material_params() -> StylizedOutlineParams { return host_uniform.material; }
fn host_instance(index:u32) -> HostInstance { return host_uniform.instances[mesh_functions::get_tag(index) & 0xFFFFFFu]; }
#else
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> outline: StylizedOutlineParams;
fn material_params() -> StylizedOutlineParams { return outline; }
#endif

fn instance_width(index:u32) -> f32 {
#ifdef NPR_HOST_INPUTS
    return host_instance(index).face_up.w;
#else
    // Replica-only tag, independent of a source StorageTable index. Absence is
    // exactly unit width, preserving authored shell coordinates bit-for-bit.
    let tag=mesh_functions::get_tag(index);
    if (tag & 0x80000000u)!=0u {return f32(tag & 127u)/63.5;}
    return 1.0;
#endif
}

@group(#{MATERIAL_BIND_GROUP}) @binding(1) var class_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var class_sampler: sampler;

struct OutlineVertexOutput {
    @builtin(position) position: vec4<f32>,
#ifdef VERTEX_UVS_A
    @location(1) uv:vec2<f32>,
#endif
#ifdef NPR_HOST_INPUTS
    @location(0) @interpolate(flat) instance_index:u32,
#endif
}

@vertex
fn vertex(vertex_in: Vertex,
#ifdef NPR_OUTLINE_SOURCE_POSITION
    @location(10) source_position: vec3<f32>,
#endif
#ifdef NPR_SKIN_EXTENDED
    @location(8) extra_joint_indices: vec4<u32>,
    @location(9) extra_joint_weights: vec4<f32>,
#endif
) -> OutlineVertexOutput {
    var deformed = deform_vertex(vertex_in);
#ifdef NPR_SKIN_EXTENDED
    deformed = add_secondary_skin(deformed, extra_joint_indices, extra_joint_weights);
#endif
    var local_position=deformed.local_position;
#ifdef VERTEX_NORMALS
    if material_params().source_state.x != 0u {
        var vertex_width=1.0;
#ifdef VERTEX_COLORS
        vertex_width=clamp(vertex_in.color.a,0.0,1.0);
#endif
        var local_width=max(material_params().source_params.x,0.0);
        let host_width=instance_width(vertex_in.instance_index);
        if host_width!=1.0 {local_width*=host_width;}
        // Preserve the authored extrusion's multiplication order at the
        // default instance width; tiny depth changes matter on touching shells.
        if material_params().source_params.z>0.5 {
#ifdef NPR_OUTLINE_SOURCE_POSITION
            local_position=source_position;
            let host_scale=instance_width(vertex_in.instance_index);
            if host_scale!=1.0 {local_position=mix(deformed.local_position,source_position,host_scale);}
#endif
        } else {
            local_position += normalize(deformed.local_normal)*local_width*vertex_width;
        }
    }
#endif
    let world_position = mesh_functions::mesh_position_local_to_world(
        deformed.world_from_local,
        vec4<f32>(local_position, 1.0),
    ).xyz;
    var clip_position = position_world_to_clip(world_position);
#ifdef VERTEX_NORMALS
    let world_normal = deformed_world_normal(deformed);
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
    var width_pixels = material_params().color_width.w * vertex_width;
    width_pixels *= instance_width(vertex_in.instance_index);
    if material_params().source_state.x == 0u && normal_length > 0.00001 && width_pixels > 0.0 {
        let pixel_to_ndc = vec2<f32>(2.0) / max(view.viewport.zw, vec2<f32>(1.0));
        let offset = normal_ndc / normal_length * pixel_to_ndc * width_pixels * clip_position.w;
        clip_position = vec4<f32>(clip_position.xy + offset, clip_position.zw);
    }
#endif
    var out: OutlineVertexOutput;
    out.position = clip_position;
#ifdef VERTEX_UVS_A
    out.uv=vertex_in.uv;
#endif
#ifdef NPR_HOST_INPUTS
    out.instance_index = vertex_in.instance_index;
#endif
    return out;
}

@fragment
fn fragment(in: OutlineVertexOutput) -> FragmentOutput {
    let params=material_params();
    var color=params.color_width.rgb;
    if params.source_state.x != 0u {
        var selected=0u;
#ifdef VERTEX_UVS_A
        if params.source_state.z==0u && params.source_state.w!=0u {
            var alpha=textureSampleLevel(class_texture,class_sampler,in.uv,0.0).a;
            if (params.source_state.w & 2u)!=0u { alpha=sample_cubic(class_texture,class_sampler,in.uv).a; }
            // Authored class order from low to high alpha: 1,4,3,5,2.
            if alpha>0.8 { selected=1u; }
            else if alpha>0.6 { selected=4u; }
            else if alpha>0.4 { selected=2u; }
            else if alpha>0.2 { selected=3u; }
        }
#endif
        if selected>0u && (params.source_state.y & (1u<<(selected-1u)))==0u { selected=0u; }
        color=pow(max(params.source_colors[selected].rgb,vec3<f32>(0.0)),vec3<f32>(params.source_params.y));
    }
    var opacity=1.0;
#ifdef NPR_HOST_INPUTS
    let input = host_instance(in.instance_index);
    let frame = host_uniform.views[input.indices.y];
    if params.source_state.x!=0u { color*=frame.ambient.rgb*frame.sharp_lit.rgb; }
    color*=frame.controls.y;
    opacity=input.tint.a;
#else
    color*=view.exposure;
#endif
    var out:FragmentOutput;
    out.color=vec4<f32>(color,opacity);
    return out;
}
