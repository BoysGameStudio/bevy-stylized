//! Independent reference repeatability and coverage metrics.
use golden_harness::CapturedImage;
use serde_json::{Value, json};

pub(super) fn repeat(a: &CapturedImage, b: &CapturedImage) -> Result<Value, String> {
    if a.width != b.width || a.height != b.height {
        return Err("capture dimensions mismatch".into());
    }
    let d = golden_harness::diff_rgba8(&a.data, &b.data).map_err(|e| e.to_string())?;
    Ok(
        json!({"passed":d.max_channel_delta<=2&&d.mean_absolute_delta<=0.5&&d.psnr_db>=48.,"stats":d}),
    )
}
pub(super) fn coverage_iou(a: &CapturedImage, b: &CapturedImage) -> Result<Value, String> {
    if a.width != b.width || a.height != b.height {
        return Err("coverage dimensions mismatch".into());
    }
    let mut intersection = 0;
    let mut union = 0;
    let mut count_a = 0;
    let mut count_b = 0;
    // Geometry rendered in white over linear 0.12 background: fixed midpoint
    // threshold in linear coverage (0.56 linear -> about 197 sRGB).
    for (a, b) in a
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.data.as_chunks::<4>().0)
    {
        let x = a[0] > 197;
        let y = b[0] > 197;
        count_a += usize::from(x);
        count_b += usize::from(y);
        intersection += usize::from(x && y);
        union += usize::from(x || y);
    }
    if count_a < 100 || count_b < 100 {
        return Err("empty/insufficient geometry mask".into());
    }
    let iou = intersection as f64 / union as f64;
    Ok(
        json!({"algorithm":"white-geometry-coverage/v1","iou":iou,"foreground_blender":count_a,"foreground_bevy":count_b,"threshold_srgb":197,"passed":iou>=0.99,"scope":"opaque static geometry; does not qualify alpha masks or cast shadows"}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_geometry_masks_cannot_pass() {
        let a = CapturedImage::new(16, 16, vec![0; 1024]).unwrap();
        assert!(coverage_iou(&a, &a).is_err());
    }
}
