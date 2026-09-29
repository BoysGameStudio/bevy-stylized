//! 身份作用域类型（docs/showcase/01 §场景、配方与验证身份）。
//!
//! 各作用域语义不可互换：
//!
//! - [`SceneId`] + [`PresetId`] 决定内容（例如 `outline`/`hybrid`）；
//! - [`SessionId`] 标记一次实际 App/Device 生命周期：`SessionPlan` 是可重复
//!   配置，`SessionId` 不是；同一计划重建设备必须产生新的 `SessionId`；
//! - [`Generation`] 标记实验内容的一次实例化，[`Revision`] 标记其语义编辑；
//! - [`SampleId`] 标记一次被批准的时间/历史采样；恢复或重新采样同一 tick
//!   不复用 `SampleId`；
//! - [`ViewId`] 在实验身份内唯一；[`RequestId`] 标记一次具体读回。
//!
//! 所有回执保留这些作用域，不能单凭 Entity、tick 或跨场景复用计数接受
//! 结果；计数耗尽必须结束所属生命周期，不能回绕。字符串身份限制为
//! ASCII 可打印子集（字母/数字/`-._/`），64 位数值身份按
//! [`crate::canon::CanonicalU64`] 编码为规范十进制字符串。

use serde::{Deserialize, Serialize};

/// 字符串身份校验失败。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid {kind} {value:?}: {reason}")]
pub struct IdentError {
    /// 身份种类（如 `SceneId`）。
    pub kind: &'static str,
    /// 被拒绝的原始值。
    pub value: String,
    /// 拒绝原因。
    pub reason: &'static str,
}

fn validate_string_id<'a>(kind: &'static str, value: &'a str) -> Result<&'a str, IdentError> {
    let fail = |reason: &'static str| IdentError {
        kind,
        value: value.to_string(),
        reason,
    };
    if value.is_empty() {
        return Err(fail("empty identifier"));
    }
    if value.len() > 128 {
        return Err(fail("identifier longer than 128 bytes"));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
    {
        return Err(fail("only ASCII alphanumeric and `-._/` are allowed"));
    }
    Ok(value)
}

macro_rules! string_identity {
    ($(#[$meta:meta])* $name:ident, $doc_scope:literal) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// 构造并校验字符集/长度。
            pub fn new(value: impl Into<String>) -> Result<Self, IdentError> {
                let value = value.into();
                validate_string_id(stringify!($name), &value)?;
                Ok(Self(value))
            }

            /// 字符串视图。
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdentError;

            fn try_from(value: String) -> Result<Self, IdentError> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.0
            }
        }
    };
}

string_identity!(
    /// 场景身份：内容选择的稳定 ID（目录注册的唯一键）。
    SceneId, "content"
);
string_identity!(
    /// preset 身份：同一 [`SceneId`] 下的内容变体。
    PresetId, "content variant"
);
string_identity!(
    /// 会话身份：一次实际 App/Device 生命周期；同计划重建设备也必须更新。
    SessionId, "session"
);
string_identity!(
    /// 采样身份：一次被批准的时间/历史采样；重新采样不复用。
    SampleId, "sample"
);
string_identity!(
    /// 视图身份：在实验身份内唯一。
    ViewId, "view"
);
string_identity!(
    /// 请求身份：一次具体读回；重复提交同 ID 返回当前状态而不重复执行。
    RequestId, "request"
);
string_identity!(
    /// 动作身份：动作图节点的稳定逻辑 ID。
    ActionId, "action"
);
string_identity!(
    /// 组合节点/目标身份：实体、材质、灯光、view 等稳定逻辑 ID。
    TargetId, "target"
);

macro_rules! numeric_identity {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        ///
        /// 64 位计数按规范十进制字符串编码（见 [`crate::canon::CanonicalU64`]），
        /// 耗尽必须结束所属生命周期，不能回绕。
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
        pub struct $name(pub crate::canon::CanonicalU64);

        impl $name {
            /// 构造。
            #[must_use]
            pub fn new(value: u64) -> Self {
                Self(crate::canon::CanonicalU64(value))
            }

            /// 取计数值。
            #[must_use]
            pub fn get(self) -> u64 {
                self.0.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

numeric_identity!(
    /// 代次：实验内容的一次实例化。
    Generation
);
numeric_identity!(
    /// 修订：实验内容的一次语义编辑；过期 revision 不能修改新场景。
    Revision
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_id_accepts_registered_names() {
        for name in ["courtyard", "courtyard-day", "outline-advanced"] {
            SceneId::new(name).expect(name);
        }
    }

    #[test]
    fn string_id_rejects_empty_control_and_space() {
        assert!(SceneId::new("").is_err());
        assert!(SceneId::new("has space").is_err());
        assert!(SceneId::new("bad\ttab").is_err());
        assert!(SceneId::new("x".repeat(129)).is_err());
    }

    #[test]
    fn numeric_id_serializes_as_decimal_string() {
        let generation = Generation::new(9_007_199_254_740_993);
        let json = serde_json::to_string(&generation).expect("serialize");
        assert_eq!(json, "\"9007199254740993\"");
        let back: Generation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, generation);
    }
}
