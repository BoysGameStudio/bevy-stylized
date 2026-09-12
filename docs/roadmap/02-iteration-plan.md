# 迭代计划（Iteration Plan）

> **Status:** Working plan — 执行规划文件，非规范性架构契约。
> **Authority:** 规范性来源仍为 `docs/roadmap/01-v1-v2-v3-roadmap.md`、`docs/migration/01-tenacity-migration-plan.md`、`docs/release/02-v1-release-gate.md` 与各 ADR。本文件只把上述规范落成可执行、可验证的迭代序列；如与规范冲突，以规范为准并修订本文件。
> **Revision:** v3（2026-09-11）— 经两轮对抗性评审修订：v2 闭环首轮全部 critical/high；v3 闭环复审残留（checklist 行映射对齐真实结构、It-3 拆分、跨层 Golden 判据、Golden 基线存储归属、Phase 0 步骤 6 归属）。

## 0. 基线快照（2026-09-11）

代码与仓库状态：

- workspace 仅含 `crates/legacy/bevy-cel` 与 `crates/legacy/bevy-cel-metadata`（Tenacity commit `b5b26ac` 冻结导入的 Reference Oracle），`cargo check --workspace` 通过（Bevy 0.19.1）。
- `bevy-stylized` facade 及 `-core` / `-render` / `-gltf` / `-profile-genshin` / `-tools` 六个目标 crate 均不存在。
- `python3 tools/check_docs.py` PASS（86 篇文档、29 个 ADR）。
- CI 仅有 `.github/workflows/docs.yml`（文档治理校验）；无 Rust 构建/测试 CI。
- `docs/release/V1_RELEASE_CHECKLIST.md` 几乎全部未勾选；9 个 ADR 处于 V1 Blocker 状态。
- 迁移进度：Phase 0 的 10 步均未关闭；其前置工作（Legacy Reference Path 导入并冻结为 Reference Oracle）已完成。注意 legacy 导入本身不在 Phase 0 的 10 步之内。

执行环境：

- 本机 Blender 5.2.1 LTS（snap）；NVIDIA RTX 5090（可做本地 GPU 渲染/Golden 捕获）。
- Blender 正式 baseline 仍为 "Pending Reference Validation"，canonical 渲染环境为 "Pending baseline qualification"（见 `docs/governance/01-invariants-risks-and-adrs.md`）。
- 对抗性评审文档未记录 reviewed commit，其结论须在导入快照 `b5b26ac` 上逐条核验后方可当作当前代码事实。

## 1. 计划原则

1. **不静默决策**：标记 V1 Blocker / Proposed / Spike 的事项不得通过实现静默变成 runtime contract。每个迭代标明依赖的 ADR；未关闭的决策以"决策队列"（§4）显式推进。
2. **Provisional 规则**：ADR 状态变更须经维护者确认，且维护者确认无 SLA。为避免关键路径挂在人员响应上：凡依赖未确认决策的实现必须标记为 provisional（代码中以 `provisional` 注明、TRACEABILITY 登记、不计入稳定公共 API），并在 0.9 冻结前全部转为已确认。Provisional 实现不得对外宣称契约语义。**例外**：ADR-0017 可发布性子决策（GPL 资产边界）不可 provisional——GPL 污染不可撤回，该子决策未确认前涉外资产不入库（It-1b 以纯自制/CC0 fixture 推进）。
3. **双轨迁移**：legacy 代码是冻结的 Reference Oracle；新 production 代码只能参考、不得依赖 `crates/legacy/`。测试侧规则：Golden 对照以**离线图像/数据对比**方式进行（production 测试读取预先捕获的 Reference 基线文件），production crate 及其测试均不得以任何形式依赖 legacy crate（含 dev-dependency）。两侧共用的图像对比 harness 放在独立的 dev-only crate（见 It-2），两侧仅以 dev-dependency 使用之，不构成对 legacy 的依赖。
4. **文档随代码走**：任何改变公共渲染契约的 PR 必须同步更新子系统规范 / ADR / TRACEABILITY / release gate 之一（README §Change rule）。
5. **证据优先**：性能结论遵守 ADR-0029（前后测量 + 视觉 delta）；平台 `Supported` 声明遵守 `docs/platforms/01-capability-and-support-policy.md` 的证据要求；无证据的平台一律标 `Experimental`。
6. **小步可验证**：每个迭代有明确退出条件（Exit Criteria），退出条件只引用本迭代结束时**物理上可勾选**的 checklist 行（映射见 §7，段/行名与 `docs/release/V1_RELEASE_CHECKLIST.md` 实际结构逐一对齐），全部通过后才进入下一迭代。

## 2. 迭代总览

| 迭代 | 对应 Release | 对应迁移 Phase | 主题 | 关键 ADR |
|---|---|---|---|---|
| It-0 | —（前置） | — | 工程基座：Rust CI、工具链固定、临时版本政策、checklist 对齐 | 0027（临时条款） |
| It-1a | 0.1（前半） | Phase 0 步骤 2–5 | Reference 工具链锁定 + 导出契约 + 导出桥 | 0011 / 0012 / 0015，0017（可发布性子决策） |
| It-1b | 0.1（后半） | Phase 0 步骤 1、7–10 | Upstream Golden 基线 + 升级/解耦政策 | 0007 / 0013 / 0014 |
| It-2 | 0.1（续） | Phase 0 步骤 6 + Phase 1 | Legacy Reference 正确性修复 + Engine Golden 设施 | —（修复 Oracle，非新契约） |
| It-3a | 0.2（前半） | Phase 2 | core + facade + gltf 骨架 | 0028、0022（最小落地） |
| It-3b | 0.2（后半） | Phase 3 | Cel@Portable 渲染切片 | 0004（spike）、0008 / 0009（草案） |
| It-4 | 0.3 Instance/View + Character | Phase 4–5 | 三层数据分离 + Character | 0004（关闭）、0020，0023（增量） |
| It-5 | 0.4 Hero + Genshin Reference Family | Phase 6 | Hero + profile-genshin + 生产后处理 + 三层 Golden | 0005 / 0006，0023（增量） |
| It-6 | 0.5 Resource Architecture | Phase 7 | 预算契约化 + backend 三方对比定型 | 0001 / 0002 / 0003 |
| It-7 | 0.6 Capability/Fallback | 跨 Phase（capability 政策线） | Capability Resolver + 确定性 fallback | 0010 / 0021 / 0026 |
| It-8 | 0.7 Pipeline/Golden/Budget CI | 跨 Phase（测试设施 CI 化） | GPU runner + benchmark 基线 | 0029（证据流程） |
| It-9a | 0.8（前半） | Phase 8 | 12 项 Showcase | — |
| It-9b | 0.8（后半） | Phase 8 | CLI + 集成/迁移文档 + 自定义 Profile 示例 | — |
| It-10 | 0.9 API/Schema Freeze → 1.0 | Phase 9 | 冻结与 V1 release gate 验收 | 0017（最终）、0023（收尾）、0027（正式） |

## 3. 迭代明细

### It-0 工程基座

- 目标：让后续每个迭代都有自动化质量门禁与可复现工具链。
- 交付物：
  1. Rust CI workflow：fmt、clippy（继承 workspace lints）、`cargo test --workspace`、基础 feature 矩阵构建（ADR-0024 的首个证据，后续迭代随 feature 增加扩展）、docs 校验入口；
  2. `rust-toolchain.toml` 固定 Rust 工具链版本；
  3. 仓库级 `CHANGELOG.md` 初始化（crate 架构要求的必备文件）；
  4. ADR-0027 临时条款（写入 ADR-0027 草案）：**V1 开发期锁定 Bevy 0.19.x**，升级须走 ADR-0027 正式流程，避免开发期跨多个 Bevy 发布窗口无章可循；
  5. **Checklist 对齐**：对照 §7 核对 `docs/release/V1_RELEASE_CHECKLIST.md`，增补缺失行（toolchain lock、export contract、upgrade policy、Engine Golden 设施等），走文档治理流程，使 §7 映射的每一行都真实存在。
- 验证：CI 在 push 触发全绿。
- 退出条件：CI workflow 合入且全绿；checklist 增补合入且 `tools/check_docs.py` PASS。注：GitHub 分支保护（branch protection）属平台设置，列为维护者配合事项，不作为本迭代退出的阻塞条件。

### It-1a — Reference 锁定与导出桥（Phase 0 步骤 2–5）

- 目标：把 upstream 工具链与导出行为从"快照未锁"变成"带 digest 的可复现契约"。
- 交付物（串行依赖按序执行）：
  1. **上游资格评估（第一步）**：核实 Gacha Setup v3.6.2 的可得性（release/tag/commit + asset digest）与其对 Blender 版本的要求；与本机 Blender 5.2.1 比对，不满足则安装被锁版本（与 snap 并存）；上游不可得时的降级方案（本地存档 + 存档合规性确认）写入评估记录；
  2. Reference Toolchain Lock：锁定 Blender 精确版本、Gacha Setup release/tag/commit、release asset digest；
  3. Blender Export Contract 定稿（glTF exporter 设置、`_NPR_*` 顶点属性与 extras 的导出形态）；
  4. Gacha Setup Upstream Behavior Contract（哪些上游行为是 Oracle 语义）；
  5. Blender Reference Export Bridge：Blender 内 Python 导出脚本 + 批处理 runner，输出 Node-Graph Signature 与 Export Manifest，fail closed；
  6. **ADR-0017 可发布性子决策（前置，阻塞涉外资产入库）**：Golden/fixture 的可发布性边界（公开仓库 fixture 自制或 CC0）与"行为复刻 vs 代码复制"边界两项先行决策——Genshin lineage 为 GPL-3.0，该边界决定 It-1b 起所有入库资产的形态。记录形态：作为 ADR-0017 的部分 Decision 写入该 ADR 正文，ADR-0017 整体状态保持 V1 Blocker 至 It-10 最终关闭；子决策未获维护者确认前，It-1b 以纯自制/CC0 fixture 推进、涉 upstream 派生资产暂缓入库；
  7. ADR-0011 / 0012 / 0015 的 Decision/Evidence/Consequences 草案，提请维护者确认。
- 验证：`tools/check_docs.py` PASS；toolchain lock 覆盖 source register §Required provenance records 中工具链相关条目；export bridge 对样例 blend 输出 signature + manifest 且缺依赖时 fail closed。
- 退出条件：checklist Authoring 段 "Blender export bridge is scriptable and fail-closed" 行及 Reference 段 "Reference artifact provenance complete" 行的工具链/导出部分可勾选（§7）。

### It-1b — Upstream Golden 基线（Phase 0 步骤 1、7–10）

- 目标：建立带 provenance 的 **Blender 侧** Upstream Golden（不受 Rust 侧 bug 影响）与 fixture 资产管线。
- 交付物：
  1. 固定 Reference Scene / Camera / Lighting / Resolution；
  2. 自制/CC0 测试 fixture 网格（带 `_NPR_*` 属性，经 export bridge 验证）——公开仓库资产合规性遵守 It-1a 的可发布性子决策；
  3. 首批 Upstream Golden：本机 Blender 捕获 + 复现脚本 + provenance 元数据；容差策略与元数据格式即 ADR-0007 草案核心内容，二者同迭代定稿；**ADR-0007 草案必须显式定义跨层对比判据**（Blender 输出 vs Engine 输出的色调映射/AA/色彩管理差异如何归一化、何为"validated"），供 It-2 的 "Engine Reference validated against upstream Golden" 使用；
  4. 复现验证定义："干净环境"具体化为**同一台机器上新装被锁 Blender 版本的独立进程重跑**（容器化/第二台机器留作 It-8 CI 议题）；
  5. Reference Upgrade Policy（ADR-0013 草案）与 Reference/Production 解耦边界（ADR-0014 草案）；
  6. ADR-0007 / 0013 / 0014 草案提请维护者确认。
- 验证：provenance 记录完整覆盖 source register 清单；复现脚本按上述定义重跑，Golden 在 ADR-0007 容差内一致。
- 退出条件：checklist Reference 段 "Blender upstream Golden produced before runtime export" 与 "Reference artifact provenance complete" 行可勾选；0.1 Reference Baseline 完成。

### It-2 — Legacy Reference 正确性 + Engine Golden 设施（Phase 0 步骤 6 + Phase 1）

- 目标：核验并修复 Reference Oracle 的已知正确性缺陷，同时建立 Engine 侧 Golden 捕获设施（后续所有 Golden 对照的机器）。
- 交付物（按序）：
  1. **评审发现核验（前置）**：对照 `docs/reviews/01-current-implementation-adversarial-review.md` 在导入快照 `b5b26ac` 上逐条复现 6 项发现，核验结果记录到迁移状态记录；已不存在/已变质的发现相应调整修复项；
  2. InternalEdge ABI 修复（独立 vertex shader 或独立 uniform 布局）+ 回归测试；
  3. Character cull mode 默认 Inherit；
  4. 真实 Pipeline 创建测试（非源码字符串断言）；
  5. WGSL/Naga 校验纳入测试；
  6. Outline 生命周期契约（replica 的 immutable/mutable 语义）；
  7. Camera/Post 组件所有权统一；
  8. **Engine 侧 Golden 设施**：headless Bevy 渲染 → 帧捕获 → 图像 diff → 容差判定 → 基线存储的完整 harness。形态：独立 dev-only crate `crates/dev/golden-harness`（`publish = false`，不依赖任何 legacy 或 production crate），legacy 与 production 测试均以 dev-dependency 使用；Golden 基线文件存仓库内 `fixtures/golden/`（PNG + provenance JSON，复用 It-1b 元数据格式），体积失控时转 Git LFS 的决策点写明；
  9. 用该 harness 捕获修复后的 Engine Reference Golden，并按 ADR-0007 跨层判据与 Upstream Golden 对比；
  10. **修复后重新冻结**：更新 legacy 冻结状态与 provenance 记录（迁移状态记录追加条目），明确修复后快照即新 Reference Oracle。
- 验证：`cargo test --workspace` 全绿且新增回归测试覆盖核验后的修复项；Engine Reference Golden 捕获可重跑，跨层对比按 ADR-0007 判据通过（或差异列入已知偏差清单）。
- 退出条件：核验后的评审 P0/P1 项全部闭环；checklist Reference 段 "Engine Reference validated against upstream Golden" 行可勾选。

### It-3a — Core + Facade + glTF 骨架（Phase 2）

- 目标：交付不依赖 render 的领域层与导入层骨架。
- 交付物：
  1. `bevy-stylized-core`：Material/Instance/View Descriptor、三轴类型（SurfaceClass / ExecutionTier / ValidationMode）、Capability Model、Quality Policy、Profile contract、schema-neutral 序列化类型；只依赖 `bevy_math`/`bevy_color`/`bevy_reflect`；**schema 版本化机制**（版本字段 + 演进规则，ADR-0025 承诺的载体）；
  2. facade `bevy-stylized`：`StylizedPlugin` + capability routing 骨架；**最小 pre-pipeline capability resolve**（ADR-0022 两阶段模型从首个 pipeline 起合规，不留窗口期）；
  3. `bevy-stylized-gltf`：注册 `_NPR_*` 顶点属性 + typed extras/sidecar 导入（迁移 Phase 3 导入侧出口项提前随骨架落地）；ADR-0028（插件初始化顺序）以本 crate 为验证载体，facade 提供 GltfPlugin 配置 helper + 插件顺序集成测试。
- 验证：`cargo test --workspace` 绿；ADR-0028 插件顺序集成测试通过；core 不出现渲染内部类型（依赖边界测试）。
- 退出条件：checklist Authoring 段 "Bevy GltfPlugin registration ordering tested" 行可勾选。

### It-3b — Cel@Portable 渲染切片（Phase 3）

- 目标：交付最小渲染切片——`Cel @ Portable` forward + fixed binding。
- 交付物：
  1. `bevy-stylized-render`：Cel@Portable forward（Base Color + Ramp + 基础 ILM/Toon Lighting + hull outline），fixed binding，不依赖 bindless/FP16/compute/HDR；outline 与 lighting 语义草案即 ADR-0008 / 0009 草案（本迭代的 hull outline 与基础 lighting 是其首个实例）；
  2. **backend 临时条款**：本切片建立在 Bevy Material 组合路径上，显式标记 provisional（ADR-0001 未关闭）；render 代码保持薄层、全部公共语义挂在 core descriptor 上，把 ADR-0001 三方对比（It-6）结论引发的返工限制在 render crate 内部；
  3. ADR-0004 spike：按 ADR-0004 Context 指定的候选机制（MeshTag / GpuArrayBuffer / 纹理 LUT indirection，细分形态写入 ADR-0004 草案）做 prototype + 100/1000 实例合批测量，只产出决策证据，不固化 ABI；
  4. **Interim budget**：budget 反射统计测试落地，阈值标为 provisional（pending ADR-0002），验证条款写为"反射统计输出且在 interim 阈值内"，避免预算 gate 先于预算定义；
  5. 首个 Pipeline/Golden 测试：Cel@Portable 输出对照 It-2 捕获的 Engine Reference Golden（离线图像对比，经 golden-harness，遵守原则 3）。
- 验证：programmatic 与 glTF/`_NPR_*` 两种接入均 ≤ 10 行（release gate API 项双双验证）；`cargo test --workspace` 绿。
- 退出条件：checklist Architecture 段 "Surface Class × Execution Tier × Validation Mode implemented without semantic conflation" 行（Cel@Portable 首个实例，provisional 标注）与 Quality/Performance 段相应行可勾选。

### It-4 — 0.3 Instance/View + Character（Phase 4–5）

- 目标：完成 Material/Instance/View 三层分离，消除核心路径 per-frame Material Asset mutation；加入 Character surface。
- 交付物：
  1. ADR-0004 依据 It-3b spike 证据关闭，instance transport 落地（Face 状态、Outline Scale、Profile Index 等 per-instance）；
  2. View/Frame 数据路径（NPR Key Light、Global Palette、Day/Night、Exposure、Camera NPR 状态）；
  3. Face SDF 从 unique material 迁移为共享材质 + per-instance 数据；
  4. Character surface（Face / Hair / Rim / Emission / Enhanced Specular）+ Character@Portable fallback 定义；
  5. ADR-0020（extras vs sidecar 优先级）随 `_NPR_*` 导入路径关闭；
  6. ADR-0023 增量：本迭代落地的 alpha mode / cull 组合补入渲染语义矩阵测试（矩阵逐迭代积累，不积压到冻结期）。
- 验证：无 per-frame asset mutation 的架构测试；100/1000 实例合批 gate（阈值随 ADR-0004 关闭而定）通过；Character Golden。
- 退出条件：checklist Architecture 段 "Material / Instance / View separation enforced" 与 "Instance transport ADR closed" 行、Authoring 段 "extras/sidecar precedence frozen" 行可勾选。

### It-5 — 0.4 Hero + Genshin Reference Family（Phase 6）

- 目标：Hero surface + `bevy-stylized-profile-genshin` + 生产侧后处理，建立三层 Golden 对比。
- 交付物：
  1. Gacha ILM / Authored Ramp / Packed Normal / MatCap / Spec Ramp / Face Correction / Gacha Palette；
  2. **生产侧后处理**（全屏 post：vignette/色差/display transform 等，Portable 兼容形态）——release gate Golden 类别与 showcase 均含 Post；
  3. profile crate 尽量不接触 render internals（验证 ADR-0025 的 Profile 边界，产出 ADR-0005 / 0006 决策证据）；
  4. Upstream Golden → Engine Reference → Hero Production 三层对比测试；
  5. ADR-0023 增量：Hero 相关组合补入语义矩阵。
- 验证：Face/Hair/Metal/Outline/Lighting/Post 六类 Golden 三层可重复对比。
- 退出条件：checklist Reference 段 "Production variants validated against Engine Reference" 行可勾选。
- 注意：profile-genshin 的资产与数值形态受 It-1a 可发布性子决策约束。

### It-6 — 0.5 Resource Architecture（Phase 7）

- 目标：把 whole-pipeline per-stage 预算从 interim 变成 CI 契约，并定型 runtime material backend。
- 交付物：
  1. UBO packing、sampler 去重、texture 预算、vertex control packing；
  2. **ADR-0001 三方对比**：Optimized ExtendedMaterial / Direct Material / Custom Render Pipeline 三条路径的 Prototype + Benchmark（目标架构要求的规范性证据），据测量关闭 ADR-0001；It-3b 的 provisional backend 按结论收敛，返工限制在 render crate 内部；
  3. bindless prototype 与 fixed/bindless 对比测量，ADR-0003 关闭；
  4. 对最终 specialization 后 pipeline layout 的反射统计 budget 测试，interim 阈值转为正式契约，ADR-0002 关闭。
- 验证：预算测试在 WebGPU Core / GLES3.1 / WebGL2 三档 limit 模型下通过。
- 退出条件：checklist Architecture 段 "Runtime Material backend ADR closed" 与 "Whole-pipeline resource budget ADR closed" 行、Quality/Performance 段 "Whole-pipeline resource-budget suite passes" 行可勾选。

### It-7 — 0.6 Capability/Fallback（跨 Phase capability 政策线）

- 目标：完整 Capability Resolver + 确定性 fallback 策略；平台支持矩阵分级。
- 交付物：两阶段 capability 生命周期的完整落地（It-3a 最小骨架的扩展）；每 SurfaceClass 的 fallback 链；ADR-0010 / 0021 / 0026 关闭；ADR-0021 关闭后，Web 相关 limit 模型从 provisional 转正式。
- 验证：fallback 矩阵测试；`docs/platforms/SUPPORT_MATRIX.md` 更新，无证据的 `Supported` 声明清零。
- 退出条件：checklist Platforms 段 "Support matrix updated with evidence" 与 "Web baseline ADR closed" 行可勾选。

### It-8 — 0.7 Pipeline/Golden/Budget CI（跨 Phase 测试设施 CI 化）

- 目标：测试金字塔 CI 化；性能基线。
- 交付物：
  1. GPU 测试 runner（本机 RTX 5090 作为 self-hosted baseline，provenance 记录硬件/驱动版本）；budget 反射统计测试保持 CPU 侧可跑（不依赖 GPU），避免单点脆弱；
  2. CI 门禁从 It-0 的 CPU-only 扩展为 CPU + GPU 两层：GPU 层失败不阻塞非渲染 PR 的策略需明确定义（过渡期规则写入 CI 文档）；
  3. 4 个固定 benchmark 场景（1 Hero、100/500/1000 Portable）与性能目标格式对齐；**显式声明：V1 只建立本机 NVIDIA baseline，其余平台在取得真实证据前一律标 `Experimental`**（release gate 允许，此处写明以避免误解）。
- 退出条件：checklist Quality/Performance 段 "Required benchmark scenes recorded" 行（本机 baseline，注明范围）可勾选。

### It-9a — Showcase（Phase 8 前半）

- 目标：12 项 showcase（Overview/Ramp/Lighting/Face/Hair/Metal/Outline/Animation/Post/Stress/Reference/Performance），复用 It-1b fixture 管线与 It-8 benchmark 场景。

### It-9b — Tools + 文档（Phase 8 后半）

- 目标：`bevy-stylized validate` / `bevy-stylized inspect` CLI；集成文档、迁移文档（含 CHANGELOG 完整化）、第三方自定义 Profile 示例。

### It-10 — 0.9 Freeze → 1.0（Phase 9）

- 目标：API/Schema 冻结；对照 release gate 逐条验收。
- 交付物：ADR-0023 渲染语义矩阵收尾（It-4 起增量积累，本迭代只补缺口不新建体系）；ADR-0017 最终许可证形态关闭；ADR-0027 正式版；全部残留 V1 Blocker 清零；provisional 标记清零。
- 退出条件：`docs/release/V1_RELEASE_CHECKLIST.md` 全部勾选，发布 1.0。

## 4. ADR 决策队列

执行中按迭代顺序推进。agent 负责产出 Decision/Evidence/Consequences 草案与原型/测量证据；状态变更须经维护者确认（provisional 规则见 §1 原则 2）。

| ADR | 需要它的迭代 | 建议方向（草案要点） | 决策性质 |
|---|---|---|---|
| ADR-0027（临时条款） | It-0 | V1 开发期锁 Bevy 0.19.x | 临时政策，It-10 转正 |
| ADR-0017（可发布性子决策） | It-1a（阻塞涉外资产入库） | Golden/fixture 可发布性边界 + 行为复刻 vs 代码复制边界（Genshin lineage 为 GPL-3.0）；作为部分 Decision 写入 ADR 正文，整体状态保持 V1 Blocker | **必须维护者决策，不可 provisional** |
| ADR-0011 Toolchain Lock | It-1a | 锁定 Blender 精确版本 + Gacha Setup release/commit + digest | 证据支撑，维护者确认 |
| ADR-0012 Export Contract | It-1a | glTF exporter 设置 + `_NPR_*`/extras 导出形态 | 同上 |
| ADR-0015 Export Bridge | It-1a | Blender Python 脚本 + 批处理，fail closed | 同上 |
| ADR-0007 Golden Chain | It-1b | 三层 Golden + provenance 元数据格式 + 容差策略 + 跨层对比判据 | 同上 |
| ADR-0013 Upgrade Policy | It-1b | 上游升级流程与 Golden 重基线规则 | 政策决策 |
| ADR-0014 Reference/Runtime 边界 | It-1b | Reference 资产与 shipping 产物的解耦边界 | 政策决策 |
| ADR-0028 glTF Plugin Ordering | It-3a | facade 提供 GltfPlugin 配置 helper，`_NPR_*` 注册先于 loader 配置 | 原型验证后确认 |
| ADR-0022（最小落地） | It-3a | 两阶段 capability 生命周期自首个 pipeline 起合规 | 已 Accepted，执行项 |
| ADR-0008 Outline Architecture | It-3b（草案） | hull outline 为 Portable 基线；高级描边归 V2 | 契约设计 |
| ADR-0009 Lighting Backends | It-3b（草案） | 基础 physical/authored lighting 语义 | 契约设计 |
| ADR-0004 Instance Transport | It-3b spike → It-4 关闭 | ADR 指定候选机制 prototype + 100/1000 合批测量 | 测量驱动 |
| ADR-0020 Extras vs Sidecar | It-4 | 随 `_NPR_*` 导入路径确定优先级 | 契约设计 |
| ADR-0023 Semantics Matrix | It-4 起增量 → It-10 收尾 | alpha/shadow/prepass/outline 组合逐迭代积累 | 增量证据 |
| ADR-0005 / 0006 | It-5 | Profile 边界与纹理 packing 语义 | 契约设计 |
| ADR-0001 Runtime Material Backend | It-6（It-3b provisional 前置） | 三方路径 Prototype + Benchmark 后选型 | 测量驱动 |
| ADR-0002 / 0003 | It-6 | 预算数值契约化、fixed/bindless 策略 | 测量驱动 |
| ADR-0010 / 0021 / 0026 | It-7 | 平台政策、Web baseline、禁用成本政策 | 政策决策 |
| ADR-0029 | It-8 | benchmark 基线与性能证据流程入 CI | 已 Accepted，执行项 |
| ADR-0017（最终形态） | It-10 | 仓库许可证与发布形态 | **必须维护者决策** |
| ADR-0027（正式版） | It-10 | Bevy 版本兼容政策 | 维护者确认 |

## 5. 风险与对策

| 风险 | 影响 | 对策 |
|---|---|---|
| Gacha Setup 上游资产/版本不可得或与本机 Blender 5.2.1 不兼容 | It-1a 阻塞 | 上游资格评估为 It-1a 第一步；必要时安装被锁 Blender 版本（与 snap 并存）；不可得则按评估记录的降级方案（本地存档 + 合规确认）并提请维护者决策 |
| GPL-3.0 lineage 衍生物入库后 ADR-0017 结论反转 | It-1b~It-9 资产返工 | 可发布性子决策前置到 It-1a 并阻塞涉外资产入库；公开 fixture 一律自制/CC0；子决策未确认时 It-1b 以纯自制 fixture 推进、派生资产缓入库 |
| ADR-0017 子决策迟迟未获确认 | It-1b 涉外资产工作停摆 | 该阻塞为有意设计（GPL 污染不可撤回）；It-1b 的 fixture 管线/复现脚本/政策草案均不依赖涉外资产，可先行交付 |
| 维护者确认无 SLA，关键路径挂人员响应 | 全计划 | §1 原则 2 的 provisional 规则（ADR-0017 除外）；决策队列随每次迭代评审 |
| ADR-0004 候选机制均不满足合批 gate | It-4 延期 | It-3b spike 前置；fallback：最小公共 ABI 只承诺语义不承诺布局 |
| ADR-0001 三方对比结论推翻 provisional backend | It-3b~It-5 返工 | render 薄层 + 语义挂 core descriptor，返工限制在 render crate 内部 |
| GPU CI 依赖单台本机 | It-8 脆弱 | provenance 记录硬件/驱动；budget 反射测试保持 CPU 可跑；GPU 层不阻塞非渲染 PR |
| Golden 跨环境抖动 | 误报 | 固定 Scene/Camera/Lighting/Resolution + ADR-0007 容差策略与跨层判据 |
| Golden 基线文件体积膨胀 | 仓库膨胀 | It-2 写明 LFS 决策点；基线仅覆盖固定场景矩阵 |
| 范围蔓延（V2/V3 特性渗入） | 路线失真 | 迭代评审时对照 V1 非目标清单逐项排除 |

## 6. 每迭代结束的固定动作

1. 按 §7 映射更新 `docs/release/V1_RELEASE_CHECKLIST.md` 相应勾选（只勾选物理上已满足的行）；
2. 更新 `TRACEABILITY.md` 相应需求的 evidence 链接（provisional 证据须标注）；
3. 在 `docs/migration/01-tenacity-migration-plan.md` §迁移状态记录追加日期条目；
4. `python3 tools/check_docs.py` 与 `cargo test --workspace` 全绿方可关闭迭代；
5. 涉及公共契约变化时同步更新相应子系统规范或 ADR。

## 7. 迭代 → Release Checklist 行映射

> 段名与行文逐字对应 `docs/release/V1_RELEASE_CHECKLIST.md`（It-0 交付物 5 负责增补缺失行，标注 ★ 的为待增补行）。退出条件只引用本迭代结束时物理上可勾选的行；同一行可被多个迭代逐步满足，由最后满足的迭代勾选。

| 迭代 | 本迭代结束时可勾选的 checklist 行（段 → 行） |
|---|---|
| It-0 | Release/Governance → "Feature matrix CI passes"（基础矩阵起步，随迭代扩展，It-10 最终确认）；Architecture decisions → "Documentation linter passes with no errors" |
| It-1a | Authoring → "Blender export bridge is scriptable and fail-closed"；Reference → ★"Reference toolchain locked (Blender / Gacha Setup / digests)"、★"Blender export contract frozen" |
| It-1b | Reference → "Blender upstream Golden produced before runtime export"、"Reference artifact provenance complete"、★"Reference upgrade policy frozen" |
| It-2 | Reference → "Engine Reference validated against upstream Golden"（按 ADR-0007 跨层判据） |
| It-3a | Authoring → "Bevy GltfPlugin registration ordering tested" |
| It-3b | Architecture → "Surface Class × Execution Tier × Validation Mode implemented without semantic conflation"（首个实例，provisional 标注）；Quality/Performance → "Golden suite passes"（起步） |
| It-4 | Architecture → "Material / Instance / View separation enforced"、"Instance transport ADR closed"；Authoring → "extras/sidecar precedence frozen"；Quality/Performance → "Automatic instancing / instance-data regression passes" |
| It-5 | Reference → "Production variants validated against Engine Reference"；Rendering semantics → "Cull inheritance tested" 等增量行 |
| It-6 | Architecture → "Runtime Material backend ADR closed"、"Whole-pipeline resource budget ADR closed"；Quality/Performance → "Whole-pipeline resource-budget suite passes" |
| It-7 | Platforms → "Support matrix updated with evidence"、"Web baseline ADR closed"、"Any `Supported` target has real runtime evidence" |
| It-8 | Quality/Performance → "Required benchmark scenes recorded"（本机 baseline，注明范围）；Platforms → "Backend-specific pipeline creation tests run" |
| It-9a | Release/Governance → ★"Showcase suite complete" |
| It-9b | Release/Governance → "Migration notes / changelog complete"、★"CLI validate/inspect shipped" |
| It-10 | 全部残留行（Architecture decisions 全段、Authoring → "`_NPR_*` glTF attribute schema frozen"、Rendering semantics 全段、Release/Governance → "License/provenance ADR closed" 与 "Bevy compatibility matrix published"） |

---

## 8. V2 迭代序列（2026-09-12 增补，v4）

V1 代码面已完成（It-0~It-10），V1 Blocker ADR 的确认与 provisional 解除属维护者动作，不阻塞 V2 的**增量、可选路径**开发（provisional 规则不变；V2 特性不得破坏 Portable 不变量，高端能力必须落在 Standard/High 且有 fallback）。

| 迭代 | 主题 | 对应 roadmap V2 项 | 依赖 |
|---|---|---|---|
| V2-a | 高级描边（depth/normal 屏幕空间描边 + hull 混合） | Advanced Outline | render 后处理设施 |
| V2-b | NPR LOD（Class×Tier 距离规则 + geometry/material/shader feature LOD） | NPR LOD | 三轴 resolver |
| V2-c | Dynamic Quality（迟滞/冷却/可观测的 tier 调节器） | Dynamic Quality | capability 生命周期 |
| V2-d | Environment NPR（风格化 fog / IBL tint / 简化 stylized shadow） | Environment NPR | view 层 |
| V2-e | GPU Instance Database（High tier storage 路径 + 实测对比） | GPU Instance Database / Bindless Production | ADR-0004 spike 证据 |
| V2-f | Asset Compiler 骨架（`bevy-stylized compile`：ILM 打包 + 校验 + runtime descriptor 生成） | Asset Compiler | tools crate |
| V2-g | 回归/性能实验室（Count × Resolution × Tier × Backend × Pose × Lighting sweep） | Full Regression/Performance Lab | benchmarks + showcase |

GPU-driven rendering（GPU culling / indirect draw / meshlet）按 roadmap 要求"以实际 Bevy/wgpu 能力和 Benchmark 为前提"，排在本批之后单独评估；V3（SDK/编辑器/声明式 Profile）不在本批。

每个 V2 迭代的退出条件：功能 + 测试 + golden/showcase 证据 + CHANGELOG + 迁移记录条目；涉及公共契约的变更同步相应子系统文档（provisional 标注）。
