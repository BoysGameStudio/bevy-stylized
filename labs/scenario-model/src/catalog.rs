//! 纯数据场景目录（docs/showcase/01 §依赖与代码所有权 / §场景、配方与验证身份）。
//!
//! 目录以注册定义为唯一源；本模块的内容是庭院展台（courtyard，
//! docs/showcase/05）的元数据（稳定 ID、
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

/// 目录条目数量（courtyard 单场景；docs/showcase/05 §10 退役 18 个旧场景后）。
pub const CATALOG_LEN: usize = 1;

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

/// 参数类型与合法范围（编译期校验合同）。`Enum` 的取值是 JSON 字符串，
/// 必须落在 `options` 声明的集合内。
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
    /// 枚举（JSON 字符串，取值限定在 `options` 内）。
    Enum {
        /// 合法取值集合（非空、无重复）。
        options: Vec<String>,
    },
}

/// 控件元数据（docs/showcase/06 §6/§11：分组与快捷档位的单一源，三平台
/// 壳共用——PC 抽屉 / browser 侧栏 / Android 折叠面板读同一份声明）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ParameterUiMeta {
    /// 面板分组（courtyard：Scene / Rendering / Content，06 §6）。
    pub group: String,
    /// U32 步进滑杆的快捷预设芯片（06 §16 D5；只是 UI 引导，schema 全域
    /// 仍合法——校验语义不变）。
    #[serde(default)]
    pub presets: Option<Vec<serde_json::Value>>,
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
    /// 控件元数据（06 §6；None = 无 UI 呈现的纯 schema 参数）。
    #[serde(default)]
    pub ui: Option<ParameterUiMeta>,
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
            ParameterKind::Enum { options } => {
                if options.is_empty() {
                    return Err(illegal("enum spec has no options".into()));
                }
                let mut seen = std::collections::BTreeSet::new();
                if !options.iter().all(|option| seen.insert(option)) {
                    return Err(illegal("enum spec has duplicate options".into()));
                }
                match value.as_str() {
                    Some(text) if !text.is_empty() && options.iter().any(|o| o == text) => Ok(()),
                    Some(_) => Err(illegal(format!("must be one of {options:?}"))),
                    None => Err(illegal("must be a string".into())),
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

/// 内置目录：庭院展台单场景（docs/showcase/05 §10：18 个旧展示场景退役，
/// 全部由 courtyard 的分区/参数面/捕获矩阵回收）。
///
/// `builder` 记录当前所有者符号（`labs/scenes::builders::courtyard`）；
/// reference-scene GLB 是必需依赖（S4：全 lane 打包，缺席 = BLOCKED）。
#[must_use]
pub fn catalog() -> Vec<CatalogEntry> {
    let view_target = |field: &str| ParameterTarget {
        scope: crate::action::TargetScope::View,
        id: "main".to_string(),
        field: field.to_string(),
    };
    let entity_target = |id: &str, field: &str| ParameterTarget {
        scope: crate::action::TargetScope::Entity,
        id: id.to_string(),
        field: field.to_string(),
    };
    // 庭院展台（docs/showcase/05）：参数面见 §8；reference-scene GLB 是
    // 必需依赖（S4：全 lane 打包，缺席 = BLOCKED）。odette 档角色台展品
    // （stage_character=odette，2026-09-29 owner 决策）经宿主注册的 odette://
    // 资产源加载，不是必需依赖——golden 判定与证据测试只用 fixture 档。
    let mut courtyard = CatalogEntry {
        id: SceneId::new("courtyard").expect("catalog id is valid"),
        summary: "庭院展台：渲染特性分区组织，v2 分区激活（docs/showcase/05）".to_string(),
        builder: BuilderBinding {
            owner: "labs/scenes".to_string(),
            symbol: "builders::courtyard".to_string(),
            view: None,
        },
        tier: None,
        assets: vec![AssetDecl {
            id: "reference-scene".to_string(),
            source: "fixtures/synthetic/reference-scene".to_string(),
            sha256: None,
        }],
        parameters: vec![
            ParameterSpec {
                name: "zone".to_string(),
                target: view_target("zone"),
                kind: ParameterKind::Enum {
                    options: [
                        "trio", "p1", "p2", "p3", "p4", "p5", "lod", "stage", "lamps", "rim",
                        "shadow", "palette", "metal", "cutout",
                    ]
                    .iter()
                    .map(|option| option.to_string())
                    .collect(),
                },
                summary: "展台内容档位（v3 展亭模式：内容在中央平台轮换装卸 + 机位联动；overview 随 v3 退役，浏览全部内容归导览模式）".to_string(),
                ui: Some(ParameterUiMeta {
                    group: "Scene".to_string(),
                    presets: None,
                }),
            },
            ParameterSpec {
                name: "time_of_day".to_string(),
                target: view_target("time_of_day"),
                kind: ParameterKind::Enum {
                    options: vec!["day".to_string(), "dusk".to_string(), "night".to_string()],
                },
                summary: "时刻（view preset 选择）".to_string(),
                ui: Some(ParameterUiMeta {
                    group: "Scene".to_string(),
                    presets: None,
                }),
            },
            ParameterSpec {
                name: "outline_mode".to_string(),
                target: view_target("outline_mode"),
                kind: ParameterKind::Enum {
                    options: vec!["off".to_string(), "hull".to_string(), "hybrid".to_string()],
                },
                summary: "描边模式（P2/P3 内容 + 相机附件；hybrid 需 Standard）".to_string(),
                ui: Some(ParameterUiMeta {
                    group: "Rendering".to_string(),
                    presets: None,
                }),
            },
            ParameterSpec {
                name: "crowd".to_string(),
                target: entity_target("lamps", "crowd"),
                kind: ParameterKind::U32 { max: 1000 },
                summary: "灯阵密度（0/100/1000；内容重建语义）".to_string(),
                // 06 §16 D5：快捷芯片渲染 0/100/1000 锚点档（含 stress 档）；
                // 全域 0..=1000 仍合法。
                ui: Some(ParameterUiMeta {
                    group: "Content".to_string(),
                    presets: Some(vec![
                        serde_json::json!(0),
                        serde_json::json!(100),
                        serde_json::json!(1000),
                    ]),
                }),
            },
            ParameterSpec {
                name: "post".to_string(),
                target: view_target("post"),
                kind: ParameterKind::Bool,
                summary: "后处理开关（相机附件）".to_string(),
                ui: Some(ParameterUiMeta {
                    group: "Rendering".to_string(),
                    presets: None,
                }),
            },
            ParameterSpec {
                name: "flags".to_string(),
                target: entity_target("flags.*", "pose"),
                kind: ParameterKind::Enum {
                    options: vec!["rest".to_string(), "posed".to_string()],
                },
                summary: "旗帜姿态（rest/posed；落点展开为 flags.0/flags.1）".to_string(),
                ui: Some(ParameterUiMeta {
                    group: "Content".to_string(),
                    presets: None,
                }),
            },
            ParameterSpec {
                name: "stage_character".to_string(),
                target: entity_target("stage", "character"),
                kind: ParameterKind::Enum {
                    options: vec!["fixture".to_string(), "odette".to_string()],
                },
                summary: "角色台展品（fixture=golden 判定内容；odette=真 Genshin 角色，交互展示档，经 odette:// 资产源）".to_string(),
                ui: Some(ParameterUiMeta {
                    group: "Content".to_string(),
                    presets: None,
                }),
            },
        ],
        presets: Vec::new(),
    };
    // face_threshold：中央前排 face 组三实例（OptionalF32；null =
    // 三阈值梯度 0.2/默认/0.8）。
    courtyard.parameters.extend((0..3).map(|i| ParameterSpec {
        name: format!("trio.face.{i}.face_threshold"),
        target: entity_target(&format!("trio.face.{i}"), "face_threshold"),
        kind: ParameterKind::OptionalF32,
        summary: "face SDF 阈值（null 回退三阈值梯度）".to_string(),
        ui: Some(ParameterUiMeta {
            group: "Content".to_string(),
            presets: None,
        }),
    }));
    vec![courtyard]
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
        // §10：18 个旧展示场景退役，courtyard 是唯一注册场景。
        assert_eq!(ids, ["courtyard"]);
        assert_eq!(validate_catalog(&entries), Ok(()));
    }

    #[test]
    fn builder_binding_records_scenes_ownership() {
        for entry in catalog() {
            assert_eq!(entry.builder.owner, "labs/scenes");
        }
    }

    /// 06 §6/§16 D5：courtyard 参数面全部声明分组；crowd 携带 0/100/1000
    /// 快捷预设芯片（U32 全域仍合法——presets 只是 UI 引导）。
    #[test]
    fn courtyard_parameters_carry_ui_groups_and_crowd_presets() {
        let entry = catalog_entry("courtyard").expect("catalog entry");
        let group_of = |name: &str| {
            entry
                .parameters
                .iter()
                .find(|spec| spec.name == name)
                .and_then(|spec| spec.ui.as_ref())
                .map(|ui| ui.group.as_str())
        };
        for name in ["zone", "time_of_day"] {
            assert_eq!(group_of(name), Some("Scene"), "{name}");
        }
        for name in ["outline_mode", "post"] {
            assert_eq!(group_of(name), Some("Rendering"), "{name}");
        }
        for name in ["crowd", "flags", "stage_character"] {
            assert_eq!(group_of(name), Some("Content"), "{name}");
        }
        for i in 0..3 {
            assert_eq!(
                group_of(&format!("trio.face.{i}.face_threshold")),
                Some("Content")
            );
        }
        let crowd = entry
            .parameters
            .iter()
            .find(|spec| spec.name == "crowd")
            .expect("crowd spec");
        assert_eq!(
            crowd.ui.as_ref().and_then(|ui| ui.presets.clone()),
            Some(vec![
                serde_json::json!(0),
                serde_json::json!(100),
                serde_json::json!(1000)
            ])
        );
        // 预设之外的值仍合法（UI 引导不改变校验语义）。
        assert_eq!(
            crowd.validate_value("crowd", &serde_json::json!(42)),
            Ok(())
        );
    }

    #[test]
    fn enum_parameter_accepts_declared_options_and_rejects_others() {
        let spec = ParameterSpec {
            name: "view.palette".to_string(),
            target: ParameterTarget {
                scope: crate::action::TargetScope::View,
                id: "main".to_string(),
                field: "palette".to_string(),
            },
            kind: ParameterKind::Enum {
                options: vec!["day".to_string(), "dusk".to_string(), "night".to_string()],
            },
            summary: "环境调色板".to_string(),
            ui: None,
        };
        // 合法枚举值通过。
        assert_eq!(
            spec.validate_value("view.palette", &serde_json::json!("dusk")),
            Ok(())
        );
        // 不在 options 内的字符串拒绝。
        assert!(matches!(
            spec.validate_value("view.palette", &serde_json::json!("noon")),
            Err(ParameterError::IllegalValue { .. })
        ));
        // 非字符串拒绝。
        for value in [
            serde_json::json!(1),
            serde_json::json!(true),
            serde_json::json!(null),
        ] {
            assert!(matches!(
                spec.validate_value("view.palette", &value),
                Err(ParameterError::IllegalValue { .. })
            ));
        }
        // 空串拒绝。
        assert!(matches!(
            spec.validate_value("view.palette", &serde_json::json!("")),
            Err(ParameterError::IllegalValue { .. })
        ));
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut entries = catalog();
        entries.push(entries[0].clone());
        assert_eq!(
            validate_catalog(&entries),
            Err(CatalogError::DuplicateId("courtyard".to_string()))
        );
    }

    #[test]
    fn courtyard_declares_reference_asset_and_parameter_keys() {
        let entry = catalog_entry("courtyard").expect("entry");
        assert_eq!(entry.builder.symbol, "builders::courtyard");
        assert_eq!(entry.assets.len(), 1);
        assert_eq!(entry.assets[0].id, "reference-scene");
        let names: Vec<&str> = entry
            .parameters
            .iter()
            .map(|spec| spec.name.as_str())
            .collect();
        for name in [
            "zone",
            "time_of_day",
            "outline_mode",
            "crowd",
            "post",
            "flags",
            "stage_character",
            "trio.face.0.face_threshold",
            "trio.face.1.face_threshold",
            "trio.face.2.face_threshold",
        ] {
            assert!(names.contains(&name), "missing parameter {name}");
        }
        // 枚举/数值参数的合法值接受与非法值拒绝（metadata 校验，无 build 依赖）。
        let mut params = crate::canon::ParameterMap::new();
        params.insert("zone".to_string(), serde_json::json!("lamps"));
        params.insert("time_of_day".to_string(), serde_json::json!("dusk"));
        params.insert("outline_mode".to_string(), serde_json::json!("hybrid"));
        params.insert("crowd".to_string(), serde_json::json!(1000));
        params.insert("post".to_string(), serde_json::json!(true));
        params.insert("flags".to_string(), serde_json::json!("posed"));
        params.insert("stage_character".to_string(), serde_json::json!("odette"));
        params.insert(
            "trio.face.1.face_threshold".to_string(),
            serde_json::json!(null),
        );
        assert_eq!(validate_parameters(&entry, &params), Ok(()));
        let mut bad = crate::canon::ParameterMap::new();
        bad.insert("time_of_day".to_string(), serde_json::json!("noon"));
        assert!(matches!(
            validate_parameters(&entry, &bad),
            Err(ParameterError::IllegalValue { .. })
        ));
        let mut bad_zone = crate::canon::ParameterMap::new();
        bad_zone.insert("zone".to_string(), serde_json::json!("p6"));
        assert!(matches!(
            validate_parameters(&entry, &bad_zone),
            Err(ParameterError::IllegalValue { .. })
        ));
        let mut over = crate::canon::ParameterMap::new();
        over.insert("crowd".to_string(), serde_json::json!(1001));
        assert!(matches!(
            validate_parameters(&entry, &over),
            Err(ParameterError::IllegalValue { .. })
        ));
    }
}
