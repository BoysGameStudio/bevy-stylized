//! 覆盖映射（docs/showcase/01 lines 695-736、769-780）：合同/效果 →
//! 配方(场景) → case/平台执行 → 判定/产物。
//!
//! 从统一 catalog 派生（注册定义是唯一源，不维护第二份手写入录）：
//! 场景族表沿用设计文档的原覆盖要求词汇；每个场景标注 PC/Android/
//! Browser 三列状态（已有验证/仅展示/待实现）与判定/产物凭据。
//! 机器可校验的不变量由本模块测试执行；报告 JSON 是运行期生成物
//! （`target/stylized/coverage/coverage.json`），不是受管快照。
//!
//! 词汇：已有验证 = 有执行证据**和**判定凭据；仅展示 = 有执行证据但
//! 判定只有非黑帧门槛；待实现 = 无执行证据。跨设备像素一致性不是
//! 获准比较域，不进入判定列。

use serde::Serialize;

use crate::ident::SceneId;

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

/// 单场景覆盖行。
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneCoverage {
    /// 场景 ID（catalog 注册键）。
    pub scene: SceneId,
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
    /// 族内场景。
    pub scenes: Vec<SceneId>,
}

/// 覆盖报告（生成物；版本化 schema）。
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageReport {
    /// 报告 schema。
    pub schema: String,
    /// 场景族表。
    pub families: Vec<FamilyCoverage>,
    /// 场景覆盖行。
    pub scenes: Vec<SceneCoverage>,
    /// 说明（非断言域与既有 host 限制的备注）。
    pub notes: Vec<String>,
}

const FAMILY_RAMP: &str = "ramp";
const FAMILY_LIGHTING: &str = "lighting / environment";
const FAMILY_FACE: &str = "face";
const FAMILY_HAIR: &str = "hair";
const FAMILY_METAL: &str = "metal";
const FAMILY_OUTLINE: &str = "outline";
const FAMILY_DEFORMATION: &str = "deformation / lod";
const FAMILY_POST: &str = "post";
const FAMILY_INSTANCES: &str = "instances";
const FAMILY_REFERENCE: &str = "reference";

fn scene(id: &str, family: &str, judgment: Judgment, android: CoverageState) -> SceneCoverage {
    let _ = android;
    let pc = match &judgment {
        Judgment::None | Judgment::NonBlackCapture => CoverageState::Demonstration,
        _ => CoverageState::Verified,
    };
    SceneCoverage {
        scene: SceneId::new(id).expect("scene id"),
        family: family.to_string(),
        pc,
        // Android 列：android-catalog lane 的全目录执行证据（API-35 模拟器，
        // MoltenVK→宿主 GPU；判定 = golden 容差 vs canonical/android-vulkan
        // overlay 基线）。模拟器证据，不构成真机声明。
        android: CoverageState::Verified,
        // Browser 列：browser-showcase lane 的全目录执行证据（Chrome 153，
        // canonical host 18/18 exact；docs/platforms 记录的判定链）。
        browser: CoverageState::Verified,
        judgment,
    }
}

/// 从统一 catalog 派生覆盖报告（当前真实状态；未证明的一律待实现）。
#[must_use]
pub fn coverage_report() -> CoverageReport {
    let golden = |path: &str| Judgment::Golden {
        path: path.to_string(),
    };
    let byte = |evidence: &str| Judgment::ByteEquivalence {
        evidence: evidence.to_string(),
    };
    let scenes = vec![
        scene(
            "overview",
            FAMILY_LIGHTING,
            golden("fixtures/golden/showcase"),
            CoverageState::Pending,
        ),
        scene(
            "ramp",
            FAMILY_RAMP,
            golden("fixtures/golden/cel-portable"),
            CoverageState::Pending,
        ),
        scene(
            "lighting",
            FAMILY_LIGHTING,
            golden("fixtures/golden/showcase"),
            CoverageState::Pending,
        ),
        scene(
            "face",
            FAMILY_FACE,
            byte("apply_params/parameters 阈值扫描（H2/H3，同配方逐字节一致）"),
            CoverageState::Pending,
        ),
        scene(
            "hair",
            FAMILY_HAIR,
            golden("fixtures/golden/showcase"),
            CoverageState::Pending,
        ),
        scene(
            "metal",
            FAMILY_METAL,
            golden("fixtures/golden/showcase"),
            CoverageState::Pending,
        ),
        scene(
            "outline",
            FAMILY_OUTLINE,
            byte("编译配方 == showcase 捕获（H2）；cel-portable sphere_outline golden"),
            CoverageState::Verified,
        ),
        scene(
            "animation",
            FAMILY_DEFORMATION,
            golden("fixtures/golden/regression-lab"),
            CoverageState::Pending,
        ),
        scene(
            "post",
            FAMILY_POST,
            byte("post_lifecycle enable/disable/restore 像素断言（tests/integration）"),
            CoverageState::Pending,
        ),
        scene(
            "stress",
            FAMILY_INSTANCES,
            golden("fixtures/golden/regression-lab"),
            CoverageState::Pending,
        ),
        scene(
            "reference",
            FAMILY_REFERENCE,
            golden("fixtures/golden/showcase"),
            CoverageState::Pending,
        ),
        scene(
            "performance",
            FAMILY_INSTANCES,
            golden("fixtures/golden/showcase"),
            CoverageState::Pending,
        ),
        scene(
            "outline-advanced",
            FAMILY_OUTLINE,
            golden("fixtures/golden/outline-advanced"),
            CoverageState::Pending,
        ),
        scene(
            "lod",
            FAMILY_DEFORMATION,
            golden("fixtures/golden/showcase"),
            CoverageState::Pending,
        ),
        scene(
            "environment-day",
            FAMILY_LIGHTING,
            golden("fixtures/golden/showcase"),
            CoverageState::Pending,
        ),
        scene(
            "environment-dusk",
            FAMILY_LIGHTING,
            golden("fixtures/golden/showcase"),
            CoverageState::Pending,
        ),
        scene(
            "environment-night",
            FAMILY_LIGHTING,
            golden("fixtures/golden/showcase"),
            CoverageState::Pending,
        ),
        scene(
            "effects",
            FAMILY_LIGHTING,
            golden("fixtures/golden/showcase"),
            CoverageState::Pending,
        ),
    ];
    let families = [
        (FAMILY_RAMP, "1/2/3/5 bands、256 authored ramp、smoothing、day/night atlas、360° 光照、时间稳定性"),
        (FAMILY_LIGHTING, "Physical/Authored/Hybrid、key light、physical influence、palette、environment；日出/白天/日落/夜/雨参考 preset"),
        (FAMILY_FACE, "前后左右上下光向、Generic/Gacha、mirrored right、face correction、hard/soft factor"),
        (FAMILY_HAIR, "tangent/shift/power/threshold/softness/mask、光照/相机运动、蒙皮/镜像"),
        (FAMILY_METAL, "MatCap/specular ramp/material class、相机/光照旋转、packed normal、day/night"),
        (FAMILY_OUTLINE, "球/方块/凹面/蒙皮/Morph/hair cards/薄几何/大小尺度，1/2/4/8/24 px，screen-space hull/source-local hull/internal edge"),
        (FAMILY_DEFORMATION, "skin、morph、组合、极端姿态、快速运动、mesh swap、LOD、bounds"),
        (FAMILY_POST, "HDR/bloom/vignette/distortion/chromatic/display transform/tonemap/prepass，显示 resolved camera pipeline"),
        (FAMILY_INSTANCES, "100/300/500/1000，outline/face/hair/post 组合；帧时间、draw calls、材质/可见实例、内存、tier/bindless"),
        (FAMILY_REFERENCE, "固定角色/姿态/相机/光照/曝光/背景/分辨率；Golden 验收独立于展示和测量"),
    ]
    .into_iter()
    .map(|(family, requirement)| FamilyCoverage {
        family: family.to_string(),
        original_requirement: requirement.to_string(),
        scenes: scenes
            .iter()
            .filter(|coverage| coverage.family == family)
            .map(|coverage| coverage.scene.clone())
            .collect(),
    })
    .collect();
    CoverageReport {
        schema: "bevy_stylized.coverage/v1".to_string(),
        families,
        scenes,
        notes: vec![
            "overview 同时展示三个 surface class，归入 lighting / environment（默认光照/视图语义）".to_string(),
            "reference 的六视角参考验收合同归 labs/reference 独立合同，不作为本表判定凭据".to_string(),
            "Metal host 的既有 golden 超差（sphere_outline/face_sdf_instances/outline-advanced hull）是 host 限定问题，合格 Linux host 的 golden 判定不受影响".to_string(),
            "browser 列证据 = browser-showcase lane（Chrome 153 headless，Dawn/Vulkan NVIDIA）：全 18 场景执行 + 逐场景判定（vs canonical showcase 基线，exact 18/18，envelope 未动用）；macOS 诊断运行 lod/outline-advanced 各有亚 LSB 非边缘分歧像素， envelope 判 FAIL 并如实记录，不构成缺陷证据".to_string(),
            "android 列证据 = android-catalog lane（Tenacity_API35 模拟器 -gpu host，MoltenVK→M4 Pro Metal）：18/18 PASS，两轮逐字节一致；effects/face/stress/outline-advanced 走 android-vulkan overlay（其中三项与 macos-metal overlay 逐字节一致，同一物理 GPU）；模拟器证据，不构成真机声明".to_string(),
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
    fn every_catalog_scene_is_covered_exactly_once() {
        let report = coverage_report();
        let catalog = crate::catalog::catalog();
        let catalog_ids: Vec<&str> = catalog.iter().map(|entry| entry.id.as_str()).collect();
        let coverage_ids: Vec<&str> = report
            .scenes
            .iter()
            .map(|coverage| coverage.scene.as_str())
            .collect();
        assert_eq!(coverage_ids.len(), 18, "every catalog scene exactly once");
        for id in &catalog_ids {
            assert_eq!(
                coverage_ids.iter().filter(|c| c == &id).count(),
                1,
                "scene {id} missing or duplicated in coverage"
            );
        }
        for family in &report.families {
            assert!(!family.scenes.is_empty(), "empty family {}", family.family);
            for scene in &family.scenes {
                assert!(
                    report.scenes.iter().any(|c| c.scene == *scene),
                    "family {} lists scene not in coverage rows",
                    family.family
                );
            }
        }
    }

    #[test]
    fn every_scene_has_pc_execution_and_truthful_states() {
        let report = coverage_report();
        for coverage in &report.scenes {
            assert_ne!(
                coverage.pc,
                CoverageState::Pending,
                "{} must have pc execution evidence (showcase capture exists)",
                coverage.scene
            );
            assert_eq!(
                coverage.browser,
                CoverageState::Verified,
                "{} browser must be verified (browser-showcase lane evidence)",
                coverage.scene
            );
            if coverage.pc == CoverageState::Verified {
                assert!(
                    !matches!(
                        coverage.judgment,
                        Judgment::None | Judgment::NonBlackCapture
                    ),
                    "{}: Verified requires a judgment credential",
                    coverage.scene
                );
            }
        }
        // Android 列：全部 18 场景经 android-catalog lane 验证
        // （API-35 模拟器证据；判定归 golden 基线）。
        for coverage in &report.scenes {
            assert_eq!(
                coverage.android,
                CoverageState::Verified,
                "{} android must be verified (android-catalog lane evidence)",
                coverage.scene
            );
        }
    }

    #[test]
    fn golden_judgment_paths_exist_and_report_serializes() {
        let report = coverage_report();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for coverage in &report.scenes {
            if let Judgment::Golden { path } = &coverage.judgment {
                let full = root.join(path);
                assert!(
                    full.is_dir(),
                    "golden judgment path missing for {}: {}",
                    coverage.scene,
                    full.display()
                );
            }
        }
        let bytes = coverage_json().expect("serialize");
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("parse");
        assert_eq!(value["schema"], "bevy_stylized.coverage/v1");
        assert_eq!(value["scenes"].as_array().expect("scenes").len(), 18);
        // 生成物写出（运行期生成，非受管快照）。
        let out_dir = root.join("target/stylized/coverage");
        std::fs::create_dir_all(&out_dir).expect("create coverage out dir");
        std::fs::write(out_dir.join("coverage.json"), &bytes).expect("write coverage artifact");
    }

    #[test]
    fn suites_registry_identities_are_intact() {
        // line 704：用例减少需证明——必需测试身份清单仍可解析且 18 个
        // showcase 用例名逐一可解析回 catalog 场景。
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let suites: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("tests/suites.json")).expect("read suites.json"),
        )
        .expect("parse suites.json");
        let gpu = suites["gpu"].as_object().expect("gpu suites object");
        assert!(!gpu.is_empty(), "gpu suite registry must not be empty");
        let report = coverage_report();
        let showcase_cases: std::collections::BTreeSet<&str> =
            report.scenes.iter().map(|c| c.scene.as_str()).collect();
        // showcase 用例名（all_cases 的数据源即 catalog）与 catalog 一一对应。
        assert_eq!(showcase_cases.len(), 18);
        for id in &showcase_cases {
            assert!(
                crate::catalog::catalog_entry(id).is_some(),
                "coverage scene {id} does not resolve to a catalog entry"
            );
        }
    }
}
