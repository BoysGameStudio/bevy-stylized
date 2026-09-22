//! 合同身份：SHA-256 域分离（docs/showcase/01 §场景、配方与验证身份）。
//!
//! hash 输入是 `(合同种类, schema/编译器版本, 规范 payload)` 的结构化编码，
//! 各字段以 32 位大端长度前缀分隔，不用裸字符串相接：
//!
//! ```text
//! SHA-256( len32(kind) ‖ kind ‖ len32(version) ‖ version ‖ len32(payload) ‖ payload )
//! ```
//!
//! recipe/session/execution/artifact 四个 hash 域由 [`ContractKind`] 明确区分；
//! 同一 payload 在不同种类下必须得到不同 hash。`payload` 始终是
//! [`crate::canon`] 产出的 JCS 规范字节；语义编译规则变更通过递增
//! [`crate::canon::CANON_COMPILER_VERSION`] 或 schema 版本表达，拒绝未知版本。

use sha2::Digest;

use crate::canon::CanonError;
use crate::recipe::{EvaluationSpec, RunSpec, SceneRecipe, SessionPlan};

/// 合同种类：hash 域。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContractKind {
    /// 配方身份（内容组合）。
    Recipe,
    /// 会话计划身份（可重复配置；注意与一次实际生命周期的 session_id 区分）。
    Session,
    /// 执行身份（计划 + run ID + 设备/驱动/backend + 实际启用能力等，
    /// 由执行层组装 payload）。
    Execution,
    /// 产物身份（证明内容相同，不证明通过了正确的规则）。
    Artifact,
}

impl ContractKind {
    /// 域标识字符串（含合同版本；编码规则变更时递增）。
    #[must_use]
    pub fn domain(self) -> &'static str {
        match self {
            ContractKind::Recipe => "scenario-model.recipe/v1",
            ContractKind::Session => "scenario-model.session/v1",
            ContractKind::Execution => "scenario-model.execution/v1",
            ContractKind::Artifact => "scenario-model.artifact/v1",
        }
    }
}

/// 合同摘要：SHA-256（长度前缀结构化编码，无歧义边界）。
#[must_use]
pub fn contract_digest(kind: ContractKind, version: &str, payload: &[u8]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    for field in [kind.domain().as_bytes(), version.as_bytes(), payload] {
        hasher.update((field.len() as u32).to_be_bytes());
        hasher.update(field);
    }
    hasher.finalize().into()
}

/// 摘要的小写十六进制表示。
#[must_use]
pub fn digest_hex(digest: &[u8; 32]) -> String {
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// 配方身份 hash（recipe 域）。
pub fn recipe_identity(recipe: &SceneRecipe) -> Result<String, CanonError> {
    let payload = recipe.canonical_bytes()?;
    Ok(digest_hex(&contract_digest(
        ContractKind::Recipe,
        crate::canon::CANON_COMPILER_VERSION,
        &payload,
    )))
}

/// 运行身份输入中的 RunSpec 部分（recipe 域之外的引用载荷）。
pub fn run_spec_identity(spec: &RunSpec) -> Result<String, CanonError> {
    let payload = spec.canonical_bytes()?;
    Ok(digest_hex(&contract_digest(
        ContractKind::Execution,
        crate::canon::CANON_COMPILER_VERSION,
        &payload,
    )))
}

/// 会话计划身份 hash（session 域；进入执行身份）。
pub fn session_identity(plan: &SessionPlan) -> Result<String, CanonError> {
    let payload = plan.canonical_bytes()?;
    Ok(digest_hex(&contract_digest(
        ContractKind::Session,
        crate::canon::CANON_COMPILER_VERSION,
        &payload,
    )))
}

/// 判定输入身份 hash（artifact 域：冻结全部验收输入）。
pub fn evaluation_identity(spec: &EvaluationSpec) -> Result<String, CanonError> {
    let payload = spec.canonical_bytes()?;
    Ok(digest_hex(&contract_digest(
        ContractKind::Artifact,
        crate::canon::CANON_COMPILER_VERSION,
        &payload,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_domains_are_separated() {
        let payload = b"{}";
        let digests: Vec<[u8; 32]> = [
            ContractKind::Recipe,
            ContractKind::Session,
            ContractKind::Execution,
            ContractKind::Artifact,
        ]
        .into_iter()
        .map(|kind| contract_digest(kind, "scenario-model.canon/v1", payload))
        .collect();
        for (i, a) in digests.iter().enumerate() {
            for b in &digests[i + 1..] {
                assert_ne!(a, b, "same payload must differ across domains");
            }
        }
    }

    #[test]
    fn structured_encoding_has_unambiguous_boundaries() {
        // 裸字符串拼接会把 ("ab","c") 与 ("a","bc") 混为同一输入；
        // 长度前缀编码必须区分。
        let a = contract_digest(ContractKind::Recipe, "ab", b"c");
        let b = contract_digest(ContractKind::Recipe, "a", b"bc");
        assert_ne!(a, b);
    }

    #[test]
    fn digest_hex_is_lowercase_64_chars() {
        let digest = contract_digest(ContractKind::Artifact, "v", b"payload");
        let hex = digest_hex(&digest);
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }
}
