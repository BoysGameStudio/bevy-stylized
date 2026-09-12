# 从 Tenacity 迁移的实施路线

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


迁移必须双轨进行：

```text
Legacy Reference Path
+
New Production Path
```

不能：

```text
先删旧 Shader
→ 凭肉眼重新实现
```

否则失去 Upstream Reference-Parity Oracle。

---

## Phase 0：Reference Freeze

1. 固定当前 Gacha/Blender Reference Shader；
2. 创建 `Reference Toolchain Lock`，锁定 Blender 精确版本、Gacha Setup release/tag/commit、release asset digest；
3. 固定 Blender glTF Export Contract；
4. 建立 Gacha Setup Upstream Behavior Contract；
5. 建立 Blender Reference Export Bridge，并让它输出 Node-Graph Signature 与 Export Manifest；
6. 修复影响 Reference 正确性的明确 bug；
7. 固定 Reference Scene、Camera、Lighting、Resolution；
8. 建立带 Provenance Metadata 的 Golden；
9. 建立 Reference Upgrade Policy；
10. 将 Reference 与 Shipping Production 解耦。

---

## Phase 1：Correctness

优先完成：

1. InternalEdge ABI；
2. Character Cull Inherit；
3. Real Pipeline Creation Test；
4. WGSL/Naga Validation；
5. Outline Lifecycle Contract；
6. Camera/Post Ownership。

---

## Phase 2：Core Extraction

建立：

```text
bevy-stylized-core
```

抽离：

- Ramp；
- Face settings；
- Lighting model；
- Material descriptor；
- Instance descriptor；
- Capability；
- Quality；
- Profile contract。

这一阶段不追求 GPU 优化。

---

## Phase 3：Cel Surface + Portable Execution Baseline

实现：

```text
Cel @ Portable
```

先满足：

- minimum resource budget；
- fixed binding；
- core ramp/control；
- registered `_NPR_*` glTF attributes + typed extras/sidecar import；
- hull outline；
- basic physical/authored lighting；
- Pipeline/Golden。

这是整个新 Renderer 的地基。

---

## Phase 4：Material / Instance / View

完成：

```text
Material
Instance
View
```

分离。

迁移：

- Face Runtime State；
- Key Light；
- Global Palette；
- Day/Night；
- Outline Instance Control。

消除核心路径的 per-frame Material Asset mutation。

---

## Phase 5：Character

加入：

- Face；
- Hair；
- Rim；
- Emission；
- Enhanced Specular。

必须定义 Character @ Portable fallback/approximation。

---

## Phase 6：Hero Surface + Genshin Reference Family

加入：

- Gacha ILM；
- Authored Ramp；
- Packed Normal；
- MatCap；
- Spec Ramp；
- Face Correction；
- Gacha Palette。

同时建立：

```text
Upstream Golden → Engine Reference → Hero Production
```

Golden 对比。

---

## Phase 7：GPU Resource Architecture

执行：

- UBO packing；
- sampler dedup；
- texture budget；
- vertex control packing；
- bindless prototype；
- fixed/bindless comparison。

在这里通过 ADR 决定最终 Runtime Material Backend。

---

## Phase 8：Showcase / Tools / Docs

完成：

- Showcase labs；
- validate/inspect CLI；
- Integration docs；
- Custom Profile example；
- Migration docs。

---

## Phase 9：V1 Freeze

停止新增大 Feature。

只处理：

- correctness；
- portability；
- API cleanup；
- performance regression；
- documentation。

---

## 迁移状态记录

### 2026-09-12：Reference-parity 修正（行为合同核查后续）

- 行为合同 9 项待核实全部经节点级证据核实（`tools/blender-reference-export/inspect_node_math.py` 转储 42 个节点组）；新发现：上游 blend 含 Blender 5.2.1 不可解析节点（cast-shadow / curvature-rim 为锁定环境下的死路径，upstream Golden 已反映该状态）。
- 修正 production 与上游的五处矛盾：ILM 通道布局（R=metal/G=AO/B=高光抑制/A=分带 ID，emission 改走 base alpha）、post display-transform 曲线常数、Genshin ramp 寻址（X 公式 + 昼夜半行分带）、sharp×soft 串联光照结构、face SDF head-basis 公式。
- 对比域归一化修复（Lambert 1/π 能量 + 背景色对齐）：production vs upstream PSNR 13.9dB → 22.5dB（parity 修正后 21.1dB，残余差距为 cast shadow 缺失与 palette 标定近似，如实记录于 docs/reference/08）。
- TRACEABILITY.md 29 条需求证据回填（7 ✅ / 21 🧪 / 1 ❌ R-021 web）；provisional 清单修正一处 ADR-0018 误标。
- 全部受影响 golden 按纪律重捕获并目检；71 suite 全绿。

### 2026-09-12：It-10 完成——冻结收尾（agent 可关闭部分）

- 渲染语义补全：Blend 在 V1 类型层面不可达（fail-closed，测试钉死）；alpha 支持矩阵发布于 `docs/authoring/05-integration-guide.md` §7；prepass/shadow 契约测试；morph/skinning GPU 测试——过程中发现并修复真 bug（新渲染器此前静默忽略 morph targets，已补进共享形变路径并重捕获受影响 golden）。
- "Reference-only 不进 shipping build"由结构性测试钉死（cargo metadata 断言生产 crate normal 依赖不含 legacy/dev crate）。
- Feature 矩阵 CI 扩展至 13 组合（本地全部验证通过；首次 push 后勾 "Feature matrix CI passes"）。
- Bevy 兼容声明发布于 SUPPORT_MATRIX（provisional pending ADR-0027）。
- **维护者交接清单**：`docs/release/03-provisional-inventory.md`——17 条 provisional 项 → 阻塞 ADR → 位置 → 现有证据。
- checklist 新增勾选：Rendering semantics 全段、Reference-only 排除、性能证据纪律、Bevy 兼容声明。
- 最终状态：`cargo test --workspace`（70 suite）/ clippy `-D warnings` / fmt / check_docs 全绿。
- 未勾选行全部为维护者决策项（9 个 V1 Blocker ADR、license、上游对照判据校准）或需首次 push 验证项。

### 2026-09-12：It-9b 完成——CLI + 集成文档 + 自定义 Profile 示例

- 新增 `crates/bevy-stylized-tools`（binary `bevy-stylized`，纯 gltf 解析、无 GPU 依赖）：`validate`（`_NPR_*` 属性契约校验 + extras/sidecar schema 版本 fail-closed，exit 0/1/2）与 `inspect`（属性/材质/网格统计 + 可驱动 class 启发式）；fixture GLB 实测 exit 0，可驱动 Cel/Character/Hero。
- 第三方 Profile 示例：`crates/bevy-stylized/examples/custom_profile.rs`（假想 Watercolor profile），注册不触 render internals（ADR-0025 契约），实跑通过。
- 新增 `docs/authoring/05-integration-guide.md`（≤10 行接入、glTF 路径、CLI、fallback 总览）。
- checklist 勾选 "CLI validate/inspect shipped" 与 "Migration notes / changelog complete"。
- 全量验证绿（68 suite）。

### 2026-09-12：It-9a 完成——12 项 Showcase

- 新增 `crates/dev/showcase`：用例注册表 + 单 CLI（`cargo run -p showcase -- <name|all> [--capture]`；交互开窗为非默认 `interactive` feature——winit 进入 workspace 特性图会破坏 GPU 测试，已隔离）；`--capture` headless 离屏写 `fixtures/golden-local/showcase/`。
- 12 用例全部本机实跑并逐张目检：overview / ramp / lighting / face（SDF 阈值扫描）/ hair / metal（matcap）/ outline（宽度级）/ animation（程序化蒙皮）/ post（vignette+chromatic 单 pass）/ stress（1000 实例）/ reference（fixture 场景 production 路径）/ performance。全部非黑图，不依赖 Odette。
- 新增操作手册 `docs/showcase/02-running-the-showcase-suite.md`；checklist 勾选 "Showcase suite complete"。
- 全量验证绿（65 suite）。

### 2026-09-12：It-8 完成——Benchmark 基线 + CI 分层

- 新增 `crates/dev/benchmarks`：4 个固定场景（hero_1 / portable_100 / portable_500 / portable_1000，1280×720，warm-up 20 + 测量 60 帧），JSON 遵循 docs/performance/targets.md（含硬件/驱动 provenance）；实测主线程 update 6.3~8.0ms，1000 实例增长平缓（合批路径佐证）。
- 本机 baseline 入库：`docs/performance/baselines/linux-vulkan-rtx5090-2026-09-11.json`（V1 唯一 baseline，其余平台保持 Experimental）。
- CI 分层：rust.yml 标注 CPU 层；新增 gpu.yml（self-hosted + if:false，runner 注册为维护者平台操作）；.github/README.md 写明两层政策与过渡规则（GPU 层只阻塞渲染 PR）。
- checklist 勾选 "Required benchmark scenes recorded" 与 "Backend-specific pipeline creation tests run"。
- 全量验证绿（61 suite）。

### 2026-09-12：It-7 完成——Capability Resolver + 确定性 fallback

- 完整两阶段 capability 生命周期：Stage B 升级为 `resolve_with_report`（输出 `StylizedFallbackReport`：降了什么、为什么）；确定性双跑逐位相等测试。
- Per-class fallback 链数据化（Cel 空链基线；Character 两步；Hero 五步），与 render 侧 portable fallback 一一对应；fallback 矩阵快照测试覆盖 3 class × 3 档 × 4 能力模型（含 WebGL2 无 storage → 降 Portable 等）。过程中修正 It-3a provisional 数据 bug（HIGH_LIMITS 超 wgpu defaults 致 High 永不可满足）。
- `docs/platforms/SUPPORT_MATRIX.md` 按证据更新：Linux/Vulkan/RTX 5090 标 Experimental（single-device，附证据列），无证据的 Supported 声明为零；未声明任何 web 支持（ADR-0021 未决）。
- 禁用特性成本测试（ADR-0026 执行项）：outline/post/instance 无组件时零 per-frame 副作用。
- checklist 勾选 Platforms 段 "Support matrix updated with evidence" 与 "Any Supported target has real runtime evidence"。
- 全量验证绿（58 suite）。

### 2026-09-12：It-6 完成——资源架构（预算核算 + ADR-0001 对比证据）

- Whole-pipeline 预算核算落地：core 新增三档 baseline 限制模型（WebGPU Core / GLES3.1 / WebGL2，数值核对自 wgpu-types 29），render 侧对 view 层做真实 pipeline layout 反射计数；六段分解（View+Mesh+Instance+Material+Profile+Safety）报告 + Portable 变体超 WebGL2 模型即失败的硬门禁。实测：Cel/Character/Outline/Hero-fallback @ Portable 全部三模型通过；Hero@Portable 不超硬上限但 sampler 11/16 偏紧（如实记录，per-tier 变体为后续优化）。
- UBO 审计：四材质全部恰好 1 个 packed UBO（钉死）；sampler 1:1 现状钉为基线，dedup 记为已记录机会。
- ADR-0001 证据（provisional）：A（ExtendedMaterial）vs B（Direct Material，shipping）500 实例实测——稳态帧耗时基本打平（4.47 vs 4.69 ms），A 免费获得 shadow/fog/prepass 生态，B 完全控制绑定与预算；建议保留 B；C（custom pipeline）集成面已记录未原型化。ADR-0003：bindless 不实施，重访阈值成文。
- checklist 勾选 "Whole-pipeline resource-budget suite passes"（阈值 provisional pending ADR-0002）。
- 全量验证绿（57 suite）。

### 2026-09-12：It-5 完成——Hero + profile-genshin + 生产后处理 + 三层 Golden 对比

- Hero surface：`StylizedHeroMaterial`（authored ramp atlas 行选、packed normal、metal matcap、spec ramp），Hero@Portable fallback 代码化；预算实测 Hero 7T/7S/1UBO（如实记录超 Cel interim 子预算，待 ADR-0002）。
- 新增 `crates/bevy-stylized-profile-genshin`（仅依赖 core，facade `profile-genshin` feature 默认关闭）：Genshin profile 契约实现，仅收录行为合同（docs/reference/07）证据支撑条目，其余标"待核实"。
- 生产后处理：`StylizedPostProcess`（vignette/chromatic/display-transform hook，FullscreenMaterial 单 pass，View 层驱动，LDR-safe）。
- 三层 Golden 对比（`crates/dev/reference-comparison`）：fixture 场景经 production / legacy engine reference / upstream Blender 三路径离屏渲染，按 docs/reference/08 草案判据出 diff 报告。实测三对均未过草案 Tier-2 判据（主因 tone response 差异与 palette 近似差距；剪影 IoU≈1.0 几何对齐正确）——未放宽容差，实测数字回填 docs/reference/08 §3 注记，判据校准留 ADR-0007。
- checklist 勾选 Rendering semantics 段 "Cull inheritance tested"。
- 全量验证绿（56 suite）。

### 2026-09-12：It-1b 完成——Upstream Golden 基线

- Headless 路径实测结论：snap Blender 5.2.1 + RTX 5090 原生 EEVEE headless 直接可用（无 xvfb/软渲染降级）。
- 固定 Reference 场景：`tools/blender-reference-export/make_reference_scene.py`（纯解析、确定性）生成 `fixtures/reference/reference_scene.blend`（自制可入库）；fixture 几何携带全部 10 个 `_NPR_*` 属性；经导出桥验证，GLB 二进制确认携带 `_NPR_*` + TANGENT + `bevy_stylized.*` extras。
- Upstream Golden：`fixtures/golden-local/upstream/` 两例（PBR 控制组 + 真实 Gacha 节点组接线），provenance JSON 齐全（toolchain/commit/digest/fixture hash/渲染器）；复现脚本 `reproduce_golden.sh` 两次重跑均 bit-identical。
- 容差与跨层判据草案：`docs/reference/08-golden-tolerance-and-cross-layer-criteria.md`（Tier 0/1/2 模型、跨层归一化规则、"validated" 定义；Tier 2 数值待实测校准）。
- toolchain lock 更新：`[export].contract = blender-gltf-npr-v1`（Draft 标注）、`[fixture]` 三个 sha256、`[golden]` revision；`[profile].genshin_schema` 保持 PENDING。
- 已知缺口：gacha_upstream 捕获为"无 lightmap 中性默认"基准行为（ILM/face-SDF 贴图路径需 wizard 生成贴图，留后续）；ADR-0007/0013/0014 仍待维护者确认。
- checklist 勾选：Reference 段 "Blender upstream Golden produced before runtime export" 与 "Reference toolchain locked"、Authoring 段 "Blender export bridge is scriptable and fail-closed"。

### 2026-09-12：It-4 完成——Instance/View 分离 + Character surface

- Instance 传输（provisional pending ADR-0004）：`StylizedInstanceData` → MeshTag 位打包（量化与保留值成文），仅 Changed 时写入；spike 证据驱动选择 MeshTagBits 为 Portable 通道，StorageTable 声明未实现（WebGL2/GLES3.0 无 storage buffer 证据）。架构测试保证核心路径零 per-frame 材质资产写（修复一处 Modified 回声风暴）。
- View/Frame：`StylizedViewDescriptor` 全字段经 view 层落地（key light / exposure / palette→ambient / 昼夜 lerp 环境光，决策成文于 `resolve_ambient_light`）。
- Character surface：`StylizedCharacterMaterial`（Face SDF 共享材质 + MeshTag 逐实例阈值/侧选择、Hair 高光、Rim、Emission），预算 3T/3S/1UBO；Character@Portable fallback 显式定义并代码化。
- ADR-0023 增量：Opaque/Mask × outline × cull 语义矩阵测试（provisional）。
- Golden：character-portable 新基线（face SDF 三实例阈值对照、Odette Character 两例），合批 gate 证明 100/1000 实例单 draw 且逐实例可见差异；对 legacy golden 的 diff 报告（非门禁）。
- 全量验证绿（51 suite）；checklist 勾选 Material/Instance/View 分离行与 instancing 回归行。

### 2026-09-11：It-3b 完成——Cel@Portable 渲染切片

- 新增 `crates/bevy-stylized-render`：`StylizedCelMaterial`（Base Color + Ramp + 基础 ILM/Toon Lighting）与 `StylizedOutlineMaterial`（inverted hull），`portable_forward.wgsl`/`portable_outline.wgsl` 独立编译单元，fixed binding，零 storage buffer；预算反射实测 3 textures / 3 samplers / 1 UBO（interim 阈值内，provisional pending ADR-0002）。
- facade 接入 ≤10 行（doc test 实证）；`StylizedPlugin` 新增默认开启的 `render` feature。
- ADR-0004 spike（证据，不固化 ABI）：MeshTag bits 与 storage 表两方案在 100/1000 实例下均保持单 draw 合批；storage 方案在 WebGL2/GLES3.0 baseline 不可用（证据已记录于 target/tmp/adr-0004-spike/）。
- Golden：合成三例（解析 ramp / 纹理 ramp 逐实例行选择 / hull 描边）+ Odette Cel 两例（full_body / face_closeup）基线建立于 `fixtures/golden-local/`；对 legacy engine golden 产出 diff 报告（非门禁，着色模型差异属预期）。重跑 diff = 0。
- 修复三个真实 bug：帧缓冲 alpha 污染、outline 副本漏拷 SkinnedMesh、hull 外扩缺顶点色宽度抑制。
- 全量验证绿（48 suite）；checklist 勾选三轴实现行与 Golden suite 起步行。

### 2026-09-11：It-3a 完成——core / facade / gltf 三 crate 骨架

- 新增 `crates/bevy-stylized-core`：三轴类型（SurfaceClass/ExecutionTier/ValidationMode）、Material/Instance/View 三层 descriptor、capability 模型与确定性 resolver（类型层面保证降 tier 不降 class）、Profile contract、schema 版本化机制；依赖白名单（bevy_math/bevy_color/bevy_reflect/serde）有测试守护。
- 新增 facade `crates/bevy-stylized`：`StylizedPlugin`、ADR-0022 两阶段 capability 落地（Stage A `WgpuSettings` 贡献 / Stage B 只读实际 features/limits，无 RenderDevice 时 Portable 兜底）、`stylized_gltf_plugin()` 配置 helper（ADR-0028 机制 provisional）。
- 新增 `crates/bevy-stylized-gltf`：10 个 `_NPR_*` 顶点属性注册（与 `tools/blender-reference-export/contracts/npr-attributes.v1.json` 正本逐项对照）、`_CEL_*` 别名表（仅数据不注册）、extras/sidecar 双通道 typed 导入（冲突显式诊断不取舍，ADR-0020 未决）。
- 新增测试 39 个（含 ADR-0028 插件顺序集成测试：正确顺序加载合成 glTF 成功、错误顺序 fail closed）；checklist Authoring 段 "Bevy GltfPlugin registration ordering tested" 勾选。
- 验证：`cargo test --workspace`（41 suite）/ clippy `-D warnings` / fmt / check_docs 全绿；facade/core `--no-default-features` 编译通过。
- 依赖方向守住：facade → core、facade →(optional) gltf → core；无任何 → legacy 依赖。

### 2026-09-11：决策内容清除

- 应维护者要求，清除此前写入仓库的全部决策性内容：ADR-0011 / 0012 / 0015 的 Draft proposal 节、ADR-0017 的可发布性子决策、ADR-0027 的临时条款，五个 ADR 文件已恢复为原始 Pending 骨架。
- 本日早些时候条目中对上述决策/草案的引用同步失效，已就地修正为事实性描述。
- 代码与工具成果（It-0 工程基座、It-1a 工具链验证与导出桥、It-2 六项修复与 Golden 设施）不受此影响，全部保留。

### 2026-09-11：It-2 完成——六项修复 + Engine Golden 设施 + Odette 实内容验证

- **修复落地（每项含回归测试）**：
  1. InternalEdge ABI：拆出 `outline_shared.wgsl` 共享变形模块 + 独立 `internal_edge_vertex.wgsl`（32B `InternalEdgeUniform`），ABI 不一致消除；
  2. `OutlineCullMode::Inherit` 新增并设为默认，不再无条件覆盖 base 材质 cull mode；
  3. `tests/shader_validation.rs`：经 Bevy ShaderCache 组合 8 个 shader × 20 组真实 def 后 naga 全量校验；
  4. `tests/render_pipeline_gpu.rs`：GPU 门控真实 pipeline 创建测试（本机 RTX 5090 实跑通过；反向验证确认它能捕获缺陷 1 类的 ABI 错误）；
  5. Outline 生命周期：材质句柄变更跟随、internal_edge 后置补生、组件移除清理、`CharacterOutline` immutable/mutable 契约成文；
  6. Camera/Post 所有权：`apply_profiles` 成为相机 NPR 组件单一写入者（需求并集），`#[require(Hdr)]` 冲突移除。
- **Engine Golden 设施**：`crates/dev/golden-harness`（离屏捕获/图像 diff/provenance，12 个无 GPU 单测）+ `crates/dev/odette-reference-lab`（Odette 三例 Golden 矩阵，资产经 `TENACITY_ROOT` 本地引用、不入库，GPU/资产缺失时优雅跳过）。
- **Odette 实内容首渲**：修复后的 legacy 栈离屏渲染 Odette（512×512，镜像 Tenacity 材质接线），full_body / face_closeup / no_outline 三例基线建立于 `fixtures/golden-local/`（gitignored，第三方资产不入库）；二次运行逐位一致；Face SDF 方向阴影、眼部发射、蒙皮均正常，无破坏性异常。
- **Reference Oracle 再冻结**：上述修复后的 `crates/legacy/` 状态即为新的冻结 Oracle 快照（本条目即 provenance 记录；原始导入来源为 Tenacity `b5b26ac`）。

### 2026-09-11：It-2 启动——评审发现核验

- 测试以 Tenacity 项目的 Odette 角色资产为真实内容测试体（仅本地引用，不入库）。
- 评审发现核验（对冻结快照 b5b26ac 逐条复核，file:line 证据齐全）：
  1. InternalEdge ABI 不匹配——**确认存在（P0）**：vertex 端按 160B `OutlineUniform` 读 binding 0，实际绑定 32B `InternalEdgeAccentSettings`（`src/outline.rs:267-323` vs `src/shaders/outline_vertex.wgsl:13-27`）；
  2. Cull mode 无条件覆盖 base 值、无 `Inherit` 变体——**确认存在**（`src/lib.rs:163-175,277`）；
  3. 无真实 pipeline 创建测试——**确认存在**（15 个测试文件无一触碰 RenderDevice/pipeline；但并非全是字符串断言，已有 App 级行为测试）；
  4. 无 WGSL/Naga 校验——**确认存在**（dev-deps 无 naga）；
  5. Outline 生命周期——**部分存在**：当前代码已有 mesh/Aabb/morph 运行时同步（review 快照后新增），但材质句柄变更不跟随、后加 internal_edge 不生效、移除组件无清理、immutable/mutable 契约仍未成文；
  6. Camera/Post 所有权——**部分存在**：已收敛至 `apply_profiles` 单系统，残留 `#[require(Hdr)]` 与 profile 驱动 remove 的冲突、prepass 组件双主人问题。

### 2026-09-11：It-1a 非阻塞交付物完成

- 上游资格评估（修正后）：正确上游为 `PaoloESAN/gacha-setup`，候选值 v3.6.2 / commit `069e51f3…` / asset sha256 `f5ab8206…` / 三个 artifact 路径与 blob SHA 十项全部验证通过，详见 `docs/reference/upstream-qualification-2026-09-11.md` §7 与 `docs/reference/reference-toolchain.toml`。
- Blender 运行时实测：本机 Blender 5.2.1 LTS headless 正常；三个 reference blend 均干净打开（主 blend 观测到 EEVEE / sRGB / Standard）；Gacha Setup v3.6.2 addon（`setup_wizard` 模块）安装并启用成功。Blender 精确版本与渲染环境已写入 toolchain lock TOML。
- Blender Reference Export Bridge 已实现于 `tools/blender-reference-export/`（`export_reference.py` + `run_batch.py` + `contracts/`），冒烟运行 3/3 成功，三次 fail-closed 触发实测（版本不符 / 签名不匹配 / 设置漂移），manifest 全部通过 schema 校验。
- 契约草案：`docs/reference/06-export-contract-blender-gltf-npr-v1.md`（导出合同 v1）与 `docs/reference/07-gacha-setup-behavior-contract-v1.md`（行为合同 v1）建立，均为 Draft 状态。
- `[export].contract` / `[profile].genshin_schema` 在 toolchain lock 中保持 PENDING（对应 ADR-0012 未决）。

### 2026-09-11：迭代计划 v3 与 It-0 工程基座

- 新增 `docs/roadmap/02-iteration-plan.md`（v3）：将路线图与迁移 Phase 落为 It-0~It-10 迭代序列，含 ADR 决策队列与迭代 → release checklist 行映射；经两轮对抗性评审修订。
- It-0 交付：Rust CI（fmt / clippy `-D warnings` / test / feature matrix）、`rust-toolchain.toml`、仓库级 `CHANGELOG.md`。
- `docs/release/V1_RELEASE_CHECKLIST.md` 增补 5 行（toolchain lock / export contract / upgrade policy / showcase / CLI）并勾选 "Documentation linter passes"；"Feature matrix CI passes" 待首次 push 后验证勾选。
- 本机基线：`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --check`、`tools/check_docs.py` 全绿。

### 2026-09-11：Legacy Reference Path 导入

- 从 Tenacity(local:`/home/x/Projects/Tenacity`,commit `b5b26ac`)拷贝 `bevy-cel` 与 `bevy-cel-metadata` 到 `crates/legacy/`,package 名保持 `bevy_cel` / `bevy_gltf_metadata`,`embedded://` shader 路径不变。
- 两个 crate 冻结为 Reference Oracle;其 public API 不是 Bevy Stylized 的稳定契约。Production Path(Phase 2 起）据此做 Core Extraction 与对照验证。
- 剥离了 `bevy_gltf_metadata` 的可选 `kawaii` feature（依赖未迁移的 `bevy_kawaii_physics`,bevy_cel 未使用）。
- 未导入 git 历史；本节即 provenance 记录。许可证沿用原 MIT 声明，仓库级许可证仍由 ADR-0017 决定。

---
