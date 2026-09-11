# CI 说明（Rust）

两层 CI 政策（It-8 落地）：

## CPU 层（GitHub-hosted，`.github/workflows/rust.yml`）

`fmt` / `clippy -D warnings` / `cargo test --workspace` / feature 矩阵。
所有 GPU 测试在 CI runner 上自动 graceful skip（无 adapter），保持绿色。

## GPU 层（self-hosted，`.github/workflows/gpu.yml`，默认不启用）

- 需要维护者在 baseline 机器（本机 RTX 5090）注册 self-hosted runner，
  并移除 gpu.yml 中的 `if: false` 条件——runner 注册是平台操作，agent
  无法代办；
- 触发范围限制在渲染相关路径（`crates/bevy-stylized*/`、`crates/dev/`、
  `fixtures/reference/`）；
- **过渡规则**：GPU 层失败只阻塞渲染相关 PR，不阻塞非渲染 PR；
- GPU 层的 golden/benchmark 证据只覆盖本机 NVIDIA baseline（V1 范围，
  其余平台一律 Experimental，见 `docs/platforms/SUPPORT_MATRIX.md`）。

## Benchmark baseline

固定 4 场景（release gate：1 Hero / 100 / 500 / 1000 Portable），
`BENCH_WRITE_BASELINE=1 cargo test -p benchmarks` 写入
`docs/performance/baselines/`（纯测量数据，可入库）。性能声明遵循
ADR-0029（前后测量 + 同设备 + 同质量契约）。
