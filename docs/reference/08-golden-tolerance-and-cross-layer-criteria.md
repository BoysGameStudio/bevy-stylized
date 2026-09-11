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

## 4. "Validated" 的定义

一个 Engine/Production 实现对 Upstream Golden 判定为 **validated**，当且仅当：

1. 满足 §2 的全部归一化前置条件；
2. 所有 Golden case 通过 §3 的全部判据（或差异逐项列入已知偏差清单并按 docs/reference/04 §变化分类 A–F 归类）；
3. 每个 case 的 `<case>.golden.json` provenance 完整（格式见 docs/reference/04 §Golden Provenance；当前实例含 `reference_toolchain`、`blender`、`gacha_setup`(+commit+digest)、`export_contract`、`profile_schema`、`fixture`(+sha256)、`renderer`、`image_sha256`、`resolution`、`color_management`、`hardware`）；
4. 复现脚本（同层 `reproduce_golden.sh`，跨层 harness 属 It-2 `crates/dev/golden-harness`）在干净进程重跑后仍在 Tier 0 容差内。

## 5. Golden 更新纪律

- Golden 只能在 docs/reference/04 的受控升级流程末尾更新，禁止"测试失败后随手更新基线"。
- 本地重捕获（同机 re-baseline）必须先用 `reproduce_golden.sh` 证明差异超出 Tier 0 阈值且可归因（Blender/驱动/硬件变化），并在 provenance 中更新 `hardware`/`platform` 字段。
- 涉 upstream 的捕获永远存 `fixtures/golden-local/`（gitignored，见 fixtures/README.md 与 docs/reference/05）；自制 fixture 的公开基线归 `fixtures/golden/`（It-2 起用，体积失控时按 It-2 决策点转 Git LFS）。
