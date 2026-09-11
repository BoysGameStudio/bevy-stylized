# 目标、定义与非目标

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


## 项目定位

目标不是做一个“好看的 Toon Shader”，而是建设：

```text
Bevy Stylized Rendering Framework / SDK
```

它必须满足：

- 独立 GitHub 仓库维护；
- 可通过 Cargo 被其它 Bevy 项目直接依赖；
- Core 不包含 Tenacity / Odette 专属逻辑；
- 数据驱动、Profile 驱动；
- 支持 glTF Authoring 与程序化 Authoring；
- 支持第三方自定义 NPR Profile；
- 覆盖 D3D12 / Vulkan / Metal / GLES / WebGPU；
- 为主机移植预留清晰 backend 边界；
- 同一份 Authoring Data 可在不同 GPU Tier 下选择不同实现；
- 低能力设备保持正确视觉语义；
- 高端设备充分使用更优 GPU 路径；
- 提供 Reference Renderer、Production Renderer、Showcase、Golden Test、Pipeline Test、Resource Budget Test、Benchmark 和 CLI；
- 长期可扩展为 Editor / DCC / Profile Ecosystem。

---

## “完美”的工程定义

不存在一个实现能在所有 GPU、所有场景、所有分辨率下同时取得绝对最高性能。

因此本文中的“效果与性能最优”定义为：

> **在明确的视觉误差、平台能力、资源预算、帧时间和内存预算约束内，选择经过测量证明的 Pareto-optimal 实现。**

任何 Production Tier 都必须有：

```text
Visual Error Budget
Frame-Time Budget
GPU Resource Budget
Memory Budget
Feature Set
Fallback Contract
```

任何“性能优化”都必须通过 Benchmark 证明。

任何“效果等价”都必须通过 Reference/Golden 证明。

---

## “全平台”的定义与三轴模型

错误定义：

```text
所有平台
→ 同一个 Shader
→ 同一个 BindGroup Layout
→ 同一套 Pass
```

正确设计必须拆成三个正交维度：

```text
Authoring Data
      │
      ▼
StylizedSurfaceClass          内容/风格语义
├─ Cel
├─ Character
└─ Hero
      ×
StylizedExecutionTier         设备/性能预算
├─ Portable
├─ Standard
└─ High
      ×
StylizedValidationMode         验证路径
├─ Production
├─ UpstreamReference
└─ EngineReference
      │
      ▼
Resolved Runtime Variant
```

三个概念不能混用：

- **Surface Class** 回答“这个材质/角色必须表达什么语义”；
- **Execution Tier** 回答“当前设备允许花多少 GPU/内存成本”；
- **Reference Mode** 回答“当前是在验证上游真值，还是执行 Production 实现”。

因此一个 Hero 在低端设备上应优先解析为：

```text
Hero @ Portable
```

而不是简单退化成另一个 Surface Class：

```text
Hero → Character → Portable   // 不再作为默认模型
```

只有 Profile 明确定义了语义允许的 Surface-Class LOD 时，才允许跨 Class 降级。

建议 Execution Tier：

| Tier | 目标 |
|---|---|
| **Portable** | 最低能力路径；不得依赖可选 GPU Feature |
| **Standard** | 主流 iOS/Android Vulkan/普通 PC |
| **High** | 高端 PC、未来主机级 GPU |

Reference 不属于 Execution Tier。Reference 是独立的验证后端/模式。

“全平台”的工程定义是：

> 同样的核心视觉语义拥有明确、确定、可测试的 fallback，并且 Support Claim 与真实 backend/device 验证等级一致。

对于 Web，必须进一步区分 WebGPU 与 WebGL2/GLES3 baseline，不能笼统写成“Web”。

---

## V1 非目标

以下内容明确不进入 V1：

- Mesh Shader；
- GPU Driven Culling；
- Indirect Draw；
- 通用 Shader Graph；
- Marketplace；
- 大型 Editor；
- 完整动态质量系统；
- 完整 Texture Streaming；
- 大规模 Profile Marketplace；
- 通用 DCC Graph Compiler。

这些属于 V2/V3。

V1 的唯一战略目标：

> **建立正确、稳定、可复用、可验证、后续无需推倒重来的 Renderer Architecture。**

---
