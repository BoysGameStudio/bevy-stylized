# 运行 Showcase 套件

> **Status:** 当前实现操作手册。跨平台统一方案见[场景设计](01-showcase-examples-fixtures.md)，
> 其中 Proposed 部分尚未实现，不能当作当前命令或平台覆盖。
> 2026-09-28（docs/showcase/05 §12 阶段 4）：18 个旧展示场景已退役，
> `apps/showcase` 注册唯一的庭院展台（courtyard），golden 判定按
> [庭院设计](05-courtyard-scene-design.md) §9 的 10 条捕获规格组织。

`apps/showcase` 当前注册唯一场景 courtyard（庭院展台），内容与相机配方由
`labs/scenes` 提供，经 `labs/scene-runtime` 的 CaseDriver 驱动。Android 另有
程序化与静态角色两种 smoke 模式，尚未共享完整目录；构建步骤见
[Contributing](../../CONTRIBUTING.md#android-vulkan-smoke)。
Android 宿主经 `driver_host` 逐帧推进同一 driver 并在设备端重放共享目录配方，
logcat 验收标记为 `ANDROID_REPLAY compiled/captured/refused/report_ready`。

## 用法

```text
CARGO_INCREMENTAL=0 cargo run -p showcase --features interactive -- courtyard # 开窗交互（参数面/机位键位表）
CARGO_INCREMENTAL=0 cargo run -p showcase -- courtyard --capture # headless 捕获
CARGO_INCREMENTAL=0 cargo run -p showcase -- courtyard --capture-spec <id|all> # 按捕获规格捕获+判定
CARGO_INCREMENTAL=0 cargo run -p showcase -- workloads --out <new-dir> # headless 九个固定 workload
```

交互模式：←/→ 循环机位 preset（panorama → center-trio → outline →
outline-west → lod-side → flags），↑/↓ 切时刻，O/P 切描边/后处理，1/2/3
切灯阵密度，Space 切旗帜姿态，R 复位机位；`,`/`.` 切用例（单场景下回绕）。
tier 需求（outline_mode=hybrid 需 Standard）超过当前已解析档时先经产品重配置
API（ADR-0030）尝试会话内升档，设备级缺口才结构化拒绝并要求新 device 会话；
资产根变更仍以结构化信息拒绝（R2）。`workloads` 子命令 headless 运行，使用默认
feature，不依赖 egui/Inspector；数值合同见[性能合同](../performance/targets.md)。
静态角色交互检查使用 `inspect` 子命令（`--features inspector`），见下文证据纪律。

捕获规格（10 条，`--capture-spec` 与 golden lane 共用身份）：`courtyard-day` /
`courtyard-dusk` / `courtyard-night` / `courtyard-trio` / `courtyard-outline-hull` /
`courtyard-outline-hybrid`（Standard tier）/ `courtyard-lod` / `courtyard-post` /
`courtyard-flags` / `courtyard-stress`。

- 捕获输出：`target/stylized/showcase/captures/<capture_id>.png`（gitignored，
  local-only）；
- 角色台加载第一方 `fixtures/synthetic/reference-scene/reference_scene.glb`
  （资产门控：缺席 = BLOCKED，不做空台降级）；
- golden 基线：10 条捕获的 canonical 基线由参考宿主（Linux/Vulkan）产出；
  其他平台经 overlay 通道候选（`fixtures/golden/showcase/<platform>/`）。
  判定 = 整图宽松（non-black + luma/chroma 包络）或整图精确 + ROI 分区精确
  （AABB 投影自动计算，双层门限）；验收记录
  `fixtures/golden/showcase.courtyard.acceptance.md`。

## 证据纪律

showcase 是 manual QA / debug 入口，不是 release gate；release 判定走
golden（`tests/integration/tests/showcase_golden.rs`，10 个
`showcase_golden_courtyard_*` 用例）与 benchmark 基线。

静态角色交互检查使用独立的 `inspect` 子命令，参见[角色检查器](03-character-inspector.md)。
它不计入固定用例列表，也不替代 Blender 参考验收。
