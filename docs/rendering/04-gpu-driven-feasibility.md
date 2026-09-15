# GPU-driven Rendering 可行性评估（Bevy 0.19.1 / wgpu 29）

> **Status:** Working evaluation — 证据采集与结论记录；**不产生生产特性**。
> 本文为未来 GPU-driven ADR 的输入（ADR 未写，本文不修改 docs/adr/*）。
> **Iteration:** V2（docs/roadmap/02 §8；roadmap V2 §GPU-driven Rendering）。
> **Machine:** RTX 5090 / Vulkan / headless offscreen（driver 与帧数据见
> `target/tmp/gpu-driven-spike/spike_results.json`）。

## 方法

两条证据线：

1. **API 调研**：直接读 vendored Bevy 0.19.1 源码（`~/.cargo/registry/.../
   bevy_pbr-0.19.1`、`bevy_render-0.19.1`），逐特性判定可用性；
2. **Spike 测量**：`labs/spikes/gpu-driven`（publish=false）——
   compute shader 视锥剔除 + 实例压缩原型，10k/50k/100k 实例，
   GPU 路径 vs CPU 同数学参考，计数相等作为正确性门。

## 能力矩阵（Bevy 0.19.1 实际暴露）

| 特性 | 判定 | 证据（源码引用） |
|---|---|---|
| Indirect draw | **可用且默认启用** | `bevy_render/src/render_phase/draw_state.rs`：`TrackedRenderPass::multi_draw_indirect` / `multi_draw_indirect_count`（pub）；`bevy_render/src/batching/gpu_preprocessing.rs:138-154`：`GpuPreprocessingMode::Culling` "is used by default"（设备有 compute 时）——即既有渲染已走 multi-draw-indirect |
| GPU culling（内建管线） | **部分可用** | `bevy_pbr/src/render/mesh_preprocess.wgsl:8`：内建网格管线的 late preprocess 在 GPU 上做 **frustum + occlusion culling**（occlusion 需 depth pyramid/prepass）。但剔除判据**不可定制**（无公开 hook 注入自定义 cull 逻辑/距离-LOD 判据）；CPU `check_visibility` 仍在跑（`bevy_camera/src/visibility/mod.rs`，`NoFrustumCulling` 可关） |
| Instance compaction | **内建但不可复用扩展** | 压缩是 indirect 批集构建的内部环节；构建块 `write_batched_instance_buffers` / `write_indirect_parameters_buffers` 是 pub（`gpu_preprocessing.rs:2334/2650`），但接入内建 material phase 的批集构建系统在 bevy_pbr crate 内部——**公开 API 无注入点** |
| Meshlets | **存在但 feature-gated + High-only** | `bevy_pbr/Cargo.toml:52`：`meshlet` cargo feature（默认关，本工作区未启用）；`bevy_pbr/src/meshlet/mod.rs:120`：需要 `TEXTURE_INT64_ATOMIC \| TEXTURE_ATOMIC \| SHADER_INT64 \| SUBGROUP \| DEPTH_CLIP_CONTROL \| IMMEDIATES`——只能属 High 档 |
| 自定义 compute（帧内） | **可用** | `bevy_render::render_resource` 全量 re-export wgpu 计算设施（`ComputePipeline`/`BindGroupLayout`/storage buffer/`PollType`）；`RenderDevice`/`RenderQueue` deref 到 wgpu handle；render graph 自定义 node 可挂进帧管线（本 spike 在 raw wgpu 层验证） |
| 自定义 indirect 结果喂内建 phase | **blocked-on-bevy** | `ViewBinnedRenderPhases` 可读（预算/测量用），但写入路径（bin/batch set 构建）在 `bevy_pbr::render::gpu_preprocess` 系统内部，无公开扩展点。要落地需自建平行 render phase + 自定义 Draw（viable-with-work，工作量大且要跟 Bevy 升级） |
| GPU LOD 选择 | **viable-with-work** | 无内建设施；本框架 V2-b 的 LOD 是 CPU 侧（`sync_lod`）。GPU LOD = 自定义 compute 写 per-instance LOD 决议 + 间接路由到不同 mesh variant——依赖上一条的注入缺口 |

## Spike 测量（`target/tmp/gpu-driven-spike/spike_results.json`）

场景：N 个包围球（确定性 LCG 生成，±200 立方体），90° 视锥朝 -Z，
剔除后可见约 1/3。三种路径，每规模 20 次取均值：

| 实例数 | CPU 剔除+建表 | GPU submit（结果留 GPU） | GPU + 计数回读 | CPU/GPU 可见数（必须相等） |
|---|---|---|---|---|
| 10,000 | 0.21 ms | 0.22 ms | 0.18 ms | 3318 / 3318 |
| 50,000 | 1.04 ms | 0.16 ms | 0.17 ms | 16775 / 16775 |
| 100,000 | 2.08 ms | 0.15 ms | 0.17 ms | 33577 / 33577 |

读法：

- **CPU 成本随 N 线性**（0.21→2.08ms），GPU dispatch 基本与 N 无关
  （~0.15ms，submit 开销主导）；100k 时 GPU 快 ~14×。
- 回读（GPU→CPU 往返）在本测量下代价小（4B 计数），但**生产形态不应
  回读**——可见表留在 GPU 直接喂 indirect draw 才是 GPU-driven 的意义；
  submit-only 行是生产相关数字。
- 正确性门：三规模 CPU/GPU 可见计数逐一相等（确定性场景）。
- 参照系：V2-g sweep 的 crowd/n1000 cell 全帧 mean ≈ 8.2ms——CPU 剔除
  在 10⁵ 实例量级才会成为帧内可分辨项；10³ 量级完全无关。

## 逐特性结论

| 特性 | 判定 | 依据 |
|---|---|---|
| Indirect Draw | **viable-now（已享有）** | 内建管线默认 multi-draw-indirect；V2-e 测量（100/1000/5000 全 1 draw）即为证据 |
| GPU Culling | **viable-now（内建）/ 自定义判据 blocked-on-bevy** | 内建 late-pass 已做 frustum+occlusion；自定义判据无注入点 |
| Instance Compaction | **viable-with-work** | 构建块 pub，但需自建 phase；收益仅在 10⁴+ 实例量级可测（spike 数据） |
| GPU LOD | **viable-with-work（依赖上一条）** | spike 证明 compute 剔除+压缩链路可行且快；落地=自定义 phase + 变体路由 |
| Meshlets | **viable-with-work / High-only** | feature-gated，6 个高级 wgpu 特性；Portable 永不涉及；工作区未启用 `meshlet` feature，启用即重编译评估 |

## 建议（喂给未来 ADR）

1. **不做自定义 GPU-driven 改造**：内建管线已经把 indirect draw 与
   GPU culling 给了所有 mesh 路径；本框架的实例规模（showcase/benchmark
   ≤5000，crowd/n1000 全帧 8.2ms）远低于 CPU 剔除成为瓶颈的阈值
   （spike：10⁵ 实例 CPU 剔除才到 2ms）。
2. **Portable 不变量不破**：全部 GPU-driven 增强路径 = High-only；
   spike/未来实现都不得触碰 Portable 材质管线（本评估未改任何生产
   shader/管线）。
3. **触发重访的条件**（任一满足再写 ADR）：实例规模常态化 ≥50k；
   或 CPU 侧 `check_visibility` 在真实场景 profiling 中 >1ms；
   或 Bevy 上游公开 batch-set 注入点（届时自定义判据成本大降）。
4. Meshlet 评估属另一条线（资产管线要过 meshopt；`meshlet_processor`
   feature），与本评估正交。

## 证据文件

- `labs/spikes/gpu-driven/`（spike crate；测试 = 正确性门 + 测量）
- `target/tmp/gpu-driven-spike/spike_results.json`（本机测量原始数据，
  gitignored）
