# 运行 Showcase 套件（It-9a）

> **Status:** Working note（操作手册；规范边界在 docs/showcase/01）。

12 个固定用例（docs/showcase/01 关卡表 / release gate §Showcase）由
`crates/dev/showcase` 承载。

## 用法

```text
cargo run -p showcase -- <name>          # 开窗交互
cargo run -p showcase -- all --capture   # headless 全量捕获
cargo test -p showcase                   # 同 --capture all（GPU-gated）
```

用例：`overview` / `ramp` / `lighting` / `face` / `hair` / `metal` /
`outline` / `animation` / `stress` / `reference` / `performance` /
（另见 `all`）。

- 捕获输出：`fixtures/golden-local/showcase/<name>.png`（gitignored，
  local-only）；
- 全部几何/纹理为程序化生成（fixtures 纪律：showcase 资产不进入自动
  测试）；
- `stress` / `performance` 复用 `crates/dev/benchmarks` 的场景构建器
  （link, don't duplicate）；performance 的数值基线走
  `docs/performance/baselines/`（It-8）。

## 证据纪律

showcase 是 manual QA / debug 入口，不是 release gate；release 判定走
golden（`crates/bevy-stylized-render/tests/`）与 benchmark 基线。
