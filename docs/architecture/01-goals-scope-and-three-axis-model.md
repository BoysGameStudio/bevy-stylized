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

## 动画、表情与角色系统的责任边界

本项目只负责风格化渲染。此边界适用于所有版本，不是延后到 V2/V3 的待办。
动画、表情和物理系统产生状态；本项目消费明确的渲染输入，并验证输出正确性。

| 能力 | 本项目责任 | 外部应用、Bevy 或 DCC 的责任 |
|---|---|---|
| 骨骼动画 | 材质和相关 pass 兼容引擎提供的蒙皮；验证变形后的法线、描边与适用的深度输出 | 动画播放、混合、状态机、重定向、IK、root motion 与骨架制作 |
| 几何表情 | 兼容 morph 权重引起的变形，验证面部着色和描边 | 表情语义、权重生成、眨眼、口型、视线、情绪和表情混合 |
| 面部阴影 | 消费有明确坐标空间的面部方向、SDF 和实例参数；验证随姿态变化的着色 | 计算姿态、选择头部节点、求解眼睛和头部朝向 |
| 材质表情 | 经需求和证据评估后，可提供通用 UV、遮罩或颜色等渲染参数；不得因此破坏实例隔离和材质共享 | 决定何时切换瞳孔、脸红或表情贴图，以及参数曲线和角色专用命名映射 |
| 头发、衣服与特效 | 渲染外部系统给出的几何和材质；只评估其中独立的 NPR 着色需求 | 物理解算、碰撞、弹簧骨、粒子发射与特效编排 |
| Blender 导入与交付 | 为参考图和渲染数据验证保留必要的姿态、skin/morph 及其来源记录 | 制作控制 rig、求解 drivers、烘焙动画和生产角色资产 |

必要的适配层只能读取外部已经求值的状态，转换为渲染参数；必须明确坐标空间、
更新时序和生命周期，不应反向控制动画系统。不得以 Odette 的骨骼名、动画名或
表情枚举定义通用产品 API，也不建立独立的动画运行时或角色控制 crate。

表中的渲染责任是评估和验证边界，不代表全部实现。当前静态 Odette 配方仍然只
验证声明的静态变体；现有 skin/morph fixture 也不证明完整角色动画或表情支持。
头部骨骼驱动的面部基向量、动态权重下的描边同步及跨 pass 一致性，必须逐项
核对实现和实际渲染证据后才能声明支持。

开发测试可以用少量确定性的骨骼姿态和 morph 权重驱动输入；这些驱动仅属于
tests/showcase，不进入产品控制逻辑。优先检查中性姿态、转头、单关节弯曲、单个
面部 morph 和两个实例的不同状态，再按缺陷补充组合与连续帧测试。

新增能力只有同时满足以下条件才进入本项目：它直接解决风格化渲染问题；输入可
由独立外部系统提供；不依赖具体角色或玩法；有明确的视觉正确性及适用性能验证。
不满足时应留在外部项目，必要时提供一个小型接入示例。

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
