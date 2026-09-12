//! `bevy_stylized.*` Schema 版本化（docs/authoring/03、ADR-0025）。
//!
//! Schema namespace 是长期数据契约，版本独立于 Cargo package 版本：
//! crate 升级不得隐式改变既有 schema 语义。
//!
//! ## 演进规则
//!
//! - **minor 递增**：只允许 additive 变更（新增可选字段、新增枚举变体的
//!   合法取值）。读取方必须能忽略未知字段继续解析同 major 的内容；
//! - **major 递增**：允许破坏性变更（改语义、删字段、改类型）。同
//!   namespace 的新旧 major 是两条独立契约，读取方遇到不支持的 major
//!   必须 fail-closed（报不支持，而不是尽力猜测）；
//! - namespace 内的版本标识统一写作 `bevy_stylized.<domain>/v<major>`，
//!   payload 内再携带 `major.minor` 精确版本；
//! - 任何对 frozen 契约的修改必须提升版本号并走
//!   docs/reference/04 的受控迁移流程。

use std::{fmt, str::FromStr};

use bevy_reflect::Reflect;

/// 命名空间前缀：所有 authoring schema 都挂在该稳定品牌 namespace 下。
pub const SCHEMA_NAMESPACE_PREFIX: &str = "bevy_stylized";

/// Material domain v1 的完整 schema 标识。
pub const SCHEMA_ID_MATERIAL_V1: StylizedSchemaId =
    StylizedSchemaId::new("bevy_stylized.material", 1, 0);

/// Instance domain v1 的完整 schema 标识（V2-e GPU Instance Database
/// 记录 schema）。
pub const SCHEMA_ID_INSTANCE_V1: StylizedSchemaId =
    StylizedSchemaId::new("bevy_stylized.instance", 1, 0);

/// View domain v1 的完整 schema 标识。
///
/// minor 1.1（additive）：新增可选 `environment` 字段（V2-d 环境模型，
/// 缺省解析为恒等默认）。
pub const SCHEMA_ID_VIEW_V1: StylizedSchemaId = StylizedSchemaId::new("bevy_stylized.camera", 1, 1);

/// `bevy_stylized.*` namespace 内某一 domain 的版本化 schema 标识。
///
/// `Display` 形态为 `bevy_stylized.<domain>/v<major>`（与
/// docs/authoring/03 的命名一致），精确 minor 版本随 payload 字段携带。
///
/// 该类型携带 `&'static str`，只做进程内标识，不直接序列化；
/// 序列化载体是 payload 中的 [`StylizedSchemaVersion`] 字段。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
pub struct StylizedSchemaId {
    /// domain namespace（不含版本后缀），如 `bevy_stylized.material`。
    pub namespace: &'static str,
    /// schema 版本。
    pub version: StylizedSchemaVersion,
}

impl StylizedSchemaId {
    /// 构造一个 schema 标识。
    pub const fn new(namespace: &'static str, major: u32, minor: u32) -> Self {
        Self {
            namespace,
            version: StylizedSchemaVersion::new(major, minor),
        }
    }
}

impl fmt::Display for StylizedSchemaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/v{}", self.namespace, self.version.major)
    }
}

/// Schema 版本号（`major.minor`）。
///
/// `Display` 为 `v<major>.<minor>`；`FromStr` 接受 `v1`、`v1.2`、`1`、`1.2`。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylizedSchemaVersion {
    /// 主版本：破坏性变更时递增（见模块文档的演进规则）。
    pub major: u32,
    /// 次版本：additive 变更时递增。
    pub minor: u32,
}

impl StylizedSchemaVersion {
    /// 构造版本号。
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// `self` 是否能按 additive 兼容规则读取 `authored` 版本的内容：
    /// 同 major 且 `authored.minor <= self.minor`。
    #[must_use]
    pub fn can_read(&self, authored: StylizedSchemaVersion) -> bool {
        self.major == authored.major && authored.minor <= self.minor
    }
}

impl fmt::Display for StylizedSchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}", self.major, self.minor)
    }
}

/// [`StylizedSchemaVersion`] 解析失败。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StylizedSchemaVersionParseError {
    /// 无法解析的原始输入。
    pub input: String,
}

impl fmt::Display for StylizedSchemaVersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid bevy_stylized schema version {:?}; expected `v<major>[.<minor>]`",
            self.input
        )
    }
}

impl std::error::Error for StylizedSchemaVersionParseError {}

impl FromStr for StylizedSchemaVersion {
    type Err = StylizedSchemaVersionParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let fail = || StylizedSchemaVersionParseError {
            input: input.to_string(),
        };
        let input = input.trim();
        let stripped = input.strip_prefix('v').unwrap_or(input);
        let mut parts = stripped.splitn(3, '.');
        let major = parts
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(fail)?
            .parse::<u32>()
            .map_err(|_| fail())?;
        let minor = match parts.next() {
            None => 0,
            Some(s) => s.parse::<u32>().map_err(|_| fail())?,
        };
        if parts.next().is_some() {
            return Err(fail());
        }
        Ok(Self { major, minor })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_parsing_accepts_documented_forms() {
        assert_eq!(
            "v1".parse::<StylizedSchemaVersion>(),
            Ok(StylizedSchemaVersion::new(1, 0))
        );
        assert_eq!(
            "v1.2".parse::<StylizedSchemaVersion>(),
            Ok(StylizedSchemaVersion::new(1, 2))
        );
        assert_eq!(
            "2.3".parse::<StylizedSchemaVersion>(),
            Ok(StylizedSchemaVersion::new(2, 3))
        );
        assert!("".parse::<StylizedSchemaVersion>().is_err());
        assert!("v".parse::<StylizedSchemaVersion>().is_err());
        assert!("v1.2.3".parse::<StylizedSchemaVersion>().is_err());
        assert!("vx".parse::<StylizedSchemaVersion>().is_err());
    }

    #[test]
    fn schema_id_display_matches_namespace_convention() {
        assert_eq!(
            SCHEMA_ID_MATERIAL_V1.to_string(),
            "bevy_stylized.material/v1"
        );
        assert!(
            SCHEMA_ID_MATERIAL_V1
                .namespace
                .starts_with(SCHEMA_NAMESPACE_PREFIX)
        );
    }

    #[test]
    fn additive_compatibility_rule() {
        let reader = StylizedSchemaVersion::new(1, 1);
        assert!(reader.can_read(StylizedSchemaVersion::new(1, 0)));
        assert!(reader.can_read(StylizedSchemaVersion::new(1, 1)));
        assert!(!reader.can_read(StylizedSchemaVersion::new(1, 2)));
        assert!(!reader.can_read(StylizedSchemaVersion::new(2, 0)));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn schema_version_serde_roundtrip() {
        let version = StylizedSchemaVersion::new(1, 2);
        let json = serde_json::to_string(&version).expect("serialize");
        let back: StylizedSchemaVersion = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(version, back);
    }
}
