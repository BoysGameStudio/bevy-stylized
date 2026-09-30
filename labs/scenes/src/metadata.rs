//! 场景元数据（无 Bevy 依赖部分）：布局常量等内容级数据。
//!
//! 目录元数据的唯一源是 `scenario-model` 的 catalog；本模块只承载
//! 构建器与目录条目共享的内容常数。

use bevy_math::Vec3;

// ---------------------------------------------------------------------------
// courtyard（docs/showcase/05）：布局常量与相机机位（builder 与断言共用）。
// ---------------------------------------------------------------------------

/// courtyard 机位 preset（§7；运行时 `SetCameraPreset` 切换的数据源）。
///
/// v3（2026-09-30 owner 决策，展亭模式）：全部内容以中央平台为唯一舞台，
/// 机位 = 每内容的取景机位（摄影师换位、展品不动）——平台内容共用南望
/// 北的 hero 机位族，lamps 用广角机位，lod 用沿轴机位。panorama 随
/// overview 退役。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CourtyardCameraPreset {
    /// 机位 ID（§7 表；v3 与内容一一对应）。
    pub id: &'static str,
    /// 相机位置。
    pub eye: Vec3,
    /// 注视目标。
    pub look: Vec3,
}

/// §7 机位表（v3：9 个 preset，center-trio 居首为默认机位；panorama 随
/// overview 退役；v4：rim/shadow/palette/metal/cutout 五台新内容机位，
/// hero 机位族正对平台）。内容↔机位一一对应见
/// [`courtyard_zone_camera_preset`]。
pub const COURTYARD_CAMERA_PRESETS: [CourtyardCameraPreset; 14] = [
    // hero 机位族（南望北，正对展台；trio 三排纵深对照组用斜上角读排）。
    CourtyardCameraPreset {
        id: "center-trio",
        eye: Vec3::new(3.2, 3.4, 5.2),
        look: Vec3::new(0.0, 1.2, 0.0),
    },
    CourtyardCameraPreset {
        id: "ramp",
        eye: Vec3::new(0.0, 1.7, 5.2),
        look: Vec3::new(0.0, 1.28, 0.0),
    },
    CourtyardCameraPreset {
        id: "outline",
        eye: Vec3::new(0.0, 1.7, 5.4),
        look: Vec3::new(0.0, 1.35, 0.0),
    },
    // outline-west：p3（hybrid 方块堆）取景机位——v2 是 P2 的镜像位，
    // v3 两内容同台；方块堆矮小（顶 ~0.95m），取景收紧。
    CourtyardCameraPreset {
        id: "outline-west",
        eye: Vec3::new(0.0, 1.3, 3.4),
        look: Vec3::new(0.0, 0.65, 0.0),
    },
    CourtyardCameraPreset {
        id: "lighting",
        eye: Vec3::new(0.0, 1.8, 5.6),
        look: Vec3::new(0.0, 1.35, 0.0),
    },
    // flags 偏高：条带向上展开（台面到条带顶 ~3.5m）。
    CourtyardCameraPreset {
        id: "flags",
        eye: Vec3::new(0.0, 2.8, 7.0),
        look: Vec3::new(0.0, 1.9, 0.0),
    },
    // stage：角色全身（基座顶 0.8 + 角色 ~2.1m）。
    CourtyardCameraPreset {
        id: "stage",
        eye: Vec3::new(0.0, 2.0, 6.2),
        look: Vec3::new(0.0, 1.45, 0.0),
    },
    // lod-side：轴从平台向正北递退、微偏西避开灯柱列（v3 评审 F2；
    // lamps/lod 永不同时激活，无穿模）；机位在轴线东南侧斜视——
    // 侧向读带距梯度（正对轴线会球体相叠）。
    CourtyardCameraPreset {
        id: "lod-side",
        eye: Vec3::new(1.8, 2.4, 7.5),
        look: Vec3::new(-2.2, 2.2, -20.0),
    },
    // lamps：环阵广角（v2 机位沿用——灯环布局未变）。
    CourtyardCameraPreset {
        id: "lamps",
        eye: Vec3::new(0.0, 2.6, 10.0),
        look: Vec3::new(0.0, 1.8, -10.0),
    },
    // v4 新内容机位（hero 机位族，正对平台横排；cutout 稍近读镂空）。
    CourtyardCameraPreset {
        id: "rim",
        eye: Vec3::new(0.0, 1.8, 5.4),
        look: Vec3::new(0.0, 1.35, 0.0),
    },
    CourtyardCameraPreset {
        id: "shadow",
        eye: Vec3::new(0.0, 1.8, 5.4),
        look: Vec3::new(0.0, 1.35, 0.0),
    },
    CourtyardCameraPreset {
        id: "palette",
        eye: Vec3::new(0.0, 1.8, 5.4),
        look: Vec3::new(0.0, 1.35, 0.0),
    },
    CourtyardCameraPreset {
        id: "metal",
        eye: Vec3::new(0.0, 1.8, 5.4),
        look: Vec3::new(0.0, 1.35, 0.0),
    },
    CourtyardCameraPreset {
        id: "cutout",
        eye: Vec3::new(0.0, 1.8, 4.6),
        look: Vec3::new(0.0, 1.3, 0.0),
    },
];

/// courtyard 内容 id（§4/§8 zone 参数取值；v3 = 展台内容档位）。
///
/// v3（2026-09-30 owner 决策）：overview 退役——同屏全显与展亭模式互斥
/// （同位必穿模）；浏览全部内容归导览模式（G）。
pub const COURTYARD_ZONE_IDS: [&str; 14] = [
    "trio", "p1", "p2", "p3", "p4", "p5", "lod", "stage", "lamps", "rim", "shadow", "palette",
    "metal", "cutout",
];

/// 内容 → 机位 id（§7 一一对应）。捕获规格与键位循环引用同一映射
/// （camera↔内容一致性校验的数据源）。
#[must_use]
pub fn courtyard_zone_camera_preset(zone: &str) -> Option<&'static str> {
    Some(match zone {
        "trio" => "center-trio",
        "p1" => "ramp",
        "p2" => "outline",
        "p3" => "outline-west",
        "p4" => "lighting",
        "p5" => "flags",
        "lod" => "lod-side",
        "stage" => "stage",
        "lamps" => "lamps",
        "rim" => "rim",
        "shadow" => "shadow",
        "palette" => "palette",
        "metal" => "metal",
        "cutout" => "cutout",
        _ => return None,
    })
}

/// 中央平台半径（§3：半径 2.2、高 0.3 圆台）。v3：唯一舞台。
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

/// LOD 轴四球（v3：展亭模式——轴从平台西南侧出发向北递退，微偏西避开
/// 灯柱列与平台，v3 评审 F2）：(球心, 半径) × 4。
///
/// 带距相对 lod-side 机位（(0, 2.4, 8.0)）重推导：半径 = 距离 × tan(5.9°)
/// （公式不变），四球各落 near/mid/far/very_far 中带（实测距离
/// 7.29/16.55/36.3/62.3，带边余量 ≥15%——落带断言钉住）。lamps/lod 永不
/// 同时激活（overview 已退役），轴穿灯环带无共存冲突。
pub const COURTYARD_LOD_BALLS: [(Vec3, f32); 4] = [
    (Vec3::new(-3.80, 0.75, 2.00), 0.75), // near，d≈7.3（平台西南前景）
    (Vec3::new(-4.22, 1.71, -7.99), 1.71), // mid，d≈16.6（灯环带北侧）
    (Vec3::new(-5.07, 3.75, -27.97), 3.75), // far，d≈36.3（墙外）
    (Vec3::new(-6.17, 6.43, -53.95), 6.43), // very_far，d≈62.3（远景）
];

/// 分区内容 AABB 表（§9 ROI 判定的区域数据源；屏幕矩形在捕获时由
/// 本表经捕获相机投影自动计算，不手工标定）。
///
/// v3（展亭模式）：除 lamps 外的全部内容以中央平台为舞台——AABB 均为
/// 平台周边的内容外包；lamps 灯环不变。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CourtyardRegionAabb {
    /// 区域 id（捕获规格 `rois` 引用）。
    pub id: &'static str,
    /// AABB 下界。
    pub min: Vec3,
    /// AABB 上界。
    pub max: Vec3,
}

/// 内容 AABB 表（含基座/支撑结构；角色台的 y 上界是角色高度的保守
/// 估计 2.95m；p2 含 hull 外扩 ~0.1m）。
pub const COURTYARD_REGION_AABBS: [CourtyardRegionAabb; 13] = [
    // 中央平台 + 基座 + 三对照组 7 球。
    CourtyardRegionAabb {
        id: "trio",
        min: Vec3::new(-2.2, 0.0, -2.2),
        max: Vec3::new(2.2, 1.8, 2.2),
    },
    // p1：4 ramp 球横排（x ±1.05 + r0.28，台面起）。
    CourtyardRegionAabb {
        id: "p1",
        min: Vec3::new(-1.4, 0.0, -0.4),
        max: Vec3::new(1.4, 1.6, 0.4),
    },
    // p2：3 描边球横排（x ±0.75 + r0.35；宽度 8 的 hull 外扩 ~0.1m 在内）。
    CourtyardRegionAabb {
        id: "p2",
        min: Vec3::new(-1.2, 0.0, -0.45),
        max: Vec3::new(1.2, 1.8, 0.45),
    },
    // p3：方块堆（2 cuboid + 1 小球；含 hull 外扩）。
    CourtyardRegionAabb {
        id: "p3",
        min: Vec3::new(-0.5, 0.3, -0.45),
        max: Vec3::new(0.55, 1.05, 0.5),
    },
    // p4：3 光照对照球横排（x ±1.0 + r0.35）。
    CourtyardRegionAabb {
        id: "p4",
        min: Vec3::new(-1.4, 0.0, -0.4),
        max: Vec3::new(1.4, 1.75, 0.4),
    },
    // p5：双旗杆（x ±0.4）+ 旗面（含 posed 旋转 0.5 rad 的摆动极值）。
    CourtyardRegionAabb {
        id: "p5",
        min: Vec3::new(-1.7, 0.3, -0.35),
        max: Vec3::new(1.5, 3.6, 0.35),
    },
    // 灯阵：北半环 r8.5–11 全部灯柱 + 灯球（v3 不变）。
    CourtyardRegionAabb {
        id: "lamps",
        min: Vec3::new(-11.12, 0.0, -11.12),
        max: Vec3::new(11.12, 1.84, 0.0),
    },
    // 角色台（台基座 r1.5 在台面上）+ GLB 角色（高度估计）。
    CourtyardRegionAabb {
        id: "stage",
        min: Vec3::new(-1.5, 0.3, -1.5),
        max: Vec3::new(1.5, 2.95, 1.5),
    },
    // v4 新内容（平台横排三球 + 基座）。
    CourtyardRegionAabb {
        id: "rim",
        min: Vec3::new(-1.2, 0.0, -0.4),
        max: Vec3::new(1.2, 1.75, 0.4),
    },
    CourtyardRegionAabb {
        id: "shadow",
        min: Vec3::new(-1.2, 0.0, -0.4),
        max: Vec3::new(1.2, 1.75, 0.4),
    },
    CourtyardRegionAabb {
        id: "palette",
        min: Vec3::new(-1.2, 0.0, -0.4),
        max: Vec3::new(1.2, 1.75, 0.4),
    },
    CourtyardRegionAabb {
        id: "metal",
        min: Vec3::new(-1.2, 0.0, -0.4),
        max: Vec3::new(1.2, 1.75, 0.4),
    },
    // cutout：双卡片（x ±0.55，竖卡）。
    CourtyardRegionAabb {
        id: "cutout",
        min: Vec3::new(-1.1, 0.3, -0.15),
        max: Vec3::new(1.1, 1.6, 0.15),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_stylized_core::{StylizedLodThresholds, resolve_lod_band};

    /// courtyard LOD 轴（v3 展亭坐标）：相对 lod-side 机位四球各落
    /// near/mid/far/very_far 中带，且离带边界 ≥15% 带宽；与中央平台
    /// （r2.2，台面 y0.3）无重叠。
    #[test]
    fn courtyard_lod_balls_land_centered_in_four_bands() {
        let thresholds = StylizedLodThresholds::default();
        let expected = [0, 1, 2, 3];
        let lod_eye = COURTYARD_CAMERA_PRESETS
            .iter()
            .find(|preset| preset.id == "lod-side")
            .expect("lod-side preset")
            .eye;
        for (i, (center, radius)) in COURTYARD_LOD_BALLS.iter().enumerate() {
            let actual = (*center - lod_eye).length();
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
            // 与中央平台（r2.2）无重叠（展台净空的几何前提）。
            let platform_distance = Vec3::new(center.x, 0.0, center.z).length();
            assert!(
                platform_distance > COURTYARD_PLATFORM_RADIUS + radius,
                "lod ball {i} overlaps the platform: {platform_distance}"
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

    /// courtyard 机位表：center-trio 居首（默认内容机位），ID 唯一，
    /// 内容↔机位映射全覆盖且双向可解析（v3：9 机位，panorama 随
    /// overview 退役）。
    #[test]
    fn courtyard_camera_presets_are_unique_with_center_trio_first() {
        assert_eq!(COURTYARD_CAMERA_PRESETS[0].id, "center-trio");
        let mut ids = std::collections::BTreeSet::new();
        for preset in COURTYARD_CAMERA_PRESETS {
            assert!(ids.insert(preset.id), "duplicate preset {}", preset.id);
        }
        assert_eq!(ids.len(), COURTYARD_CAMERA_PRESETS.len());
        // 内容↔机位：每个内容 id 解析到存在的机位。
        for zone in COURTYARD_ZONE_IDS {
            let preset = courtyard_zone_camera_preset(zone).expect("zone resolves");
            assert!(
                COURTYARD_CAMERA_PRESETS
                    .iter()
                    .any(|entry| entry.id == preset),
                "zone {zone} camera {preset} not in preset table"
            );
        }
        assert_eq!(courtyard_zone_camera_preset("overview"), None);
        assert_eq!(courtyard_zone_camera_preset("nowhere"), None);
    }

    /// 内容 AABB 表：包含对应内容的全部球体/基座/支撑结构（v3：全部
    /// 内容在中央平台上）。
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
        // 内容球心 ± 半径（v3 平台摆位：p1 x±1.05 / p2 x±0.75 / p4 x±1，
        // 球心高 1.28/1.35/1.35，z=0）。
        for (id, xs, y, radius) in [
            ("p1", vec![-1.05, -0.35, 0.35, 1.05], 1.28, 0.28),
            ("p2", vec![-0.75, 0.0, 0.75], 1.35, 0.35),
            ("p4", vec![-1.0, 0.0, 1.0], 1.35, 0.35),
        ] {
            let stand = region(id);
            for x in xs {
                let center = Vec3::new(x, y, 0.0);
                assert!(
                    contains(stand, center - Vec3::splat(radius), 0.0)
                        && contains(stand, center + Vec3::splat(radius), 0.0),
                    "{id} ball at {center}"
                );
            }
        }
        // p5：旗杆顶（台面起 2.9m）与 posed 旗面极值。
        let flags = region("p5");
        assert!(contains(flags, Vec3::new(-0.4, 2.9, 0.0), 0.0));
        assert!(contains(flags, Vec3::new(0.4, 2.9, 0.0), 0.0));
        assert!(contains(flags, Vec3::new(1.35, 3.5, 0.2), 0.05));
        // p3：方块堆三件（大盒 0.6×0.25×0.5 @y0.425、小盒 0.3³ @y0.7、
        // 球 r0.18 @(0.15,0.73,0.1)）。
        let pile = region("p3");
        for corner in [
            Vec3::new(-0.3, 0.3, -0.25),
            Vec3::new(0.3, 0.55, 0.25),
            Vec3::new(-0.25, 0.55, -0.1),
            Vec3::new(0.05, 0.85, 0.2),
            Vec3::new(0.33, 0.91, 0.28),
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
        // 角色台（台面上基座 r1.5 + 角色）。
        let stage = region("stage");
        assert!(contains(stage, Vec3::new(1.5, 0.8, 0.0), 0.0));
        assert!(contains(stage, Vec3::new(0.0, 2.95, 0.0), 0.0));
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
