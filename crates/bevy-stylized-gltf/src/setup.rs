//! `GltfPlugin` 配置入口与初始化顺序守卫。
//!
//! **Provisional（ADR-0028 未决）**：`_NPR_*` attribute 必须在
//! `GltfPlugin` 加入 `App` **之前**注册到 plugin 上——`GltfPlugin::finish`
//! 会把 `custom_vertex_attributes` 克隆进 `GltfLoader`，之后没有任何公开
//! API 可以补救。ADR-0028 尚未选定正式机制；当前选择是
//! "builder 配置 + fail-closed 顺序守卫"，证据见 facade 的插件顺序集成测试。

use bevy::{app::App, gltf::GltfPlugin};

use crate::attributes::StylizedGltfPluginExt;

/// 返回已注册全部 `_NPR_*` 顶点属性的 [`GltfPlugin`]（正确入口）。
///
/// 用法（provisional，pending ADR-0028）：
///
/// ```ignore
/// App::new()
///     .add_plugins(DefaultPlugins.build().disable::<GltfPlugin>())
///     .add_plugins(npr_gltf_plugin())
///     .add_plugins(StylizedPlugin::default());
/// ```
#[must_use]
pub fn npr_gltf_plugin() -> GltfPlugin {
    GltfPlugin::default().add_npr_vertex_attributes()
}

/// glTF 初始化顺序错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StylizedGltfConfigError {
    /// `GltfPlugin` 已经加入 App：attribute 表已在 `finish` 时烤进
    /// `GltfLoader`，此时再配置会被静默丢弃，因此 fail-closed。
    #[error(
        "GltfPlugin is already added to this App; _NPR_* vertex attributes must be \
         registered on the GltfPlugin BEFORE it is added (use `npr_gltf_plugin()` or \
         `GltfPlugin::add_npr_vertex_attributes()`). Configuring after app build would \
         silently drop the attributes."
    )]
    GltfPluginAlreadyAdded,
}

/// 顺序守卫：断言本 App **尚未**加入 `GltfPlugin`。
///
/// 任何需要"在 GltfPlugin 配置前执行"的 setup 路径都应先调用本函数；
/// 错误顺序下返回 [`StylizedGltfConfigError::GltfPluginAlreadyAdded`]，
/// 而不是让 attribute 被静默丢弃。
pub fn check_gltf_plugin_not_added(app: &App) -> Result<(), StylizedGltfConfigError> {
    if app.is_plugin_added::<GltfPlugin>() {
        Err(StylizedGltfConfigError::GltfPluginAlreadyAdded)
    } else {
        Ok(())
    }
}
