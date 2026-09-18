#define_import_path bevy_stylized_render::physical::physical_forward
#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    mesh_functions,
    mesh_bindings::mesh,
    morph::morph_tangent,
    view_transformations::position_world_to_clip,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#import bevy_stylized_render::common::npr_common::{
    deform_vertex, add_secondary_skin, deformed_world_normal,
}

struct PhysicalCelParams {
    transform: vec4<f32>,
    atlas: vec4<u32>,
    controls: vec4<f32>,
    rim: vec4<f32>,
    rim_color: vec4<f32>,
}
#ifdef NPR_HOST_INPUTS
#import bevy_stylized_render::common::host_inputs::{HostInstance, HostView}
struct PhysicalHostUniform {
    material: PhysicalCelParams,
    instances: array<HostInstance,128>,
    views: array<HostView,16>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> physical_host: PhysicalHostUniform;
#else
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> physical: PhysicalCelParams;
#endif
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var ramp_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var ramp_sampler: sampler;

@vertex
fn vertex(vertex_in: Vertex,
#ifdef NPR_SKIN_EXTENDED
    @location(8) extra_joint_indices: vec4<u32>,
    @location(9) extra_joint_weights: vec4<f32>,
#endif
) -> VertexOutput {
    var deformed=deform_vertex(vertex_in);
#ifdef NPR_SKIN_EXTENDED
    deformed=add_secondary_skin(deformed,extra_joint_indices,extra_joint_weights);
#endif
    var out: VertexOutput;
    out.world_position=mesh_functions::mesh_position_local_to_world(deformed.world_from_local,vec4<f32>(deformed.local_position,1.0));
    out.position=position_world_to_clip(out.world_position.xyz);
#ifdef VERTEX_NORMALS
    out.world_normal=deformed_world_normal(deformed);
#endif
#ifdef VERTEX_UVS_A
    out.uv=vertex_in.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b=vertex_in.uv_b;
#endif
#ifdef VERTEX_COLORS
    out.color=vertex_in.color;
#endif
#ifdef VERTEX_TANGENTS
    var tangent=vertex_in.tangent;
#ifdef MORPH_TARGETS
    let vertex_index=vertex_in.index-mesh[vertex_in.instance_index].first_vertex_index;
    for(var i=0u;i<bevy_pbr::morph::layer_count(vertex_in.instance_index);i+=1u) {
        let weight=bevy_pbr::morph::weight_at(i,vertex_in.instance_index);
        if weight!=0.0 {tangent+=vec4<f32>(weight*morph_tangent(vertex_index,i,vertex_in.instance_index),0.0);}
    }
#endif
    out.world_tangent=mesh_functions::mesh_tangent_local_to_world(deformed.world_from_local,tangent,vertex_in.instance_index);
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index=vertex_in.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    let world_from_local=mesh_functions::get_world_from_local(vertex_in.instance_index);
    out.visibility_range_dither=mesh_functions::get_visibility_range_dither_level(vertex_in.instance_index,world_from_local[3]);
#endif
    return out;
}

fn luminance(color:vec3<f32>)->f32 {return dot(color,vec3<f32>(0.2126,0.7152,0.0722));}
fn ramp_row(coordinate:f32,row:u32,params:PhysicalCelParams)->vec3<f32> {
    let count=max(params.atlas.z,1u);
    let scaled=min(coordinate,0.999999)*f32(count);
    let band=min(u32(floor(scaled)),count-1u);
    let next=min(band+1u,count-1u);
    let y=(f32(row)+0.5)/f32(max(params.atlas.y,1u));
    let a=textureSampleLevel(ramp_texture,ramp_sampler,vec2<f32>((f32(band)+0.5)/f32(count),y),0.0).rgb;
    let b=textureSampleLevel(ramp_texture,ramp_sampler,vec2<f32>((f32(next)+0.5)/f32(count),y),0.0).rgb;
    let smoothing=clamp(params.transform.z,0.0,0.5);
    if smoothing==0.0 {return a;}
    return mix(a,b,smoothstep(1.0-smoothing,1.0,fract(scaled)));
}

@fragment
fn fragment(in:VertexOutput,@builtin(front_facing) front:bool)->FragmentOutput {
#ifdef NPR_HOST_INPUTS
    var params=physical_host.material;
    let host=physical_host.instances[mesh_functions::get_tag(in.instance_index)&0xFFFFFFu];
    let host_view=physical_host.views[host.indices.y];
    params.controls.x=host_view.controls.x;
    params.rim_color=vec4<f32>(host.rim.rgb,1.0);
    params.rim.w=host.rim.w;
    params.controls.y=select(0.0,1.0,host.rim.w>0.0);
#else
    let params=physical;
#endif
    var input=pbr_input_from_standard_material(in,front);
#ifdef NPR_HOST_INPUTS
    input.material.base_color*=host.tint;
    input.material.emissive+=vec4<f32>(host.emission.rgb*host.emission.w,0.0);
#endif
    let authored_alpha=input.material.base_color.a;
    input.material.base_color=alpha_discard(input.material,input.material.base_color);
#ifdef NPR_HOST_BLEND
    input.material.base_color.a=authored_alpha;
#endif
    var out:FragmentOutput;
    if (input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT)!=0u {
        out.color=input.material.base_color;
    } else {
        out.color=apply_pbr_lighting(input);
        if params.transform.w>0.5 {
            let emissive=input.material.emissive.rgb*input.material.base_color.a;
            let lit=max(out.color.rgb-emissive,vec3<f32>(0.0));
            let proxy=min(max(luminance(lit)/max(luminance(input.material.base_color.rgb),0.0001),0.0),1.0);
            let coordinate=clamp(proxy*params.transform.x+params.transform.y,0.0,1.0);
            let rows=max(params.atlas.y,1u);
            let first=min(params.atlas.x,rows-1u);
            let second=min(first+params.atlas.w,rows-1u);
            let band=mix(ramp_row(coordinate,first,params),ramp_row(coordinate,second,params),clamp(params.controls.x,0.0,1.0));
            let remap=min(band/max(vec3<f32>(proxy),vec3<f32>(0.05)),vec3<f32>(4.0));
            var color=lit*remap+emissive;
            if params.controls.y>0.5 {
                let fresnel=clamp(1.0-max(dot(input.N,input.V),0.0),0.0,1.0);
                let powered=pow(max(fresnel,0.00001),params.rim.x);
                var rim=select(0.0,1.0,powered>=params.rim.y);
                if params.rim.z>0.0 {rim=smoothstep(max(params.rim.y-params.rim.z,0.0),min(params.rim.y+params.rim.z,1.0),powered);}
                color+=params.rim_color.rgb*params.rim.w*rim*params.controls.z*proxy;
            }
            out.color=vec4<f32>(color,out.color.a);
        }
    }
    out.color=main_pass_post_lighting_processing(input,out.color);
    return out;
}
