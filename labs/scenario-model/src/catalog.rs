//! 纯数据场景目录（docs/showcase/01 §依赖与代码所有权 / §场景、配方与验证身份）。
//!
//! 目录以注册定义为唯一源；本模块的内容是 17 个注册用例的元数据（稳定 ID、
//! 能力/tier 需求、资产依赖、参数 schema 占位）。元数据必须可在不启用
//! renderer/glTF/UI 的构建中枚举——本模块不依赖任何渲染、glTF 或 UI 类型。
//!
//! [`BuilderBinding`] 把当前所有者位置（`labs/scenes`）与构建/视图符号记录
//! 为数据；绑定核对（重复 ID、缺失实现、声明符号一致性）由 `labs/scenes`
//! 的 registry 对本目录执行。
//!
//! 目录校验拒绝重复 ID（[`validate_catalog`]）；未知类型、非法范围、不适用
//! 参数及未声明的资产依赖在执行前失败（配方侧校验见 [`crate::recipe`]，
//! 目录交叉校验见 [`require_declared_assets`]）。

use serde::{Deserialize, Serialize};

use bevy_stylized_core::axis::StylizedExecutionTier;

use crate::ident::{PresetId, SceneId};
use crate::recipe::{AssetDecl, SceneRecipe};

/// 目录条目数量（`apps/showcase::all_cases` 的 17 个注册项，顺序固定）。
pub const CATALOG_LEN: usize = 17;

/// 构建器绑定的数据记录（本阶段不做绑定，只记录所有者与符号）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct BuilderBinding {
    /// 当前所有者位置。
    pub owner: String,
    /// 构建函数符号（所有者 crate 内）。
    pub symbol: String,
    /// 可选视图 preset 函数符号（如 environment 三套调色板）。
    pub view: Option<String>,
}

/// 参数 schema 占位（当前注册表没有参数定义时的显式占位，
/// 不是第二份参数定义）。
/// 参数落点（写入作用域；R12：参数写入只落在声明的作用域）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ParameterTarget {
    /// 作用域（entity = 实例槽位；view = 视图设置）。
    pub scope: crate::action::TargetScope,
    /// 稳定逻辑 ID（entity 作用域的场景节点；view 作用域为 `main`）。
    pub id: String,
    /// 落点字段（如 `face_threshold`、`exposure`）。
    pub field: String,
}

/// 参数类型与合法范围（编译期校验合同）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ParameterKind {
    /// 有限 f32（可带闭区间范围）。
    F32 {
        /// 下界（含）。
        min: f32,
        /// 上界（含）。
        max: f32,
    },
    /// 可选有限 f32（null = 回退材质默认）。
    OptionalF32,
    /// 非负整数（含上界）。
    U32 {
        /// 上界（含）。
        max: u32,
    },
    /// 布尔。
    Bool,
}

/// 参数 schema 条目：键 → 落点 + 类型/范围。目录是参数定义的唯一源；
/// 用户保存的配方只携带实例数据（键与值）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ParameterSpec {
    /// 参数键（配方 `parameters` 的键，如 `instances.1.face_threshold`）。
    pub name: String,
    /// 落点。
    pub target: ParameterTarget,
    /// 类型与合法范围。
    pub kind: ParameterKind,
    /// 说明。
    pub summary: String,
}

/// 参数校验失败（不适用参数在执行前失败）。
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum ParameterError {
    /// 未声明的参数键。
    #[error("undeclared parameter key: {0:?}")]
    UnknownKey(String),
    /// 类型不符或非法取值。
    #[error("parameter {key:?}: {reason}")]
    IllegalValue {
        /// 参数键。
        key: String,
        /// 原因。
        reason: String,
    },
}

impl ParameterSpec {
    /// 校验一个参数值（类型/范围；有限性）。
    pub fn validate_value(
        &self,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), ParameterError> {
        let illegal = |reason: String| ParameterError::IllegalValue {
            key: key.to_string(),
            reason,
        };
        match &self.kind {
            ParameterKind::F32 { min, max } => {
                let Some(number) = value.as_f64() else {
                    return Err(illegal("must be a finite number".into()));
                };
                if !number.is_finite() || (number as f32) < *min || (number as f32) > *max {
                    return Err(illegal(format!("out of range {min}..={max}: {number}")));
                }
                Ok(())
            }
            ParameterKind::OptionalF32 => {
                if value.is_null() {
                    return Ok(());
                }
                match value.as_f64() {
                    Some(number) if number.is_finite() => Ok(()),
                    _ => Err(illegal("must be null or a finite number".into())),
                }
            }
            ParameterKind::U32 { max } => match value.as_u64() {
                Some(number) if number <= u64::from(*max) => Ok(()),
                _ => Err(illegal(format!("must be an integer in 0..={max}"))),
            },
            ParameterKind::Bool => {
                if value.is_boolean() {
                    Ok(())
                } else {
                    Err(illegal("must be a boolean".into()))
                }
            }
        }
    }
}

/// 按目录条目的参数 schema 校验配方参数表（不适用参数在执行前失败）。
pub fn validate_parameters(
    entry: &CatalogEntry,
    parameters: &crate::canon::ParameterMap,
) -> Result<(), ParameterError> {
    for (key, value) in parameters {
        let Some(spec) = entry.parameters.iter().find(|spec| spec.name == *key) else {
            return Err(ParameterError::UnknownKey(key.clone()));
        };
        spec.validate_value(key, value)?;
    }
    Ok(())
}

/// 目录条目：稳定 ID、能力/tier 需求、资产依赖与参数 schema 占位。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
    /// 稳定场景 ID（目录唯一键）。
    pub id: SceneId,
    /// 摘要。
    pub summary: String,
    /// 构建器绑定记录（数据，不绑定）。
    pub builder: BuilderBinding,
    /// 最低执行 tier 需求（如 screen-space 描边是 Standard+ 特性）。
    pub tier: Option<StylizedExecutionTier>,
    /// 声明的资产依赖。
    #[serde(default)]
    pub assets: Vec<AssetDecl>,
    /// 参数 schema（当前注册表无参数定义时为占位空表）。
    #[serde(default)]
    pub parameters: Vec<ParameterSpec>,
    /// 声明的内容 preset。
    #[serde(default)]
    pub presets: Vec<PresetId>,
}

/// 目录校验失败。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CatalogError {
    /// 重复场景 ID。
    #[error("duplicate catalog scene id: {0}")]
    DuplicateId(String),
    /// 配方缺少目录要求的资产依赖声明。
    #[error("recipe for {scene} does not declare required asset dependency {asset}")]
    UndeclaredAssetDependency {
        /// 场景 ID。
        scene: String,
        /// 缺失的资产 ID。
        asset: String,
    },
    /// 配方请求 tier 低于目录条目要求。
    #[error("recipe for {scene} requests {requested:?} but catalog requires {required:?}")]
    InsufficientTier {
        /// 场景 ID。
        scene: String,
        /// 目录要求。
        required: StylizedExecutionTier,
        /// 配方请求（None 表示默认 Portable）。
        requested: Option<StylizedExecutionTier>,
    },
}

/// 校验目录：拒绝重复 ID。
pub fn validate_catalog(entries: &[CatalogEntry]) -> Result<(), CatalogError> {
    let mut ids = std::collections::BTreeSet::new();
    for entry in entries {
        if !ids.insert(entry.id.as_str()) {
            return Err(CatalogError::DuplicateId(entry.id.to_string()));
        }
    }
    Ok(())
}

/// 配方对目录条目的交叉校验：目录声明的资产依赖必须在配方清单中声明，
/// 配方请求 tier 不得低于条目要求；违反在执行前失败。
pub fn require_declared_assets(
    recipe: &SceneRecipe,
    entry: &CatalogEntry,
) -> Result<(), CatalogError> {
    for dependency in &entry.assets {
        if !recipe.assets.iter().any(|asset| asset.id == dependency.id) {
            return Err(CatalogError::UndeclaredAssetDependency {
                scene: entry.id.to_string(),
                asset: dependency.id.clone(),
            });
        }
    }
    if let Some(required) = entry.tier {
        let requested = recipe.tier.unwrap_or_default();
        if requested < required {
            return Err(CatalogError::InsufficientTier {
                scene: entry.id.to_string(),
                required,
                requested: recipe.tier,
            });
        }
    }
    Ok(())
}

/// 内置目录：`apps/showcase` 现有 17 个注册用例（顺序与注册表一致）。
///
/// `builder` 记录当前所有者符号（`labs/scenes::builders::*`，视图 preset 在
/// `labs/scenes::views::*`）；`reference`
/// 绑定 `fixtures/synthetic/reference-scene` 资产依赖；`outline-advanced`
/// 声明 `Standard` tier 需求；environment 三项共享一个构建器、以视图
/// preset 符号区分昼雾/黄昏/夜。
#[must_use]
pub fn catalog() -> Vec<CatalogEntry> {
    let entry = |id: &str,
                 summary: &str,
                 symbol: &str,
                 view: Option<&str>,
                 tier: Option<StylizedExecutionTier>,
                 assets: Vec<AssetDecl>| {
        CatalogEntry {
            id: SceneId::new(id).expect("catalog id is valid"),
            summary: summary.to_string(),
            builder: BuilderBinding {
                owner: "labs/scenes".to_string(),
                symbol: symbol.to_string(),
                view: view.map(str::to_string),
            },
            tier,
            assets,
            parameters: Vec::new(),
            presets: Vec::new(),
        }
    };
    let reference_scene = || AssetDecl {
        id: "reference-scene".to_string(),
        source: "fixtures/synthetic/reference-scene".to_string(),
        sha256: None,
    };
    vec![
        entry(
            "overview",
            "三材质球 + 轮廓概览",
            "builders::overview",
            None,
            None,
            vec![],
        ),
        entry(
            "ramp",
            "ramp 渐变采样",
            "builders::ramp",
            None,
            None,
            vec![],
        ),
        entry(
            "lighting",
            "灯光/阴影模型",
            "builders::lighting",
            None,
            None,
            vec![],
        ),
        {
            let mut face = entry("face", "脸部材质语义", "builders::face", None, None, vec![]);
            // face 的参数面：逐实例 face SDF 阈值（OptionalF32；null = 材质默认）
            // 与 ramp/profile 行索引（U32, 0..=255）。落点 = 注册的逻辑实例。
            face.parameters = (0..3)
                .flat_map(|i| {
                    [
                        ParameterSpec {
                            name: format!("instances.{i}.face_threshold"),
                            target: ParameterTarget {
                                scope: crate::action::TargetScope::Entity,
                                id: format!("instances.{i}"),
                                field: "face_threshold".to_string(),
                            },
                            kind: ParameterKind::OptionalF32,
                            summary: "face SDF 阈值（null 回退材质默认）".to_string(),
                        },
                        ParameterSpec {
                            name: format!("instances.{i}.profile_index"),
                            target: ParameterTarget {
                                scope: crate::action::TargetScope::Entity,
                                id: format!("instances.{i}"),
                                field: "profile_index".to_string(),
                            },
                            kind: ParameterKind::U32 { max: 255 },
                            summary: "profile/ramp 行索引".to_string(),
                        },
                    ]
                })
                .collect();
            face
        },
        entry("hair", "头发材质语义", "builders::hair", None, None, vec![]),
        entry(
            "metal",
            "金属/ramp 对照",
            "builders::metal",
            None,
            None,
            vec![],
        ),
        entry(
            "outline",
            "几何轮廓",
            "builders::outline",
            None,
            None,
            vec![],
        ),
        entry(
            "animation",
            "骨骼动画采样",
            "builders::animation",
            None,
            None,
            vec![],
        ),
        entry("post", "后处理链", "builders::post", None, None, vec![]),
        entry(
            "stress",
            "实例压力负载",
            "builders::stress",
            None,
            None,
            vec![],
        ),
        entry(
            "reference",
            "合成参考场景（固定六视角输入域）",
            "builders::reference",
            None,
            None,
            vec![reference_scene()],
        ),
        entry(
            "performance",
            "性能测量负载",
            "builders::performance",
            None,
            None,
            vec![],
        ),
        entry(
            "outline-advanced",
            "screen-space 描边（Standard+ 特性）",
            "builders::outline_advanced",
            None,
            Some(StylizedExecutionTier::Standard),
            vec![],
        ),
        entry("lod", "LOD 阈值切换", "builders::lod", None, None, vec![]),
        entry(
            "environment-day",
            "环境调色板：昼雾",
            "builders::environment",
            Some("views::environment_day"),
            None,
            vec![],
        ),
        entry(
            "environment-dusk",
            "环境调色板：黄昏",
            "builders::environment",
            Some("views::environment_dusk"),
            None,
            vec![],
        ),
        entry(
            "environment-night",
            "环境调色板：夜",
            "builders::environment",
            Some("views::environment_night"),
            None,
            vec![],
        ),
    ]
}

/// 按 ID 查目录条目。
#[must_use]
pub fn catalog_entry(id: &str) -> Option<CatalogEntry> {
    catalog().into_iter().find(|entry| entry.id.as_str() == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_catalog_matches_showcase_registry() {
        let entries = catalog();
        assert_eq!(entries.len(), CATALOG_LEN);
        let ids: Vec<&str> = entries.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "overview",
                "ramp",
                "lighting",
                "face",
                "hair",
                "metal",
                "outline",
                "animation",
                "post",
                "stress",
                "reference",
                "performance",
                "outline-advanced",
                "lod",
                "environment-day",
                "environment-dusk",
                "environment-night",
            ]
        );
        assert_eq!(validate_catalog(&entries), Ok(()));
    }

    #[test]
    fn outline_advanced_requires_standard_tier() {
        let entry = catalog_entry("outline-advanced").expect("entry");
        assert_eq!(entry.tier, Some(StylizedExecutionTier::Standard));
    }

    #[test]
    fn reference_declares_reference_scene_asset() {
        let entry = catalog_entry("reference").expect("entry");
        assert_eq!(entry.assets.len(), 1);
        assert_eq!(entry.assets[0].source, "fixtures/synthetic/reference-scene");
    }

    #[test]
    fn environment_trio_shares_builder_with_three_view_presets() {
        for (id, view) in [
            ("environment-day", "views::environment_day"),
            ("environment-dusk", "views::environment_dusk"),
            ("environment-night", "views::environment_night"),
        ] {
            let entry = catalog_entry(id).expect("entry");
            assert_eq!(entry.builder.symbol, "builders::environment");
            assert_eq!(entry.builder.view.as_deref(), Some(view));
        }
    }

    #[test]
    fn builder_binding_records_scenes_ownership() {
        for entry in catalog() {
            assert_eq!(entry.builder.owner, "labs/scenes");
        }
    }

    #[test]
    fn face_declares_instance_threshold_and_profile_parameters() {
        let entry = catalog_entry("face").expect("entry");
        let names: Vec<&str> = entry
            .parameters
            .iter()
            .map(|spec| spec.name.as_str())
            .collect();
        for i in 0..3 {
            assert!(names.contains(&format!("instances.{i}.face_threshold").as_str()));
            assert!(names.contains(&format!("instances.{i}.profile_index").as_str()));
        }
        // 元数据校验：声明键接受合法值，未声明/越界/类型错拒绝（无 build 依赖）。
        let mut params = crate::canon::ParameterMap::new();
        params.insert(
            "instances.1.face_threshold".to_string(),
            serde_json::json!(0.2),
        );
        params.insert(
            "instances.1.profile_index".to_string(),
            serde_json::json!(3),
        );
        assert_eq!(validate_parameters(&entry, &params), Ok(()));
        let mut bad = crate::canon::ParameterMap::new();
        bad.insert(
            "instances.1.profile_index".to_string(),
            serde_json::json!(300),
        );
        assert!(matches!(
            validate_parameters(&entry, &bad),
            Err(ParameterError::IllegalValue { .. })
        ));
        let mut unknown = crate::canon::ParameterMap::new();
        unknown.insert("nope".to_string(), serde_json::json!(1));
        assert_eq!(
            validate_parameters(&entry, &unknown),
            Err(ParameterError::UnknownKey("nope".to_string()))
        );
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut entries = catalog();
        entries.push(entries[0].clone());
        assert_eq!(
            validate_catalog(&entries),
            Err(CatalogError::DuplicateId("overview".to_string()))
        );
    }
}
