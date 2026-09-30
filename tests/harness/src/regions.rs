//! ROI 分区（docs/showcase/05 §9）：分区 3D AABB 经捕获相机投影自动
//! 计算屏幕矩形（不手工标定）、golden.json 的 `regions` 扩展记录、
//! 同一捕获内 ROI 两两不重叠断言（防跨区污染判定归属）。

use bevy::math::{Mat4, Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// 屏幕矩形（像素，左上角原点）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenRect {
    /// 左边缘。
    pub x: u32,
    /// 上边缘。
    pub y: u32,
    /// 宽。
    pub width: u32,
    /// 高。
    pub height: u32,
}

impl ScreenRect {
    /// 两矩形是否有正面积交集。
    #[must_use]
    pub fn intersects(&self, other: &ScreenRect) -> bool {
        self.x < other.x + other.width
            && other.x < self.x + self.width
            && self.y < other.y + other.height
            && other.y < self.y + self.height
    }
}

/// golden.json 的 regions 扩展条目（additive；缺省 = 整图判定，与现状
/// 兼容）。阈值在首次基线验收时锁定（§9：按区域分别锁定）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoldenRegion {
    /// 区域 id（分区 AABB 表/捕获规格的 `rois` 引用）。
    pub id: String,
    /// 屏幕矩形（AABB 投影自动计算）。
    pub rect: ScreenRect,
    /// per-pixel 每通道阈值。
    pub max_channel_delta: u8,
    /// 失败像素比上限。
    pub max_fail_ratio: f64,
}

/// 分区 AABB → 屏幕矩形：8 角点经 view-projection 投影的包围盒
/// （floor/ceil 取整并钳进画幅）。全部角点在相机背后 = None（不可见）。
/// 部分角点在背后时按可见角点成盒（矩形可能偏小，如实记录于注释——
/// 庭院全部 ROI 在各自捕获机位下整体在相机前方）。
#[must_use]
pub fn project_aabb_to_screen(
    min: Vec3,
    max: Vec3,
    view_proj: Mat4,
    width: u32,
    height: u32,
) -> Option<ScreenRect> {
    let mut lo = Vec2::INFINITY;
    let mut hi = Vec2::NEG_INFINITY;
    let mut visible = 0_u32;
    for x in [min.x, max.x] {
        for y in [min.y, max.y] {
            for z in [min.z, max.z] {
                let clip = view_proj * Vec3::new(x, y, z).extend(1.0);
                if clip.w <= 1e-6 {
                    continue;
                }
                let ndc = clip.truncate().truncate() / clip.w;
                let screen = Vec2::new(
                    (ndc.x + 1.0) * 0.5 * width as f32,
                    (1.0 - ndc.y) * 0.5 * height as f32,
                );
                visible += 1;
                lo = lo.min(screen);
                hi = hi.max(screen);
            }
        }
    }
    if visible == 0 {
        return None;
    }
    let x = lo.x.floor().max(0.0) as u32;
    let y = lo.y.floor().max(0.0) as u32;
    let right = hi.x.ceil().min(width as f32) as u32;
    let bottom = hi.y.ceil().min(height as f32) as u32;
    Some(ScreenRect {
        x,
        y,
        width: right.saturating_sub(x),
        height: bottom.saturating_sub(y),
    })
}

/// 捕获内 ROI 校验：两两不重叠（§9 防跨区污染判定归属）且全部落在
/// 画幅内。返回全部违规项（不短路）。
pub fn validate_regions(
    regions: &[GoldenRegion],
    width: u32,
    height: u32,
) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();
    for (index, region) in regions.iter().enumerate() {
        if region.rect.x + region.rect.width > width || region.rect.y + region.rect.height > height
        {
            violations.push(format!(
                "region {} rect {:?} exceeds the {}x{} frame",
                region.id, region.rect, width, height
            ));
        }
        for other in &regions[index + 1..] {
            if region.rect.intersects(&other.rect) {
                violations.push(format!(
                    "regions {} and {} overlap ({:?} x {:?})",
                    region.id, other.id, region.rect, other.rect
                ));
            }
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 确定性：固定相机（原点望 −Z，fov 90°）+ 固定 AABB 的投影矩形
    /// 是精确可复算的（手算值硬编码）。
    #[test]
    fn aabb_projection_is_deterministic() {
        let view_proj = Mat4::perspective_rh(90.0_f32.to_radians(), 1.0, 0.1, 100.0)
            * Mat4::look_at_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        // AABB (−1,−1,−5)..(1,1,−3)：最近面 z=−3 主导，x/y ∈ ±1/3 NDC
        // → 屏幕 33.33..66.67（100×100 画幅）→ floor/ceil = 33..67。
        let rect = project_aabb_to_screen(
            Vec3::new(-1.0, -1.0, -5.0),
            Vec3::new(1.0, 1.0, -3.0),
            view_proj,
            100,
            100,
        )
        .expect("visible");
        assert_eq!(
            rect,
            ScreenRect {
                x: 33,
                y: 33,
                width: 34,
                height: 34
            }
        );
        // 同一输入重算逐位一致（确定性）。
        assert_eq!(
            project_aabb_to_screen(
                Vec3::new(-1.0, -1.0, -5.0),
                Vec3::new(1.0, 1.0, -3.0),
                view_proj,
                100,
                100,
            ),
            Some(rect)
        );
        // 全部角点在相机背后 → None。
        assert_eq!(
            project_aabb_to_screen(
                Vec3::new(-1.0, -1.0, 3.0),
                Vec3::new(1.0, 1.0, 5.0),
                view_proj,
                100,
                100,
            ),
            None
        );
        // 出画幅的 AABB 钳进画幅。
        let clamped = project_aabb_to_screen(
            Vec3::new(-100.0, -100.0, -5.0),
            Vec3::new(100.0, 100.0, -3.0),
            view_proj,
            100,
            100,
        )
        .expect("visible");
        assert_eq!(clamped.x, 0);
        assert_eq!(clamped.y, 0);
        assert!(clamped.x + clamped.width <= 100);
        assert!(clamped.y + clamped.height <= 100);
    }

    #[test]
    fn region_validation_rejects_overlap_and_out_of_bounds() {
        let region = |id: &str, x, y, w, h| GoldenRegion {
            id: id.to_string(),
            rect: ScreenRect {
                x,
                y,
                width: w,
                height: h,
            },
            max_channel_delta: 0,
            max_fail_ratio: 0.0,
        };
        let ok = vec![region("a", 0, 0, 10, 10), region("b", 10, 0, 10, 10)];
        assert_eq!(validate_regions(&ok, 100, 100), Ok(()));
        let overlapping = vec![region("a", 0, 0, 11, 10), region("b", 10, 0, 10, 10)];
        let violations = validate_regions(&overlapping, 100, 100).expect_err("overlap");
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains('a') && violations[0].contains('b'));
        let out = vec![region("a", 90, 90, 20, 20)];
        let violations = validate_regions(&out, 100, 100).expect_err("out of bounds");
        assert_eq!(violations.len(), 1);
    }
}

#[cfg(test)]
mod capture_spec_projection_tests {
    //! 捕获规格 × 分区 AABB × 机位表的端到端（CPU）校验：每条带 ROI 的
    //! 捕获，其区域屏幕矩形（AABB 经捕获相机投影）两两不重叠——§9 的
    //! 「布局即合同」断言，布局变更必须先过它。
    use super::*;
    use bevy::math::Vec3;

    /// 捕获相机的 view-projection（Camera3d 默认：fov 45° 垂直，1:1）。
    fn view_proj(preset_id: &str) -> Mat4 {
        let preset = scenes::metadata::COURTYARD_CAMERA_PRESETS
            .iter()
            .find(|preset| preset.id == preset_id)
            .expect("camera preset");
        Mat4::perspective_rh(45.0_f32.to_radians(), 1.0, 0.1, 200.0)
            * Mat4::look_at_rh(preset.eye, preset.look, Vec3::Y)
    }

    #[test]
    fn capture_spec_rois_project_to_non_overlapping_rects() {
        for spec in scenario_model::courtyard_capture_specs() {
            if spec.rois.is_empty() {
                continue;
            }
            let view_proj = view_proj(&spec.camera);
            let regions: Vec<GoldenRegion> = spec
                .rois
                .iter()
                .map(|id| {
                    let aabb = scenes::metadata::COURTYARD_REGION_AABBS
                        .iter()
                        .find(|region| region.id == id)
                        .expect("region aabb");
                    let rect = project_aabb_to_screen(aabb.min, aabb.max, view_proj, 512, 512)
                        .unwrap_or_else(|| panic!("region {id} not visible in {}", spec.camera));
                    GoldenRegion {
                        id: id.clone(),
                        rect,
                        max_channel_delta: 0,
                        max_fail_ratio: 0.0,
                    }
                })
                .collect();
            assert_eq!(
                validate_regions(&regions, 512, 512),
                Ok(()),
                "{} must have non-overlapping rois",
                spec.capture_id
            );
        }
    }
}
