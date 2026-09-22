//! 场景↔宿主边界：内容构建器返回的相机配方（`build` feature）。
//!
//! 构建器只组装内容（几何/材质/蒙皮/灯光相关组件），不创建相机、
//! 捕获 target 或窗口；宿主按本配方生成相机并绑定输出（离屏 target、
//! tonemapping、窗口表面）。

use bevy::camera::Projection;
use bevy::math::Vec3;
use bevy_stylized::stylized_render::post::StylizedPostProcess;

/// 相机附件：内容侧声明、宿主侧绑定的 per-camera 组件。
///
/// 只携带参数数据；把附件插入哪台相机实体由宿主决定。
#[derive(Clone, Debug)]
pub enum CameraAttachment {
    /// 无附件。
    None,
    /// 后处理参数（View 层开关默认开）。
    PostProcess(StylizedPostProcess),
    /// screen-space 内部边缘（Standard+；插入默认参数）。
    ScreenOutline,
    /// prepass 组件（深度/法线/运动向量；内容级的逐相机声明）。
    Prepass {
        /// DepthPrepass。
        depth: bool,
        /// NormalPrepass。
        normal: bool,
        /// MotionVectorPrepass。
        motion_vector: bool,
    },
}

/// 相机配方：eye/look + 投影 + 相机附件声明。
#[derive(Clone, Debug)]
pub struct CameraRecipe {
    /// 相机位置。
    pub eye: Vec3,
    /// 注视目标（宿主以 `Vec3::Y` 为 up 朝向）。
    pub look: Vec3,
    /// 显式投影（None = 产品/引擎默认投影；integration 夹具的正交投影等
    /// 内容级相机配置由此携带）。
    pub projection: Option<Projection>,
    /// 相机附件声明。
    pub attachment: CameraAttachment,
}

impl CameraRecipe {
    /// 无附件配方（默认投影）。
    #[must_use]
    pub fn new(eye: Vec3, look: Vec3) -> Self {
        Self {
            eye,
            look,
            projection: None,
            attachment: CameraAttachment::None,
        }
    }

    /// 带附件配方（默认投影）。
    #[must_use]
    pub fn with_attachment(eye: Vec3, look: Vec3, attachment: CameraAttachment) -> Self {
        Self {
            eye,
            look,
            projection: None,
            attachment,
        }
    }

    /// 指定投影（返回自身便于链式构造）。
    #[must_use]
    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.projection = Some(projection);
        self
    }
}
