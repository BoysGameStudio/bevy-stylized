//! `scenes`：统一跨平台场景工作台的场景内容层
//! （docs/showcase/01 §依赖与代码所有权）。
//!
//! 职责：注册场景及参数元数据；几何/纹理/蒙皮原语、内容组合和产品装配。
//! 本层无 runner、UI、判定或测量。
//!
//! Feature 结构（目录元数据必须可在不启用 renderer/glTF/UI 的构建中枚举）：
//!
//! - 默认（仅元数据）：[`metadata`]（布局常量等内容数据）与 [`views`]
//!   （内容级视图 preset）只依赖 `scenario-model` / `bevy-stylized-core`
//!   纯数据，不引入 Bevy App 或渲染 API；
//! - `build` feature：[`builders`]（内容构建器）、[`recipe`]（相机配方）
//!   与 [`registry`]（SceneId ↔ 构建实现的绑定表）才引入产品渲染 API
//!   与 Bevy。
//!
//! 场景↔宿主边界：构建器不创建相机、捕获 target 或窗口，返回
//! [`CameraRecipe`]（eye/look + 相机附件声明）；输出绑定归宿主
//! （"场景返回内容与相机配方，宿主绑定输出"）。

pub mod metadata;
pub mod views;

#[cfg(feature = "build")]
pub mod builders;
#[cfg(feature = "build")]
pub mod recipe;
#[cfg(feature = "build")]
pub mod registry;

#[cfg(feature = "build")]
pub use builders::SceneTarget;
#[cfg(feature = "build")]
pub use recipe::{CameraAttachment, CameraRecipe};
#[cfg(feature = "build")]
pub use registry::{SceneBinding, ViewPresetFn, bindings, validate_against_catalog};
