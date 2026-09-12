# Provisional 清单（It-10 冻结交接）

> **Status:** Working document — 维护者 ADR 决策的交接清单。
> **生成方式：** `grep -rniE "provisional" crates/` 全量扫描 + 逐条人工核对
> （2026-09-12）。每条：provisional 项 → 阻塞 ADR → 位置 → 现有证据。
> **纪律：** ADR 状态变更只能由维护者确认；本清单中的每一项在对应 ADR
> 关闭前不得对外宣称契约语义。

## 渲染后端与预算

| 项 | 阻塞 ADR | 位置 | 现有证据 |
|---|---|---|---|
| 全部材质建立在 Bevy `Material` 组合路径（薄层临时 backend） | ADR-0001 | crates/bevy-stylized-render/src/lib.rs（模块文档）、material.rs / character.rs / hero.rs 模块头 | A/B 对比测量（`target/tmp/adr-0001-spike/`）：A 首帧 602.9ms/稳态 4.47ms，B 首帧 1118.6ms/稳态 4.69ms（500 实例） |
| 预算阈值为 interim（safety margin、子预算上限） | ADR-0002 | crates/bevy-stylized-core/src/budget.rs（`INTERIM_SAFETY_MARGIN`）、crates/bevy-stylized-render/tests/interim_budget.rs | 全 class pipeline 反射实测 `target/tmp/interim-budget/pipeline_budget.json`（三模型全 fit） |
| Bindless 不实施 + 重访阈值 | ADR-0003 | `target/tmp/adr-0001-spike/summary.md`（方法论 + 阈值） | 500 实例 1 draw（spike 测量） |

## 数据通道与降级

| 项 | 阻塞 ADR | 位置 | 现有证据 |
|---|---|---|---|
| MeshTag 位段布局（instance 传输方案 A）；StorageTable 声明未实现 | ADR-0004 | crates/bevy-stylized-render/src/instance.rs、cel/character/hero WGSL 的位段注释 | 100/1000 实例 1 draw + 逐实例差异图（`tests/instance_batching.rs`、spike 报告） |
| Per-class fallback 链与 `*_portable_fallback` 的触发语义（tier_reduced 即应用全链） | 无专属 ADR（fallback 策略；ADR-0018 已 Accepted 不覆盖此项） | crates/bevy-stylized-core/src/capability.rs（`STYLIZED_FALLBACK_CHAINS`）、render/src/character.rs、hero.rs | 快照式 fallback 矩阵测试（3 class × 3 档 × 4 能力模型） |
| View 层映射常数（昼夜 ambient 亮度/色温、ILM AO floor、display-transform hook 曲线） | ADR-0009 | render/src/view.rs（`NIGHT_AMBIENT_*`）、render/src/post.rs、character WGSL AO floor | view 同步单测 + character golden |
| Outline 生命周期为最小副本同步（bounds/LOD/despawn 完整语义未定） | ADR-0008 | render/src/outline.rs（`StylizedOutline`/`sync_outline_replicas`）、outline WGSL | outline golden + Odette 发卡排障（顶点色宽度抑制） |
| Screen-space 描边（V2-a）：tier 门控 + overlay 实现 + 混合模式 | ADR-0008 | render/src/screen_outline.rs、outline/screen_outline.wgsl | outline-advanced 三捕获 golden + 边缘计数断言 + gating 单测 |
| Alpha/prepass/shadow 语义矩阵（Opaque/Mask 支持、Blend 类型层不可达、prepass/shadow 显式关闭） | ADR-0023 | render/tests/semantics_matrix.rs、docs/authoring/05 §7 矩阵表 | 矩阵数据驱动测试 + golden |
| NPR LOD（V2-b）：距离分带政策、per-instance flags 通道（MeshTag bit 0..=2）、几何换 mesh、`StylizedProfile::lod_policy` hook；`StylizedTierOverride` 移入 render（facade re-export） | ADR-0018 | core/src/lod.rs、render/src/lod.rs、hero WGSL per-instance flags | render 单测 6 个 + facade governor→LOD 端到端 + showcase `lod` 捕获（四带梯度目检） |

## Hero / Profile / 导入

| 项 | 阻塞 ADR | 位置 | 现有证据 |
|---|---|---|---|
| Hero 纹理 packing/语义（ILM 通道约定、matcap/spec ramp 形态） | ADR-0005/0006 | render/src/hero.rs、hero WGSL | 预算反射（7T/7S/1UBO）+ 三层 golden 报告 |
| profile-genshin schema 版本（reference-toolchain.toml 中 PENDING）；"待核实"条目（ramp 行序/material-id 映射） | ADR-0019 | crates/bevy-stylized-profile-genshin/src/lib.rs | profile 契约测试；行为契约 docs/reference/07（draft） |
| extras vs sidecar 冲突：双通道读取 + 冲突诊断、拒绝静默取舍 | ADR-0020 | crates/bevy-stylized-gltf/src/metadata.rs（`import_material`） | 冲突/一致/单通道三向单测 |
| `_NPR_*` 注册机制（builder + fail-closed 顺序守卫） | ADR-0028 | gltf/src/setup.rs、facade `stylized_gltf_plugin()` | 插件顺序集成测试（正确加载 + 错误顺序报错） |

## 平台与生命周期

| 项 | 阻塞 ADR | 位置 | 现有证据 |
|---|---|---|---|
| Tier 需求表数值（Standard/High 需求）与 baseline 限制模型 | ADR-0010 | core/src/capability.rs（`STYLIZED_TIER_REQUIREMENTS`）、budget.rs | 三模型对照表 + fallback 矩阵测试 |
| Web baseline（WebGL2 vs WebGPU 区分仅在数据层） | ADR-0021 | core/src/budget.rs（`LIMIT_MODEL_WEBGL2`）、SUPPORT_MATRIX.md | 无 web 实测；矩阵中全部 🚧 |
| Disabled-feature 零成本（组件缺失时系统无工作） | ADR-0026 | render/tests/disabled_feature_cost.rs | 三个 early-out 测试 |
| Bevy/工具链兼容声明（0.19.1 钉版、1.98.x、本机唯一实测平台） | ADR-0027 | docs/platforms/SUPPORT_MATRIX.md §Compatibility statement | 本机全量测试/捕获/benchmark 基线 |

## 统计

| 阻塞 ADR | 条数 |
|---|---|
| 0001 | 1（覆盖 4 个材质 + 4 个 WGSL） |
| 0002 | 1 |
| 0003 | 1 |
| 0004 | 1 |
| 0005/0006 | 1 |
| 0007 | tier2 阈值草案校准（docs/reference/08 已记实测注记） |
| 0008 | 1 |
| 0009 | 1 |
| 0010 | 1 |
| 0018 | 1（V2-b NPR LOD） |
| 0019 | 1 |
| 0020 | 1 |
| 0021 | 1 |
| 0023 | 1 |
| 0026 | 1 |
| 0027 | 1 |
| 0028 | 1 |
| （无专属 ADR：fallback 链触发语义） | 1 |
