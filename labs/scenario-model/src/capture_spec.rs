//! 捕获规格（docs/showcase/05 §9）：courtyard 十条捕获的机读形式。
//!
//! 每条捕获是一份纯数据记录——lane 控制文件与 golden 目录靠它寻址；
//! 规格是数据，新增捕获不改 harness 代码。判定模式 =
//! 整图宽松（non-black + luma/chroma 包络）或整图精确，外加可选 ROI
//! 分区精确判定（区域 id 引用 `scenes::metadata::COURTYARD_REGION_AABBS`，
//! 屏幕矩形在捕获时由分区 AABB 经捕获相机投影自动计算——不手工标定）。
//!
//! 捕获 ID 只允许 `-`（`_`↔`-` 派生约定：lane 从 `showcase_golden_<id
//! 下划线形>` 测试名反解捕获 ID）。参数钉值：§8 参数集的完整钉值在
//! 摄入时展开（未钉参数取 §0 默认状态），本表对五个枚举/数值键全部
//! 显式钉值，face_threshold 三键缺省（null = 三阈值梯度）。

use serde::{Deserialize, Serialize};

use bevy_stylized_core::axis::StylizedExecutionTier;

use crate::canon::ParameterMap;
use crate::ident::SceneId;

/// 整图判定档位（§9 方案 A：宽松抓整体崩坏，精确做像素判定）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum WholeFrameJudgment {
    /// 宽松：non-black + 全图 luma/chroma 统计包络。
    Loose,
    /// 精确：per-pixel 每通道阈值 + 失败像素比双层门限。
    Strict,
}

/// 单条捕获规格。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CaptureSpec {
    /// 捕获身份（只允许 `-`）。
    pub capture_id: String,
    /// 场景（catalog 注册键）。
    pub scene: SceneId,
    /// 机位 id（§7；场景的机位表，courtyard = `COURTYARD_CAMERA_PRESETS`）。
    pub camera: String,
    /// 参数钉值（§8 键；未钉 = §0 默认）。
    pub parameters: ParameterMap,
    /// tier 桶（缺省 Portable；Standard 桶只用于 tier 门控捕获）。
    pub tier: Option<StylizedExecutionTier>,
    /// 整图判定档位。
    pub whole_frame: WholeFrameJudgment,
    /// ROI 分区精确判定（区域 id 表；空 = 只整图判定）。
    pub rois: Vec<String>,
}

/// 捕获规格校验失败。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CaptureSpecError {
    /// 捕获 ID 含 `_` 或为空（`_`↔`-` 派生约定）。
    #[error("capture id {0:?} must be non-empty and use only '-' separators")]
    IllegalCaptureId(String),
    /// 机位 id 为空。
    #[error("capture {0} has an empty camera id")]
    EmptyCamera(String),
    /// ROI 区域 id 为空或重复。
    #[error("capture {0} has empty or duplicate roi ids")]
    IllegalRois(String),
    /// 重复捕获 ID。
    #[error("duplicate capture id: {0}")]
    DuplicateId(String),
    /// 参数钉值未通过场景的 catalog 参数 schema 校验。
    #[error("capture {capture} parameter error: {reason}")]
    IllegalParameters {
        /// 捕获 ID。
        capture: String,
        /// 原因。
        reason: String,
    },
}

/// 校验捕获规格集合（结构 + 参数 schema 交叉校验）。
pub fn validate_capture_specs(specs: &[CaptureSpec]) -> Result<(), CaptureSpecError> {
    let mut ids = std::collections::BTreeSet::new();
    for spec in specs {
        if spec.capture_id.is_empty() || spec.capture_id.contains('_') {
            return Err(CaptureSpecError::IllegalCaptureId(spec.capture_id.clone()));
        }
        if !ids.insert(spec.capture_id.clone()) {
            return Err(CaptureSpecError::DuplicateId(spec.capture_id.clone()));
        }
        if spec.camera.is_empty() {
            return Err(CaptureSpecError::EmptyCamera(spec.capture_id.clone()));
        }
        let mut rois = std::collections::BTreeSet::new();
        if spec.rois.iter().any(|roi| roi.is_empty())
            || !spec.rois.iter().all(|roi| rois.insert(roi))
        {
            return Err(CaptureSpecError::IllegalRois(spec.capture_id.clone()));
        }
        if let Some(entry) = crate::catalog::catalog_entry(spec.scene.as_str()) {
            crate::catalog::validate_parameters(&entry, &spec.parameters).map_err(|error| {
                CaptureSpecError::IllegalParameters {
                    capture: spec.capture_id.clone(),
                    reason: error.to_string(),
                }
            })?;
        }
    }
    Ok(())
}

/// courtyard 的十七条捕获（§9 矩阵的机读形式；顺序固定；v4 增 5 条 cel 特性档）。
///
/// v2（2026-09-29 owner 决策改版）：分区激活——每条捕获钉 `zone`，
/// camera 与 zone 一一对应（一致性由 metadata 层的
/// `courtyard_zone_camera_preset` 校验，见 scenes metadata 测试）。
/// v1 的全景系捕获（courtyard-day 等 panorama 机位 + ROI 集）随
/// 「不同屏全显」决策退役；判定收敛为整图精确为主（一帧一主体），
/// ROI 机制保留（courtyard-night 的 lamps 区域双层门限）。
#[must_use]
pub fn courtyard_capture_specs() -> Vec<CaptureSpec> {
    let spec = |id: &str,
                camera: &str,
                parameters: ParameterMap,
                tier: Option<StylizedExecutionTier>,
                whole_frame: WholeFrameJudgment,
                rois: &[&str]| {
        CaptureSpec {
            capture_id: id.to_string(),
            scene: SceneId::new("courtyard").expect("scene id"),
            camera: camera.to_string(),
            parameters,
            tier,
            whole_frame,
            rois: rois.iter().map(|roi| roi.to_string()).collect(),
        }
    };
    let pins = |zone: &str, time: &str, outline: &str, crowd: u64, post: bool, flags: &str| {
        ParameterMap::from([
            ("zone".to_string(), serde_json::json!(zone)),
            ("time_of_day".to_string(), serde_json::json!(time)),
            ("outline_mode".to_string(), serde_json::json!(outline)),
            ("crowd".to_string(), serde_json::json!(crowd)),
            ("post".to_string(), serde_json::json!(post)),
            ("flags".to_string(), serde_json::json!(flags)),
        ])
    };
    let day = |zone: &str| pins(zone, "day", "hull", 100, false, "rest");
    vec![
        spec(
            "courtyard-trio",
            "center-trio",
            day("trio"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-ramp",
            "ramp",
            day("p1"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-outline-hull",
            "outline",
            day("p2"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        // hybrid（v2 目标 = 整图精确）：v1 的噪声源（背景灯球/墙沿剪影的
        // 屏幕空间边缘翻转）随分区激活结构性消除——P3 视图背景只剩环境壳。
        // 若 host 噪声在纯净背景下复现，按 §9 噪声口径回退 loose+p3 ROI
        // 并记录证据，不预先放宽。
        spec(
            "courtyard-outline-hybrid",
            "outline-west",
            pins("p3", "day", "hybrid", 100, false, "rest"),
            Some(StylizedExecutionTier::Standard),
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-lighting",
            "lighting",
            day("p4"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-flags",
            "flags",
            pins("p5", "day", "hull", 100, false, "posed"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-stage",
            "stage",
            day("stage"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-lod",
            "lod-side",
            day("lod"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-dusk",
            "lighting",
            pins("p4", "dusk", "hull", 100, false, "rest"),
            None,
            WholeFrameJudgment::Loose,
            &[],
        ),
        spec(
            "courtyard-night",
            "lamps",
            pins("lamps", "night", "hull", 100, false, "rest"),
            None,
            WholeFrameJudgment::Loose,
            &["lamps"],
        ),
        spec(
            "courtyard-post",
            "center-trio",
            pins("trio", "day", "hull", 100, true, "rest"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-stress",
            "lamps",
            pins("lamps", "night", "hull", 1000, false, "rest"),
            None,
            WholeFrameJudgment::Loose,
            &[],
        ),
        // V4 cel 特性补全（2026-09-30，调研缺口收编；纯增量——既有 12 条
        // 规格与基线不动）。
        spec(
            "courtyard-rim",
            "rim",
            day("rim"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-shadow",
            "shadow",
            day("shadow"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-palette",
            "palette",
            day("palette"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-metal",
            "metal",
            day("metal"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
        spec(
            "courtyard-cutout",
            "cutout",
            day("cutout"),
            None,
            WholeFrameJudgment::Strict,
            &[],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courtyard_capture_specs_are_valid_and_match_the_matrix() {
        let specs = courtyard_capture_specs();
        assert_eq!(specs.len(), 17, "§9 捕获矩阵 17 条（v3 12 + v4 5）");
        assert_eq!(validate_capture_specs(&specs), Ok(()));
        // 全 pin 键集合一致（zone + 五个枚举/数值键）。
        for spec in &specs {
            let keys: std::collections::BTreeSet<&str> =
                spec.parameters.keys().map(String::as_str).collect();
            assert_eq!(
                keys,
                ["zone", "time_of_day", "outline_mode", "crowd", "post", "flags"]
                    .into_iter()
                    .collect(),
                "{} must pin all six scalar keys",
                spec.capture_id
            );
        }
        // 抽查矩阵行：hybrid 走 Standard 桶 + 整图精确（v2 目标）；
        // stress 是 night+crowd=1000 宽松。
        let hybrid = specs
            .iter()
            .find(|spec| spec.capture_id == "courtyard-outline-hybrid")
            .expect("hybrid capture");
        assert_eq!(hybrid.tier, Some(StylizedExecutionTier::Standard));
        assert_eq!(hybrid.whole_frame, WholeFrameJudgment::Strict);
        assert_eq!(hybrid.parameters["zone"], serde_json::json!("p3"));
        let stress = specs
            .iter()
            .find(|spec| spec.capture_id == "courtyard-stress")
            .expect("stress capture");
        assert_eq!(stress.whole_frame, WholeFrameJudgment::Loose);
        assert_eq!(stress.parameters["crowd"], serde_json::json!(1000));
        assert_eq!(stress.parameters["time_of_day"], serde_json::json!("night"));
        assert_eq!(stress.parameters["zone"], serde_json::json!("lamps"));
        let night = specs
            .iter()
            .find(|spec| spec.capture_id == "courtyard-night")
            .expect("night capture");
        assert_eq!(night.rois, ["lamps"]);
        // v1 全景系 ID 不再出现。
        assert!(
            specs
                .iter()
                .all(|spec| spec.capture_id != "courtyard-day" && spec.camera != "panorama"),
            "panorama 机位与 courtyard-day 随 v1 退役"
        );
    }

    #[test]
    fn illegal_capture_ids_and_parameters_are_rejected() {
        let mut bad_id = courtyard_capture_specs();
        bad_id[0].capture_id = "courtyard_day".to_string();
        assert!(matches!(
            validate_capture_specs(&bad_id),
            Err(CaptureSpecError::IllegalCaptureId(_))
        ));
        let mut bad_param = courtyard_capture_specs();
        bad_param[0]
            .parameters
            .insert("time_of_day".to_string(), serde_json::json!("noon"));
        assert!(matches!(
            validate_capture_specs(&bad_param),
            Err(CaptureSpecError::IllegalParameters { .. })
        ));
        let mut dup = courtyard_capture_specs();
        dup.push(dup[0].clone());
        assert!(matches!(
            validate_capture_specs(&dup),
            Err(CaptureSpecError::DuplicateId(_))
        ));
    }
}
