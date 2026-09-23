# 运行 Showcase 套件

> **Status:** 当前实现操作手册。跨平台统一方案见[场景设计](01-showcase-examples-fixtures.md)，
> 其中 Proposed 部分尚未实现，不能当作当前命令或平台覆盖。

`apps/showcase` 当前注册 18 个固定用例，内容与相机配方由 `labs/scenes` 提供，
经 `labs/scene-runtime` 的 CaseDriver 驱动。Android 另有程序化与静态角色两种
smoke 模式，尚未共享完整目录；构建步骤见 [Contributing](../../CONTRIBUTING.md#android-vulkan-smoke)。
Android 宿主经 `driver_host` 逐帧推进同一 driver 并在设备端重放共享目录配方，
logcat 验收标记为 `ANDROID_REPLAY compiled/captured/refused/report_ready`。

## 用法

```text
CARGO_INCREMENTAL=0 cargo run -p showcase --features interactive -- <name> # 开窗交互
CARGO_INCREMENTAL=0 cargo run -p showcase -- all --capture # headless 全量捕获
CARGO_INCREMENTAL=0 cargo run -p showcase -- workloads --out <new-dir> # headless 九个固定 workload
CARGO_INCREMENTAL=0 cargo test -p showcase -- --ignored # 全场景非黑捕获检查（GPU-gated）
```

交互模式在同一窗口会话内用 ←/→ 切换用例；tier 需求超过当前已解析档时先经
产品重配置 API（ADR-0030）尝试会话内升档，设备级缺口才结构化拒绝并要求新
device 会话；资产根变更仍以结构化信息拒绝（R2）。`workloads` 子命令 headless 运行，使用默认 feature，不依赖
egui/Inspector；数值合同见[性能合同](../performance/targets.md)。静态角色
交互检查使用 `inspect` 子命令（`--features inspector`），见下文证据纪律。

用例：`overview` / `ramp` / `lighting` / `face` / `hair` / `metal` /
`outline` / `animation` / `post` / `stress` / `reference` / `performance` /
`outline-advanced` / `lod` / `environment-day` / `environment-dusk` /
`environment-night` / `effects`；`all` 选择全部固定用例。

- 捕获输出：`target/stylized/showcase/<name>.png`（gitignored，
  local-only）；
- 多数场景使用程序化几何/纹理；`reference` 加载第一方
  `fixtures/synthetic/reference-scene/reference_scene.glb`；
- `effects` 展示效果网格光照基（per-material key light 覆盖，
  [导出合同](../reference/03-blender-export-contract.md)
  效果网格光照基节）：同 view 光下无覆盖/覆盖两球对比；
- `stress` / `performance` 使用 `labs/scenes` 的共享场景构建器
  （link, don't duplicate；benchmarks 测量同一构建器）。`stress` 为 1000 实例；
  `performance` 展示组合为 Hero + 100 实例，数值采集与验收见[性能合同](../performance/targets.md)。

## 证据纪律

showcase 是 manual QA / debug 入口，不是 release gate；release 判定走
golden（`tests/integration/tests/`）与 benchmark 基线。

静态角色交互检查使用独立的 `inspect` 子命令，参见[角色检查器](03-character-inspector.md)。
它不计入固定用例列表，也不替代 Blender 参考验收。
