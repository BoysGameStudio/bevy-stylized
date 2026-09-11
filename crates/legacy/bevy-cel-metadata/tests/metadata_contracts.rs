//! Metadata decoding and resolution contract tests.

use std::path::PathBuf;

use bevy_gltf_metadata::{
    CEL_SCHEMA_VERSION, CelSchemaVersion, MetadataError, MetadataInput, MetadataSource,
    decode_metadata, load_sidecar, resolve_metadata,
};

#[derive(Debug, thiserror::Error)]
enum TestError {
    #[error("operation unexpectedly succeeded")]
    UnexpectedSuccess,
    #[error("operation returned the wrong error variant")]
    WrongErrorVariant,
}

fn extension_source() -> MetadataSource {
    MetadataSource::Extension {
        asset: PathBuf::from("hero.glb"),
        extension: "CEL_metadata".to_owned(),
    }
}

fn sidecar_source() -> MetadataSource {
    MetadataSource::Sidecar(PathBuf::from("hero.cel.json"))
}

#[test]
fn decode_accepts_valid_v1_and_unknown_fields() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let json = br#"{
        "schema_version": 1,
        "future_envelope_field": true,
        "nodes": {
            "Hero": {
                "future_node_field": {"ignored": true},
                "domains": {"render": {"outline_width": 0.03}},
                "components": {"example::Health": {"value": 100}}
            }
        }
    }"#;

    // When
    let decoded = decode_metadata(json, extension_source())?;

    // Then
    assert_eq!(decoded.schema_version, CelSchemaVersion::V1);
    assert_eq!(decoded.schema_version.as_u32(), CEL_SCHEMA_VERSION);
    assert_eq!(
        decoded.nodes["Hero"].domains["render"]["outline_width"],
        0.03
    );
    assert_eq!(
        decoded.nodes["Hero"].components["example::Health"]["value"],
        100
    );
    Ok(())
}

#[test]
fn decode_rejects_malformed_shape_with_json_path() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let json = br#"{
        "schema_version": 1,
        "nodes": {"Hero": {"domains": [], "components": {}}}
    }"#;

    // When
    let Err(error) = decode_metadata(json, sidecar_source()) else {
        return Err(TestError::UnexpectedSuccess.into());
    };

    // Then
    match error {
        MetadataError::Json {
            origin, json_path, ..
        } => {
            assert_eq!(origin, sidecar_source());
            assert_eq!(json_path, "$.nodes.Hero.domains");
        }
        _ => return Err(TestError::WrongErrorVariant.into()),
    }
    Ok(())
}

#[test]
fn decode_rejects_unknown_schema_version_with_version_path()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let json = br#"{"schema_version": 7, "nodes": {}}"#;

    // When
    let Err(error) = decode_metadata(json, extension_source()) else {
        return Err(TestError::UnexpectedSuccess.into());
    };

    // Then
    match error {
        MetadataError::UnsupportedVersion {
            origin,
            json_path,
            found,
            supported,
        } => {
            assert_eq!(origin, extension_source());
            assert_eq!(json_path, "$.schema_version");
            assert_eq!(found, 7);
            assert_eq!(supported, CEL_SCHEMA_VERSION);
        }
        _ => return Err(TestError::WrongErrorVariant.into()),
    }
    Ok(())
}

#[test]
fn resolve_uses_sidecar_as_fallback_and_extension_per_domain_precedence()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let extension = br#"{
        "schema_version": 1,
        "nodes": {"Hero": {
            "domains": {"render": {"source": "extension"}},
            "components": {"example::Health": {"value": 200}},
            "references": {"bones": ["ExtensionBone"]}
        }}
    }"#;
    let sidecar = br#"{
        "schema_version": 1,
        "nodes": {"Hero": {
            "domains": {
                "render": {"source": "sidecar"},
                "physics": {"mass": 12.0}
            },
            "components": {
                "example::Health": {"value": 100},
                "example::Name": {"value": "Hero"}
            },
            "references": {
                "bones": ["SidecarBone"],
                "prefabs": ["hero.glb"],
                "colliders": ["HeroCollider"]
            }
        }}
    }"#;

    // When
    let resolved = resolve_metadata(
        Some(MetadataInput::new(extension, extension_source())),
        Some(MetadataInput::new(sidecar, sidecar_source())),
    )?;

    // Then
    assert_eq!(
        resolved.metadata.nodes["Hero"].domains["render"]["source"],
        "extension"
    );
    assert_eq!(
        resolved.metadata.nodes["Hero"].domains["physics"]["mass"],
        12.0
    );
    assert_eq!(
        resolved.metadata.nodes["Hero"].components["example::Health"]["value"],
        200
    );
    assert_eq!(
        resolved.sources["$.nodes.Hero.domains.render"],
        extension_source()
    );
    assert_eq!(
        resolved.sources["$.nodes.Hero.domains.physics"],
        sidecar_source()
    );
    assert_eq!(
        resolved.metadata.nodes["Hero"].references.bones,
        ["ExtensionBone"]
    );
    assert_eq!(
        resolved.metadata.nodes["Hero"].references.prefabs,
        ["hero.glb"]
    );
    assert_eq!(
        resolved.sources["$.nodes.Hero.references.bones"],
        extension_source()
    );
    assert_eq!(
        resolved.sources["$.nodes.Hero.references.colliders"],
        sidecar_source()
    );
    Ok(())
}

#[test]
fn resolve_uses_sidecar_when_extension_is_absent() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let sidecar = br#"{
        "schema_version": 1,
        "nodes": {"Hero": {"domains": {"render": {"ramp": "warm"}}}}
    }"#;

    // When
    let resolved = resolve_metadata(None, Some(MetadataInput::new(sidecar, sidecar_source())))?;

    // Then
    assert_eq!(
        resolved.metadata.nodes["Hero"].domains["render"]["ramp"],
        "warm"
    );
    assert_eq!(
        resolved.sources["$.nodes.Hero.domains.render"],
        sidecar_source()
    );
    Ok(())
}

#[test]
fn load_sidecar_reports_malformed_input_without_leaving_temporary_files()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("malformed.cel.json");
    std::fs::write(
        &path,
        br#"{"schema_version":1,"nodes":{"Hero":{"domains":[]}}}"#,
    )?;

    // When
    let Err(error) = load_sidecar(path.clone()) else {
        return Err(TestError::UnexpectedSuccess.into());
    };

    // Then
    eprintln!("{error}");
    match error {
        MetadataError::Json {
            origin, json_path, ..
        } => {
            assert_eq!(origin, MetadataSource::Sidecar(path));
            assert_eq!(json_path, "$.nodes.Hero.domains");
        }
        _ => return Err(TestError::WrongErrorVariant.into()),
    }
    Ok(())
}
