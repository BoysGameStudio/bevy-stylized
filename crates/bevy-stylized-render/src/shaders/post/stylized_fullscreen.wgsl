#define_import_path bevy_stylized_render::post::stylized_fullscreen

// 单 pass 全屏后处理（It-5，provisional pending ADR-0009）。
//
// 单 pass 约束（见 post.rs 模块文档）：vignette / chromatic / display
// transform hook 合成在这一个 pass 内；chromatic = 0 时只采中心颜色。
// LDR-safe：无任何 HDR 依赖。

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct StylizedFullscreenEffect {
    // x: vignette 强度, y: chromatic, z: display transform 模式, w: 预留
    strengths: vec4<f32>,
    // x: vignette 半径, y: vignette 软过渡
    vignette: vec4<f32>,
}

@group(0) @binding(2) var<uniform> settings: StylizedFullscreenEffect;

// 占位 display transform hook（简单 filmic-ish 压缩；正式曲线属
// profile/ADR-0009）。
fn simple_compress(x: vec3<f32>) -> vec3<f32> {
    return clamp(x * (1.2 * x + vec3<f32>(0.05)) / (x * (x + vec3<f32>(0.6)) + vec3<f32>(0.2)),
        vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let centered = in.uv - vec2<f32>(0.5);
    let radius_squared = dot(centered, centered);

    var color: vec3<f32>;
    let alpha = textureSample(screen_texture, texture_sampler, in.uv).a;
    if settings.strengths.y > 0.0 {
        // Chromatic：径向 RGB 分离（仅启用时付三路采样成本）。
        let radial = select(vec2<f32>(0.0), normalize(centered), radius_squared > 0.000001);
        let offset = radial * settings.strengths.y;
        color = vec3<f32>(
            textureSample(screen_texture, texture_sampler, clamp(in.uv + offset, vec2<f32>(0.0), vec2<f32>(1.0))).r,
            textureSample(screen_texture, texture_sampler, in.uv).g,
            textureSample(screen_texture, texture_sampler, clamp(in.uv - offset, vec2<f32>(0.0), vec2<f32>(1.0))).b,
        );
    } else {
        color = textureSample(screen_texture, texture_sampler, in.uv).rgb;
    }

    // Vignette。
    let radial_distance = length(centered) * 1.41421356;
    let falloff = smoothstep(
        settings.vignette.x - settings.vignette.y,
        settings.vignette.x,
        radial_distance,
    );
    color *= 1.0 - settings.strengths.x * falloff;

    if settings.strengths.z > 0.5 {
        color = simple_compress(color);
    }
    return vec4<f32>(color, alpha);
}
