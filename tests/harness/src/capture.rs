//! Headless offscreen capture: render a camera into an [`Image`] target and
//! read the pixels back to the CPU. No window is ever created.
//!
//! ## 时钟域与 CaptureStamp（docs/showcase/01 R3/R13 的中性证据层）
//!
//! harness 定义三个时钟域，stamp 只绑定前两个；墙钟属于性能测量域
//! （`labs/benchmarks`），不进入捕获身份：
//!
//! - **主世界 update 序号**（[`CaptureClock::updates_completed`]）：
//!   `app.update()` 的完成次数（每个 App 从 0 开始单调递增，系统挂在
//!   `Last`）。这是 [`CaptureStamp::issued_update_index`] 与
//!   [`CaptureStamp::completed_update_index`] 的时钟域。
//! - **渲染世界 Cleanup 帧计数**：render schedule 的执行次数（与
//!   `readiness::Probe` 的帧计数同域），经共享原子镜像回主世界；
//!   [`CaptureStamp::completed_render_frame`] 记录接受完成时点的该值。
//!
//! 绑定语义（经本模块 GPU 证据测试验证）：
//!
//! - 在两次 update 之间调用 [`request_capture`]（harness 消费者的既有用法）
//!   时，`issued_update_index == K` 意味着像素来自第 K+1 次 update 的提取：
//!   提取在 Main schedule 之后执行，复制命令与该帧渲染在同一 submission
//!   中提交（引擎 `gpu_readback` 路径）。系统内发请求则属于当次 update
//!   的提取；调用方应记录用法差异。
//! - 完成身份 = 首次被接受的 `ReadbackComplete`（引擎对存留的 `Readback`
//!   组件每帧重复请求读回；首次完成后 harness 即 despawn 请求实体，
//!   晚到的完成被丢弃，见 `duplicate_completions_*` 测试）。接受的完成
//!   对应该请求实体的首次复制，因此逐帧身份跨提取/提交/读回成立。
//! - 快照不可变：引擎在 `map_async` 回调内把映射区复制为独立 `Vec`
//!   后 unmap，缓冲池只复用 GPU buffer，绝不复用交出的字节；
//!   `try_take*` 取走后 inbox 不再持有该请求的数据。
//! - 实验 tick、有理步长、稳定逻辑 ID 与提取前屏障是 scene-runtime 的
//!   映射职责（phase F）；本层不提供、也不假设这些概念。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use bevy::{
    app::Last,
    asset::{Assets, RenderAssetUsages},
    camera::RenderTarget,
    ecs::schedule::IntoScheduleConfigs,
    prelude::{
        App, Bundle, Camera, Commands, Entity, Handle, Image, On, Res, ResMut, Resource, World,
    },
    render::{
        Render, RenderApp, RenderSystems,
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        renderer::initialize_renderer,
        settings::{RenderResources, WgpuSettings},
    },
};

/// RGBA8 bytes per pixel for the fixed capture format.
pub const CAPTURE_BYTES_PER_PIXEL: usize = 4;

/// An offscreen render target image plus its capture resolution.
#[derive(Clone, Debug)]
pub struct CaptureTarget {
    /// The render-target image handle.
    pub image: Handle<Image>,
    /// Capture width in pixels.
    pub width: u32,
    /// Capture height in pixels.
    pub height: u32,
}

/// A CPU copy of one captured frame in tightly packed RGBA8 row-major order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Tightly packed RGBA8 pixels, `width * height * 4` bytes.
    pub data: Vec<u8>,
}

impl CapturedImage {
    /// Builds a capture from raw bytes, rejecting inconsistent sizes.
    #[must_use]
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        (data.len() == width as usize * height as usize * CAPTURE_BYTES_PER_PIXEL).then_some(Self {
            width,
            height,
            data,
        })
    }
}

/// Creates the offscreen target image every capture camera renders into.
pub fn create_capture_target(images: &mut Assets<Image>, width: u32, height: u32) -> CaptureTarget {
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;
    CaptureTarget {
        image: images.add(image),
        width,
        height,
    }
}

/// Camera components binding one camera to the offscreen target.
pub fn capture_target_bundle(target: &CaptureTarget) -> impl Bundle {
    (
        Camera::default(),
        RenderTarget::Image(target.image.clone().into()),
    )
}

/// 捕获时钟（主世界 update 序号 + 请求序列；时钟域见模块文档）。
#[derive(Resource, Default)]
struct CaptureClock {
    /// 已完成的 `app.update()` 次数。
    updates_completed: u64,
    /// 下一个请求序列号。
    next_sequence: u64,
}

fn tick_capture_clock(mut clock: ResMut<CaptureClock>) {
    clock.updates_completed += 1;
}

/// 渲染世界 Cleanup 帧计数的主世界镜像（与 `readiness::Probe` 帧计数同域）。
#[derive(Resource, Clone, Default)]
struct CaptureRenderFrame(Arc<Mutex<u64>>);

fn tick_render_frame(counter: Res<CaptureRenderFrame>) {
    let mut frame = counter.0.lock().expect("render frame lock");
    *frame += 1;
}

/// 完成时点（接受 `ReadbackComplete` 时记录）。
#[derive(Clone, Copy, Debug)]
struct CompletionMark {
    /// 接受时已完成的主世界 update 次数。
    update_index: u64,
    /// 接受时点的渲染世界 Cleanup 帧计数（渲染器缺失或未运行为 None）。
    render_frame: Option<u64>,
}

#[derive(Resource, Default)]
struct CaptureInbox {
    pending: HashSet<Entity>,
    frames: HashMap<Entity, (Vec<u8>, CompletionMark)>,
}

/// 捕获身份：跨提取/提交/读回的逐帧证明（时钟域定义见模块文档）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureStamp {
    /// 请求序列号（per App 单调递增，从 0 开始）。
    pub sequence: u64,
    /// 请求时点已完成的主世界 update 次数。
    pub issued_update_index: u64,
    /// 完成被接受时点已完成的主世界 update 次数。
    pub completed_update_index: u64,
    /// 完成被接受时点的渲染世界 Cleanup 帧计数（无渲染器时为 None）。
    pub completed_render_frame: Option<u64>,
}

/// 带回执的捕获结果：图像 + 身份 stamp。
#[derive(Clone, Debug)]
pub struct StampedCapture {
    /// 捕获图像（独立拥有的字节，见模块文档的快照不可变性）。
    pub image: CapturedImage,
    /// 身份回执。
    pub stamp: CaptureStamp,
}

/// Installs the readback-completed observer. Call once per app before
/// requesting captures.
pub fn install(app: &mut App) {
    app.init_resource::<CaptureInbox>();
    app.init_resource::<CaptureClock>();
    app.init_resource::<CaptureRenderFrame>();
    app.add_systems(Last, tick_capture_clock);
    let mirror = app.world().resource::<CaptureRenderFrame>().clone();
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app
            .insert_resource(mirror)
            .add_systems(Render, tick_render_frame.in_set(RenderSystems::Cleanup));
    }
    app.add_observer(
        |event: On<ReadbackComplete>,
         mut commands: Commands,
         mut inbox: ResMut<CaptureInbox>,
         clock: Res<CaptureClock>,
         render_frame: Res<CaptureRenderFrame>| {
            // Several GPU copies may already be in flight when the first one
            // completes. Only the first completion belongs to this request;
            // late events must neither despawn again nor recreate consumed data.
            if !inbox.pending.remove(&event.entity) {
                return;
            }
            let mark = CompletionMark {
                update_index: clock.updates_completed,
                render_frame: match *render_frame.0.lock().expect("render frame lock") {
                    0 => None,
                    frame => Some(frame),
                },
            };
            inbox
                .frames
                .insert(event.entity, (event.data.clone(), mark));
            // `Readback` re-copies every frame while attached; one shot is enough.
            commands.entity(event.entity).despawn();
        },
    );
}

/// A pending frame capture request.
#[derive(Clone, Debug)]
pub struct CaptureRequest {
    entity: Entity,
    target: CaptureTarget,
    sequence: u64,
    issued_update_index: u64,
}

impl CaptureRequest {
    /// 请求序列号（per App 单调递增）。
    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// 请求时点已完成的主世界 update 次数（时钟域见模块文档）。
    #[must_use]
    pub fn issued_update_index(&self) -> u64 {
        self.issued_update_index
    }
}

/// Requests a texture readback starting on the next rendered frame.
pub fn request_capture(world: &mut World, target: CaptureTarget) -> CaptureRequest {
    let entity = world.spawn(Readback::texture(target.image.clone())).id();
    let (sequence, issued_update_index) = {
        let mut clock = world.resource_mut::<CaptureClock>();
        let sequence = clock.next_sequence;
        clock.next_sequence += 1;
        (sequence, clock.updates_completed)
    };
    world.resource_mut::<CaptureInbox>().pending.insert(entity);
    CaptureRequest {
        entity,
        target,
        sequence,
        issued_update_index,
    }
}

/// 取消一个未完成的请求：接受队列移除 + despawn 读回实体。
///
/// 取消后 [`try_take`] / [`try_take_stamped`] 永远返回 `None`；引擎若仍
/// 投递晚到的完成事件，会因不在接受队列而被忽略，不影响后续请求的身份。
pub fn cancel_capture(world: &mut World, request: CaptureRequest) {
    world
        .resource_mut::<CaptureInbox>()
        .pending
        .remove(&request.entity);
    if let Ok(entity) = world.get_entity_mut(request.entity) {
        entity.despawn();
    }
}

/// Returns the captured frame once the readback has landed.
///
/// The GPU copy buffer is row-padded to 256 bytes; the returned image is
/// always tightly packed RGBA8.
pub fn try_take(world: &mut World, request: CaptureRequest) -> Option<CapturedImage> {
    try_take_stamped(world, request).map(|capture| capture.image)
}

/// [`try_take`] 的带回执形式：返回图像与身份 [`CaptureStamp`]。
pub fn try_take_stamped(world: &mut World, request: CaptureRequest) -> Option<StampedCapture> {
    let (data, mark) = world
        .resource_mut::<CaptureInbox>()
        .frames
        .remove(&request.entity)?;
    let image = CapturedImage::new(
        request.target.width,
        request.target.height,
        depad_rgba8(
            &data,
            request.target.width as usize,
            request.target.height as usize,
        ),
    )?;
    Some(StampedCapture {
        image,
        stamp: CaptureStamp {
            sequence: request.sequence,
            issued_update_index: request.issued_update_index,
            completed_update_index: mark.update_index,
            completed_render_frame: mark.render_frame,
        },
    })
}

/// Strips the 256-byte row alignment wgpu applies to texture copy buffers.
#[must_use]
pub fn depad_rgba8(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let tight = width * CAPTURE_BYTES_PER_PIXEL;
    let padded = tight.div_ceil(256) * 256;
    if padded == tight {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(tight * height);
    for row in 0..height {
        out.extend_from_slice(&data[row * padded..row * padded + tight]);
    }
    out
}

/// Probes for a headless GPU render device without creating a window.
///
/// Returns `None` when no adapter is available (e.g. GPU-less CI). The probe
/// runs on a separate thread because Bevy's renderer initialization panics
/// when no adapter is found; the thread boundary turns that panic into a
/// graceful skip.
#[must_use]
pub fn probe_render_resources() -> Option<RenderResources> {
    let resources = if std::env::var("STYLIZED_GPU").as_deref() == Ok("disabled") {
        None
    } else {
        std::thread::spawn(|| {
            let settings = WgpuSettings::default();
            let backends = settings.backends.unwrap_or_default();
            bevy::tasks::block_on(initialize_renderer(backends, None, &settings))
        })
        .join()
        .ok()
    };
    assert!(
        resources.is_some() || std::env::var("STYLIZED_REQUIRE_GPU").as_deref() != Ok("1"),
        "required GPU unavailable; this is not a passed test"
    );
    resources
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamped_take_records_sequence_and_clock_domains() {
        let mut app = App::new();
        install(&mut app);
        let target = || CaptureTarget {
            image: Handle::default(),
            width: 64,
            height: 1,
        };
        // 首个请求：未跑过 update，issued=0，序列 0。
        let first = request_capture(app.world_mut(), target());
        assert_eq!(first.sequence(), 0);
        assert_eq!(first.issued_update_index(), 0);
        app.world_mut().trigger(ReadbackComplete {
            entity: first.entity,
            data: vec![1; 256],
        });
        let capture = try_take_stamped(app.world_mut(), first).expect("completion");
        assert_eq!(capture.image.data, vec![1; 256]);
        assert_eq!(
            capture.stamp,
            CaptureStamp {
                sequence: 0,
                issued_update_index: 0,
                completed_update_index: 0,
                completed_render_frame: None,
            }
        );
        // 跑过两次 update 后：序列继续，issued 反映主世界时钟。
        app.update();
        app.update();
        let second = request_capture(app.world_mut(), target());
        assert_eq!(second.sequence(), 1);
        assert_eq!(second.issued_update_index(), 2);
    }

    #[test]
    fn cancelled_request_ignores_late_completion_and_keeps_later_identities() {
        let mut app = App::new();
        install(&mut app);
        let target = || CaptureTarget {
            image: Handle::default(),
            width: 64,
            height: 1,
        };
        let cancelled = request_capture(app.world_mut(), target());
        cancel_capture(app.world_mut(), cancelled.clone());
        // 晚到的完成被忽略：无可取结果，inbox 不留数据。
        app.world_mut().trigger(ReadbackComplete {
            entity: cancelled.entity,
            data: vec![9; 256],
        });
        assert!(try_take(app.world_mut(), cancelled).is_none());
        let inbox = app.world().resource::<CaptureInbox>();
        assert!(inbox.pending.is_empty() && inbox.frames.is_empty());
        // 后续请求身份不回绕、内容正常。
        let next = request_capture(app.world_mut(), target());
        assert_eq!(next.sequence(), 1);
        app.world_mut().trigger(ReadbackComplete {
            entity: next.entity,
            data: vec![2; 256],
        });
        let capture = try_take_stamped(app.world_mut(), next).expect("completion");
        assert_eq!(capture.image.data, vec![2; 256]);
        assert_eq!(capture.stamp.sequence, 1);
    }

    #[test]
    fn duplicate_completions_cannot_replace_or_recreate_consumed_frames() {
        let mut app = App::new();
        install(&mut app);
        let request = request_capture(
            app.world_mut(),
            CaptureTarget {
                image: Handle::default(),
                width: 64,
                height: 1,
            },
        );
        let entity = request.entity;
        app.world_mut().trigger(ReadbackComplete {
            entity,
            data: vec![1; 256],
        });
        app.world_mut().trigger(ReadbackComplete {
            entity,
            data: vec![2; 256],
        });
        assert_eq!(
            try_take(app.world_mut(), request.clone()).unwrap().data,
            vec![1; 256]
        );
        app.world_mut().trigger(ReadbackComplete {
            entity,
            data: vec![3; 256],
        });
        assert!(try_take(app.world_mut(), request).is_none());
        let inbox = app.world().resource::<CaptureInbox>();
        assert!(inbox.pending.is_empty() && inbox.frames.is_empty());
    }

    #[test]
    fn depad_strips_row_alignment_padding() {
        // Given a 2x2 image whose 8-byte rows are padded to 256 bytes
        let width = 2;
        let height = 2;
        let padded_row = 256;
        let mut padded = vec![0_u8; padded_row * height];
        for (i, byte) in padded.iter_mut().enumerate() {
            *byte = i as u8;
        }

        // When
        let tight = depad_rgba8(&padded, width, height);

        // Then
        assert_eq!(tight.len(), width * height * CAPTURE_BYTES_PER_PIXEL);
        assert_eq!(tight[..8], padded[..8]);
        assert_eq!(tight[8..], padded[padded_row..padded_row + 8]);
    }

    #[test]
    fn depad_keeps_aligned_rows_untouched() {
        // Given
        let data: Vec<u8> = (0..64 * 4 * 3).map(|i| i as u8).collect();

        // When
        let out = depad_rgba8(&data, 64, 3);

        // Then
        assert_eq!(out, data);
    }

    #[test]
    fn captured_image_rejects_inconsistent_byte_count() {
        assert!(CapturedImage::new(2, 2, vec![0; 16]).is_some());
        assert!(CapturedImage::new(2, 2, vec![0; 15]).is_none());
    }
}

/// Linear floating-point target for matched offline sampling. This is not a
/// requirement of the production renderer's portable path.
pub fn create_linear_capture_target(
    images: &mut Assets<Image>,
    width: u32,
    height: u32,
) -> CaptureTarget {
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 8],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;
    CaptureTarget {
        image: images.add(image),
        width,
        height,
    }
}

/// Resolve a linear HDR readback using a regular box in linear light, then encode
/// sRGB once. Each renderer must use the same declared grid and resolve policy.
pub fn try_take_linear_resolved(
    world: &mut World,
    request: CaptureRequest,
    scale: u32,
) -> Result<Option<CapturedImage>, String> {
    let Some((data, _mark)) = world
        .resource_mut::<CaptureInbox>()
        .frames
        .remove(&request.entity)
    else {
        return Ok(None);
    };
    let (width, height) = (
        request.target.width as usize,
        request.target.height as usize,
    );
    let scale = scale as usize;
    if scale == 0 || !width.is_multiple_of(scale) || !height.is_multiple_of(scale) {
        return Err("invalid resolve grid".into());
    }
    let row = (width * 8).div_ceil(256) * 256;
    if data.len() != row * height {
        return Err("invalid linear readback size".into());
    }
    let mut result = Vec::with_capacity(width * height * 4 / (scale * scale));
    for y in 0..height / scale {
        for x in 0..width / scale {
            let mut sum = [0.0f64; 4];
            for dy in 0..scale {
                for dx in 0..scale {
                    let offset = (y * scale + dy) * row + (x * scale + dx) * 8;
                    for c in 0..4 {
                        let v = half::f16::from_le_bytes([
                            data[offset + c * 2],
                            data[offset + c * 2 + 1],
                        ])
                        .to_f32();
                        if !v.is_finite() {
                            return Err("nonfinite linear capture".into());
                        }
                        sum[c] += f64::from(v);
                    }
                }
            }
            for (c, v) in sum.into_iter().enumerate() {
                let linear = (v / (scale * scale) as f64).clamp(0.0, 1.0);
                let encoded = if c == 3 {
                    linear
                } else if linear <= 0.0031308 {
                    linear * 12.92
                } else {
                    1.055 * linear.powf(1.0 / 2.4) - 0.055
                };
                result.push((encoded * 255.).round() as u8);
            }
        }
    }
    Ok(CapturedImage::new(
        (width / scale) as u32,
        (height / scale) as u32,
        result,
    ))
}

#[cfg(test)]
mod linear_resolve_tests {
    use super::*;
    #[test]
    fn hdr_samples_are_averaged_before_display_clipping() {
        let mut app = App::new();
        install(&mut app);
        let request = request_capture(
            app.world_mut(),
            CaptureTarget {
                image: Handle::default(),
                width: 2,
                height: 2,
            },
        );
        let mut data = vec![0; 512];
        for y in 0..2 {
            for x in 0..2 {
                for (c, v) in [if x == 0 { 0.0 } else { 2.0 }, 0.0, 0.0, 1.0]
                    .into_iter()
                    .enumerate()
                {
                    let bytes = half::f16::from_f32(v).to_le_bytes();
                    let offset = y * 256 + x * 8 + c * 2;
                    data[offset..offset + 2].copy_from_slice(&bytes);
                }
            }
        }
        app.world_mut().trigger(ReadbackComplete {
            entity: request.entity,
            data,
        });
        let image = try_take_linear_resolved(app.world_mut(), request, 2)
            .unwrap()
            .unwrap();
        assert_eq!((image.width, image.height), (1, 1));
        assert_eq!(image.data, [255, 0, 0, 255]);
    }
}
