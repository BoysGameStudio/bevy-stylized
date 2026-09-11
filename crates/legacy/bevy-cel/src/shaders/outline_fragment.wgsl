#import bevy_pbr::forward_io::FragmentOutput
#import bevy_cel::outline_shared::OutlineVertexOutput
#import bevy_cel::outline_vertex::{OutlineUniform, outline}

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var outline_lightmap_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var outline_lightmap_sampler: sampler;

@fragment
fn fragment(_in: OutlineVertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    var color = outline.color_width.rgb;
    if outline.source_state.x != 0u {
        var selected_id = 0u;
#ifdef VERTEX_UVS_A
        if outline.source_state.z == 0u && outline.source_state.w != 0u {
            let alpha = textureSampleLevel(
                outline_lightmap_texture,
                outline_lightmap_sampler,
                _in.uv,
                0.0,
            ).a;
            // Gacha outline classes intentionally remap alpha bands to
            // colors 1,4,3,5,2.
            if alpha > 0.8 {
                selected_id = 1u;
            } else if alpha > 0.6 {
                selected_id = 4u;
            } else if alpha > 0.4 {
                selected_id = 2u;
            } else if alpha > 0.2 {
                selected_id = 3u;
            }
        }
#endif
        if selected_id > 0u
            && (outline.source_state.y & (1u << (selected_id - 1u))) == 0u
        {
            selected_id = 0u;
        }
        if selected_id == 0u {
            color = outline.source_color_one.rgb;
        } else if selected_id == 1u {
            color = outline.source_color_two.rgb;
        } else if selected_id == 2u {
            color = outline.source_color_three.rgb;
        } else if selected_id == 3u {
            color = outline.source_color_four.rgb;
        } else {
            color = outline.source_color_five.rgb;
        }
        color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(2.2))
            * outline.source_modulation.rgb;
    }
    out.color = vec4<f32>(color, 1.0);
    return out;
}
