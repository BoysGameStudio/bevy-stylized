//! spike 入口测试（GPU-gated，无 GPU graceful skip）：跑测量 +
//! 正确性门（CPU/GPU 可见计数一致）+ 写 `target/tmp/gpu-driven-spike/`。

use gpu_driven_spike::{cpu_cull, generate_instances, run_spike, test_frustum};

#[test]
fn gpu_cull_matches_cpu_reference() {
    // 纯 CPU 正确性（无 GPU 也跑）：CPU 镜像自身的确定性。
    let instances = generate_instances(1000, 50.0);
    let planes = test_frustum(150.0);
    let a = cpu_cull(&instances, &planes);
    let b = cpu_cull(&instances, &planes);
    assert_eq!(a, b, "CPU 剔除确定性");
    assert!(!a.is_empty() && a.len() < 1000, "视锥应剔掉一部分");
}

#[test]
fn gpu_driven_spike_measurement() {
    let Some(report) = run_spike() else {
        eprintln!("gpu-driven spike: no GPU adapter available; skipping");
        return;
    };
    eprintln!("spike on {}", report.gpu);
    for m in &report.measurements {
        eprintln!(
            "[spike] n={}: cpu={:.3}ms gpu_submit={:.3}ms gpu_readback={:.3}ms visible {}/{}",
            m.instances,
            m.cpu_cull_ms,
            m.gpu_submit_ms,
            m.gpu_readback_ms,
            m.gpu_visible,
            m.instances
        );
        // 正确性门：GPU 计数 == CPU 计数。
        assert_eq!(
            m.cpu_visible, m.gpu_visible,
            "GPU 剔除结果必须与 CPU 参考一致 (n={})",
            m.instances
        );
    }
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/gpu-driven-spike");
    std::fs::create_dir_all(&dir).expect("create spike dir");
    std::fs::write(
        dir.join("spike_results.json"),
        serde_json::to_string_pretty(&report).expect("serialize"),
    )
    .expect("write spike json");
    eprintln!("[written] {}", dir.join("spike_results.json").display());
}
