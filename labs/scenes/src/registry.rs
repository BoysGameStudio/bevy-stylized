//! 注册表（`build` feature）：SceneId ↔ 构建实现的绑定表。
//!
//! 设计（docs/showcase/01）：`scenes` 的注册项把元数据与构建实现绑定为
//! 唯一目录；绑定表核对重复 ID、缺失实现和声明/编译能力是否一致。
//! 元数据的唯一源是 `scenario-model` 的 catalog；本表把每个目录条目
//! 绑定到一个构建实现，并记录构建符号供绑定表核对。

use bevy::prelude::World;
use bevy_stylized_core::StylizedViewDescriptor;

use crate::builders;
use crate::recipe::CameraRecipe;
use crate::views;

/// 内容构建器签名：组装内容，返回相机配方。
///
/// 世界边界（而非 App）：宿主可以从独占系统（交互模式的事件循环内）
/// 或 headless 包装两侧驱动同一构建器。
pub type SceneBuildFn = fn(&mut World) -> CameraRecipe;

/// 视图 preset 签名：覆盖视图描述符（内容级，输出绑定归宿主）。
pub type ViewPresetFn = fn(&mut StylizedViewDescriptor);

/// 单条绑定：目录 SceneId → 构建实现（+ 可选视图 preset）。
#[derive(Clone, Copy, Debug)]
pub struct SceneBinding {
    /// 目录场景 ID（与 `scenario-model` catalog 条目一一对应）。
    pub id: &'static str,
    /// 构建函数符号（catalog `BuilderBinding.symbol` 核对用）。
    pub symbol: &'static str,
    /// 构建实现。
    pub build: SceneBuildFn,
    /// 可选视图 preset（符号 + 实现）。
    pub view: Option<(&'static str, ViewPresetFn)>,
}

/// 全部 17 条绑定（顺序与 catalog 一致）。
#[must_use]
pub fn bindings() -> Vec<SceneBinding> {
    use builders as b;
    vec![
        SceneBinding {
            id: "overview",
            symbol: "builders::overview",
            build: b::overview,
            view: None,
        },
        SceneBinding {
            id: "ramp",
            symbol: "builders::ramp",
            build: b::ramp,
            view: None,
        },
        SceneBinding {
            id: "lighting",
            symbol: "builders::lighting",
            build: b::lighting,
            view: None,
        },
        SceneBinding {
            id: "face",
            symbol: "builders::face",
            build: b::face,
            view: None,
        },
        SceneBinding {
            id: "hair",
            symbol: "builders::hair",
            build: b::hair,
            view: None,
        },
        SceneBinding {
            id: "metal",
            symbol: "builders::metal",
            build: b::metal,
            view: None,
        },
        SceneBinding {
            id: "outline",
            symbol: "builders::outline",
            build: b::outline,
            view: None,
        },
        SceneBinding {
            id: "animation",
            symbol: "builders::animation",
            build: b::animation,
            view: None,
        },
        SceneBinding {
            id: "post",
            symbol: "builders::post",
            build: b::post,
            view: None,
        },
        SceneBinding {
            id: "stress",
            symbol: "builders::stress",
            build: b::stress,
            view: None,
        },
        SceneBinding {
            id: "reference",
            symbol: "builders::reference",
            build: b::reference,
            view: None,
        },
        SceneBinding {
            id: "performance",
            symbol: "builders::performance",
            build: b::performance,
            view: None,
        },
        SceneBinding {
            id: "outline-advanced",
            symbol: "builders::outline_advanced",
            build: b::outline_advanced,
            view: None,
        },
        SceneBinding {
            id: "lod",
            symbol: "builders::lod",
            build: b::lod,
            view: None,
        },
        SceneBinding {
            id: "environment-day",
            symbol: "builders::environment",
            build: b::environment,
            view: Some(("views::environment_day", views::environment_day)),
        },
        SceneBinding {
            id: "environment-dusk",
            symbol: "builders::environment",
            build: b::environment,
            view: Some(("views::environment_dusk", views::environment_dusk)),
        },
        SceneBinding {
            id: "environment-night",
            symbol: "builders::environment",
            build: b::environment,
            view: Some(("views::environment_night", views::environment_night)),
        },
    ]
}

/// 绑定表核对失败。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// 绑定表内重复 SceneId。
    #[error("duplicate scene id in binding table: {0}")]
    DuplicateId(String),
    /// catalog 条目没有构建实现。
    #[error("catalog scene {0} has no builder binding")]
    MissingBuilder(String),
    /// 绑定表存在 catalog 之外的场景。
    #[error("binding table has scene {0} not present in the catalog")]
    UnknownScene(String),
    /// catalog 记录的构建符号与绑定表不一致。
    #[error(
        "catalog scene {scene} records builder {recorded} but binding table declares {declared}"
    )]
    SymbolMismatch {
        /// 场景 ID。
        scene: String,
        /// catalog 记录。
        recorded: String,
        /// 绑定表声明。
        declared: String,
    },
    /// catalog 记录的视图 preset 符号与绑定表不一致。
    #[error(
        "catalog scene {scene} records view {recorded:?} but binding table declares {declared:?}"
    )]
    ViewMismatch {
        /// 场景 ID。
        scene: String,
        /// catalog 记录。
        recorded: Option<String>,
        /// 绑定表声明。
        declared: Option<String>,
    },
}

/// 绑定表核对：重复 ID、缺失实现、多余实现、声明的构建/视图符号一致。
pub fn validate_against_catalog() -> Result<(), RegistryError> {
    let bindings = bindings();
    let catalog = scenario_model::catalog();
    let mut ids = std::collections::BTreeSet::new();
    for binding in &bindings {
        if !ids.insert(binding.id) {
            return Err(RegistryError::DuplicateId(binding.id.to_string()));
        }
    }
    for entry in &catalog {
        let binding = bindings
            .iter()
            .find(|binding| binding.id == entry.id.as_str())
            .ok_or_else(|| RegistryError::MissingBuilder(entry.id.to_string()))?;
        if binding.symbol != entry.builder.symbol {
            return Err(RegistryError::SymbolMismatch {
                scene: entry.id.to_string(),
                recorded: entry.builder.symbol.clone(),
                declared: binding.symbol.to_string(),
            });
        }
        let declared_view = binding.view.map(|(symbol, _)| symbol.to_string());
        if declared_view != entry.builder.view {
            return Err(RegistryError::ViewMismatch {
                scene: entry.id.to_string(),
                recorded: entry.builder.view.clone(),
                declared: declared_view,
            });
        }
    }
    for binding in &bindings {
        if !catalog.iter().any(|entry| entry.id.as_str() == binding.id) {
            return Err(RegistryError::UnknownScene(binding.id.to_string()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_table_matches_catalog() {
        validate_against_catalog().expect("binding table must satisfy the catalog");
        let ids: Vec<&str> = bindings().iter().map(|binding| binding.id).collect();
        let catalog_ids: Vec<String> = scenario_model::catalog()
            .iter()
            .map(|entry| entry.id.to_string())
            .collect();
        assert_eq!(ids, catalog_ids, "binding order follows the catalog");
    }
}
