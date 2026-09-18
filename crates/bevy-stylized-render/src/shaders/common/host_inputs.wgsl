#define_import_path bevy_stylized_render::common::host_inputs

// ABI v1: matches host_inputs.rs. No extra uniform binding or storage is used.
struct HostInstance {
    tint: vec4<f32>, emission: vec4<f32>, rim: vec4<f32>,
    face_forward: vec4<f32>, face_up: vec4<f32>, indices: vec4<u32>,
}
struct HostView {
    direction: vec4<f32>, key_color: vec4<f32>, ambient: vec4<f32>,
    soft_lit: vec4<f32>, soft_shadow: vec4<f32>,
    sharp_lit: vec4<f32>, sharp_shadow: vec4<f32>,
    controls: vec4<f32>, background: vec4<f32>, fog: vec4<f32>,
}
