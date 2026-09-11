//! Reference-only 排除检查（release gate："Reference-only code/assets
//! excluded from default shipping build"）。
//!
//! 规则：facade `bevy-stylized` 的 normal 依赖闭包中不得出现任何 legacy
//! oracle crate 或 dev-only crate（golden-harness / odette lab / 等）。
//! 以 `cargo metadata`（不经 cargo tree 的文本格式）为准——依赖图是
//! 结构事实，文本格式可能变。

use std::process::Command;

/// 禁止出现在 facade 生产依赖闭包中的包名。
const FORBIDDEN: &[&str] = &[
    "bevy_cel",
    "bevy_gltf_metadata",
    "golden-harness",
    "odette-reference-lab",
    "reference-comparison",
    "showcase",
    "benchmarks",
];

#[test]
fn facade_normal_dependency_closure_excludes_reference_and_dev_crates() {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata must run");
    assert!(output.status.success());
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("metadata json");
    let packages = metadata["packages"].as_array().expect("packages");
    let facade = packages
        .iter()
        .find(|p| p["name"] == "bevy-stylized")
        .expect("facade package present");

    let mut offenders = Vec::new();
    for dep in facade["dependencies"].as_array().expect("deps") {
        // kind: null = normal（"dev"/"build" 排除）。
        if !dep["kind"].is_null() {
            continue;
        }
        let name = dep["name"].as_str().expect("name");
        if FORBIDDEN.contains(&name) {
            offenders.push(name.to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "facade normal deps must not include reference/dev crates: {offenders:?}"
    );

    // 传递闭包：core/gltf/render/profile-genshin 也不得依赖 forbidden 包。
    for production in [
        "bevy-stylized-core",
        "bevy-stylized-gltf",
        "bevy-stylized-render",
        "bevy-stylized-profile-genshin",
    ] {
        let package = packages
            .iter()
            .find(|p| p["name"] == production)
            .unwrap_or_else(|| panic!("{production} present"));
        for dep in package["dependencies"].as_array().expect("deps") {
            if !dep["kind"].is_null() {
                continue;
            }
            let name = dep["name"].as_str().expect("name");
            assert!(
                !FORBIDDEN.contains(&name),
                "{production} must not depend on {name}"
            );
        }
    }
}
