//! CLI 集成测试：合成 glTF + 真实 fixture（exit code 契约）。

use std::{path::Path, process::Command};

/// 合成一个最小 .gltf（单三角形 + 指定 `_NPR_MASK` accessor 形态）。
fn write_synthetic(dir: &Path, mask_component_type: u32, mask_type: &str) -> std::path::PathBuf {
    // positions(9 f32) + mask(3 f32 or 9 f32 if Vec3)
    let mut bin = Vec::new();
    for v in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        for c in v {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    let mask_offset = bin.len();
    let mask_len = if mask_type == "VEC3" { 36 } else { 12 };
    bin.resize(mask_offset + mask_len, 0);
    let gltf = serde_json::json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0, "_NPR_MASK": 1 },
                "mode": 4
            }]
        }],
        "buffers": [{ "uri": "data.bin", "byteLength": bin.len() }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": mask_offset, "byteLength": mask_len }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0] },
            { "bufferView": 1, "componentType": mask_component_type, "count": 3, "type": mask_type }
        ]
    });
    std::fs::write(dir.join("data.bin"), &bin).expect("write bin");
    let path = dir.join("test.gltf");
    std::fs::write(&path, serde_json::to_string_pretty(&gltf).expect("json")).expect("write gltf");
    path
}

fn run_cli(args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_bevy-stylized"))
        .args(args)
        .output()
        .expect("run cli");
    (
        output.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

#[test]
fn fixture_glb_validates_clean() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/reference/reference_scene.glb"
    );
    let (code, out) = run_cli(&["validate", path]);
    assert_eq!(code, 0, "fixture must validate clean: {out}");
    assert!(
        out.contains("_NPR_FACE_SDF"),
        "should list attributes: {out}"
    );
    assert!(!out.contains("[error]"), "no errors expected: {out}");
}

#[test]
fn fixture_glb_inspect_reports_classes() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/reference/reference_scene.glb"
    );
    let (code, out) = run_cli(&["inspect", path]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("meshes=4"), "{out}");
    assert!(
        out.contains("Character"),
        "face SDF present ⇒ Character: {out}"
    );
    assert!(
        out.contains("Hero"),
        "hair/mask attrs present ⇒ Hero: {out}"
    );
}

#[test]
fn wrong_accessor_type_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    // 契约：_NPR_MASK 必须 Scalar/F32；给 Vec3 → error → exit 1。
    let path = write_synthetic(dir.path(), 5126, "VEC3");
    let (code, out) = run_cli(&["validate", path.to_str().expect("utf8")]);
    assert_eq!(code, 1, "contract violation must exit 1: {out}");
    assert!(out.contains("契约"), "{out}");
}

#[test]
fn correct_synthetic_gltf_passes_with_warnings() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_synthetic(dir.path(), 5126, "SCALAR");
    let (code, out) = run_cli(&["validate", path.to_str().expect("utf8")]);
    assert_eq!(code, 0, "warn-only missing attrs must not fail: {out}");
    assert!(
        out.contains("[warning]"),
        "missing optional attrs warn: {out}"
    );
}

#[test]
fn json_output_is_machine_readable() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/reference/reference_scene.glb"
    );
    let (code, out) = run_cli(&["validate", "--json", path]);
    assert_eq!(code, 0, "{out}");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid json");
    assert_eq!(parsed["violation"], false);
    assert_eq!(parsed["attributes"].as_array().expect("array").len(), 10);
}

#[test]
fn missing_file_exits_2() {
    let (code, out) = run_cli(&["validate", "no/such/file.glb"]);
    assert_eq!(code, 2, "io error must exit 2: {out}");
}

#[test]
fn usage_error_exits_2() {
    let (code, _) = run_cli(&["frobnicate"]);
    assert_eq!(code, 2);
}
