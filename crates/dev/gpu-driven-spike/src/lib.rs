//! GPU-driven rendering 可行性 spike（V2 评估；证据采集，非生产特性）。
//!
//! 原型：**compute shader 视锥剔除 + 实例压缩**（GPU 驱动的最小有意义
//! 元素）。实例 = 包围球（center+radius），视锥 = 6 平面；可见实例索引
//! 压缩进输出表 + atomic 计数。
//!
//! 测量（10k/50k/100k 实例，RTX 5090 Vulkan）：
//!
//! - `cpu_cull`：同数学的 CPU 逐实例剔除 + 可见表构建（对应 Bevy
//!   `check_visibility` 的 CPU 路径角色）；
//! - `gpu_cull_submit`：compute dispatch + 完成等待（**结果留在 GPU**
//!   ——这是 GPU-driven 的卖点：可见表直接喂 indirect draw，不回读）；
//! - `gpu_cull_readback`：dispatch + 计数回读（跨 GPU→CPU 往返的真实
//!   成本；生产路径应避免，此处如实测量）；
//! - 正确性门：GPU 计数必须等于 CPU 计数（确定性 LCG 场景）。
//!
//! 集成缺口（诚实记录）：本原型在 raw wgpu 层驱动（经 golden-harness
//! 的 probe 设备，即 Bevy 同一设备）；把自定义 compute 结果接进 Bevy
//! 0.19 的 binned render phase 需要 crate-private 的批集构建路径
//! （`bevy_pbr::render::gpu_preprocess`），公开 API 无注入点——详见
//! docs/rendering/04。

use std::time::Instant;

use bevy::render::{render_resource::*, settings::RenderResources};

/// 剔除 compute shader（binding 布局见 [`CullBuffers`]）。
const CULL_WGSL: &str = r"
struct CullInstance {
    center: vec3<f32>,
    radius: f32,
}

struct CullParams {
    // 6 个视锥平面（xyz = 法线朝内, w = 距离项）。
    planes: array<vec4<f32>, 6>,
    count: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
}

@group(0) @binding(0) var<uniform> params: CullParams;
@group(0) @binding(1) var<storage, read> instances: array<CullInstance>;
@group(0) @binding(2) var<storage, read_write> visible: array<u32>;
@group(0) @binding(3) var<storage, read_write> visible_count: atomic<u32>;

@compute @workgroup_size(64)
fn cull(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.count {
        return;
    }
    let inst = instances[id.x];
    var inside = true;
    for (var i = 0u; i < 6u; i += 1u) {
        let plane = params.planes[i];
        if dot(plane.xyz, inst.center) + plane.w < -inst.radius {
            inside = false;
        }
    }
    if inside {
        visible[atomicAdd(&visible_count, 1u)] = id.x;
    }
}
";

/// 单个实例（包围球；16B 对齐与 WGSL struct 一致）。
#[derive(Clone, Copy, Debug)]
pub struct CullInstance {
    /// 中心 xyz + 半径 w。
    pub center_radius: [f32; 4],
}

/// 视锥平面（xyz 法线朝内，w 距离项）。
pub type FrustumPlanes = [[f32; 4]; 6];

/// 确定性场景生成（LCG；两側同一场景才可比较）。
///
/// 把球放在 ±extent 立方体内；约一半落在测试视锥外（视锥为
/// 朝 -Z 的 90° 锥，置于原点）。
pub fn generate_instances(n: u32, extent: f32) -> Vec<CullInstance> {
    let mut state = 0x9E3779B9u64;
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 33) as u32) as f32 / u32::MAX as f32
    };
    (0..n)
        .map(|_| CullInstance {
            center_radius: [
                (next() * 2.0 - 1.0) * extent,
                (next() * 2.0 - 1.0) * extent,
                -next() * extent * 2.0, // z ∈ [-2e, 0]
                0.5,
            ],
        })
        .collect()
}

/// 测试视锥：原点朝 -Z 的 90° 锥（水平/垂直半角 45°）+ near/far 平面。
/// 平面 (n, w) 的 inside 条件：`dot(n, c) + w >= -r`。
#[must_use]
pub fn test_frustum(far: f32) -> FrustumPlanes {
    let s = std::f32::consts::FRAC_1_SQRT_2;
    [
        [s, 0.0, -s, 0.0],      // 左：x - z ≥ 0
        [-s, 0.0, -s, 0.0],     // 右：-x - z ≥ 0
        [0.0, s, -s, 0.0],      // 下：y - z ≥ 0
        [0.0, -s, -s, 0.0],     // 上：-y - z ≥ 0
        [0.0, 0.0, -1.0, -0.1], // near：-z ≥ 0.1（z ≤ -0.1）
        [0.0, 0.0, 1.0, far],   // far：z ≥ -far
    ]
}

/// CPU 镜像剔除（与 WGSL 逐字同数学；正确性参照 + CPU 成本基线）。
pub fn cpu_cull(instances: &[CullInstance], planes: &FrustumPlanes) -> Vec<u32> {
    let mut visible = Vec::with_capacity(instances.len() / 2);
    for (i, inst) in instances.iter().enumerate() {
        let [x, y, z, radius] = inst.center_radius;
        let inside = planes
            .iter()
            .all(|p| p[0] * x + p[1] * y + p[2] * z + p[3] >= -radius);
        if inside {
            visible.push(i as u32);
        }
    }
    visible
}

struct GpuCuller {
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
    params: Buffer,
    instances: Buffer,
    visible: Buffer,
    count: Buffer,
    count_staging: Buffer,
    capacity: u32,
}

impl GpuCuller {
    fn new(device: &bevy::render::renderer::RenderDevice, capacity: u32) -> Self {
        // SAFETY：spike 内联 WGSL 为可信常量（与 shader_validation 测试
        // 同一纪律：非法 shader 会被 naga 拒绝，见 tests/spike.rs 的运行）。
        let shader = unsafe {
            device.create_shader_module(ShaderModuleDescriptor {
                label: Some("gpu-driven-spike cull"),
                source: ShaderSource::Wgsl(CULL_WGSL.into()),
            })
        };
        let bind_group_layout = device.create_bind_group_layout(
            None,
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        );
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("cull layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            ..PipelineLayoutDescriptor::default()
        });
        let pipeline = device.create_compute_pipeline(&RawComputePipelineDescriptor {
            label: Some("cull pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cull"),
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });
        let params = device.create_buffer(&BufferDescriptor {
            label: Some("cull params"),
            size: 6 * 16 + 16,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instances = device.create_buffer(&BufferDescriptor {
            label: Some("cull instances"),
            size: capacity as u64 * 16,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let visible = device.create_buffer(&BufferDescriptor {
            label: Some("cull visible"),
            size: capacity as u64 * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let count = device.create_buffer(&BufferDescriptor {
            label: Some("cull count"),
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let count_staging = device.create_buffer(&BufferDescriptor {
            label: Some("cull count staging"),
            size: 4,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            bind_group_layout,
            params,
            instances,
            visible,
            count,
            count_staging,
            capacity,
        }
    }

    fn upload(
        &self,
        queue: &bevy::render::renderer::RenderQueue,
        planes: &FrustumPlanes,
        instances: &[CullInstance],
    ) {
        assert!(instances.len() as u32 <= self.capacity);
        let mut params_bytes = Vec::with_capacity(6 * 16 + 16);
        for plane in planes {
            for v in plane {
                params_bytes.extend_from_slice(&v.to_le_bytes());
            }
        }
        params_bytes.extend_from_slice(&(instances.len() as u32).to_le_bytes());
        params_bytes.extend_from_slice(&[0u8; 12]);
        queue.write_buffer(&self.params, 0, &params_bytes);
        let mut instance_bytes = Vec::with_capacity(instances.len() * 16);
        for inst in instances {
            for v in inst.center_radius {
                instance_bytes.extend_from_slice(&v.to_le_bytes());
            }
        }
        queue.write_buffer(&self.instances, 0, &instance_bytes);
        queue.write_buffer(&self.count, 0, &0u32.to_le_bytes());
    }

    /// dispatch + 完成等待（不回读；结果留在 GPU）。
    fn dispatch_submit_only(
        &self,
        device: &bevy::render::renderer::RenderDevice,
        queue: &bevy::render::renderer::RenderQueue,
        n: u32,
    ) {
        queue.write_buffer(&self.count, 0, &0u32.to_le_bytes());
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(
                0,
                &device.create_bind_group(
                    None,
                    &self.bind_group_layout,
                    &[
                        BindGroupEntry {
                            binding: 0,
                            resource: self.params.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: self.instances.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: self.visible.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: self.count.as_entire_binding(),
                        },
                    ],
                ),
                &[],
            );
            pass.dispatch_workgroups(n.div_ceil(64), 1, 1);
        }
        queue.submit([encoder.finish()]);
        device.poll(PollType::wait_indefinitely()).expect("poll");
    }

    /// dispatch + 回读可见计数（含 GPU→CPU 往返）。
    fn dispatch_with_readback(
        &self,
        device: &bevy::render::renderer::RenderDevice,
        queue: &bevy::render::renderer::RenderQueue,
        n: u32,
    ) -> u32 {
        queue.write_buffer(&self.count, 0, &0u32.to_le_bytes());
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(
                0,
                &device.create_bind_group(
                    None,
                    &self.bind_group_layout,
                    &[
                        BindGroupEntry {
                            binding: 0,
                            resource: self.params.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: self.instances.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: self.visible.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: self.count.as_entire_binding(),
                        },
                    ],
                ),
                &[],
            );
            pass.dispatch_workgroups(n.div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&self.count, 0, &self.count_staging, 0, 4);
        queue.submit([encoder.finish()]);
        let slice = self.count_staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(MapMode::Read, move |result| {
            sender.send(result).expect("map callback");
        });
        device.poll(PollType::wait_indefinitely()).expect("poll");
        receiver.recv().expect("map result").expect("map ok");
        let data = slice.get_mapped_range();
        let count = u32::from_le_bytes(data[..4].try_into().expect("4 bytes"));
        drop(data);
        self.count_staging.unmap();
        count
    }
}

/// 单个规模的测量结果。
#[derive(Clone, Debug, serde::Serialize)]
pub struct SpikeMeasurement {
    /// 实例数。
    pub instances: u32,
    /// CPU 剔除（含可见表构建）耗时 ms。
    pub cpu_cull_ms: f64,
    /// GPU dispatch + 完成等待（结果留 GPU）耗时 ms。
    pub gpu_submit_ms: f64,
    /// GPU dispatch + 计数回读耗时 ms。
    pub gpu_readback_ms: f64,
    /// CPU / GPU 可见计数（必须相等——正确性门）。
    pub cpu_visible: u32,
    /// GPU 计数。
    pub gpu_visible: u32,
}

/// spike 报告。
#[derive(Clone, Debug, serde::Serialize)]
pub struct SpikeReport {
    /// 用途声明。
    pub provisional: bool,
    /// 迭代。
    pub iteration: &'static str,
    /// GPU。
    pub gpu: String,
    /// 说明。
    pub notes: &'static str,
    /// 测量。
    pub measurements: Vec<SpikeMeasurement>,
}

/// spike 规模轴。
pub const SIZES: [u32; 3] = [10_000, 50_000, 100_000];
/// 每规模重复次数（取均值；dispatch 成本在微秒级，单次测量噪声大）。
pub const REPEATS: usize = 20;

/// 跑 spike。返回 `None` = 无 GPU（graceful skip）。
pub fn run_spike() -> Option<SpikeReport> {
    let resources = golden_harness::probe_render_resources()?;
    let RenderResources(device, queue, adapter_info, ..) = resources;
    let extent = 200.0_f32;
    let max_n = *SIZES.iter().max().expect("sizes");
    let culler = GpuCuller::new(&device, max_n);
    let planes = test_frustum(extent * 3.0);

    let mut measurements = Vec::new();
    for &n in &SIZES {
        let instances = generate_instances(n, extent);

        // CPU 基线。
        let start = Instant::now();
        let mut cpu_visible = 0;
        for _ in 0..REPEATS {
            cpu_visible = cpu_cull(&instances, &planes).len() as u32;
        }
        let cpu_ms = start.elapsed().as_secs_f64() * 1000.0 / REPEATS as f64;

        culler.upload(&queue, &planes, &instances);

        // GPU：submit + wait（结果留 GPU）。
        let start = Instant::now();
        for _ in 0..REPEATS {
            culler.dispatch_submit_only(&device, &queue, n);
        }
        let gpu_submit_ms = start.elapsed().as_secs_f64() * 1000.0 / REPEATS as f64;

        // GPU：submit + 回读。
        let start = Instant::now();
        let mut gpu_visible = 0;
        for _ in 0..REPEATS {
            gpu_visible = culler.dispatch_with_readback(&device, &queue, n);
        }
        let gpu_readback_ms = start.elapsed().as_secs_f64() * 1000.0 / REPEATS as f64;

        measurements.push(SpikeMeasurement {
            instances: n,
            cpu_cull_ms: cpu_ms,
            gpu_submit_ms,
            gpu_readback_ms,
            cpu_visible,
            gpu_visible,
        });
    }

    Some(SpikeReport {
        provisional: true,
        iteration: "V2-gpu-driven-eval",
        gpu: format!("{} ({:?})", adapter_info.name, adapter_info.backend),
        notes: "gpu_submit = 结果留 GPU（生产形态：可见表直接喂 indirect draw）；\
                gpu_readback = 含 GPU→CPU 计数往返（生产应避免，此处如实测量）；\
                CPU 基线为同数学逐实例剔除（check_visibility 的角色）。",
        measurements,
    })
}
