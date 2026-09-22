//! 黄金合同向量：配对 输入模型 → 期望规范字节 + 期望 SHA-256 摘要。
//!
//! 向量文件在 `tests/vectors/*.json`，字段：
//!
//! - `model`：`json`（纯 RFC 8785 字节层，无语义展开）、`scene_recipe`、
//!   `session_plan`；
//! - `kind` + `version`：摘要输入的合同种类域与编译器版本（仅 typed 模型）；
//! - `input`：输入 JSON（typed 模型经完整摄入管线：严格解析 → 版本门控 →
//!   类型校验 → 合同校验）；
//! - `canonical`：期望的规范 UTF-8 字节（JSON 字符串承载）；
//! - `sha256`：期望摘要小写十六进制；`json` 模型为 `null`。
//!
//! 跨语言实现必须不经修改通过同一组向量（docs/showcase/01：Python/UI 传输
//! 规范字节与 hash，接收方按同一协议核对）。独立的 Python 消费脚本属于
//! 后续阶段；当前向量先由本测试冻结。

use std::path::PathBuf;

use scenario_model::canon;
use scenario_model::identity::{self, ContractKind};
use scenario_model::recipe::{SceneRecipe, SessionPlan};

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors")
}

fn contract_kind(name: &str) -> ContractKind {
    match name {
        "recipe" => ContractKind::Recipe,
        "session" => ContractKind::Session,
        "execution" => ContractKind::Execution,
        "artifact" => ContractKind::Artifact,
        other => panic!("unknown contract kind {other:?}"),
    }
}

#[test]
fn golden_contract_vectors() {
    let mut files: Vec<PathBuf> = std::fs::read_dir(vectors_dir())
        .expect("vectors directory must exist")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    assert!(files.len() >= 5, "expected at least 5 contract vectors");

    for path in files {
        let bytes = std::fs::read(&path).expect("read vector");
        // 向量文件自身也走重复键拒绝的摄入路径。
        let vector = canon::parse_strict(&bytes).expect("vector parses strictly");
        let name = vector["name"].as_str().expect("name").to_string();
        let version = vector["version"].as_str().expect("version");
        assert_eq!(
            version,
            canon::CANON_COMPILER_VERSION,
            "{name}: vector compiler version drifted"
        );
        let input = serde_json::to_string(&vector["input"]).expect("input re-encodes");
        let expected_canonical = vector["canonical"].as_str().expect("canonical");

        let canonical: Vec<u8> = match vector["model"].as_str().expect("model") {
            "json" => {
                let value = canon::parse_strict(input.as_bytes()).expect("strict input");
                canon::jcs_bytes(&value).expect("jcs")
            }
            "scene_recipe" => SceneRecipe::from_json_slice(input.as_bytes())
                .unwrap_or_else(|error| panic!("{name}: recipe ingestion failed: {error}"))
                .canonical_bytes()
                .expect("canonical bytes"),
            "session_plan" => SessionPlan::from_json_slice(input.as_bytes())
                .unwrap_or_else(|error| panic!("{name}: session plan ingestion failed: {error}"))
                .canonical_bytes()
                .expect("canonical bytes"),
            other => panic!("{name}: unknown model {other:?}"),
        };

        assert_eq!(
            String::from_utf8(canonical.clone()).expect("canonical is utf-8"),
            expected_canonical,
            "{name}: canonical bytes mismatch"
        );

        if let Some(expected_hex) = vector["sha256"].as_str() {
            let kind = contract_kind(vector["kind"].as_str().expect("kind"));
            let digest =
                identity::digest_hex(&identity::contract_digest(kind, version, &canonical));
            assert_eq!(digest, expected_hex, "{name}: sha256 mismatch");
        }
        println!("vector {name}: ok");
    }
}
