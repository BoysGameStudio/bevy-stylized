//! Golden baseline storage: PNG plus a provenance sidecar JSON.
//!
//! The provenance fields follow
//! `docs/reference/04-upstream-contract-upgrade-and-golden.md` §Golden
//! Provenance, extended with the engine commit, GPU/driver identity, and the
//! capture timestamp so every baseline stays traceable.

use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::capture::CapturedImage;

/// Provenance sidecar stored next to every golden PNG as `<case>.golden.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoldenProvenance {
    /// Reference toolchain identifier (see the reference toolchain lock).
    pub reference_toolchain: String,
    /// Exact validated Blender version, when the fixture is upstream-derived.
    pub blender: Option<String>,
    /// Gacha Setup version.
    pub gacha_setup: String,
    /// Gacha Setup commit or digest, when known.
    pub gacha_setup_commit: Option<String>,
    /// Export contract identifier.
    pub export_contract: String,
    /// Profile schema identifier.
    pub profile_schema: String,
    /// Fixture/case identifier.
    pub fixture: String,
    /// Renderer identifier that produced the capture.
    pub renderer: String,
    /// Git commit of this repository at capture time.
    pub engine_git_commit: String,
    /// Content identity for a Git-free source distribution, when provided.
    #[serde(default)]
    pub engine_source_revision: Option<String>,
    /// GPU adapter name and backend.
    pub gpu: String,
    /// GPU driver name and version string.
    pub driver: String,
    /// Capture time as Unix epoch seconds.
    pub captured_unix_seconds: u64,
    /// Capture width in pixels.
    pub width: u32,
    /// Capture height in pixels.
    pub height: u32,
    /// FNV-1a-64 digest of the PNG file bytes, lowercase hex.
    pub png_fnv1a64: String,
}

impl GoldenProvenance {
    /// Fills engine identity, GPU/driver, timestamp, and geometry for a capture.
    #[must_use]
    pub fn for_capture(
        reference_toolchain: &str,
        blender: Option<String>,
        gacha_setup: &str,
        gacha_setup_commit: Option<String>,
        export_contract: &str,
        profile_schema: &str,
        fixture: &str,
        renderer: &str,
        gpu: &str,
        driver: &str,
        captured: &CapturedImage,
    ) -> Self {
        Self {
            reference_toolchain: reference_toolchain.to_owned(),
            blender,
            gacha_setup: gacha_setup.to_owned(),
            gacha_setup_commit,
            export_contract: export_contract.to_owned(),
            profile_schema: profile_schema.to_owned(),
            fixture: fixture.to_owned(),
            renderer: renderer.to_owned(),
            engine_git_commit: engine_git_commit(),
            engine_source_revision: std::env::var("STYLIZED_SOURCE_REVISION").ok().or_else(|| {
                let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()?
                    .parent()?;
                std::fs::read_to_string(root.join("SOURCE_REVISION"))
                    .ok()
                    .map(|s| s.trim().to_owned())
            }),
            gpu: gpu.to_owned(),
            driver: driver.to_owned(),
            captured_unix_seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs()),
            width: captured.width,
            height: captured.height,
            // Filled by `write_baseline` once the PNG bytes exist.
            png_fnv1a64: String::new(),
        }
    }
}

/// The PNG and sidecar paths belonging to one golden case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoldenPaths {
    /// Baseline PNG path.
    pub png: PathBuf,
    /// Provenance sidecar path (`<case>.golden.json`).
    pub sidecar: PathBuf,
}

impl GoldenPaths {
    /// Builds the paths for `case` inside `directory`.
    #[must_use]
    pub fn new(directory: &Path, case: &str) -> Self {
        Self {
            png: directory.join(format!("{case}.png")),
            sidecar: directory.join(format!("{case}.golden.json")),
        }
    }

    /// Whether a stored baseline exists.
    #[must_use]
    pub fn baseline_exists(&self) -> bool {
        self.png.is_file() && self.sidecar.is_file()
    }
}

/// Errors produced while reading or writing golden baselines.
#[derive(Debug, thiserror::Error)]
pub enum GoldenError {
    /// Filesystem failure.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// PNG encode/decode failure.
    #[error(transparent)]
    Image(#[from] image::ImageError),
    /// Image and provenance no longer describe the same accepted artifact.
    #[error("baseline image/provenance checksum mismatch")]
    ChecksumMismatch,
    /// Sidecar JSON failure.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Encodes a captured frame as PNG bytes.
pub fn encode_png(captured: &CapturedImage) -> Result<Vec<u8>, GoldenError> {
    let image: image::RgbaImage =
        image::ImageBuffer::from_raw(captured.width, captured.height, captured.data.clone())
            .ok_or_else(|| {
                GoldenError::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "capture byte count does not match its dimensions",
                ))
            })?;
    let mut bytes = io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png)?;
    Ok(bytes.into_inner())
}

/// Decodes PNG bytes into a captured frame.
pub fn decode_png(bytes: &[u8]) -> Result<CapturedImage, GoldenError> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.to_rgba8();
    let (width, height) = image.dimensions();
    CapturedImage::new(width, height, image.into_raw()).ok_or_else(|| {
        GoldenError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            "decoded PNG byte count does not match its dimensions",
        ))
    })
}

/// Writes the baseline PNG and provenance sidecar, creating the directory.
pub fn write_baseline(
    paths: &GoldenPaths,
    captured: &CapturedImage,
    provenance: &mut GoldenProvenance,
) -> Result<(), GoldenError> {
    let png = encode_png(captured)?;
    provenance.png_fnv1a64 = fnv1a64_hex(&png);
    if let Some(parent) = paths.png.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&paths.png, png)?;
    fs::write(&paths.sidecar, serde_json::to_string_pretty(provenance)?)?;
    Ok(())
}

/// Reads the baseline PNG and its provenance sidecar.
pub fn read_baseline(
    paths: &GoldenPaths,
) -> Result<(CapturedImage, GoldenProvenance), GoldenError> {
    let bytes = fs::read(&paths.png)?;
    let provenance: GoldenProvenance = serde_json::from_str(&fs::read_to_string(&paths.sidecar)?)?;
    if fnv1a64_hex(&bytes) != provenance.png_fnv1a64 {
        return Err(GoldenError::ChecksumMismatch);
    }
    let captured = decode_png(&bytes)?;
    if captured.width != provenance.width || captured.height != provenance.height {
        return Err(GoldenError::ChecksumMismatch);
    }
    Ok((captured, provenance))
}

/// FNV-1a-64 digest of `bytes` as lowercase hex.
#[must_use]
pub fn fnv1a64_hex(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}

/// Git commit of this repository, or `"unknown"` outside a working tree.
#[must_use]
pub fn engine_git_commit() -> String {
    std::process::Command::new("git")
        .args(["-C", env!("CARGO_MANIFEST_DIR"), "rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map_or_else(|| "unknown".to_owned(), |commit| commit.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_image() -> CapturedImage {
        let mut data = Vec::new();
        for y in 0..8_u8 {
            for x in 0..8_u8 {
                data.extend_from_slice(&[x * 32, y * 32, 128, 255]);
            }
        }
        CapturedImage::new(8, 8, data).expect("consistent size")
    }

    #[test]
    fn png_roundtrip_preserves_every_pixel() {
        // Given
        let captured = sample_image();

        // When
        let decoded = decode_png(&encode_png(&captured).expect("encode")).expect("decode");

        // Then
        assert_eq!(decoded, captured);
    }

    #[test]
    fn baseline_roundtrip_restores_pixels_and_provenance() {
        // Given
        let directory = tempfile::tempdir().expect("tempdir");
        let paths = GoldenPaths::new(directory.path(), "case_a");
        let captured = sample_image();
        let mut provenance = GoldenProvenance::for_capture(
            "engine-golden-v1",
            None,
            "test-fixtures",
            None,
            "blender-gltf-npr-v1",
            "gacha/v1",
            "case_a",
            "test-renderer",
            "test-gpu",
            "test-driver",
            &captured,
        );

        // When
        write_baseline(&paths, &captured, &mut provenance).expect("write");
        let (read_pixels, read_provenance) = read_baseline(&paths).expect("read");

        // Then
        assert_eq!(read_pixels, captured);
        assert_eq!(read_provenance.fixture, "case_a");
        assert_eq!(read_provenance.width, 8);
        assert_eq!(read_provenance.height, 8);
        assert_eq!(read_provenance.png_fnv1a64.len(), 16);
        assert!(!read_provenance.png_fnv1a64.is_empty());
        assert!(paths.baseline_exists());
        // A syntactically valid replacement image must not inherit old provenance.
        let mut changed = captured.clone();
        changed.data[0] ^= 255;
        fs::write(&paths.png, encode_png(&changed).unwrap()).unwrap();
        assert!(matches!(
            read_baseline(&paths),
            Err(GoldenError::ChecksumMismatch)
        ));
    }

    #[test]
    fn fnv_digest_is_stable_and_changes_with_content() {
        assert_eq!(fnv1a64_hex(b""), "cbf29ce484222325");
        assert_ne!(fnv1a64_hex(b"a"), fnv1a64_hex(b"b"));
    }

    #[test]
    fn missing_baseline_is_reported() {
        let directory = tempfile::tempdir().expect("tempdir");
        let paths = GoldenPaths::new(directory.path(), "absent");
        assert!(!paths.baseline_exists());
        assert!(read_baseline(&paths).is_err());
    }
}
