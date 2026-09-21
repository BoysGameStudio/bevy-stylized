# 运行 Showcase 套件

> **Status:** 当前实现操作手册。跨平台统一方案见[场景设计](01-showcase-examples-fixtures.md)，
> 其中 Proposed 部分尚未实现，不能当作当前命令或平台覆盖。

`apps/showcase` 当前注册 17 个固定用例。Android 另有程序化与静态角色两种
smoke 模式，尚未共享完整目录；构建步骤见 [Contributing](../../CONTRIBUTING.md#android-vulkan-smoke)。

## 用法

```text
CARGO_INCREMENTAL=0 cargo run -p showcase --features interactive -- <name> # 开窗交互
CARGO_INCREMENTAL=0 cargo run -p showcase -- all --capture # headless 全量捕获
CARGO_INCREMENTAL=0 cargo test -p showcase -- --ignored # 全场景非黑捕获检查（GPU-gated）
```

用例：`overview` / `ramp` / `lighting` / `face` / `hair` / `metal` /
`outline` / `animation` / `post` / `stress` / `reference` / `performance` /
`outline-advanced` / `lod` / `environment-day` / `environment-dusk` /
`environment-night`；`all` 选择全部固定用例。

- 捕获输出：`target/stylized/showcase/<name>.png`（gitignored，
  local-only）；
- 多数场景使用程序化几何/纹理；`reference` 加载第一方
  `fixtures/synthetic/reference-scene/reference_scene.glb`；
- `stress` / `performance` 复用 `labs/benchmarks` 的场景构建器
  （link, don't duplicate）。`stress` 为 1000 实例；`performance` 展示组合为
  Hero + 100 实例，数值采集与验收见[性能合同](../performance/targets.md)。

## 证据纪律

showcase 是 manual QA / debug 入口，不是 release gate；release 判定走
golden（`tests/integration/tests/`）与 benchmark 基线。

静态角色交互检查使用独立的 `inspect` 子命令，参见[角色检查器](03-character-inspector.md)。
它不计入固定用例列表，也不替代 Blender 参考验收。
