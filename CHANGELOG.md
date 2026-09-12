# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Semantic Versioning applies once the first release is published.

## [Unreleased]

### Added

- V2-g：Regression/Performance Lab（provisional；ADR-0029 证据格式扩展）——
  - 新 dev crate `crates/dev/regression-lab`（lib + CLI）：sweep 矩阵 `Count{1,100,1000} × Resolution{512²,720p} × Tier{Portable,Standard} × Lighting{day,night} × Pose{bind,animated}`（crowd 实例场景 24 cells + strip 蒙皮条带场景 16 cells + High 探测 2 cells = 42 cells；High 请求档如实记录解析结果）；Backend 轴只跑 vulkan，其余记录 `skipped-unavailable`（不伪造）；
  - 测量子采样（文档化）：15 warm-up + 40 测量帧（benchmarks 为 20+60），保持分钟级墙钟；帧统计 mean/median/p95/min/max；4 个代表 cell 的 golden 子集捕获（`fixtures/golden-local/regression-lab/`，首跑写基线，后续 diff）；
  - 回归门禁：`--gate BASELINE.json` 按 target_id 对齐，mean 帧时劣化 >10% 记回归并 exit 1（缺失 cell 如实记录不算回归）；gate 语义纯逻辑单测覆盖；
  - 本机基线：`docs/performance/baselines/sweep-nvidia-geforce-rtx-5090--vulkan-2026-09-12.json`（可入库测量数据）。
- V2 GPU-driven Rendering 可行性评估（评估交付，非生产特性）——
  - API 调研 + spike 证据：`docs/rendering/04-gpu-driven-feasibility.md`（能力矩阵：indirect draw 内建默认启用 / 内建管线 GPU frustum+occlusion culling / instance compaction 构建块 pub 但无 phase 注入点 / meshlet feature-gated High-only / 自定义 compute 可用）；
  - spike crate `crates/dev/gpu-driven-spike`（publish=false）：compute 视锥剔除 + 实例压缩原型（raw wgpu 层，golden-harness probe 设备），10k/50k/100k 实例——CPU 剔除线性 0.21→2.08ms vs GPU dispatch ≈0.15ms 平坦（100k 时 ~14×），CPU/GPU 可见计数逐一相等的正确性门；原始数据 `target/tmp/gpu-driven-spike/spike_results.json`；
  - 结论：不做自定义 GPU-driven 改造（内建已覆盖；实例规模距瓶颈两个量级）；Portable 管线零触碰；重访条件写入文档。
- 三层对比 loop-closure（dev-only，reference-comparison）——
  - production 路径接线侧车声明纹理（export-sidecar-v1.1 + golden provenance 的 Tenacity 本地路径，env-gated `TENACITY_ROOT`）：Body/Hair ← ILM lightmap + shadow ramp（Genshin 寻址）、Head ← face lightmap（SDF softness 0.001 取侧车 ground truth）；`Use Specular Ramp=0` 不接线；metal_map **不接线**（上游 Metallic 块公式与我方 matcap 语义不同源，实测绑定反降 PSNR——记录在案的语义缺口）；face_shadow 无生产槽位（wiring gap 记录）；
  - 结构性发现：上游 diffuse 项不乘 key light 能量（能量由 palette lit 携带）；对比域 palette lit 设为 fixture 太阳能量色（去掉 1/π，能量只进 soft 层），shadow 色取侧车 Daytime Shadow Color；
  - 结果：production_vs_upstream **PSNR 16.9 → 21.0dB**；判据不放宽（仍 REPORT）；dated 记录见 docs/reference/08。
- V2-e：GPU Instance Database first cut（provisional pending ADR-0004；**scope 诚实声明**：StorageTable 传输 + 双保真契约落地，完整 bindless 生产路径不在本切片）——
  - core：`StylizedInstanceGpuRecord`（全精度实例记录 schema：face_threshold f32 / face_side / outline_scale f32 / profile_index u32 / palette_index / quality_flags / light_tint；新 schema 命名空间 `bevy_stylized.instance/v1`；**两档保真契约**写入类型文档：MeshTag = 粗粒度子集（u8 量化阈值/u7 outline/u8 profile），表 = 全字段全精度）；`StylizedFallbackStep::InstanceTableToMeshTag`（additive 枚举变体，登记通道回退语义）；
  - render：`instance_table` 模块——`StylizedInstanceTableEntry`（记录组件）+ `StylizedInstanceSlot`（槽位）+ `StylizedInstanceTable` 资源（`ShaderBuffer` 数据资产，48B/槽位）+ `sync_instance_table`（懒初始化、确定性槽位分配、脏时全量重写、容量倍增、**无 per-frame 写**）；`sync_instance_tags` 双布局打包（带槽位 → 24-bit 索引 + flags；否则位打包）；`select_transport` 落地选择规则：**tier ≥ Standard ⇒ StorageTable**（Standard 的 tier 需求表已要求 storage buffer 特性，解析到该档即蕴含设备能力），否则 MeshTagBits；`StylizedCharacterTableMaterial`（与 MeshTag 变体共享 shader/参数语义，`#[storage(7, read_only)]` 表 binding + `INSTANCE_STORAGE_TABLE` shader def；**Portable 永不实例化**——三个主材质 layout 保持零 storage binding，测试钉死）；
  - 证据：`tests/instance_table.rs`——全精度 face 阈值梯度 golden（8 球 × 量化网格外阈值，盘内亮度单调梯度 + ≥5 可区分档；table vs tag 同场景捕获对比，每球亮度最大偏差 3/765）；测量对比（100/1000/5000 × 两通道 × 稳态/churn，报告 `target/tmp/instance-db/instance_db_results.json` + summary.md）：**两通道全规模 1 draw（合批完整）；稳态 CPU 成本一致；churn（每帧全量变更）@5000：表 17.8ms vs MeshTag 29.6ms**（与 ADR-0004 spike 方向一致）；选择矩阵单测（Portable→MeshTag / Standard→Table / High→Table / 降档回退）；pack/unpack 全精度 roundtrip + MeshTag 量化损失对照钉死。
- V2-d：Environment NPR first cut（provisional pending ADR-0009；**scope 诚实声明**：完整 stylized IBL / reflection probes 不在本切片，提供调色板驱动的近似）——
  - core：新增 `environment` 模块——`StylizedEnvironment { fog, shadow_tint, zenith_tint }`（View 层数据，附加于 `StylizedViewDescriptor.environment`，additive serde 默认；`StylizedFog` 线性 start/end + 雾色（`is_valid()` 校验，非法 fail-closed 无雾）、`StylizedShadowTint`（乘法 tint，强度 0 = 恒等）、`zenith_tint`（平涂天空 = 相机清屏色，渐变天空球推迟）、host 镜像 `stylized_fog_factor`（与 WGSL 逐字一致）；**默认值全恒等**（雾关/强度 0/zenith None）；view schema minor 递增 v1.0 → v1.1（additive 规则，v1.0 payload 缺省解析行为不变）；
  - render：`sync_stylized_view` 新增 `environment_distance_fog` 映射——环境状态经 **Bevy `DistanceFog` uniform 通道**（view bind group 既有 binding 13）到达 shader：`color`=雾色、`falloff=Linear(start,end)`、`directional_light_color` 复用为阴影染色通道（rgb+强度；我方 shader 不消费 Bevy 原生散射语义）；恒等默认不挂组件（无 `DISTANCE_FOG` shader def，零开销零布局变化），非恒等时挂/更新、回恒等时移除；zenith → `Camera.clear_color`（`None` 不触碰）；**零新 binding、零纹理、零材质资产写**；
  - shader：`npr_common.wgsl` 新增 `environment_fog_apply`（线性距离雾）/ `environment_shadow_tint` + `environment_shadow_tint_strength`（乘法阴影染色），全部 `#ifdef DISTANCE_FOG` 门控 + 强度守卫（strength/factor 0 时不做任何算术 → **逐位恒等**）；cel（背光权重加权的阴影 tint + 雾）/ character / hero（palette soft/sharp shadow 色 tint + 雾）三 shader 接线；shader_validation 新增 DISTANCE_FOG 组合覆盖；outline 描边 shader 本切片不挂雾（已记录）；
  - 证据：showcase 新增 `environment-day` / `environment-dusk` / `environment-night` 三用例（同场景：双列递退球 + 地面；昼雾 / 黄昏暖雾+紫天 / 夜蓝雾三套调色板，雾距离梯度与天色清晰可辨）；**既有 golden 零回归**（golden_cel/character/odette/outline_advanced/instance_batching/morph_skinning 全部对 V2-d 前基线 matched，证明默认恒等）；`interim_budget` 全绿（无新 binding）；core 3 个环境单测 + render 3 个映射/同步单测。
- V2-b：NPR LOD（provisional pending ADR-0018 的 LOD 落地条款；不变量：只削减 tier/feature/几何，永不降 Surface Class）——
  - core：新增 `lod` 模块——`StylizedLodThresholds`（mid/far/very_far 起始距离，默认 10/25/50，非单调 fail-closed 落 near 带）、`StylizedLodBandPolicy`（tier 上限 + feature 削减列表 + 可选简化 variant）、`StylizedLodPolicy`（4 带默认政策：near 不动 / mid cap Standard / far cap Portable（= portable variant 全链）/ very_far 追加 face 硬 step + hair 高光关闭）、`resolve_lod_band`/`resolve_lod`（纯函数确定性分带）；`StylizedProfile::lod_policy()` hook（默认 `None`，不破坏现有 profile 实现）；
  - render：新增 `lod` 模块——`StylizedLodPolicyResource`（默认政策资源）、`StylizedLodMeshes`（band→mesh 映射组件，声明映射才换 `Mesh3d`，近距自动恢复 base）、`sync_lod`（PostUpdate，先于 `sync_instance_tags`）：按最近相机距离分带 → 有效 tier = min(band tier_cap, `StylizedTierOverride`) → guard-write `StylizedInstanceData.flags` bit 0..=2（MeshTag 位，**零 per-frame 材质资产变异**）；`StylizedTierOverride` 自 facade 移入 render（单一真源；facade 经 re-export 保持 `bevy_stylized::StylizedTierOverride` 公共路径不变；facade `dynamic_quality` 模块随之仅在 `render` feature 下编译）；hero shader 逐实例消费 flags（matcap/spec ramp/packed normal 关闭、face SDF 硬 step、hair 高光关闭），与材质级 `HERO_FLAG_PORTABLE_FALLBACK` 并存；
  - 证据：render 单测 6 个（分带确定性、portable cap 蕴含全链、override min 语义、非 LOD 位保留、几何换 mesh + 近距恢复、零材质变异回归守卫）、facade 端到端测试 `governor_override_reaches_per_instance_lod`（合成帧时 → governor 降档 → override → near 实例 flags 置位）、showcase 新增 `lod` 用例（同 hero 角色 4 球距离 6/17/32/55 → near/mid/far/very_far 四带梯度 + far 起几何低模，布局分带确定性单测）。
- V2-a Advanced Outline（provisional pending ADR-0008）——
  - `StylizedScreenOutline`（per-camera opt-in）+ `StylizedScreenOutlineMaterial`：screen-space depth/normal 边缘检测 overlay（Transparent3d + Blend，无深度写，屏幕覆盖三角形直出 clip）；
  - `StylizedOutlineMode`（Hull/ScreenSpace/Hybrid）+ `resolve_outline_mode` tier 门控（Standard+ 才允许 screen-space，回落 hull；core 登记 `StylizedFallbackStep::ScreenOutlineToHull`）；
  - 主材质 prepass 能力开启（成本门控在相机 DepthPrepass/NormalPrepass 组件，默认相机零成本；语义矩阵测试已更新契约）；
  - golden 证据 `fixtures/golden-local/outline-advanced/`（hull-only / screen-only / hybrid 三捕获 + 边缘像素语义断言：hull=229 < screen=342 < hybrid=442）；
  - showcase 新增 `outline-advanced` 用例。
- 行为契约核实修复（docs/reference/07 §3 全量核实后的对齐）——
- V2-c：Dynamic Quality（迟滞/冷却/可观测的 tier 调节器）——
  - core：新增 `governor` 模块 `StylizedQualityGovernor`（纯逻辑状态机，零 Bevy App 依赖）：每 tick 消费 `QualitySignals`（帧时/分辨率/场景复杂度/可选热压力与电量），产出 `QualityDecision`（目标 tier + 次级调整建议 `QualityAdjustment` + reason + 信号快照）；EMA 帧时平滑 + 严格迟滞带（升档阈值 < 降档阈值，构造时 fail-closed 校验）+ 升/降独立冷却 + 升档资格计数，单次 transition 只移动一档；transition 历史为固定容量环形缓冲；热路径零分配（reason 为 `&'static str`，历史构造时预分配）；11 个单元测试（阈值穿越、迟滞带、冷却窗口、LCG 抖动抗振荡、快慢帧交替单调性、热/电强制降档、复杂度预算、环形历史、确定性双跑）；
  - facade：新增 `StylizedDynamicQualityPlugin`（opt-in，默认关闭；未装 FrameTimeDiagnosticsPlugin 时自动补装）+ `StylizedDynamicQuality` 资源 + `dynamic_quality_system`（PostUpdate）：FrameTimeDiagnostics 帧时（或 `gpu_frame_time_override_ms` 注入点）喂入 governor，tier 变更写入 `StylizedTierOverride` 并发出 `StylizedQualityDecisionEvent`（buffered message）；Stage B 解析档经 `set_ceiling` 同步为升档上限；决策到实际渲染 variant 切换的落地属于 V2-b（LOD）工作；
  - 5 个 facade 集成测试（`tests/dynamic_quality.rs`）：opt-in 默认关闭、合成喂入降档/回升全链路 + 事件、Stage B Portable 兜底钳制上限、disabled 冻结、真实 DiagnosticsStore 喂入。

- V2-f：Asset Compiler 骨架——`bevy-stylized compile [--json] [--out DIR] <FILE.glb|FILE.gltf>...`（crates/bevy-stylized-tools，默认输出 `<input-stem>.compiled/`）——
  - compile 蕴含 validate（fail-closed：契约违规 exit 1 且不产出任何文件；先规划后落盘，无部分写入）；
  - **Runtime Material**：每个 glTF material 一份 `StylizedMaterialDescriptor` JSON（`materials/<name>.json`，schema 固定 `bevy_stylized.material/v1` v1.0）；extras/sidecar 双通道经 `import_material` 合并，冲突升级为 error（ADR-0020 未定优先级，拒绝静默取舍）；无元数据的 material 产出默认 descriptor；
  - **Packed Maps**：ILM 打包——extras `bevy_stylized.compile/v1` 键（或 sidecar 顶层 `compile` 键）声明四张单通道 PNG 源，合成一张 RGBA8 PNG（R=metal / G=AO / B=specular-suppression / A=material-ID，上游已核实布局 docs/reference/07 §3.2），以 `ContinuousMask` 语义引用写回 descriptor；该 compile payload 为 provisional schema，major 不符 fail-closed；
  - **纹理引用改写**：descriptor 已声明纹理与 glTF baseColorTexture（外部 URI 复制 / 嵌入 blob 导出）统一落 `textures/`，uri 改写为相对输出目录；目标名冲突显式报错；
  - `--json` 报告含每个产物的 kind/路径/字节数/SHA-256（与 fixtures provenance 约定一致）；exit code 与 validate/inspect 一致（0/1/2）；
  - 新增 workspace 依赖 `image`（仅 png feature：ILM 打包的 PNG 解码/编码，gltf crate 不解码图片）与 `sha2`（报告哈希）；round-trip 测试：fixture GLB descriptor 经 core serde 路径回读 + 合成四通道 ILM 源逐像素断言。

- 行为契约核实修复（docs/reference/07 §3 全量核实后的对齐）——
  - ILM 通道改上游布局：R=metal 遮罩（>0.9）/G=AO/B=高光抑制（1.03−B）/A=材质分带 ID（分带阈值 0.2/0.4/0.6/0.8）；cel/character/hero 三个 shader 同步；emission mask 改走 base color alpha（上游 alpha switch 语义，provisional）；
  - sharp/soft 双档串联相乘光照（palette 四色为材质静态 authored 参数，上游 `Global Material Properties` 对应）；
  - face SDF 改 head-basis 公式（headright = normalize(cross(forward, up))，forward+up 材质参数；阈值/侧选择仍走 MeshTag 实例通道）；
  - Genshin-profile ramp 寻址（hero 的 `with_genshin_ramp`：X 公式 + ILM.A 分带 + 材质静态 day/night 行带；通用 MeshTag 行寻址保留）；
  - post 的 display-transform hook 换上游已核实有理曲线 x(1.36x+0.047)/(x(0.93x+0.56)+0.14)；
  - 新增 alpha mask GPU 证据测试（quad 左红右 discard）；golden 重捕获（character-portable / showcase）并逐张目检；三层对比重构后 production_vs_upstream PSNR 21.1dB（残余误差=cast shadow 结构性缺失 + authored palette 未入导出桥，如实记录于 docs/reference/08）。
- It-10：冻结清扫（可由 agent 关闭的部分）——
  - 渲染语义补全：alpha/prepass/shadow 契约测试（Blend 类型层不可达 + prepass/shadow 显式关闭钉死）、发布矩阵表（docs/authoring/05 §7）、morph/skinning 渲染测试（`tests/morph_skinning.rs`，蒙皮/morph 形变真实可见）；
  - 形态目标支持补齐到 `npr_common.wgsl` 共享形变路径（此前 shader 静默忽略 morph——冻结前修掉）；
  - reference-only 排除检查（`tests/no_reference_deps.rs`：cargo metadata 断言 facade/生产 crate 的 normal 依赖不含 legacy/dev crate）；
  - CI feature 矩阵扩展至 core/facade 全特性组合 + gltf + tools（本地逐一验证）；
  - `docs/platforms/SUPPORT_MATRIX.md` 兼容性声明（provisional pending ADR-0027）；
  - `docs/release/03-provisional-inventory.md`：全量 provisional 清单（项 → 阻塞 ADR → 位置 → 证据），维护者决策交接用。
- It-9b：CLI + 集成文档 + 自定义 Profile 示例——
  - 新 crate `crates/bevy-stylized-tools`（binary `bevy-stylized`）：`validate`（`_NPR_*` 属性存在性/形态 vs 契约数据、extras/sidecar schema 版本 fail-closed；exit 0/1/2，human/`--json` 双输出）与 `inspect`（属性发现/场景计数/class 启发式，启发式已文档化）；纯 gltf 解析，无需 Bevy App/GPU；7 个集成测试（合成 gltf + 真实 fixture GLB）；
  - 第三方 profile 示例 `crates/bevy-stylized/examples/custom_profile.rs`（可运行）+ 注册契约测试 `tests/custom_profile.rs`（ADR-0025：只经 core 契约，不触 render internals）；
  - `docs/authoring/05-integration-guide.md`（≤10 行接入 / glTF 路径 / CLI / fallback 行为 / 自定义 profile / showcase 入口）。
- It-9a：Showcase 套件——新 dev crate `crates/dev/showcase`：12 个固定用例（overview / ramp / lighting / face / hair / metal / outline / animation / post / stress / reference / performance），CLI `cargo run -p showcase -- <name|all> [--capture]`（交互开窗 / headless 捕获到 `fixtures/golden-local/showcase/`）；`stress`/`performance` 复用 benchmarks 场景构建器（benchmarks 的 `build_hero_1`/`build_portable_instances` 转 pub）；全部程序化资产；GPU-gated 全量捕获测试断言非黑图；操作手册 `docs/showcase/02-running-the-showcase-suite.md`。
- It-8：Benchmark 基线 + CI 分层——
  - 新 dev crate `crates/dev/benchmarks`：release gate 的 4 个固定场景（`hero_1` / `portable_100` / `portable_500` / `portable_1000`）headless 离屏 1280×720，20 帧 warm-up + 60 帧测量，JSON 输出遵循 docs/performance/targets.md 的 target record（scene/class/tier/backend/分辨率/帧统计/硬件 provenance；GPU 计时未启用并显式为 null）；
  - 本机 baseline：`docs/performance/baselines/linux-vulkan-rtx5090-2026-09-11.json`（V1 只建立本机 NVIDIA baseline，其余平台 Experimental）；
  - CI 分层：`.github/workflows/rust.yml` 标注为 CPU 层；新增 `.github/workflows/gpu.yml`（`runs-on: self-hosted` + `if: false`，维护者注册 runner 后启用）与 `.github/README.md`（两层政策 + 过渡规则：GPU 层失败不阻塞非渲染 PR）。
- It-7：完整 Capability Resolver + 确定性 fallback + 平台矩阵卫生——
  - core：`resolve_with_report`（variant + 人可读 `StylizedFallbackReport`：降档原因/缺失特性/应用步骤）、`STYLIZED_FALLBACK_CHAINS` per-class 降级链数据（Cel 空基线；Character：face SDF 硬 step + hair→rim；Hero：先关 matcap/spec ramp/packed normal，再 Character 步骤）；
  - 快照式 fallback 矩阵测试：3 class × 3 档 × 4 能力模型（webgl2/gles31/webgpu-core/high-end）对显式期望表逐格断言；确定性双跑一致；
  - facade：`StylizedResolvedCapabilities.report` 携带 FallbackReport（Stage B 直出）；
  - 修正 It-3a provisional HIGH_LIMITS 数据 bug（bind groups 8/UBO 16 超 wgpu defaults → 4/12）；
  - disabled-feature 成本测试（ADR-0026 provisional）：无 outline/post 组件时对应系统零工作（无实体产生/无资产写入）；
  - `docs/platforms/SUPPORT_MATRIX.md`：仅 Linux/Vulkan 行升级为本机 RTX 5090 实测证据（golden 捕获 + naga 校验 + 管线创建），Public status = Experimental（single-device）；其余行不变，无证据不声明 Supported。
- It-6：Resource Architecture——
  - whole-pipeline 预算核算：core 新增 `budget` 模块（三个 baseline 限制模型 WebGPU Core / GLES3.1 / WebGL2 作为数据 const；`StylizedPipelineBudget` = View+Mesh+Instance+Material+Profile+Safety 分解）；render 新增 `budget` 模块（view layout 真实反射 + 材质 AsBindGroup 反射 + 按目标模型换算 storage→uniform）；GPU-gated 测试把 Cel/Character/Outline/Hero-fallback @ Portable 钉死在 WebGL2 模型内，Hero 全量如实记录；报告写 `target/tmp/interim-budget/pipeline_budget.json`；
  - UBO packing / sampler dedup 审计测试：每材质恰好 1 个 packed UBO（钉死）；sampler 1:1 纹理槽位现状记录为 dedup 优化机会；
  - ADR-0001 spike（`target/tmp/adr-0001-spike/`）：方案 A（ExtendedMaterial 原型）vs B（shipping direct Material）实测 500 实例——A 首帧 602.9ms / 稳态 4.47ms，B 首帧 1118.6ms / 稳态 4.69ms；C（custom pipeline）集成面与 bindless（ADR-0003）重访阈值写入 summary；结论保持 provisional；
  - 材质双面渲染经 `bind_group_data` 特化键进入全部三个主材质（ADR-0023 矩阵的 cull 维度）。
- It-5：Hero surface + Genshin profile + 生产后处理 + 三层 Golden 对比——
  - `StylizedHeroMaterial`（render crate）：Character + authored ramp（atlas 行经 MeshTag profile_index）+ packed normal（xy/z 重建）+ metal matcap（ILM.B mask）+ spec ramp；`apply_hero_portable_fallback` / `hero_params_for_variant` 定义 Hero@Portable 降级（关 matcap/spec ramp/packed normal）；预算实测 7 纹理/7 sampler/1 UBO（超 Cel interim 子预算，报告于 `target/tmp/interim-budget/`）；
  - 新 crate `bevy-stylized-profile-genshin`（ADR-0019：V1 唯一锁定 family）：实现 core 的 `StylizedProfile` 契约；metadata_map 键常量与 palette 槽位表（仅 docs/reference/07 签名证据支撑的条目，其余标注"待核实"）；facade 经 `profile-genshin` feature 接入（默认关闭，ADR-0024）；
  - 生产后处理 `StylizedPostProcess`（单 pass FullscreenMaterial：vignette + chromatic + display-transform hook；View 层驱动；LDR-safe；单 pass 约束写入文档）；
  - 三层 Golden 对比（`crates/dev/reference-comparison`，dev-only）：fixture 场景经 production(Hero) / legacy engine reference / upstream Blender 三路径渲染，按 docs/reference/08 §3 Tier-2 草案判据（边缘排除带 + 阴影/剪影 IoU，golden-harness 新增 `tier2` 模块）输出报告；
  - 材质双面渲染经 `bind_group_data` 特化键（cull 继承语义不变）。
- It-4：Material/Instance/View 三层分离 + Character surface——
  - Instance 传输（provisional pending ADR-0004）：`StylizedInstanceData` 组件 → MeshTag 位打包（profile_index/face_threshold/face_side/outline_scale/flags），`sync_instance_tags` 仅变更时写入；无 per-frame 材质资产变异（回归测试钉死）；`select_transport` 依 Stage B 解析选择通道（当前恒 MeshTagBits，StorageTable 声明未实现）；
  - View/Frame 路径扩展：`StylizedViewDescriptor` 的 palette[0] → AmbientLight 颜色、day_night_factor → 昼/夜环境光亮度与色温 lerp（显式决策：不改 ramp 行）、exposure → 相机 Exposure，全部经 Bevy view 对象，不写材质；
  - `StylizedCharacterMaterial`：Cel + Face SDF（共享材质 + 逐实例 MeshTag 阈值/侧选择，UV 镜像采单张 SDF）+ Hair 高光（ILM.G mask）+ Rim + Emission（ILM.A）；ILM 通道约定 R=AO（带 readability floor）/G=hair/A=emission；Character@Portable fallback（face→硬 step、hair→rim 补偿）由 `character_params_for_variant` 依 resolver 结果显式应用；
  - ADR-0023 增量：Opaque/Mask × outline × cull 继承的语义矩阵数据驱动测试（provisional）；
  - 共享 shader 模块 `common/npr_common.wgsl`（顶点形变 + 解析 ramp + face 光位投影），cel/character 分编译单元。
- It-3a: production crate 骨架——`bevy-stylized-core`（三轴类型 / Material-Instance-View 描述符 / 能力模型与确定性 `resolve_variant` / schema 版本化 / 依赖边界守卫测试）、`bevy-stylized` facade（`StylizedPlugin`、ADR-0022 两阶段能力管线、`GltfPlugin` 配置 helper）、`bevy-stylized-gltf`（10 个 `_NPR_*` 顶点属性注册、extras/sidecar typed 导入与冲突诊断）。ADR-0028 机制为 provisional（builder 配置 + fail-closed 顺序守卫）。
- It-3b：Cel@Portable 渲染切片——新 crate `bevy-stylized-render`（provisional，pending ADR-0001）：`StylizedCelMaterial`（base color / ramp 纹理或解析 fallback / ILM AO / half-lambert toon / rim / alpha mask）、`StylizedOutlineMaterial` + `StylizedOutline`（inverted hull：裁剪空间法线外扩 + front-cull + 顶点色宽度抑制）、View 层同步（`StylizedViewSettings` → DirectionalLight/Exposure）、ADR-0022 Stage B 消费（fail-closed 硬失败检查）。
- Interim 预算反射测试（材质 AsBindGroup layout 统计，阈值 provisional pending ADR-0002）：Cel 3 纹理/3 sampler/1 UBO，Outline 0/0/1。
- Golden：`fixtures/golden-local/cel-portable/`（解析 ramp / 纹理 ramp + MeshTag 行选择 / hull outline / 100 实例合批图）与 `fixtures/golden-local/odette-cel/`（Odette Cel）；`fixtures/golden-local/character-portable/`（合成 face SDF 三实例 + Odette Character，含对 legacy golden 的 provisional diff 报告）。
- ADR-0004 spike 证据（`target/tmp/adr-0004-spike/`）：MeshTag 与 storage-buffer 表两种 instance 通道在 100/1000 实例下均保持 1 次 draw（自动合批未破坏）；storage 方案对 WebGL2/GLES3.0 不可用的限制已记录。
- Imported Tenacity `bevy_cel` / `bevy_gltf_metadata` (commit `b5b26ac`) as frozen legacy reference crates under `crates/legacy/`.
- Iteration plan `docs/roadmap/02-iteration-plan.md` (v3) with ADR decision queue and iteration → release checklist mapping.
- Engineering baseline (It-0): Rust CI workflow (fmt / clippy / test / feature matrix), `rust-toolchain.toml` pinned to 1.98.0, ADR-0027 interim clause locking Bevy 0.19.x during V1 development.
