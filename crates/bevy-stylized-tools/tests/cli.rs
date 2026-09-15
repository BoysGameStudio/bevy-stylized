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
        "/../../fixtures/synthetic/reference-scene/reference_scene.glb"
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
        "/../../fixtures/synthetic/reference-scene/reference_scene.glb"
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
        "/../../fixtures/synthetic/reference-scene/reference_scene.glb"
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

// ---------------------------------------------------------------------------
// compile（V2-f Asset Compiler 骨架）
// ---------------------------------------------------------------------------

/// 合成带 `bevy_stylized.material/v1` + `bevy_stylized.compile/v1` extras 的
/// 单 material glTF，以及四张 2x2 单通道 ILM 源 PNG（每通道不同像素值）。
fn write_synthetic_with_ilm(dir: &Path) -> std::path::PathBuf {
    let tex = dir.join("tex");
    std::fs::create_dir_all(&tex).expect("mkdir tex");
    // 每通道 2x2 不同值，行主序。
    let channels: [(&str, [u8; 4]); 4] = [
        ("metal", [0, 64, 128, 255]),
        ("ao", [10, 20, 30, 40]),
        ("spec", [1, 2, 3, 4]),
        ("id", [50, 100, 150, 200]),
    ];
    for (name, pixels) in channels {
        let image = image::GrayImage::from_raw(2, 2, pixels.to_vec()).expect("gray image");
        image::DynamicImage::ImageLuma8(image)
            .save(tex.join(format!("{name}.png")))
            .expect("write png");
    }
    let gltf = serde_json::json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0 },
                "mode": 4,
                "material": 0
            }]
        }],
        "materials": [{
            "name": "SynthMat",
            "extras": {
                "bevy_stylized.material/v1": { "surface_class": "hero" },
                "bevy_stylized.compile/v1": {
                    "schema_version": { "major": 1, "minor": 0 },
                    "packs": [{
                        "name": "synth_ilm",
                        "kind": "ilm",
                        "channels": {
                            "metal": "tex/metal.png",
                            "ao": "tex/ao.png",
                            "specular_suppression": "tex/spec.png",
                            "material_id": "tex/id.png"
                        }
                    }]
                }
            }
        }],
        "buffers": [{ "uri": "data.bin", "byteLength": 36 }],
        "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0] }
        ]
    });
    let path = dir.join("asset.gltf");
    std::fs::write(&path, serde_json::to_string_pretty(&gltf).expect("json")).expect("write gltf");
    path
}

#[test]
fn compile_fixture_emits_parseable_descriptors() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/synthetic/reference-scene/reference_scene.glb"
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join("out");
    let (code, text) = run_cli(&["compile", "--out", out.to_str().expect("utf8"), fixture]);
    assert_eq!(code, 0, "fixture compile must succeed: {text}");

    // 4 个 fixture material 各自产出 descriptor，且能经 core serde 路径解析。
    let materials_dir = out.join("materials");
    let mut names: Vec<String> = std::fs::read_dir(&materials_dir)
        .expect("materials dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names.len(), 4, "one descriptor per material: {names:?}");
    for file in &names {
        let json = std::fs::read_to_string(materials_dir.join(file)).expect("read descriptor");
        let descriptor: bevy_stylized_core::StylizedMaterialDescriptor =
            serde_json::from_str(&json).expect("descriptor must parse via core serde path");
        assert_eq!(
            descriptor.schema_version,
            bevy_stylized_core::StylizedSchemaVersion::new(1, 0)
        );
        assert_eq!(descriptor.name, file.trim_end_matches(".json"));
    }
}

#[test]
fn compile_packs_ilm_and_rewrites_texture_ref() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_synthetic_with_ilm(dir.path());
    let out = dir.path().join("out");
    let (code, text) = run_cli(&[
        "compile",
        "--json",
        "--out",
        out.to_str().expect("utf8"),
        input.to_str().expect("utf8"),
    ]);
    assert_eq!(code, 0, "compile must succeed: {text}");

    // --json 报告：产物清单 + sha256。
    let report: serde_json::Value = serde_json::from_str(&text).expect("valid json report");
    assert_eq!(report["command"], "compile");
    assert_eq!(report["violation"], false);
    let emitted = report["emitted"].as_array().expect("emitted array");
    let ilm = emitted
        .iter()
        .find(|e| e["path"] == "textures/synth_ilm.png")
        .expect("packed ILM emitted");
    assert_eq!(ilm["kind"], "texture");
    assert_eq!(ilm["sha256"].as_str().expect("hash").len(), 64);
    assert!(
        emitted
            .iter()
            .any(|e| e["path"] == "materials/SynthMat.json" && e["kind"] == "material")
    );

    // packed ILM 通道内容：R=metal / G=AO / B=spec / A=id（逐像素）。
    let packed = image::open(out.join("textures/synth_ilm.png"))
        .expect("decode packed")
        .to_rgba8();
    assert_eq!(packed.dimensions(), (2, 2));
    let expected = [
        [0, 10, 1, 50],
        [64, 20, 2, 100],
        [128, 30, 3, 150],
        [255, 40, 4, 200],
    ];
    for (index, want) in expected.iter().enumerate() {
        let (x, y) = (index as u32 % 2, index as u32 / 2);
        assert_eq!(packed.get_pixel(x, y).0, *want, "pixel ({x},{y})");
    }

    // descriptor 回读：schema 版本 + ContinuousMask 引用相对输出目录。
    let json = std::fs::read_to_string(out.join("materials/SynthMat.json")).expect("descriptor");
    let descriptor: bevy_stylized_core::StylizedMaterialDescriptor =
        serde_json::from_str(&json).expect("parse via core serde");
    assert_eq!(
        descriptor.surface_class,
        bevy_stylized_core::StylizedSurfaceClass::Hero
    );
    assert!(
        descriptor
            .textures
            .iter()
            .any(|t| t.uri == "textures/synth_ilm.png"
                && t.semantic == bevy_stylized_core::StylizedTextureSemantic::ContinuousMask),
        "packed ILM ref must be in descriptor: {:?}",
        descriptor.textures
    );
}

#[test]
fn compile_missing_pack_source_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_synthetic_with_ilm(dir.path());
    std::fs::remove_file(dir.path().join("tex/ao.png")).expect("remove ao");
    let out = dir.path().join("out");
    let (code, text) = run_cli(&[
        "compile",
        "--out",
        out.to_str().expect("utf8"),
        input.to_str().expect("utf8"),
    ]);
    assert_eq!(code, 1, "missing ILM source must exit 1: {text}");
    assert!(
        text.contains("AO(G)"),
        "diagnostic should name the channel: {text}"
    );
    // fail-closed：不写出任何产物。
    assert!(
        !out.join("materials/SynthMat.json").exists(),
        "no partial artifacts on violation"
    );
}

#[test]
fn compile_contract_violation_blocks_emission() {
    let dir = tempfile::tempdir().expect("tempdir");
    // _NPR_MASK 形态错误 → validate 违规 → compile 蕴含 validate，exit 1 且不产出。
    let path = write_synthetic(dir.path(), 5126, "VEC3");
    let out = dir.path().join("out");
    let (code, text) = run_cli(&[
        "compile",
        "--out",
        out.to_str().expect("utf8"),
        path.to_str().expect("utf8"),
    ]);
    assert_eq!(code, 1, "compile implies validate: {text}");
    assert!(!out.exists(), "no output dir on validation failure");
}

#[test]
fn compile_out_flag_rejected_for_other_commands() {
    let (code, _) = run_cli(&["validate", "--out", "x", "no/such/file.glb"]);
    assert_eq!(code, 2, "--out is compile-only");
}

#[test]
fn compile_directives_from_sidecar() {
    let dir = tempfile::tempdir().expect("tempdir");
    // 合成 glTF 不带 compile extras，指令走 sidecar 顶层 `compile` 键。
    let gltf = serde_json::json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{
            "primitives": [{ "attributes": { "POSITION": 0 }, "mode": 4, "material": 0 }]
        }],
        "materials": [{ "name": "Body" }],
        "buffers": [{ "uri": "data.bin", "byteLength": 36 }],
        "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
              "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0] }
        ]
    });
    let input = dir.path().join("asset.gltf");
    std::fs::write(&input, serde_json::to_string_pretty(&gltf).expect("json")).expect("write gltf");
    let tex = dir.path().join("tex");
    std::fs::create_dir_all(&tex).expect("mkdir tex");
    for (name, value) in [("metal", 7u8), ("ao", 8), ("spec", 9), ("id", 10)] {
        let image = image::GrayImage::from_raw(1, 1, vec![value]).expect("gray");
        image::DynamicImage::ImageLuma8(image)
            .save(tex.join(format!("{name}.png")))
            .expect("png");
    }
    let sidecar = serde_json::json!({
        "schema": "bevy-stylized.sidecar/v1",
        "materials": { "Body": { "surface_class": "character" } },
        "compile": {
            "Body": {
                "packs": [{
                    "name": "side_ilm",
                    "kind": "ilm",
                    "channels": {
                        "metal": "tex/metal.png",
                        "ao": "tex/ao.png",
                        "specular_suppression": "tex/spec.png",
                        "material_id": "tex/id.png"
                    }
                }]
            }
        }
    });
    // asset.gltf → sidecar 为 asset.npr.json。
    std::fs::write(
        dir.path().join("asset.npr.json"),
        serde_json::to_string_pretty(&sidecar).expect("json"),
    )
    .expect("write sidecar");

    let out = dir.path().join("out");
    let (code, text) = run_cli(&[
        "compile",
        "--out",
        out.to_str().expect("utf8"),
        input.to_str().expect("utf8"),
    ]);
    assert_eq!(code, 0, "sidecar compile must succeed: {text}");
    let packed = image::open(out.join("textures/side_ilm.png"))
        .expect("decode packed")
        .to_rgba8();
    assert_eq!(packed.get_pixel(0, 0).0, [7, 8, 9, 10]);
    let json = std::fs::read_to_string(out.join("materials/Body.json")).expect("descriptor");
    let descriptor: bevy_stylized_core::StylizedMaterialDescriptor =
        serde_json::from_str(&json).expect("parse via core serde");
    assert_eq!(
        descriptor.surface_class,
        bevy_stylized_core::StylizedSurfaceClass::Character
    );
    assert!(
        descriptor
            .textures
            .iter()
            .any(|t| t.uri == "textures/side_ilm.png")
    );
}

#[test]
fn usage_error_exits_2() {
    let (code, _) = run_cli(&["frobnicate"]);
    assert_eq!(code, 2);
}

#[cfg(feature = "profile-genshin")]
#[test]
fn strict_profile_covers_every_material_and_publishes_atomically() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("asset.glb");
    let original = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/synthetic/reference-scene/reference_scene.glb"
    ))
    .unwrap();
    std::fs::write(&input, &original).unwrap();
    let document = gltf::Gltf::from_slice(&original).unwrap();
    image::RgbaImage::from_pixel(2, 2, image::Rgba([200, 150, 100, 255]))
        .save(temp.path().join("color.png"))
        .unwrap();
    let materials:Vec<_> = document.materials().map(|m|serde_json::json!({
        "name":m.name().unwrap(),"role":"brow","base_color":"color.png","ilm":null,"ramp":null,"face_sdf":null,
        "shadow_position":0.55,"ramp_width":1.0,"face_softness":0.01,"face_forward":[0,0,1],"face_up":[0,1,0],
        "metallic":false,"specular_ramp":false,"packed_normal":false
    })).collect();
    let mut payload = serde_json::json!({"schema":"bevy_stylized.genshin-input/v1","variant":"explicit-test",
        "materials":materials,"palette":{"Ambient Colour":[1,1,1,1],"Soft Lit Colour":[1,1,1,1],"Sharp Lit Colour":[1,1,1,1],"Soft Shadow Color":[0.5,0.5,0.5,1],"Sharp Shadow Color":[0.5,0.5,0.5,1]}});
    let profile = input.with_extension("genshin.json");
    std::fs::write(&profile, serde_json::to_vec(&payload).unwrap()).unwrap();
    let out = temp.path().join("compiled");
    let command = |out: &Path| {
        Command::new(env!("CARGO_BIN_EXE_bevy-stylized"))
            .args([
                "compile",
                "--profile",
                "genshin",
                "--strict",
                "--json",
                "--out",
            ])
            .arg(out)
            .arg(&input)
            .output()
            .unwrap()
    };
    assert!(command(&out).status.success());
    let sidecar = bevy_stylized_gltf::load_sidecar(out.join("asset.npr.json")).unwrap();
    assert_eq!(sidecar.materials.len(), document.materials().len());
    assert_eq!(std::fs::read(&input).unwrap(), original);
    let accepted = std::fs::read(out.join("compile-manifest.json")).unwrap();
    assert_eq!(command(&out).status.code(), Some(2));
    assert_eq!(
        std::fs::read(out.join("compile-manifest.json")).unwrap(),
        accepted
    );
    payload["materials"].as_array_mut().unwrap().pop();
    std::fs::write(&profile, serde_json::to_vec(&payload).unwrap()).unwrap();
    let bad = temp.path().join("incomplete");
    assert_eq!(command(&bad).status.code(), Some(1));
    assert!(
        !bad.exists(),
        "partial profile coverage must not emit a package"
    );
}

#[cfg(not(feature = "profile-genshin"))]
#[test]
fn unavailable_profile_cannot_fall_back_to_generic_compile() {
    let output = Command::new(env!("CARGO_BIN_EXE_bevy-stylized"))
        .args(["compile", "--profile", "genshin", "--strict", "missing.glb"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("profile-genshin"));
}
