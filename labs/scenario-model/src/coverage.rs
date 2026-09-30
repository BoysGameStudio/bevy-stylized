//! 覆盖映射（docs/showcase/01 lines 695-736、769-780）：合同/效果 →
//! 配方(场景) → case/平台执行 → 判定/产物。
//!
//! 从统一 catalog / 捕获规格派生（注册定义是唯一源，不维护第二份手写
//! 入录）：courtyard 退役迁移（docs/showcase/05 §10）后，覆盖行按**捕获**
//! 组织（v2 起 12 条捕获规格，`crate::capture_spec::courtyard_capture_specs`），
//! 不再按场景平铺。每行标注 PC/Android/Browser 三列状态（已有验证/
//! 仅展示/待实现）与判定/产物凭据。机器可校验的不变量由本模块测试执行；
//! 报告 JSON 是运行期生成物（`target/stylized/coverage/coverage.json`），
//! 不是受管快照。
//!
//! 词汇：已有验证 = 有执行证据**和**判定凭据；仅展示 = 有执行证据但
//! 判定只有非黑帧门槛；待实现 = 无执行证据。跨设备像素一致性不是
//! 获准比较域，不进入判定列。

use serde::Serialize;

/// 覆盖状态（设计文档词汇）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageState {
    /// 已有验证（执行 + 判定凭据）。
    Verified,
    /// 仅展示（执行证据有，判定只有非黑帧门槛）。
    Demonstration,
    /// 待实现（无执行证据）。
    Pending,
}

/// 判定/产物凭据。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Judgment {
    /// Golden 基线（已接受基线目录）。
    Golden {
        /// 基线路径（仓库相对）。
        path: String,
    },
    /// 字节级等价证据（说明来源阶段/合同）。
    ByteEquivalence {
        /// 证据描述。
        evidence: String,
    },
    /// 结构化重放报告（schema 标识）。
    ReplayReport {
        /// 报告 schema。
        schema: String,
    },
    /// 非黑帧捕获门槛（capture_all 的自动检查）。
    NonBlackCapture,
    /// 无判定。
    None,
}

/// 单条捕获覆盖行（05 §10：按捕获/分区组织）。
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureCoverage {
    /// 捕获 ID（捕获规格唯一键）。
    pub capture: String,
    /// 所属场景族。
    pub family: String,
    /// PC 列。
    pub pc: CoverageState,
    /// Android 列。
    pub android: CoverageState,
    /// Browser 列。
    pub browser: CoverageState,
    /// 判定/产物凭据。
    pub judgment: Judgment,
}

/// 场景族行（原覆盖要求来自设计文档）。
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyCoverage {
    /// 场景族名。
    pub family: String,
    /// 原覆盖要求（docs/showcase/01 场景族表）。
    pub original_requirement: String,
    /// 族内捕获。
    pub captures: Vec<String>,
}

/// 覆盖报告（生成物；版本化 schema）。
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageReport {
    /// 报告 schema。
    pub schema: String,
    /// 场景族表。
    pub families: Vec<FamilyCoverage>,
    /// 捕获覆盖行。
    pub captures: Vec<CaptureCoverage>,
    /// 说明（非断言域与既有 host 限制的备注）。
    pub notes: Vec<String>,
}

const FAMILY_COURTYARD: &str = "courtyard";

/// 从捕获规格派生覆盖报告（当前真实状态；未证明的一律待实现）。
///
/// 判定凭据（docs/showcase/05 §12 V2-3，验收记录
/// `fixtures/golden/showcase.courtyard.acceptance.md`）：v2 12 条捕获的
/// canonical（Linux/Vulkan 参考宿主）基线 + 同机重捕判定闭环
///（2026-09-29 v2 条目；v1 凭据为 dated 历史）。browser/android 两列：
/// v1 的执行/判定证据对应已退役的全景帧，不随 v2 帧变更继承——v2 证据
/// 产生前两列一律 Pending（不是倒退，是帧语义变更后的如实状态）。
#[must_use]
pub fn coverage_report() -> CoverageReport {
    let golden = |path: &str| Judgment::Golden {
        path: path.to_string(),
    };
    let captures: Vec<CaptureCoverage> = crate::capture_spec::courtyard_capture_specs()
        .iter()
        .map(|spec| CaptureCoverage {
            capture: spec.capture_id.clone(),
            family: FAMILY_COURTYARD.to_string(),
            pc: CoverageState::Verified,
            android: CoverageState::Pending,
            browser: CoverageState::Pending,
            judgment: golden("fixtures/golden/showcase"),
        })
        .collect();
    let families = [(
        FAMILY_COURTYARD,
        "合成庭院：三对照组/描边/光照/LOD/灯阵/时刻/后处理分区同屏（docs/showcase/05）",
    )]
    .into_iter()
    .map(|(family, requirement)| FamilyCoverage {
        family: family.to_string(),
        original_requirement: requirement.to_string(),
        captures: captures
            .iter()
            .map(|coverage| coverage.capture.clone())
            .collect(),
    })
    .collect();
    CoverageReport {
        schema: "bevy_stylized.coverage/v2".to_string(),
        families,
        captures,
        notes: vec![
            "overview 同时展示三个 surface class，归入 lighting / environment（默认光照/视图语义）".to_string(),
            "reference 的六视角参考验收合同归 labs/reference 独立合同，不作为本表判定凭据".to_string(),
            "Metal host 的既有 golden 超差（sphere_outline/face_sdf_instances/outline-advanced hull）是 host 限定问题，合格 Linux host 的 golden 判定不受影响".to_string(),
            "v2 改版（05 §12，2026-09-29 owner 决策）：不同屏全显——覆盖行从 v1 的 10 条全景系捕获改为 12 条分区激活捕获（zone 钉值，camera↔zone 一一对应）；判定收敛为整图精确为主（一帧一主体），ROI 机制保留（夜景 lamps）；pc 列 = canonical（linux-vulkan 参考宿主）基线 + 同机重捕判定闭环（fixtures/golden/showcase.courtyard.acceptance.md v2 条目）".to_string(),
            "v3 展亭模式（2026-09-30 owner 决策）：内容同位轮换——12 条规格的机位/摆位重排并整体重捕（验收记录 v3 条目）；v4 增 5 条 cel 特性档捕获（rim/shadow/palette/metal/cutout，同判定口径）".to_string(),
            "browser/android 两列 Pending 是 v2 帧语义变更后的如实状态：v1 证据（browser 9/10 exact + outline-hybrid Dawn/naga 分歧；android 模拟器 10/10 两轮）对应已退役的全景帧，不继承；v2 证据待各自宿主重跑（overlay 重播种 + lane 复跑）".to_string(),
        ],
    }
}

/// 报告 JSON（生成物字节；由运行方决定写出位置）。
pub fn coverage_json() -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec_pretty(&coverage_report())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_capture_spec_is_covered_exactly_once() {
        let report = coverage_report();
        let spec_ids: Vec<String> = crate::capture_spec::courtyard_capture_specs()
            .iter()
            .map(|spec| spec.capture_id.clone())
            .collect();
        let coverage_ids: Vec<&str> = report
            .captures
            .iter()
            .map(|coverage| coverage.capture.as_str())
            .collect();
        assert_eq!(coverage_ids.len(), 17, "every capture spec exactly once");
        for id in &spec_ids {
            assert_eq!(
                coverage_ids.iter().filter(|c| *c == id).count(),
                1,
                "capture {id} missing or duplicated in coverage"
            );
        }
        for family in &report.families {
            assert!(
                !family.captures.is_empty(),
                "empty family {}",
                family.family
            );
            for capture in &family.captures {
                assert!(
                    report.captures.iter().any(|c| c.capture == *capture),
                    "family {} lists capture not in coverage rows",
                    family.family
                );
            }
        }
    }

    #[test]
    fn capture_states_are_truthful() {
        let report = coverage_report();
        for coverage in &report.captures {
            // pc 列：canonical（linux-vulkan 参考宿主）v2 基线 + 同机重捕
            // 判定闭环（验收记录 fixtures/golden/showcase.courtyard.acceptance.md
            // v2 条目）→ Verified。
            assert_eq!(
                coverage.pc,
                CoverageState::Verified,
                "{} must have pc judgment evidence (canonical acceptance)",
                coverage.capture
            );
            // browser/android：v1 证据对应已退役的全景帧，不随 v2 帧语义
            // 变更继承；v2 证据产生前两列一律 Pending（如实状态，不是倒退）。
            assert_eq!(
                coverage.browser,
                CoverageState::Pending,
                "{}",
                coverage.capture
            );
            assert_eq!(
                coverage.android,
                CoverageState::Pending,
                "{} android must be pending until v2 lane evidence exists",
                coverage.capture
            );
            assert!(
                matches!(coverage.judgment, Judgment::Golden { .. }),
                "{}: Verified requires a judgment credential",
                coverage.capture
            );
        }
    }

    #[test]
    fn golden_judgment_paths_exist_and_report_serializes() {
        let report = coverage_report();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for coverage in &report.captures {
            if let Judgment::Golden { path } = &coverage.judgment {
                let full = root.join(path);
                assert!(
                    full.is_dir(),
                    "golden judgment path missing for {}: {}",
                    coverage.capture,
                    full.display()
                );
            }
        }
        let bytes = coverage_json().expect("serialize");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("parse");
        assert_eq!(value["schema"], "bevy_stylized.coverage/v2");
        assert_eq!(value["captures"].as_array().expect("captures").len(), 17);
        // 生成物写出（运行期生成，非受管快照）。
        let out_dir = root.join("target/stylized/coverage");
        std::fs::create_dir_all(&out_dir).expect("create coverage out dir");
        std::fs::write(out_dir.join("coverage.json"), &bytes).expect("write coverage artifact");
    }

    #[test]
    fn suites_registry_identities_are_intact() {
        // line 704：用例减少需证明——必需测试身份清单仍可解析且全部
        // showcase 捕获身份逐一可解析回捕获规格。
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let suites: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("tests/suites.json")).expect("read suites.json"),
        )
        .expect("parse suites.json");
        let gpu = suites["gpu"].as_object().expect("gpu suites object");
        assert!(!gpu.is_empty(), "gpu suite registry must not be empty");
        let report = coverage_report();
        // showcase 捕获身份（`_`↔`-` 派生：showcase_golden_<id 下划线形>）
        // 与捕获规格一一对应。
        let showcase_cases: std::collections::BTreeSet<&str> =
            report.captures.iter().map(|c| c.capture.as_str()).collect();
        assert_eq!(showcase_cases.len(), 17);
        let specs: std::collections::BTreeSet<String> =
            crate::capture_spec::courtyard_capture_specs()
                .iter()
                .map(|spec| spec.capture_id.clone())
                .collect();
        for id in &showcase_cases {
            assert!(
                specs.contains(*id),
                "coverage capture {id} does not resolve to a capture spec"
            );
        }
    }
}
