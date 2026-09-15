//! Export JSON Schema directly from the authoritative Rust serialization types.
use std::path::PathBuf;
fn main() {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(
        args.next()
            .expect("usage: stylized-schema OUT_DIR [--check]"),
    );
    let check = args.next().as_deref() == Some("--check");
    let schemas = [
        (
            "material-v1.schema.json",
            schemars::schema_for!(bevy_stylized_core::StylizedMaterialDescriptor),
        ),
        (
            "instance-v1.schema.json",
            schemars::schema_for!(bevy_stylized_core::StylizedInstanceGpuRecord),
        ),
        (
            "view-v1.schema.json",
            schemars::schema_for!(bevy_stylized_core::StylizedViewDescriptor),
        ),
        (
            "sidecar-v1.schema.json",
            schemars::schema_for!(bevy_stylized_gltf::StylizedSidecar),
        ),
    ];
    if !check {
        std::fs::create_dir_all(&out).expect("create schema directory");
    }
    for (name, schema) in schemas {
        let text = serde_json::to_string_pretty(&schema).unwrap() + "\n";
        let path = out.join(name);
        if check {
            assert_eq!(
                std::fs::read_to_string(&path).expect("schema snapshot missing"),
                text,
                "stale schema: {name}"
            );
        } else {
            std::fs::write(path, text).expect("write schema");
        }
    }
}
