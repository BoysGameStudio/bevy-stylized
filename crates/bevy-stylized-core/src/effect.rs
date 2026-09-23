//! 效果网格（effect mesh）的光照基数据（slice 2，2026-09-23）。
//!
//! 来源：上游 HoyoToon "Light Vectors" Geometry Nodes 修饰器把动态光照装置
//! 烘焙成**每网格一组**的命名属性（lightDir/headForward/headUp——同一网格
//! 内逐顶点恒定）。导出链求值后作为标量写入 handoff/sidecar，不做逐顶点
//! 顶点属性传输（同一网格内逐顶点恒定，顶点属性只会复制 682/484 份）。
//!
//! 语义（与 docs/reference/07 及导出脚本一致）：
//!
//! - `light_direction`：打包光照方位（上游为打包值；当前锁定源记录为
//!   `[-π/2, 0, 0]` 弧度方位角编码——不是单位向量，使用方必须按上游
//!   "Body Shader" 的 NdotL 解码语义解释）；
//! - `head_forward` / `head_up`：头部基准向量（face 光照同一坐标约定：
//!   导出侧 `[x, z, -y]` 转换）。

use bevy_reflect::Reflect;

/// 单个效果网格的光照基（per-mesh uniform；不是逐顶点数据）。
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StylizedEffectLightBasis {
    /// 打包光照方向（上游编码；当前锁定源 = `[-π/2, 0, 0]`）。
    pub light_direction: [f32; 3],
    /// 头部前向（face 光照基准；导出侧已完成 `[x, z, -y]` 转换）。
    pub head_forward: [f32; 3],
    /// 头部上向（face 光照基准）。
    pub head_up: [f32; 3],
}
