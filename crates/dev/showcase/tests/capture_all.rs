//! Showcase 全量捕获测试（GPU-gated，无 GPU graceful skip）。
//!
//! 跑全部 17 个用例并断言：捕获非黑（平均亮度超过地板线）、PNG 已写盘
//! （`fixtures/golden-local/showcase/`，gitignored）。

use showcase::all_cases;

#[test]
fn all_showcase_cases_capture_non_black() {
    let Some(resources) = golden_harness::probe_render_resources() else {
        eprintln!("showcase: no GPU adapter available; skipping");
        return;
    };
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/golden-local/showcase");
    std::fs::create_dir_all(&out_dir).expect("create showcase output dir");
    for case in all_cases() {
        let image = showcase::capture_case(&case, resources.clone());
        let mean_luma = image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|px| f32::from(px[0]) + f32::from(px[1]) + f32::from(px[2]))
            .sum::<f32>()
            / (image.width * image.height) as f32
            / 3.0;
        assert!(
            mean_luma > 1.0,
            "showcase case {} captured black (mean luma {mean_luma})",
            case.name
        );
        let png = golden_harness::encode_png(&image).expect("encode png");
        let path = out_dir.join(format!("{}.png", case.name));
        std::fs::write(&path, png).expect("write png");
        eprintln!("[captured] {} (mean luma {mean_luma:.1})", case.name);
    }
}
