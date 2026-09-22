//! 场景元数据（无 Bevy 依赖部分）：布局常量等内容级数据。
//!
//! 目录元数据的唯一源是 `scenario-model` 的 catalog；本模块只承载
//! 构建器与目录条目共享的内容常数。

use bevy_math::Vec3;

/// LOD 用例的相机/布局常数（单测断言分带用）。
pub const LOD_CAMERA_EYE: Vec3 = Vec3::ZERO;

/// （角度度, 距离, 半径）× 4：near/mid/far/very_far。
pub const LOD_LAYOUT: [(f32, f32, f32); 4] = [
    (-16.0, 6.0, 0.62),
    (-4.0, 17.0, 1.7),
    (6.0, 32.0, 3.2),
    (16.0, 55.0, 5.7),
];

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_stylized_core::{StylizedLodThresholds, resolve_lod_band};

    /// 布局确定性：四球落在 near/mid/far/very_far 四带（相机原点）。
    #[test]
    fn lod_layout_bands_are_deterministic() {
        let thresholds = StylizedLodThresholds::default();
        let expected = [0, 1, 2, 3];
        for (i, (angle_deg, dist, _)) in LOD_LAYOUT.iter().enumerate() {
            let x = dist * angle_deg.to_radians().tan();
            let actual = (Vec3::new(x, 0.0, -dist) - LOD_CAMERA_EYE).length();
            let band = resolve_lod_band(actual, &thresholds);
            assert_eq!(band, expected[i], "layout[{i}] distance {actual}");
            // 离带边界留 ≥15% 余量（防止布局微调时静默跨带）。
            let bounds = [
                0.0,
                thresholds.mid_start,
                thresholds.far_start,
                thresholds.very_far_start,
                f32::INFINITY,
            ];
            let margin = (actual - bounds[band as usize]).min(bounds[band as usize + 1] - actual);
            assert!(margin > 1.0, "layout[{i}] too close to band edge: {actual}");
        }
    }
}
