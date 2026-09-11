//! Whole-pipeline 预算测量（docs/rendering/01；interim，pending ADR-0002）。
//!
//! 测量而非声明：
//!
//! - **View**：render world 的 `MeshPipelineViewLayouts::get_view_layout`（空
//!   key = 最小 view layout），对 descriptor entries 按 stage 可见性计数——
//!   真实 pipeline 布局的反射，不是手抄常数；
//! - **Mesh**：model uniform/storage（随目标模型的 storage 可用性）+
//!   skinning buffer（蒙皮材质时）；我们的 shader 在 fragment 读
//!   `mesh_functions::get_tag`，故 mesh 绑定计入 FRAGMENT；
//! - **Material**：`AsBindGroup::bind_group_layout_entries`（`force_no_bindless`）；
//! - **Instance**：MeshTag 通道不占绑定（0）；storage 表方案落地后在此计数；
//! - **Profile**：当前无 profile 绑定（0）；
//! - **Safety margin**：core 的 [`INTERIM_SAFETY_MARGIN`]。

use bevy::render::render_resource::{AsBindGroup, BindingType, BufferBindingType, ShaderStages};
use bevy_stylized_core::{StylizedBudgetLimits, StylizedStageUsage};

/// 统计一个 binding 类型在指定 stage 的用量。
fn count_entry(usage: &mut StylizedStageUsage, ty: &BindingType) {
    match ty {
        BindingType::Texture { .. } => usage.textures += 1,
        BindingType::Sampler(_) => usage.samplers += 1,
        BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            ..
        } => usage.uniform_buffers += 1,
        BindingType::Buffer {
            ty: BufferBindingType::Storage { .. },
            ..
        } => usage.storage_buffers += 1,
        _ => {}
    }
}

/// 按 stage 统计一组 bind group layout entries。
#[must_use]
pub fn count_layout_entries(
    entries: &[bevy::render::render_resource::BindGroupLayoutEntry],
    stage: ShaderStages,
) -> StylizedStageUsage {
    let mut usage = StylizedStageUsage::default();
    for entry in entries {
        if entry.visibility.contains(stage) {
            count_entry(&mut usage, &entry.ty);
        }
    }
    usage
}

/// 材质 bind group 的 per-stage 用量（需 RenderDevice；`force_no_bindless`
/// 保证按 fixed-binding 计数）。
#[must_use]
pub fn count_material_usage<M: AsBindGroup>(
    render_device: &bevy::render::renderer::RenderDevice,
    stage: ShaderStages,
) -> StylizedStageUsage {
    count_layout_entries(&M::bind_group_layout_entries(render_device, true), stage)
}

/// Mesh bind group 用量（按**目标限制模型**评估，不按当前设备）：
/// model buffer 在 storage 可用时是 storage buffer，WebGL2 模型下是
/// uniform；蒙皮再加 1 个 skinning buffer。我们 fragment 读 `get_tag`，
/// 故顶点/片元双 stage 计入。
#[must_use]
pub fn mesh_usage_for_model(model: &StylizedBudgetLimits, skinned: bool) -> StylizedStageUsage {
    let storage_available = model.max_storage_buffers_per_stage > 0;
    let mut usage = StylizedStageUsage::default();
    if storage_available {
        usage.storage_buffers += 1;
    } else {
        usage.uniform_buffers += 1;
    }
    if skinned {
        if storage_available {
            usage.storage_buffers += 1;
        } else {
            usage.uniform_buffers += 1;
        }
    }
    usage
}

/// Instance 层（MeshTag 通道）：不占绑定。
#[must_use]
pub const fn instance_usage() -> StylizedStageUsage {
    StylizedStageUsage {
        textures: 0,
        samplers: 0,
        uniform_buffers: 0,
        storage_buffers: 0,
    }
}
