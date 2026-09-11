//! End-to-end CLI contracts for CEL glTF validation.

mod validator_support;

use std::{fs, io};

use serde_json::Value;
use validator_support::{asset_path, run};

#[test]
fn help_exits_zero_when_requested() -> Result<(), Box<dyn std::error::Error>> {
    // Given / When
    let output = run(&["--help"])?;

    // Then
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)?.contains("Usage:"));
    Ok(())
}

#[test]
fn missing_input_exits_two_when_format_is_json() -> Result<(), Box<dyn std::error::Error>> {
    // Given / When
    let output = run(&["--format", "json"])?;

    // Then
    assert_eq!(output.status.code(), Some(2));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(envelope["schema"], "cel_gltf_validate.output");
    assert_eq!(envelope["schema_version"], 1);
    assert_eq!(envelope["diagnostics"][0]["code"], "CLI_INPUT_MISSING");
    assert_eq!(envelope["diagnostics"][0]["asset_path"], "$cli.input");
    Ok(())
}

#[test]
fn unsupported_format_exits_two_when_requested() -> Result<(), Box<dyn std::error::Error>> {
    // Given / When
    let output = run(&["--format", "yaml", "asset.glb"])?;

    // Then
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr)?.contains("CLI_FORMAT_UNSUPPORTED"));
    Ok(())
}

#[test]
fn unreadable_input_exits_one_with_typed_json_diagnostic() -> Result<(), Box<dyn std::error::Error>>
{
    // Given
    let directory = tempfile::tempdir()?;
    let missing = directory.path().join("missing.glb");

    // When
    let output = run(&["--format", "json", &missing.to_string_lossy()])?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(envelope["diagnostics"][0]["code"], "INPUT_UNREADABLE");
    assert_eq!(
        envelope["diagnostics"][0]["file_path"],
        missing.to_string_lossy().as_ref()
    );
    assert_eq!(envelope["diagnostics"][0]["asset_path"], "$file");
    Ok(())
}

#[test]
fn malformed_glb_exits_one_with_container_location() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempfile::tempdir()?;
    let malformed = directory.path().join("malformed.glb");
    fs::write(&malformed, b"glTF")?;

    // When
    let output = run(&["--format", "json", &malformed.to_string_lossy()])?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(envelope["diagnostics"][0]["code"], "GLB_HEADER_TRUNCATED");
    assert_eq!(envelope["diagnostics"][0]["asset_path"], "$glb.header");
    Ok(())
}

#[test]
fn unsupported_input_extension_exits_one_with_json_diagnostic()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("asset.txt");
    fs::write(&path, b"{}")?;

    // When
    let output = run(&["--format", "json", &path.to_string_lossy()])?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        envelope["diagnostics"][0]["code"],
        "INPUT_EXTENSION_UNSUPPORTED"
    );
    assert_eq!(envelope["diagnostics"][0]["asset_path"], "$file.extension");
    Ok(())
}

#[test]
fn json_output_is_deterministic_and_versioned() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let path = asset_path("toon_sphere.glb");
    let argument = path.to_string_lossy();

    // When
    let first = run(&["--format", "json", &argument])?;
    let second = run(&["--format", "json", &argument])?;

    // Then
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    let envelope: Value = serde_json::from_slice(&first.stdout)?;
    assert_eq!(envelope["schema"], "cel_gltf_validate.output");
    assert_eq!(envelope["schema_version"], 1);
    assert_eq!(envelope["valid"], true);
    assert_eq!(envelope["files"], serde_json::json!([argument.as_ref()]));
    assert_eq!(envelope["diagnostics"], serde_json::json!([]));
    Ok(())
}

#[test]
fn every_task4_glb_exits_zero_without_writes() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let fixture_names = [
        "cape_grid.glb",
        "eye_emission_fixture.glb",
        "face_sdf_fixture.glb",
        "hair_tangent_strip.glb",
        "ilm_two_region_mesh.glb",
        "morph_cube.glb",
        "skein_level_fixture.glb",
        "skinned_two_bone_mesh.glb",
        "toon_sphere.glb",
    ];

    // When / Then
    for name in fixture_names {
        let path = asset_path(name);
        let before = fs::read(&path)?;
        let output = run(&["--format", "json", &path.to_string_lossy()])?;
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(fs::read(path)?, before, "validator changed {name}");
    }
    Ok(())
}

#[test]
fn human_output_reports_success_concisely() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let path = asset_path("toon_sphere.glb");

    // When
    let output = run(&[&path.to_string_lossy()])?;

    // Then
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.starts_with("VALID "));
    assert!(!stdout.contains("\n\n"));
    assert!(
        output.stderr.is_empty(),
        "{}",
        io::Error::other("unexpected stderr")
    );
    Ok(())
}
