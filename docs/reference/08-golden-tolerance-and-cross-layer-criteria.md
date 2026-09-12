# Golden 容差与跨层对比判据（Draft）

> **Status:** Draft（pending ADR-0007，未经维护者确认前不构成测试契约）
> **Authority:** 本文档是 ADR-0007 的草案载体；规范化来源为 docs/reference/04-upstream-contract-upgrade-and-golden.md（Golden Provenance）与 docs/roadmap/02-iteration-plan.md §3 It-1b 交付物 3（跨层对比判据必须显式定义）。
> **机器可读载体：** 同层对比工具 `tools/blender-reference-export/compare_golden.py`，复现入口 `tools/blender-reference-export/reproduce_golden.sh`。

## 1. 容差分层模型

Golden 对比按"比较双方的距离"分三层，容差从严到宽。任何一层放宽都必须有测量证据，不允许拍脑袋放宽。

### Tier 0：同机同工具链复现（same-toolchain reproduction）

定义（It-1b 交付物 4 的"干净环境"具体化）：同一台机器、同一个被锁 Blender 版本（当前 5.2.1 LTS）、全新独立进程、由 generator 脚本重建场景后重渲。

| 指标 | 阈值 | 说明 |
|---|---|---|
| per-pixel max abs delta | ≤ 2/255（≈0.00784） | 允许个别像素的非确定性抖动 |
| per-pixel mean abs delta | ≤ 0.5/255（≈0.00196） | 整体不得有系统性偏移 |
| PSNR | ≥ 48 dB | 地板线，防止"平均小但局部糊" |

目标值是 **bit-identical**。It-1b 实测（RTX 5090 + snap Blender 5.2.1，EEVEE，512×512，两个 case 各重跑两次）：max=0、mean=0、PSNR=∞，全部 bit-identical。阈值只是防线，不是预期。

### Tier 1：同工具链跨 GPU/驱动

同一 Blender 版本、不同 GPU 或驱动版本的复现。**暂无证据**（V1 只有本机 RTX 5090 一台 baseline 机器，见 docs/roadmap/02-iteration-plan.md §5 风险表）。在取得第二台机器的真实测量前，Tier 1 阈值保持 **PENDING**，不得预设；跨机对比在阈值落地前一律人工评审。

### Tier 2：跨层对比（Blender EEVEE vs Bevy Engine）

Upstream Golden（Blender EEVEE）与 Engine Golden（Bevy runtime）的对比。两侧是不同渲染器，像素级一致不是目标；目标是**语义一致 + 数值有界**。

## 2. 跨层归一化规则（Tier 2 前置条件）

比较之前两侧必须归一化到同一输出形态，否则对比无意义：

| 维度 | Blender 侧 | Bevy 侧 |
|---|---|---|
| 色彩管理 | view_transform=Standard、display=sRGB（锁定值，见 docs/reference/reference-toolchain.toml） | tonemapping **关闭**（不得用 AgX/ACES 等 display transform），输出 display-referred sRGB |
| 输出格式 | 8-bit PNG、sRGB、固定 512×512 | 同左；分辨率必须相等，禁止重采样对齐 |
| AA | EEVEE 内置 AA（场景默认值锁定于 fixture） | 关闭 TAA/MSAA，或使用与 Reference 对齐的固定 AA 设置并记录在 `<case>.golden.json` |
| 相机/灯光 | fixture 锁定（make_reference_scene.py 常量） | 从同一 fixture/GLB 读取，不得另行手调 |
| 帧状态 | 单帧、无动画、无 motion blur | 同左 |

凡无法满足归一化条件的差异（例如某侧不支持的 shader 特性）不进入像素对比，记入"已知偏差清单"并标注 docs/reference/04 的分类（A–F）。

## 3. Tier 2 判据（草案值，待 It-2 实测校准）

归一化后按以下判据判定：

1. **边缘排除带**：物体轮廓/描边等高频边缘像素（任一通道局部 3×3 梯度 > 32/255 的像素）不参与 per-pixel 上限判定，只参与均值统计——AA 实现差异集中在边缘。
2. **非边缘 per-pixel max abs delta** ≤ 16/255（草案值）。
3. **全图 mean abs delta** ≤ 4/255（草案值）。
4. **PSNR** ≥ 30 dB（草案值）。
5. **语义区域检查**：阴影遮罩（亮度低于场景 median 的像素集合）IoU ≥ 0.95；剪影遮罩（前景/背景分割）IoU ≥ 0.99。语义检查失败即失败，不允许用像素均值掩盖结构性错误。

草案值来源：It-1b 无跨层实现可测（Bevy 侧 Engine Golden 设施属于 It-2），数值为保守起点；It-2 捕获首个 Engine Reference Golden 后必须用真实测量数据回填校准，并在 ADR-0007 正文中记录证据。

> **It-5 实测记录（2026-09-12，RTX 5090，reference_scene.glb，三层对比）**：
> 首批跨层测量显示草案值对真实内容**偏紧**：engine_reference_vs_upstream
> max_nonedge=0.486 / mean=0.136 / PSNR=13.5dB / shadow IoU=0.950 /
> silhouette IoU=0.980；production_vs_engine max_nonedge=0.675 / mean=0.226 /
> PSNR=8.8dB / shadow IoU=0.997 / silhouette IoU=1.000。主要已知偏差：
> 两侧 tone response（Bevy 侧无 Standard view transform 的 roll-off，高能量
> key light 下 production 偏曝）、上游 palette/ramp 语义在 Bevy 侧的近似
> 差距。判据校准（可能含 tone 归一化前置步骤）待 ADR-0007 关闭时按
> docs/reference/04 流程处理；当前数字存
> `fixtures/golden-local/reference-scene/*.tier2.json`。
>
> **It-5 后续（2026-09-12，同日补充）——tone/能量归一化修复**：上述"偏曝"
> 的主因不是 Standard view transform 的 roll-off（Standard 实为纯 sRGB
> 传递，无 roll-off），而是 EEVEE 漫反射的 Lambert `1/π` 能量归一在 Bevy
> NPR 路径中的缺失 + 背景 world 色未对齐。已在**对比域**（
> `crates/dev/reference-comparison`，生产渲染器不变）落地归一化：
> key light 能量 ×1/π + 清屏色=world 色。修复后：
> production_vs_upstream max_nonedge=0.584 / mean=0.0314 / PSNR=22.5dB /
> shadow IoU=0.947 / silhouette IoU=0.996；engine_vs_upstream
> mean=0.126 / PSNR=13.5dB / silhouette IoU=0.993；production_vs_engine
> max_nonedge=0.475 / mean=0.124 / PSNR=13.9dB。草案判据仍未通过
> （未放宽任何阈值），残余差距主要在 palette/ramp 语义近似与无阴影投射
> （已知偏差，A 类）。
>
> **It-5 契约核实修复（2026-09-12 同日，doc 07 §3 全量核实后）**：
> 生产渲染器按已核实数学重构——ILM 通道改上游布局（R=metal/G=AO/B=高光
> 抑制/A=材质分带）、sharp/soft 双档串联相乘光照、head-basis face SDF、
> Genshin ramp 寻址公式、上游 tonemap 有理曲线入 post hook。重构后
> production_vs_upstream：max_nonedge=1.000 / mean=0.0385 / PSNR=21.1dB /
> shadow IoU=0.939 / silhouette IoU=0.996（production_vs_engine：
> max=0.541 / mean=0.157 / PSNR=11.9dB；engine_vs_upstream 不变）。
> 残余误差集中于：① 地面投影区域（V1 无 cast shadow，结构性，A 类）；
> ② 下颌阴影色调（上游 face 路径的粉色阴影色是 authored 数据，导出桥
> 尚未把 palette/ramp 元数据带入 fixture——属 docs/reference/06 的后续
> 工作）；③ 发卡等 per-material palette 区分（同上）。对比域 palette 为
> 标定近似值（provisional，标注于 reference-comparison 源码）。
> 结论：结构正确性优先于像素贴近度；草案判据不放宽。
>
> **Gap 闭环（2026-09-12，同日第三轮）——palette ground truth + ILM 贴图路径**：
> ① 导出桥 sidecar 升级为 `export-sidecar-v1.1`，新增
> `bevy_stylized.genshin.ground_truth`（main 角色）：全局 palette 的存放位置
> 已核实为 `Global Material Properties` 内的 ShaderNodeAttribute 节点
> （Lighting Panel rig / wizard 运行时注入；picker 默认与 interface 默认均为
> 白色），材质级 ground truth（Daytime/Nighttime Shadow Color (0.9,0.7,0.75)、
> 5 色 outline 等 64 项/材质实例）已可提取；`crates/dev/reference-comparison`
> 已改为消费 sidecar palette（白色四色 + Shadow Position=0.55），标定近似值
> （(3.45,3.3,3.1) 等）废弃。② upstream golden 重基线：`gacha_upstream` 接入
> 真实 ILM/face-SDF/ramp 贴图（Tenacity odette-npr 本地 sidecar，永不入库；
> 通道按 doc 07 §3.2 已核实布局接线），fixture 升 `reference_scene_v2`
> （发卡补 UV 层）。重基线后三层对比：production_vs_upstream
> max_nonedge=0.631 / mean=0.0772 / PSNR=16.9dB / shadow IoU=0.927 /
> silhouette IoU=0.996；engine_reference_vs_upstream max=0.475 / mean=0.112 /
> PSNR=14.2dB；production_vs_engine max=0.224 / mean=0.0443 / PSNR=22.8dB。
> PSNR 较上一轮（21.1dB）下降是**预期且诚实**的：upstream golden 现在携带
> ILM 纹理内容（AO/metal 区块），production 路径尚未消费该数据——残余误差
> 从"palette 近似"转为"ILM 数据路径缺失"（已知偏差，待 `_NPR_*`/ILM 导入
> 路径落地后回收）。production_vs_engine 显著收敛（11.9→22.8dB）则印证
> palette 标定误差已被 ground truth 消除。判据不放宽，仍为 REPORT。
>
> **工具发现（同日）**：Blender PNG 输出**文件字节跨进程不稳定**（内嵌时间
> 戳类元数据），像素内容则逐位一致（compare_golden max=0.0）。Golden
> provenance 与 `[golden].revision` 因此改用**像素内容哈希**
> （`image_pixels_sha256`，见各 `<case>.golden.json` 与
> docs/reference/reference-toolchain.toml）；文件字节哈希仅作辅助。
>
> **Loop-closure（2026-09-12，同日第四轮）——production 消费 ILM/face 纹理
> + 能量归一**：① `reference-comparison` 的 production 路径按侧车声明接线
> 纹理（Tenacity 本地 odette-npr，env-gated 同 golden_odette）：Body/Hair ←
> ILM lightmap + shadow ramp（Genshin 寻址，Shadow Position=0.55 /
> width=1.0），Head ← face lightmap（SDF，softness 取侧车 ground truth
> 0.001）；`Use Specular Ramp=0`（不接线）。② **结构性发现**：已核实上游
> 光照结构中 diffuse 项不乘 key light 能量——**能量由 palette lit 色携带**；
> golden 实际使用的是 rig 注入值（未入任何导出物），接口默认全白会让
> production 整体欠曝。对比域因此把 palette lit 设为 fixture 太阳能量色
> （(1,0.956,0.898)×3.0，与 SUN_LINEAR 同源、去掉 1/π——半兰伯特路径无
> Lambert 归一；能量只进 soft 层，sharp lit 保持白以避免串联双乘过曝），
> shadow 色取侧车材质级 ground truth（Daytime Shadow Color (0.9,0.7,0.75)）。
> ③ 结果：production_vs_upstream **PSNR 16.9→21.0dB**（mean 0.0772→0.0379）；
> shadow IoU 0.927→0.896（亮侧能量上移改变了阴影掩码边界，如实记录）；
> production_vs_engine 22.8→14.3dB（legacy 中性 ramp 路径未做同等能量归一，
> 预期分离）。④ 残余缺口（已知偏差，不放宽判据）：metal matcap 块公式与
> 上游 Metallic 块语义不同源（dark/light tint 混合 + brightness 缩放），绑定
> metal_map 实测反引入误差（16.7dB < 不绑 16.9dB）→ **不接线**，待上游
> Metallic 块数学核实后落地；face_shadow（blush overlay）生产材质无槽位
> （wiring gap，Face Blush Strength=0 时无影响）；无 cast shadow（结构性，
> A 类）。

## 4. "Validated" 的定义

一个 Engine/Production 实现对 Upstream Golden 判定为 **validated**，当且仅当：

1. 满足 §2 的全部归一化前置条件；
2. 所有 Golden case 通过 §3 的全部判据（或差异逐项列入已知偏差清单并按 docs/reference/04 §变化分类 A–F 归类）；
3. 每个 case 的 `<case>.golden.json` provenance 完整（格式见 docs/reference/04 §Golden Provenance；当前实例含 `reference_toolchain`、`blender`、`gacha_setup`(+commit+digest)、`export_contract`、`profile_schema`、`fixture`(+sha256)、`renderer`、`image_sha256` + `image_pixels_sha256`（像素内容哈希，2026-09-12 起为主）、`resolution`、`color_management`、`hardware`，涉 wizard 模拟时另有 `textures` + `wizard_injection_emulated`）；
4. 复现脚本（同层 `reproduce_golden.sh`，跨层 harness 属 It-2 `crates/dev/golden-harness`）在干净进程重跑后仍在 Tier 0 容差内。

## 5. Golden 更新纪律

- Golden 只能在 docs/reference/04 的受控升级流程末尾更新，禁止"测试失败后随手更新基线"。
- 本地重捕获（同机 re-baseline）必须先用 `reproduce_golden.sh` 证明差异超出 Tier 0 阈值且可归因（Blender/驱动/硬件变化），并在 provenance 中更新 `hardware`/`platform` 字段。
- 涉 upstream 的捕获永远存 `fixtures/golden-local/`（gitignored，见 fixtures/README.md 与 docs/reference/05）；自制 fixture 的公开基线归 `fixtures/golden/`（It-2 起用，体积失控时按 It-2 决策点转 Git LFS）。
