//! 场景元数据（无 Bevy 依赖部分）：布局常量等内容级数据。
//!
//! 目录元数据的唯一源是 `scenario-model` 的 catalog；本模块只承载
//! 构建器与目录条目共享的内容常数。

use bevy_math::Vec3;

// ---------------------------------------------------------------------------
// courtyard（docs/showcase/05）：布局常量与相机机位（builder 与断言共用）。
// ---------------------------------------------------------------------------

/// courtyard 全景（panorama）默认机位眼点（§7）。
pub const COURTYARD_PANORAMA_EYE: Vec3 = Vec3::new(0.0, 3.4, 16.0);

/// courtyard 机位 preset（§7；运行时 `SetCameraPreset` 切换的数据源）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CourtyardCameraPreset {
    /// 机位 ID（panorama / center-trio / outline / lod-side / flags）。
    pub id: &'static str,
    /// 相机位置。
    pub eye: Vec3,
    /// 注视目标。
    pub look: Vec3,
}

/// §7 机位 + outline-west（§9 验收期新增：hybrid 捕获的 P3 专属机位，
/// 见该常量旁的修正注记）。panorama 为默认，居首。
pub const COURTYARD_CAMERA_PRESETS: [CourtyardCameraPreset; 6] = [
    CourtyardCameraPreset {
        id: "panorama",
        eye: Vec3::new(0.0, 3.4, 16.0),
        look: Vec3::new(0.0, 1.3, -2.0),
    },
    CourtyardCameraPreset {
        id: "center-trio",
        eye: Vec3::new(3.5, 4.0, 4.0),
        look: Vec3::new(0.0, 1.25, -0.2),
    },
    CourtyardCameraPreset {
        id: "outline",
        eye: Vec3::new(4.2, 1.8, 6.8),
        look: Vec3::new(2.75, 1.4, 4.76),
    },
    // outline-west：outline 的镜像机位，指向 P3（西南，hybrid 方块堆）。
    // §7 原 outline 机位只覆盖 P2；hybrid 捕获（方块堆 hull + 屏幕空间
    // 内边缘）需要 P3 为主体的专属机位——单机位兼顾两展台的 45° fov
    // 取景会把两区压到画幅边缘且球径 ~26px，宽度梯度不可读。
    CourtyardCameraPreset {
        id: "outline-west",
        eye: Vec3::new(-4.2, 1.8, 6.8),
        look: Vec3::new(-2.75, 1.4, 4.76),
    },
    CourtyardCameraPreset {
        id: "lod-side",
        eye: Vec3::new(70.0, 7.0, -8.0),
        look: Vec3::new(-4.6, 2.5, -17.3),
    },
    CourtyardCameraPreset {
        id: "flags",
        eye: Vec3::new(0.0, 2.4, -0.4),
        look: Vec3::new(-2.75, 2.0, -4.76),
    },
];

/// 展台环（§3：半径 5.5，60° 均布 6 座；0.8×0.8×1.0 方柱，顶面 y=1.0）。
/// 顺序 = 方位角 0°/60°/…/300°（自 +X 逆时针）：P1 东 / P2 东南 / P3 西南 /
/// P4 西 / P5 西北 / 东北（无指派展品，§4 只指派 5 座）。
pub const COURTYARD_PODIUM_RING: [Vec3; 6] = [
    Vec3::new(5.5, 0.0, 0.0),
    Vec3::new(2.75, 0.0, 4.7631),
    Vec3::new(-2.75, 0.0, 4.7631),
    Vec3::new(-5.5, 0.0, 0.0),
    Vec3::new(-2.75, 0.0, -4.7631),
    Vec3::new(2.75, 0.0, -4.7631),
];

/// 展台占位净空半径（0.8×0.8 方柱半对角线 0.566 取整上界）。
pub const COURTYARD_PODIUM_CLEARANCE: f32 = 0.6;

/// 中央平台半径（§3：半径 2.2、高 0.3 圆台）。
pub const COURTYARD_PLATFORM_RADIUS: f32 = 2.2;

/// 中央平台三对照组（§4）：(球心, 半径) × 7。
/// 排轴平行 X，从前到后：face 前排 z=+0.9 / hair 中排 z=0 / metal 后排
/// z=−0.9。排距相对设计文档的 ±0.6 放宽到 ±0.9——文档坐标下中/后排同列
/// 球心距 0.6 < 半径和 0.8（几何重叠），以「球心不重叠」断言为准修正；
/// 球心 y=1.32（face，r=0.32）/ 1.40（hair/metal，r=0.40）按文档锁定。
pub const COURTYARD_TRIO_BALLS: [(Vec3, f32); 7] = [
    (Vec3::new(-0.75, 1.32, 0.9), 0.32),
    (Vec3::new(0.0, 1.32, 0.9), 0.32),
    (Vec3::new(0.75, 1.32, 0.9), 0.32),
    (Vec3::new(-0.5, 1.4, 0.0), 0.40),
    (Vec3::new(0.5, 1.4, 0.0), 0.40),
    (Vec3::new(-0.5, 1.4, -0.9), 0.40),
    (Vec3::new(0.5, 1.4, -0.9), 0.40),
];

/// LOD 轴四球（§6 验证坐标表的 panorama 修正 + 西侧轨道版）：(球心,
/// 半径) × 4。
///
/// 文档 §6 表的距离列（6.5/16.5/34/60）反推自**已废弃**的 eye z=11.5
/// 方案；相对现行 panorama eye (0, 3.4, 16.0) 原坐标的 near 球实距
/// 10.6m，落入 mid 带。后经基线捕获检视：中轴布置的 far/very_far 球
/// 在全景遮挡角色台、near 球卡在画框左下边缘。现版把 LOD 轴整体移到
/// 西侧轨道，保留文档的目标距离与角直径（半径 = 距离 × tan(5.9°)，
/// 球心 y = 半径），四球相对 panorama eye 各落 near/mid/far/very_far
/// 中带（实距 7.24/19.03/34.53/60.60，余量 21%–40%），且与展台环、
/// 中央平台、角色台 (0,0,−8) 无重叠。
pub const COURTYARD_LOD_BALLS: [(Vec3, f32); 4] = [
    (Vec3::new(-1.2, 0.67, 9.4), 0.67), // near，d≈7.2（南门通道前景，下中部入镜）
    (Vec3::new(-4.2, 1.95, -2.5), 1.95), // mid，d≈19.0（展台环西/西北间空档）
    (Vec3::new(-6.0, 3.51, -18.0), 3.51), // far，d≈34.5（西侧墙外）
    (Vec3::new(-8.0, 6.21, -44.0), 6.21), // very_far，d≈60.6（西侧远景）
];

/// courtyard 分区 AABB（§9 ROI 判定的区域数据源；屏幕矩形在捕获时由
/// 本表经捕获相机投影自动计算，不手工标定）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CourtyardRegionAabb {
    /// 区域 id（捕获规格 `rois` 引用）。
    pub id: &'static str,
    /// AABB 下界。
    pub min: Vec3,
    /// AABB 上界。
    pub max: Vec3,
}

/// 分区 AABB 表（含展台/平台/基座等支撑结构；角色台的 y 上界是角色
/// 高度的保守估计 2.5m；p3 含 hull 外扩 ~0.05m）。
pub const COURTYARD_REGION_AABBS: [CourtyardRegionAabb; 8] = [
    // 中央平台 + 基座 + 三对照组 7 球。
    CourtyardRegionAabb {
        id: "trio",
        min: Vec3::new(-2.2, 0.0, -2.2),
        max: Vec3::new(2.2, 1.8, 2.2),
    },
    // P1：展台 + 4 ramp 球。
    CourtyardRegionAabb {
        id: "p1",
        min: Vec3::new(4.16, 0.0, -0.4),
        max: Vec3::new(6.84, 1.56, 0.4),
    },
    // P2：展台 + 3 描边球（宽度 8 的 hull 外扩 ~0.1m 已在 max 内）。
    CourtyardRegionAabb {
        id: "p2",
        min: Vec3::new(2.35, 0.0, 3.66),
        max: Vec3::new(3.15, 1.7, 5.86),
    },
    // P3：展台 + 方块堆（2 cuboid + 1 小球；含 hull 外扩）。
    CourtyardRegionAabb {
        id: "p3",
        min: Vec3::new(-3.2, 0.0, 4.31),
        max: Vec3::new(-2.3, 1.66, 5.21),
    },
    // P4：展台 + 3 光照对照球。
    CourtyardRegionAabb {
        id: "p4",
        min: Vec3::new(-6.86, 0.0, -0.4),
        max: Vec3::new(-4.14, 1.7, 0.4),
    },
    // P5：展台 + 旗杆 + 旗面（含 posed 旋转 0.5 rad 的摆动极值
    // x≈−4.21/y≈3.5）。
    CourtyardRegionAabb {
        id: "p5",
        min: Vec3::new(-4.25, 0.0, -5.17),
        max: Vec3::new(-1.45, 3.65, -4.36),
    },
    // 灯阵：北半环 r8.5–11 全部灯柱 + 灯球。
    CourtyardRegionAabb {
        id: "lamps",
        min: Vec3::new(-11.12, 0.0, -11.12),
        max: Vec3::new(11.12, 1.84, 0.0),
    },
    // 角色台 + GLB 角色（高度估计）。
    CourtyardRegionAabb {
        id: "stage",
        min: Vec3::new(-1.5, 0.0, -9.5),
        max: Vec3::new(1.5, 2.5, -6.5),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_stylized_core::{StylizedLodThresholds, resolve_lod_band};

    /// courtyard LOD 轴（§6 修正版坐标）：相对 panorama eye 四球各落
    /// near/mid/far/very_far 中带，且离带边界 ≥15% 带宽。
    #[test]
    fn courtyard_lod_balls_land_centered_in_four_bands() {
        let thresholds = StylizedLodThresholds::default();
        let expected = [0, 1, 2, 3];
        for (i, (center, radius)) in COURTYARD_LOD_BALLS.iter().enumerate() {
            let actual = (*center - COURTYARD_PANORAMA_EYE).length();
            let band = resolve_lod_band(actual, &thresholds);
            assert_eq!(band, expected[i], "lod ball {i} distance {actual}");
            let bounds = [
                0.0,
                thresholds.mid_start,
                thresholds.far_start,
                thresholds.very_far_start,
                f32::INFINITY,
            ];
            let lower = bounds[band as usize];
            let upper = bounds[band as usize + 1];
            let width = if upper.is_finite() {
                upper - lower
            } else {
                lower
            };
            let margin = (actual - lower).min(upper - actual) / width;
            assert!(
                margin >= 0.15,
                "lod ball {i} too close to band edge: distance {actual}, margin {margin}"
            );
            // 与角色台（(0,0,−8)，r=1.5）无重叠（中轴无遮挡的几何前提）。
            let stage = Vec3::new(0.0, 0.0, -8.0);
            let stage_distance = Vec3::new(center.x - stage.x, 0.0, center.z - stage.z).length();
            assert!(
                stage_distance > radius + 1.5,
                "lod ball {i} overlaps the character stage: {stage_distance}"
            );
        }
    }

    /// courtyard 中央平台三对照组：7 球两两无重叠（球心距 > 半径和）。
    #[test]
    fn courtyard_trio_balls_do_not_overlap() {
        for (i, (center_a, radius_a)) in COURTYARD_TRIO_BALLS.iter().enumerate() {
            for (j, (center_b, radius_b)) in COURTYARD_TRIO_BALLS.iter().enumerate().skip(i + 1) {
                let distance = (*center_a - *center_b).length();
                assert!(
                    distance > radius_a + radius_b,
                    "trio balls {i}/{j} overlap: distance {distance}, radii {}",
                    radius_a + radius_b
                );
            }
        }
    }

    /// courtyard 展台环 6 座：与中央平台（r=2.2）及 LOD 轴四球无重叠
    /// （XZ 平面；展台净空半径 COURTYARD_PODIUM_CLEARANCE）。
    #[test]
    fn courtyard_podium_ring_clears_platform_and_lod_axis() {
        for (i, podium) in COURTYARD_PODIUM_RING.iter().enumerate() {
            let platform_distance = Vec3::new(podium.x, 0.0, podium.z).length();
            assert!(
                platform_distance > COURTYARD_PLATFORM_RADIUS + COURTYARD_PODIUM_CLEARANCE,
                "podium {i} overlaps the central platform: {platform_distance}"
            );
            for (j, (center, radius)) in COURTYARD_LOD_BALLS.iter().enumerate() {
                let distance = Vec3::new(podium.x - center.x, 0.0, podium.z - center.z).length();
                assert!(
                    distance > radius + COURTYARD_PODIUM_CLEARANCE,
                    "podium {i} overlaps lod ball {j}: distance {distance}, clearance {}",
                    radius + COURTYARD_PODIUM_CLEARANCE
                );
            }
        }
    }

    /// courtyard 机位表：panorama 居首且与 COURTYARD_PANORAMA_EYE 一致，
    /// ID 唯一。
    #[test]
    fn courtyard_camera_presets_are_unique_with_panorama_first() {
        assert_eq!(COURTYARD_CAMERA_PRESETS[0].id, "panorama");
        assert_eq!(COURTYARD_CAMERA_PRESETS[0].eye, COURTYARD_PANORAMA_EYE);
        let mut ids = std::collections::BTreeSet::new();
        for preset in COURTYARD_CAMERA_PRESETS {
            assert!(ids.insert(preset.id), "duplicate preset {}", preset.id);
        }
    }

    /// 分区 AABB 表：包含对应分区的全部球体/展台/支撑结构。
    #[test]
    fn courtyard_region_aabbs_contain_their_zones() {
        let region = |id: &str| {
            COURTYARD_REGION_AABBS
                .iter()
                .find(|region| region.id == id)
                .expect("region")
        };
        let contains = |region: &CourtyardRegionAabb, point: Vec3, pad: f32| {
            point.x >= region.min.x - pad
                && point.x <= region.max.x + pad
                && point.y >= region.min.y - pad
                && point.y <= region.max.y + pad
                && point.z >= region.min.z - pad
                && point.z <= region.max.z + pad
        };
        // 三对照组 7 球 + 中央平台。
        let trio = region("trio");
        for (center, radius) in COURTYARD_TRIO_BALLS {
            for corner in [center - Vec3::splat(radius), center + Vec3::splat(radius)] {
                assert!(contains(trio, corner, 0.0), "trio ball at {center}");
            }
        }
        assert!(
            contains(trio, Vec3::new(2.2, 0.0, 2.2), 0.0),
            "platform rim"
        );
        // 展台柱（0.8×1.0×0.8，顶面 y=1.0）在各自分区 AABB 内。
        for (id, podium_index) in [("p1", 0), ("p2", 1), ("p3", 2), ("p4", 3), ("p5", 4)] {
            let podium = COURTYARD_PODIUM_RING[podium_index];
            let stand = region(id);
            for corner in [
                Vec3::new(podium.x - 0.4, 0.0, podium.z - 0.4),
                Vec3::new(podium.x + 0.4, 1.0, podium.z + 0.4),
            ] {
                assert!(contains(stand, corner, 0.0), "{id} podium");
            }
        }
        // 分区展品球心 ± 半径。
        for (id, centers, radius) in [
            (
                "p1",
                vec![
                    Vec3::new(4.45, 1.28, 0.0),
                    Vec3::new(5.15, 1.28, 0.0),
                    Vec3::new(5.85, 1.28, 0.0),
                    Vec3::new(6.55, 1.28, 0.0),
                ],
                0.28,
            ),
            (
                "p2",
                vec![
                    Vec3::new(2.75, 1.35, 4.01),
                    Vec3::new(2.75, 1.35, 4.7631),
                    Vec3::new(2.75, 1.35, 5.51),
                ],
                0.35,
            ),
            (
                "p4",
                vec![
                    Vec3::new(-6.5, 1.35, 0.0),
                    Vec3::new(-5.5, 1.35, 0.0),
                    Vec3::new(-4.5, 1.35, 0.0),
                ],
                0.35,
            ),
        ] {
            let stand = region(id);
            for center in centers {
                assert!(
                    contains(stand, center - Vec3::splat(radius), 0.0)
                        && contains(stand, center + Vec3::splat(radius), 0.0),
                    "{id} ball at {center}"
                );
            }
        }
        // P5：旗杆顶 (y=3.6) 与 posed 旗面极值（root (−1.9,1.2) 起
        // 旋转 0.5 rad 的最远角点 x≈−4.21）。
        let flags = region("p5");
        assert!(contains(flags, Vec3::new(-3.15, 3.6, -4.7631), 0.0));
        assert!(contains(flags, Vec3::new(-4.21, 3.5, -4.7631), 0.05));
        // P3：方块堆三件（大盒 0.6×0.25×0.5 @y1.125、小盒 0.3³ @y1.4、
        // 球 r0.18 @(−2.6,1.43,4.86)）。
        let pile = region("p3");
        for corner in [
            Vec3::new(-3.05, 1.0, 4.51),
            Vec3::new(-2.45, 1.25, 5.01),
            Vec3::new(-3.0, 1.55, 4.66),
            Vec3::new(-2.42, 1.61, 5.04),
        ] {
            assert!(contains(pile, corner, 0.0), "p3 pile corner {corner}");
        }
        // 灯阵：环带四至（r 8.5–11 + 灯球 r0.12，北半环）。
        let lamps = region("lamps");
        for corner in [
            Vec3::new(11.12, 1.84, 0.0),
            Vec3::new(-11.12, 1.84, 0.0),
            Vec3::new(0.0, 1.84, -11.12),
            Vec3::new(8.5, 0.0, -8.5),
        ] {
            assert!(contains(lamps, corner, 0.01), "lamp ring corner {corner}");
        }
        // 角色台。
        let stage = region("stage");
        assert!(contains(stage, Vec3::new(1.5, 0.5, -8.0), 0.0));
        assert!(contains(stage, Vec3::new(0.0, 0.0, -9.5), 0.0));
        // AABB 合法性（min < max、id 唯一）。
        let mut ids = std::collections::BTreeSet::new();
        for region in COURTYARD_REGION_AABBS {
            assert!(region.min.cmplt(region.max).all(), "{} inverted", region.id);
            assert!(ids.insert(region.id), "duplicate region {}", region.id);
        }
    }

    /// 捕获规格交叉校验：机位 id 与 ROI 区域 id 都能解析到本层数据。
    #[test]
    fn capture_specs_resolve_to_presets_and_regions() {
        for spec in scenario_model::courtyard_capture_specs() {
            assert!(
                COURTYARD_CAMERA_PRESETS
                    .iter()
                    .any(|preset| preset.id == spec.camera),
                "unknown camera preset {}",
                spec.camera
            );
            for roi in &spec.rois {
                assert!(
                    COURTYARD_REGION_AABBS.iter().any(|region| region.id == roi),
                    "unknown region {roi}"
                );
            }
        }
    }
}
