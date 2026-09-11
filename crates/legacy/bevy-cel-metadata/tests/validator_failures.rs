//! Independent malformed-copy validation scenarios.

mod validator_support;

use std::{fs, io, path::Path, process::Output};

use serde_json::{Value, json};
use validator_support::{asset_path, run};

fn read_glb(path: &Path) -> Result<(Value, Vec<u8>), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let chunk_length = u32::from_le_bytes(bytes[12..16].try_into()?) as usize;
    let json_end = 20 + chunk_length;
    let json = serde_json::from_slice(bytes[20..json_end].trim_ascii_end())?;
    Ok((json, bytes[json_end..].to_vec()))
}

fn write_glb(path: &Path, json: &Value, trailing: &[u8]) -> io::Result<()> {
    let mut json_bytes = serde_json::to_vec(json)?;
    json_bytes.resize(json_bytes.len().next_multiple_of(4), b' ');
    let total_length = 20 + json_bytes.len() + trailing.len();
    let mut bytes = Vec::with_capacity(total_length);
    bytes.extend_from_slice(b"glTF");
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(total_length)
            .map_err(io::Error::other)?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(
        &u32::try_from(json_bytes.len())
            .map_err(io::Error::other)?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&0x4e4f_534a_u32.to_le_bytes());
    bytes.extend_from_slice(&json_bytes);
    bytes.extend_from_slice(trailing);
    fs::write(path, bytes)
}

fn diagnostic_codes(output: &Output) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    let diagnostics = envelope["diagnostics"]
        .as_array()
        .ok_or_else(|| io::Error::other("diagnostics was not an array"))?;
    diagnostics
        .iter()
        .map(|diagnostic| {
            diagnostic["code"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| io::Error::other("diagnostic code was not a string").into())
        })
        .collect()
}

fn run_probe(path: &std::path::Path) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(run(&["--format", "json", &path.to_string_lossy()])?)
}

#[test]
fn missing_image_reference_fails_independently() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("missing-image.glb");
    let (mut document, trailing) = read_glb(&asset_path("toon_sphere.glb"))?;
    document["images"] = json!([{"uri": "missing.png"}]);
    write_glb(&path, &document, &trailing)?;

    // When
    let output = run_probe(&path)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(diagnostic_codes(&output)?, ["FILE_REFERENCE_MISSING"]);
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(envelope["diagnostics"][0]["asset_path"], "$.images[0].uri");
    Ok(())
}

#[test]
fn unsupported_metadata_version_fails_independently() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("wrong-version.glb");
    let (mut document, trailing) = read_glb(&asset_path("toon_sphere.glb"))?;
    document["extensions"] = json!({"CEL_metadata": {"schema_version": 2, "nodes": {}}});
    document["extensionsUsed"] = json!(["CEL_metadata"]);
    write_glb(&path, &document, &trailing)?;

    // When
    let output = run_probe(&path)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(diagnostic_codes(&output)?, ["METADATA_VERSION_UNSUPPORTED"]);
    Ok(())
}

#[test]
fn invalid_cel_attribute_format_fails_independently() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("invalid-attribute.glb");
    let (mut document, trailing) = read_glb(&asset_path("toon_sphere.glb"))?;
    let accessor = document["meshes"][0]["primitives"][0]["attributes"]["_CEL_MASK"]
        .as_u64()
        .ok_or_else(|| io::Error::other("fixture CEL accessor was not an integer"))?;
    document["accessors"][usize::try_from(accessor)?]["type"] = json!("VEC3");
    write_glb(&path, &document, &trailing)?;

    // When
    let output = run_probe(&path)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(diagnostic_codes(&output)?, ["CEL_ATTRIBUTE_FORMAT_INVALID"]);
    Ok(())
}

#[test]
fn unknown_skin_joint_fails_as_bone_target() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("broken-bone.glb");
    let (mut document, trailing) = read_glb(&asset_path("skinned_two_bone_mesh.glb"))?;
    document["skins"][0]["joints"][0] = json!(99_999);
    write_glb(&path, &document, &trailing)?;

    // When
    let output = run_probe(&path)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(diagnostic_codes(&output)?, ["BONE_TARGET_UNKNOWN"]);
    Ok(())
}

#[test]
fn malformed_bone_reference_is_distinct_from_unknown_target()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("malformed-bone.glb");
    let (mut document, trailing) = read_glb(&asset_path("skinned_two_bone_mesh.glb"))?;
    document["skins"][0]["joints"][0] = json!("RootBone");
    write_glb(&path, &document, &trailing)?;

    // When
    let output = run_probe(&path)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(diagnostic_codes(&output)?, ["BONE_REFERENCE_MALFORMED"]);
    Ok(())
}

#[test]
fn broken_prefab_reference_fails_independently() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("broken-prefab.glb");
    fs::copy(asset_path("toon_sphere.glb"), &path)?;
    fs::write(
        directory.path().join("broken-prefab.cel.json"),
        serde_json::to_vec(&json!({
            "schema_version": 1,
            "nodes": {"ToonSphere": {"references": {"prefabs": ["missing.glb"]}}}
        }))?,
    )?;

    // When
    let output = run_probe(&path)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(diagnostic_codes(&output)?, ["PREFAB_TARGET_UNKNOWN"]);
    Ok(())
}

#[test]
fn broken_collider_reference_fails_independently() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("broken-collider.glb");
    fs::copy(asset_path("toon_sphere.glb"), &path)?;
    fs::write(
        directory.path().join("broken-collider.cel.json"),
        serde_json::to_vec(&json!({
            "schema_version": 1,
            "nodes": {"ToonSphere": {"references": {"colliders": ["MissingCollider"]}}}
        }))?,
    )?;

    // When
    let output = run_probe(&path)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(diagnostic_codes(&output)?, ["COLLIDER_TARGET_UNKNOWN"]);
    Ok(())
}

#[test]
fn unknown_reflected_component_type_fails_independently() -> Result<(), Box<dyn std::error::Error>>
{
    // Given
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("unknown-component.glb");
    fs::copy(asset_path("toon_sphere.glb"), &path)?;
    fs::write(
        directory.path().join("unknown-component.cel.json"),
        serde_json::to_vec(&json!({
            "schema_version": 1,
            "nodes": {"ToonSphere": {"components": {"example::Missing": {}}}}
        }))?,
    )?;

    // When
    let output = run_probe(&path)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(diagnostic_codes(&output)?, ["COMPONENT_TYPE_UNKNOWN"]);
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        envelope["diagnostics"][0]["asset_path"],
        "$.nodes.ToonSphere.components.example::Missing"
    );
    Ok(())
}

#[test]
fn kawaii_component_is_unknown_without_registration() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("kawaii-disabled.glb");
    fs::copy(asset_path("toon_sphere.glb"), &path)?;
    fs::write(
        directory.path().join("kawaii-disabled.cel.json"),
        serde_json::to_vec(&json!({
            "schema_version": 1,
            "nodes": {"ToonSphere": {"components": {
                "bevy_kawaii_physics::ecs::components::KawaiiPhysics": {}
            }}}
        }))?,
    )?;

    let output = run_probe(&path)?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(diagnostic_codes(&output)?, ["COMPONENT_TYPE_UNKNOWN"]);
    Ok(())
}

#[test]
fn mixed_metadata_sources_keep_each_diagnostic_file_path() -> Result<(), Box<dyn std::error::Error>>
{
    // Given
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("mixed-sources.glb");
    let sidecar_path = directory.path().join("mixed-sources.cel.json");
    let (mut document, trailing) = read_glb(&asset_path("toon_sphere.glb"))?;
    document["extensions"] = json!({
        "CEL_metadata": {
            "schema_version": 1,
            "nodes": {"ToonSphere": {"references": {"colliders": ["MissingCollider"]}}}
        }
    });
    write_glb(&path, &document, &trailing)?;
    fs::write(
        &sidecar_path,
        serde_json::to_vec(&json!({
            "schema_version": 1,
            "nodes": {"ToonSphere": {"references": {"prefabs": ["missing.glb"]}}}
        }))?,
    )?;

    // When
    let output = run_probe(&path)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        envelope["diagnostics"][0]["file_path"],
        sidecar_path.to_string_lossy().as_ref()
    );
    assert_eq!(
        envelope["diagnostics"][1]["file_path"],
        path.to_string_lossy().as_ref()
    );
    Ok(())
}
