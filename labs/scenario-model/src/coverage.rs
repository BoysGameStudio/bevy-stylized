//! 覆盖映射（docs/showcase/01 lines 695-736、769-780）：合同/效果 →
//! 配方(场景) → case/平台执行 → 判定/产物。
//!
//! 从统一 catalog / 捕获规格派生（注册定义是唯一源，不维护第二份手写
//! 入录）：courtyard 退役迁移（docs/showcase/05 §10）后，覆盖行按**捕获**
//! 组织（10 条捕获规格，`crate::capture_spec::courtyard_capture_specs`），
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
/// 判定凭据（docs/showcase/05 §12 阶段 3，2026-09-28 验收记录
/// `fixtures/golden/showcase.courtyard.acceptance.md`）：10 条捕获的
/// macos-metal overlay 基线 + 同机重捕判定闭环；canonical（Linux/Vulkan
/// 参考宿主）基线未建 = canonical 验收 BLOCKED，不是 PASS。
/// browser/android 两列：lane 迁移（§10 M5）后的执行证据未产生 → Pending。
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
            android: CoverageState::Verified,
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
            "browser 列证据 = browser-showcase lane（Chrome 153 headless，Dawn/Vulkan NVIDIA）：全 18 场景执行 + 逐场景判定（vs canonical showcase 基线，exact 18/18，envelope 未动用）；macOS 诊断运行 lod/outline-advanced 各有亚 LSB 非边缘分歧像素， envelope 判 FAIL 并如实记录，不构成缺陷证据".to_string(),
            "android 列证据 = android-catalog lane（Tenacity_API35 模拟器 -gpu host，MoltenVK→M4 Pro Metal）：18/18 PASS，两轮逐字节一致；effects/face/stress/outline-advanced 走 android-vulkan overlay（其中三项与 macos-metal overlay 逐字节一致，同一物理 GPU）；模拟器证据，不构成真机声明".to_string(),
            "courtyard 迁移（05 §10，2026-09-28）：覆盖行从场景改为 10 条捕获规格；pc 列 = macos-metal overlay 基线 + 同机重捕判定闭环（fixtures/golden/showcase.courtyard.acceptance.md）；canonical 基线未建 = BLOCKED（参考宿主缺席，不是 PASS）；browser/android 列 Pending 是 lane 迁移前的真实状态".to_string(),
            "android catalog lane（2026-09-28，Tenacity_API35 模拟器 -gpu host，MoltenVK→M4 Pro Metal）：10/10 PASS 两轮（android-vulkan overlay 播种 + 非重基线判定闭环；9 条 strict 逐字节，outline-hybrid loose+rois）；S4 打包（GLB 进 APK assets）与 catalog 捕获规格端到端验证；驻留模式落地（report_and_stay + ANDROID_RESIDENT 标记；触摸事件已接，手势/面板属交互 V3）。模拟器证据，不构成真机声明".to_string(),
            "browser lane（2026-09-28，macOS M4 Pro / Chrome 153 Dawn）：?spec= 合同与 S4 HTTP 资产通道端到端可用；角色台 NPR 化（StylizedMaterialSet + StylizedAssetIntegrationPlugin，决策 2026-09-28）后 9/10 PASS（browser vs native exact）；courtyard-outline-hybrid 的 screen-space 描边在 Dawn(Tint) vs wgpu/Metal(naga) 间系统性分歧（2488 edge 像素，超 envelope，同 Metal screen-outline 平台差异一类，SUPPORT_MATRIX 2026-09-28 条目；放宽判定属 owner 决策）。列维持 Pending 是如实状态".to_string(),
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
        assert_eq!(coverage_ids.len(), 10, "every capture spec exactly once");
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
            // pc 列：macos-metal overlay 基线 + 同机重捕判定闭环（验收记录
            // fixtures/golden/showcase.courtyard.acceptance.md）→ Verified。
            assert_eq!(
                coverage.pc,
                CoverageState::Verified,
                "{} must have pc judgment evidence (overlay acceptance)",
                coverage.capture
            );
            // browser：Dawn/naga screen-outline 分歧未决（放宽属 owner 决策）→
            // Pending；android：android-catalog lane 模拟器证据（执行 + 判定
            // 凭据，不构成真机声明）→ Verified。
            assert_eq!(
                coverage.browser,
                CoverageState::Pending,
                "{}",
                coverage.capture
            );
            assert_eq!(
                coverage.android,
                CoverageState::Verified,
                "{} android must be verified (android-catalog lane emulator evidence)",
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
        assert_eq!(value["captures"].as_array().expect("captures").len(), 10);
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
        assert_eq!(showcase_cases.len(), 10);
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
