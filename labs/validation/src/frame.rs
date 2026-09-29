//! 整图宽松判定（docs/showcase/05 §9 方案 A 的整图层）：non-black +
//! 全图 luma/chroma 统计包络——抓全黑、整体色差与大面积缺失，不承担
//! 逐像素判定（精确判定归 ROI 分区）。

use serde::{Deserialize, Serialize};

use crate::image::ImageView;

/// 全图统计（luma = Rec.601；chroma = 每像素 max(r,g,b)−min(r,g,b)）。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameStats {
    /// 平均亮度。
    pub mean_luma: f64,
    /// 亮度标准差（大面积缺失/整图被压平的探针）。
    pub stddev_luma: f64,
    /// 平均色度（整体色偏/褪色的探针）。
    pub mean_chroma: f64,
}

/// 计算全图统计。
#[must_use]
pub fn frame_stats(view: ImageView) -> FrameStats {
    let pixels = view.rgba8.as_chunks::<4>().0;
    let count = pixels.len().max(1) as f64;
    let mut luma_sum = 0_f64;
    let mut luma_sq_sum = 0_f64;
    let mut chroma_sum = 0_f64;
    for pixel in pixels {
        let (r, g, b) = (
            f64::from(pixel[0]),
            f64::from(pixel[1]),
            f64::from(pixel[2]),
        );
        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
        luma_sum += luma;
        luma_sq_sum += luma * luma;
        chroma_sum += r.max(g).max(b) - r.min(g).min(b);
    }
    let mean = luma_sum / count;
    FrameStats {
        mean_luma: mean,
        stddev_luma: (luma_sq_sum / count - mean * mean).max(0.0).sqrt(),
        mean_chroma: chroma_sum / count,
    }
}

/// 整图宽松包络（区间在首次基线验收时锁定）。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameEnvelope {
    /// 平均亮度下限（non-black 门槛）。
    pub min_mean_luma: f64,
    /// 平均亮度上限。
    pub max_mean_luma: f64,
    /// 亮度标准差下限（大面积缺失探针）。
    pub min_stddev_luma: f64,
    /// 平均色度下限。
    pub min_mean_chroma: f64,
    /// 平均色度上限（整体色偏探针）。
    pub max_mean_chroma: f64,
}

/// 整图宽松判定：逐项检查并报告全部违规（不短路）。
///
/// # Errors
///
/// 返回人可读的违规项。
pub fn evaluate_frame_envelope(
    stats: FrameStats,
    envelope: FrameEnvelope,
) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();
    if stats.mean_luma < envelope.min_mean_luma {
        violations.push(format!(
            "mean luma {:.3} below non-black floor {:.3}",
            stats.mean_luma, envelope.min_mean_luma
        ));
    }
    if stats.mean_luma > envelope.max_mean_luma {
        violations.push(format!(
            "mean luma {:.3} above envelope ceiling {:.3}",
            stats.mean_luma, envelope.max_mean_luma
        ));
    }
    if stats.stddev_luma < envelope.min_stddev_luma {
        violations.push(format!(
            "luma stddev {:.3} below {:.3} (frame flattened)",
            stats.stddev_luma, envelope.min_stddev_luma
        ));
    }
    if stats.mean_chroma < envelope.min_mean_chroma {
        violations.push(format!(
            "mean chroma {:.3} below {:.3} (colors washed out)",
            stats.mean_chroma, envelope.min_mean_chroma
        ));
    }
    if stats.mean_chroma > envelope.max_mean_chroma {
        violations.push(format!(
            "mean chroma {:.3} above {:.3} (global color shift)",
            stats.mean_chroma, envelope.max_mean_chroma
        ));
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_stats_of_uniform_and_split_images() {
        // 均匀中灰：stddev=0、chroma=0。
        let flat = vec![128_u8; 4 * 4 * 4];
        let view = ImageView::new(4, 4, &flat).expect("view");
        let stats = frame_stats(view);
        assert!((stats.mean_luma - 128.0).abs() < 1e-9);
        assert!(stats.stddev_luma < 1e-4);
        assert_eq!(stats.mean_chroma, 0.0);
        // 半黑半白：mean=127.5，stddev=127.5。
        let mut split = Vec::new();
        for i in 0..16 {
            let v = if i < 8 { 0 } else { 255 };
            split.extend_from_slice(&[v, v, v, 255]);
        }
        let view = ImageView::new(4, 4, &split).expect("view");
        let stats = frame_stats(view);
        assert!((stats.mean_luma - 127.5).abs() < 1e-9);
        assert!((stats.stddev_luma - 127.5).abs() < 1e-9);
        // 纯色（红）：chroma=255。
        let mut red = Vec::new();
        for _ in 0..4 {
            red.extend_from_slice(&[255, 0, 0, 255]);
        }
        let view = ImageView::new(2, 2, &red).expect("view");
        assert_eq!(frame_stats(view).mean_chroma, 255.0);
    }

    #[test]
    fn envelope_catches_black_flat_and_shifted_frames() {
        let envelope = FrameEnvelope {
            min_mean_luma: 10.0,
            max_mean_luma: 240.0,
            min_stddev_luma: 5.0,
            min_mean_chroma: 0.0,
            max_mean_chroma: 200.0,
        };
        // 全黑：低于 non-black 门槛且被压平。
        let black = vec![0_u8; 16];
        let stats = frame_stats(ImageView::new(2, 2, &black).expect("view"));
        let violations = evaluate_frame_envelope(stats, envelope).expect_err("black");
        assert_eq!(violations.len(), 2);
        // 正常画面通过。
        let stats = FrameStats {
            mean_luma: 120.0,
            stddev_luma: 40.0,
            mean_chroma: 30.0,
        };
        assert_eq!(evaluate_frame_envelope(stats, envelope), Ok(()));
        // 整体色偏（超色度上限）。
        let shifted = FrameStats {
            mean_chroma: 220.0,
            ..stats
        };
        let violations = evaluate_frame_envelope(shifted, envelope).expect_err("shifted");
        assert_eq!(violations.len(), 1);
    }
}
