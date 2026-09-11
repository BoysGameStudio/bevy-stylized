#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct StylizedFullscreenEffect {
    strengths: vec4<f32>,
    vignette: vec4<f32>,
}

@group(0) @binding(2) var<uniform> settings: StylizedFullscreenEffect;

fn genshin_display_transform(x: vec3<f32>) -> vec3<f32> {
    let mapped = x * (1.36 * x + vec3<f32>(0.047))
        / (x * (0.93 * x + vec3<f32>(0.56)) + vec3<f32>(0.14));
    return clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let centered = in.uv - vec2<f32>(0.5);
    let radius_squared = dot(centered, centered);
    let distorted = clamp(
        vec2<f32>(0.5) + centered * (1.0 + settings.strengths.y * radius_squared),
        vec2<f32>(0.0),
        vec2<f32>(1.0),
    );
    let radial = select(vec2<f32>(0.0), normalize(centered), radius_squared > 0.000001);
    let chromatic_offset = radial * settings.strengths.z;
    let center_sample = textureSample(screen_texture, texture_sampler, distorted);
    let color = vec3<f32>(
        textureSample(screen_texture, texture_sampler, clamp(distorted + chromatic_offset, vec2<f32>(0.0), vec2<f32>(1.0))).r,
        center_sample.g,
        textureSample(screen_texture, texture_sampler, clamp(distorted - chromatic_offset, vec2<f32>(0.0), vec2<f32>(1.0))).b,
    );
    let radial_distance = length(centered) * 1.41421356;
    let falloff = smoothstep(
        settings.vignette.x - settings.vignette.y,
        settings.vignette.x,
        radial_distance,
    );
    let vignette = 1.0 - settings.strengths.x * falloff;
    var output_color = color * vignette;
    if settings.strengths.w > 0.5 {
        output_color = genshin_display_transform(output_color);
    }
    return vec4<f32>(output_color, center_sample.a);
}
