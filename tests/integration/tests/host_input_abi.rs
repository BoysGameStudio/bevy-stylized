//! GPU ABI proof only; this does not qualify dynamic material rendering.
use bevy::{
    math::{UVec4, Vec4},
    render::{
        render_resource::{
            BindGroupDescriptor, BindGroupEntry, BufferDescriptor, BufferInitDescriptor,
            BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor, MapMode, PollType,
            RawComputePipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderType,
            encase::internal::WriteInto,
        },
        settings::RenderResources,
    },
};
use bevy_stylized_render::{
    character::StylizedCharacterParams,
    hero::StylizedHeroParams,
    host_inputs::{HostInstanceRecord, HostViewRecord, pack_host_material_uniform},
    material::StylizedCelParams,
    outline::StylizedOutlineParams,
};
use std::{borrow::Cow, sync::mpsc, time::Duration};

fn verify_prefix<T: ShaderType + WriteInto + Default>(resources: &RenderResources) {
    let instance = HostInstanceRecord {
        tint: Vec4::new(2.0, 0.5, 0.25, 0.7),
        indices: UVec4::new(7, 1, 9, 0x80),
        ..Default::default()
    };
    let second_view = HostViewRecord {
        key_color: Vec4::splat(0.25),
        ..Default::default()
    };
    let bytes = pack_host_material_uniform(
        &T::default(),
        &[instance],
        &[HostViewRecord::default(), second_view],
    )
    .unwrap();
    let prefix_lanes = (bytes.len() - 14_848) / 16;
    let shader = format!(
        r#"
struct Instance {{ tint: vec4<f32>, emission: vec4<f32>, rim: vec4<f32>,
  forward: vec4<f32>, up: vec4<f32>, indices: vec4<u32> }};
struct View {{ direction: vec4<f32>, key: vec4<f32>, ambient: vec4<f32>,
  soft_lit: vec4<f32>, soft_shadow: vec4<f32>, sharp_lit: vec4<f32>,
  sharp_shadow: vec4<f32>, controls: vec4<f32>, background: vec4<f32>, fog: vec4<f32> }};
struct Frame {{ prefix: array<vec4<f32>, {prefix_lanes}>,
  instances: array<Instance, 128>, views: array<View, 16> }};
@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(1) fn main() {{
  output[0] = bitcast<u32>(frame.instances[0].tint.x);
  output[1] = frame.instances[0].indices.y;
  output[2] = bitcast<u32>(frame.views[frame.instances[0].indices.y].key.x);
  output[3] = bitcast<u32>(frame.instances[127].tint.w);
}}
"#
    );
    let render_device = &resources.0;
    let device = render_device.wgpu_device();
    let module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("host-input-ABI"),
        source: ShaderSource::Wgsl(Cow::Owned(shader)),
    });
    let pipeline = device.create_compute_pipeline(&RawComputePipelineDescriptor {
        label: Some("host-input-ABI"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let input = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("host-input-uniform"),
        contents: &bytes,
        usage: BufferUsages::UNIFORM,
    });
    let output = render_device.create_buffer(&BufferDescriptor {
        label: Some("host-input-output"),
        size: 16,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = render_device.create_buffer(&BufferDescriptor {
        label: Some("host-input-readback"),
        size: 16,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let layout = pipeline.get_bind_group_layout(0);
    let group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("host-input-ABI"),
        layout: &layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 16);
    let submission = resources.1.submit([encoder.finish()]);
    let slice = readback.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(MapMode::Read, move |result| sender.send(result).unwrap());
    device
        .poll(PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    receiver
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let mapped = slice.get_mapped_range();
    let values = mapped
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect::<Vec<_>>();
    assert_eq!(
        values,
        [2.0_f32.to_bits(), 1, 0.25_f32.to_bits(), 1.0_f32.to_bits()]
    );
    drop(mapped);
    readback.unmap();
    eprintln!(
        "{}: {} bytes, GPU record offsets verified",
        std::any::type_name::<T>(),
        bytes.len()
    );
}

#[test]
#[ignore = "requires GPU; run the explicit host_input_abi suite"]
fn bounded_host_uniform_records_match_gpu_layout() {
    let Some(resources) = golden_harness::probe_render_resources() else {
        return;
    };
    verify_prefix::<StylizedCelParams>(&resources);
    verify_prefix::<StylizedCharacterParams>(&resources);
    verify_prefix::<StylizedHeroParams>(&resources);
    verify_prefix::<StylizedOutlineParams>(&resources);
}
