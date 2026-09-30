# Courtyard 捕获基线验收记录

> 本文件为 dated 验收记录的追加式日志：2026-09-28 条目记录 macos-metal
> overlay 验收（当时 canonical 缺席 = BLOCKED）；2026-09-29 条目记录
> canonical（linux-vulkan 参考宿主）基线捕获与验收。旧条目不改写。

## 2026-09-28 条目：macos-metal overlay 验收

> **日期**：2026-09-28 ｜ **阶段**：docs/showcase/05 §12 阶段 3（基线捕获与验收）
> **宿主**：macOS + Apple M4 Pro（Metal），单进程串行
> **机制**：`showcase -- courtyard --capture-spec <id|all>`（apps/showcase/src/capture_spec.rs）；
> 基线写出走 `GOLDEN_REBASELINE=1` + `resolve_capture_paths` 平台 overlay 通道
> （`fixtures/golden/showcase/macos-metal/`）。

## Canonical 缺席声明（BLOCKED，非 PASS）

canonical 基线（`fixtures/golden/showcase/` 顶层，Linux/Vulkan 参考宿主）**未建**：
本机是 macos-metal，按仓库合同只有参考宿主可以写 canonical。庭院 10 条捕获的
canonical 验收因此整体 **BLOCKED**——本记录证明的是 macos-metal overlay 候选的
存在性与同机判定闭环，不构成 canonical 验收。跨设备像素一致性不在获准比较域。

## 判定闭环证据

每条捕获：写/读基线 → **同机重捕**（新 App 实例）→ 判定。strict 捕获同机
重捕逐字节一致（max_delta=0）；loose 捕获走 non-black + luma/chroma 包络；
ROI 走区域双层门限。负面对照已验证：机位变更后未重基线的三条捕获
（trio/lod/flags）被判 FAIL（max_delta 105–211），修复后重基线转 PASS——
判定链路非空转。

## 首轮锁定阈值

- ROI（trio/p1/p4/lamps）：`max_channel_delta = 2`、`max_fail_ratio = 0.005`（首锁值，
  复核时点 = 本文档日期）；
- strict 整图：既有 `Tolerance::default()`（max_channel_delta 4 / mean 0.5 / PSNR 40dB）；
- loose 整图包络（由基线统计派生）：

| 捕获 | mean_luma | stddev | chroma | 包络 [luma_lo, luma_hi] / min_stddev / max_chroma |
|---|---|---|---|---|
| courtyard-day | 211.7 | 29.1 | 27.4 | [127.0, 306.4] / 17.5 / 61.1 |
| courtyard-dusk | 188.2 | 35.8 | 40.8 | [112.9, 273.5] / 21.5 / 81.2 |
| courtyard-night | 135.4 | 65.1 | 24.7 | [81.2, 199.6] / 39.1 / 57.1 |
| courtyard-stress | 135.4 | 65.1 | 25.1 | [81.2, 199.6] / 39.1 / 57.7 |

## 逐条捕获

| 捕获 | 机位 | 钉值（非默认项） | tier | regions | 判定 |
|---|---|---|---|---|---|
| courtyard-day | panorama | （全默认 §0） | Portable | trio (158,238,196,96) / p1 (410,253,102,65) / p4 (0,248,103,70) | PASS（loose + 3 ROI 全过） |
| courtyard-dusk | panorama | time_of_day=dusk | Portable | — | PASS（loose） |
| courtyard-night | panorama | time_of_day=night | Portable | lamps (0,219,512,95) | PASS（loose + lamps ROI） |
| courtyard-trio | center-trio (3.5,4.0,4.0)→(0,1.25,−0.2) | — | Portable | — | PASS（strict 逐字节） |
| courtyard-outline-hull | outline | — | Portable | — | PASS（strict 逐字节） |
| courtyard-outline-hybrid | outline | outline_mode=hybrid | **Standard** | — | PASS（strict 逐字节） |
| courtyard-lod | lod-side (70,7,−8)→(−4.6,2.5,−17.3) | — | Portable | — | PASS（strict 逐字节） |
| courtyard-post | panorama | post=on | Portable | — | PASS（strict 逐字节） |
| courtyard-flags | flags (0,2.4,−0.4)→(−2.75,2.0,−4.76) | flags=posed | Portable | — | PASS（strict 逐字节） |
| courtyard-stress | panorama | night + crowd=1000 | Portable | — | PASS（loose + non-black） |

regions 的屏幕矩形由分区 AABB（`scenes::metadata::COURTYARD_REGION_AABBS`）
经捕获相机投影自动计算，未手工标定；courtyard-day 的 ROI 集收敛记录见
`scenario-model/src/capture_spec.rs` 的修正注记（lamps/p5/p2 在 panorama 下
与其他 ROI 投影交叠，其判定由 night/flags/outline-hull 专属捕获承担）。

## 捕获期发现并已修复的缺陷（本阶段副产物）

1. **蒙皮旗面剔除缺陷**：旗面条带的剔除 Aabb 按 bind 姿态（原点）计算，关节
   移到展台后非原点机位（flags）视锥剔除致旗面消失；且 `calculate_bounds`
   的 `AssetChanged<Mesh3d>` 路径会在网格资产事件后覆盖手工 Aabb。修复：
   `builders::courtyard_p5_flags` 为条带插入覆盖旗面实际范围的 Aabb +
   `NoAutoAabb`（bevy_camera visibility 既有合同），回归测试
   `flag_strips_carry_overridden_culling_aabb`。
2. **机位修正**（§7 表注明「取景实施时验证」）：center-trio / lod-side / flags
   三个机位经捕获验证后调整（center-trio 斜 3/4 高位使三排 7 球分离可读；
   lod-side 移到 (70,7,−8) 东侧远观使四球并排；flags 原机位与西移后的
   mid LOD 球冲突，移到 (0,2.4,−0.4)）。

## 已知限制（如实记录）

- 夜景整体曝光偏亮（共享夜景调色板既有行为，05 §5 修正项，留待 canonical
  基线锁定阶段处理）；
- 本记录的全部判定是**同机自比对**（写基线后同机重捕）；跨后端分歧的判定
  链归 browser/android lane 与 canonical 宿主（阶段 4）。

---

## 修正条目（2026-09-28，验收复核后追加）

**outline 机位覆盖错误（复核发现）**：原 `outline` 机位 (4.2,1.8,6.8)→(2.75,1.4,4.76)
只覆盖 P2（东南描边球），P3（西南方块堆）不在画框内——hybrid 捕获的判定目标
（方块堆 hull + 屏幕空间内边缘）不在画面中。复核同时发现初版验收的 hybrid 图
自评有误（把 P2 展台柱的屏幕空间边缘误读为方块堆内棱线）。

修正（现行数据）：

- `outline-west` 新机位（第 6 个 preset）：eye (−4.2, 1.8, 6.8) →
  look (−2.75, 1.4, 4.76)（outline 的镜像，P3 为主体）；`courtyard-outline-hybrid`
  改挂该机位。方案选择记录：单机位覆盖 P2+P3 需 eye 后退至 z≈13.5（45° fov 横向
  半宽约束），球径缩到 ~26px、描边宽度梯度不可读，故选 per-capture 专属机位
  （CaptureSpec 支持）而非公用机位；
- `courtyard-outline-hybrid` 判定模式：strict 整图 → **loose 整图 + p3 区域精确**
  （双层门限）。理由：screen-space 描边在背景灯球/墙沿剪影处有逐运行 2–4 像素
  的边缘翻转（深度次序竞争经屏幕空间边缘检测放大成整像素明暗翻转；crowd=0 对照
  捕获逐字节稳定，确认噪声源为灯阵/背景而非 P3 主体），strict 整图在此类噪声下
  不可验收；判定目标（P3 的 hull+内边缘）归 p3 ROI 精确承担，与设计「个别持续
  noisy 区域不全局放宽」一致。新增 `p3` 分区 AABB：
  min (−3.20, 0.0, 4.31) / max (−2.30, 1.66, 5.21)（含 hull 外扩）；
- 重捕：outline-hybrid 重基线（macos-metal overlay）；outline-hull 未变
  （复判逐字节一致）；最终全量判定 10/10 PASS（hybrid: p3 ROI 逐字节一致 ×3 运行
  稳定）。

---

## Browser lane 首跑（2026-09-28，macOS M4 Pro，Chrome 153 / Dawn）

**结果：FAIL（1/10 PASS + 9/10 FAIL），唯一失败原因已隔离定位。**

依赖与通道（全部验证可用）：

- Chrome 153.0.8010.53（应用束二进制）、Node 24、wasm-bindgen-cli 0.2.126
  （与 Cargo.lock 精确匹配）、wasm32 target、Pillow；
- **S4 browser 资产通道：可用**——reference GLB 经 HTTP asset source 加载
  （角色在场即证明；GLB 零纹理，无外部贴图依赖）；
- **`?spec=<capture_id>` 合同端到端可用**——10/10 捕获在浏览器端执行并
  发布完整 RGBA 载荷；native 侧 `STYLIZED_BROWSER_SPEC` 同路径工作。

判定（两腿结构：browser vs 同机 native 重捕 + native vs macos-metal
overlay 基线）：

| 捕获 | browser vs native | native vs baseline | 结论 |
|---|---|---|---|
| courtyard-day | FAIL（角色区域） | PASS（loose+rois） | FAIL |
| courtyard-dusk | FAIL（角色区域） | PASS（strict） | FAIL |
| courtyard-night | FAIL（角色区域） | PASS（loose+rois） | FAIL |
| courtyard-trio | FAIL（角色区域） | PASS（strict） | FAIL |
| courtyard-outline-hull | FAIL（角色区域） | PASS（strict） | FAIL |
| courtyard-outline-hybrid | FAIL | PASS（loose+rois） | FAIL |
| courtyard-lod | FAIL（角色区域） | PASS（strict） | FAIL |
| courtyard-post | FAIL（角色区域） | PASS（strict） | FAIL |
| courtyard-flags | **exact** | PASS（strict） | **PASS** |
| courtyard-stress | FAIL（角色区域） | PASS（strict） | FAIL |

**唯一失败原因（已隔离）**：reference GLB 角色在浏览器（Dawn/Metal）
下渲染为纯黑（0,0,0），同机 native（wgpu/Metal）渲染为正常棕色；差异
严格限于角色区域（day bbox (195,206)-(276,264)，该区域外逐字节一致）。
诊断链：fixture 材质为 vanilla `StandardMaterial`（pbrMetallicRoughness
baseColorFactor 棕系、无 extras、GLB 零纹理、侧车无 materials 映射），
由 stylized view 层注入的 `DirectionalLight` 照明——**不经过** NPR 材质
管线（showcase/courtyard 捕获路径从未挂 `StylizedAssetIntegrationPlugin`，
实测 std=4/character=105/adoption=0）；同一 vanilla PBR 在 wgpu/Metal 正常、
在 Dawn/Metal 纯黑。程序化 NPR 内容（Cel/Character/Hero、蒙皮旗面、
灯阵 emission）在两端逐字节一致，证明 stylized 渲染管线本身不受此问题
影响。该分歧属上游 Bevy PBR 在 Dawn/Metal 的 host 限定问题，需要
**上游级调查**（环境光/光照路径），或内容级决策（角色台改挂 NPR 材质——
将改变全部 10 条基线的角色像素，需要独立的重基线决策）。

**本阶段修复（lane 工具）**：native-vs-baseline 判定从单一 golden 容差
改为 sidecar regions 驱动的双模式（strict 整图容差 / loose 包络 + ROI
双层门限，`tools/checks/browser.py`、`android_catalog.py`）——修正了
hybrid 等 loose+ROI 捕获被 strict 整图误判的问题；修复后 9/10 nvc 腿
全过。**lane 变量遮蔽 bug**：`captures` 目录变量遮蔽捕获 id 列表已修。
**wasm inline_js**：`current_spec` 导出与 `?spec=` 读取补齐（阶段 4
遗漏）。

browser 列 coverage 维持 **Pending**（无全绿执行证据；上表如实记录
当前真实状态与隔离原因）。

---

## 角色台 NPR 化与 browser lane 复跑（2026-09-28）

**决策**：角色台从 vanilla StandardMaterial 改挂 NPR 材质（docs/showcase/05
§4「角色特性在角色本体上的真实 NPR 效果展示」；同时是 browser lane 转绿
的必要条件——vanilla PBR 在 Dawn/Metal 下渲染纯黑）。

**实现**（adoption 合同的标准两半）：

- 构建器（`labs/scenes/src/builders.rs::courtyard_reference_stage`）：
  `WorldAssetRoot` 同时挂 `StylizedMaterialSet`——四个 FixtureMat* 材质
  按名映射到 Character class 描述符（`base_color_factor` 取 GLB 内联
  baseColorFactor：Body (0.72,0.42,0.30) / Head (0.86,0.66,0.56) /
  Hair (0.32,0.20,0.11) / Ground (0.18,0.20,0.24)，double_sided 按 GLB
  材质声明）；纹理表为空（fixture 零纹理）；
- 宿主（场景↔宿主边界：adoption 系统归宿主注册，材质集数据归构建器）：
  `apps/showcase`（headless 捕获与 interactive）、`apps/browser-showcase`、
  `labs/scene-runtime` 的四个 GPU 证据 App 均加挂
  `StylizedAssetIntegrationPlugin`（android-smoke 原有，未动）；

**重捕与判定**：10 条捕获全部重基线（macos-metal overlay，角色入镜全部
捕获），同机重捕判定 10/10 PASS（strict 逐字节；loose 包络 + ROI 全过）。
亲眼看图复核：角色现为 NPR cel 着色可读（深发色/肤色底 + 明暗分带），
不再是 PBR 棕块，更不是黑块；trio/flags 等特写机位复核通过。

**browser lane 复跑**（Chrome 153 Dawn vs 同机 wgpu/Metal）：

| 捕获 | 结果 |
|---|---|
| day / dusk / night / trio / outline-hull / lod / post / flags / stress | **PASS（browser vs native exact）** |
| outline-hybrid | FAIL：browser vs native 差 2488 像素（全部 edge-confined，max 216）——Dawn(Tint) 与 wgpu/Metal(naga) 的 screen-space 描边边缘系统性分歧（同 2026-09-23 记录的 Metal screen-outline 平台差异一类）；native vs baseline loose+rois 过。 |

lane 终态：9/10 PASS + 1 个已记录的 host 限定差异（SUPPORT_MATRIX
2026-09-28 条目；放宽判定属 owner 决策，未做）。

**遗留（如实记录）**：(1) vanilla PBR 在 Dawn/Metal 纯黑的上游差异未做
上游调查（SUPPORT_MATRIX 已记）；(2) outline-hybrid 的浏览器侧
screen-outline 判定放宽与否是 owner 决策点；(3) hybrid 的 screen-outline
本体在 native 端判定不变（loose+p3 ROI，既有合同）。

---

## Android catalog lane 首跑（2026-09-28，Tenacity_API35 模拟器 -gpu host，MoltenVK→M4 Pro Metal）

**结果：PASS（10/10，两轮：播种 + 判定闭环）。**

- **S4 打包**：reference-scene GLB 经 `STYLIZED_ANDROID_CATALOG_DIR` →
  gradle `sourceSets.main.assets.srcDirs` 打进 APK assets（既有机制，验证
  端到端可用：catalog 模式加载 courtyard 时资产在场）；
- **捕获规格**：catalog-scene.txt 语义（捕获 ID）端到端工作，10/10 捕获
  在设备端执行并报告；outline-hybrid 设备端解析 `resolved_tier = Standard`；
- **驻留模式**（§11.5 地基）：catalog 捕获报告写盘后应用不再退出
  （`report_and_exit` → `report_and_stay`，logcat 标记
  `ANDROID_RESIDENT alive`）；触摸事件经既有 input 通道到达共享层
  （`ANDROID_RESIDENT touch` 日志；手势分解/参数面板属交互 V3 后续）；
  per-capture 进程隔离仍由 runner force-stop 承担，lane 语义不变；
- **android-vulkan overlay 播种**（GOLDEN_REBASELINE=1）：基线缺席时
  预检 BLOCKED；显式重基线时设备捕获直接写 overlay 候选 + sidecar
  （regions 从 macos-metal sidecar 携带——分区 AABB×机位投影是平台无关
  的规格数据）；
- **判定闭环**（第二轮，非重基线）：10/10 PASS，9 条 strict 逐字节
  （max_delta=0），outline-hybrid 走 loose+rois（fail_ratio 预算内；
  屏幕空间描边的 host 噪声与 PC 同类）。

角色（NPR 化后）在设备端渲染正确（day 捕获逐字节等于基线）。模拟器证据，
不构成真机声明。

---

## 2026-09-29 条目：canonical 基线捕获与验收（linux-vulkan 参考宿主）

> **日期**：2026-09-29 ｜ **阶段**：docs/showcase/05 §12 阶段 3（canonical 基线）
> **宿主**：Linux + NVIDIA GeForce RTX 5090（Vulkan），驱动 NVIDIA 615.71.09，
> 单进程串行（本机即 docs/reference/08 §1 的 canonical 参考宿主，
> `CANONICAL_GOLDEN_PLATFORM = "linux-vulkan"`）
> **机制**：`GOLDEN_REBASELINE=1 showcase -- courtyard --capture-spec all`；
> `resolve_capture_paths` 在 canonical 宿主上写 `fixtures/golden/showcase/`
> 顶层（canonical），不写 overlay。捕获后同机重捕判定（新 App 实例）。

**canonical 基线自此建立**：庭院 10 条捕获的 canonical 集合
（`fixtures/golden/showcase/*.png` + `*.golden.json`）落成，
2026-09-28 条目的「Canonical 缺席声明（BLOCKED）」由本条目解除——
该声明作为 dated 历史保留不改写。既有 macos-metal / android-vulkan
两套基线留在各自平台子目录，按 `resolve_baseline_paths` 合同降为
overlay（只在各自平台查询时优先；canonical 宿主不查询 overlay）。

### 判定闭环证据（写基线 → 同机重捕 → 判定，10/10 PASS）

| 捕获 | 判定 | 结果 |
|---|---|---|
| courtyard-day | loose + rois[trio,p1,p4] | PASS（luma 211.6 / stddev 29.5 / chroma 27.4；3 ROI max_delta=0 fail_ratio=0） |
| courtyard-dusk | loose | PASS（luma 188.1 / stddev 35.9 / chroma 40.8） |
| courtyard-night | loose + rois[lamps] | PASS（luma 135.1 / stddev 65.2 / chroma 24.7；lamps ROI max_delta=0 fail_ratio=0） |
| courtyard-trio | strict | PASS（逐字节，max_delta=0，psnr=inf） |
| courtyard-outline-hull | strict | PASS（逐字节） |
| courtyard-outline-hybrid | loose + rois[p3] | PASS（luma 197.5 / stddev 54.0 / chroma 23.5；p3 ROI max_delta=0 fail_ratio=0） |
| courtyard-lod | strict | PASS（逐字节） |
| courtyard-post | strict | PASS（逐字节） |
| courtyard-flags | strict | PASS（逐字节） |
| courtyard-stress | loose + non-black | PASS（luma 135.1 / stddev 65.2 / chroma 25.1） |

regions 屏幕矩形由分区 AABB × 捕获机位投影在捕获时自动计算（平台无关
规格数据），与 2026-09-28 锁定值逐项一致：day 的 trio (158,238,196,96) /
p1 (410,253,102,65) / p4 (0,248,103,70)，night 的 lamps (0,219,512,95)，
hybrid 的 p3 (106,185,313,327)；阈值继承首锁值
`max_channel_delta=2 / max_fail_ratio=0.005`，未放宽。

### 亲眼看图验收（逐张复核 canonical PNG）

- day/dusk/night/stress 全景：三对照组、展台环、角色台（NPR cel 着色角色，
  深发色/肤色底可读，非 PBR 棕块/黑块）、灯阵、西侧 LOD 轴全部在场；
  dusk 暖橙雾 + 紫红天顶、night 深蓝雾 + 灯阵 emission 符合 §5 色板；
  stress 灯阵 crowd=1000 密集成带；
- trio：7 球三排分离可读，face 阈值梯度与 hair/metal off/on 对照可辨；
- outline-hull：P2 纵列 hull 宽度 0/2/8 梯度可读；
- outline-hybrid：P3 方块堆 hull + 屏幕空间内棱线清晰（outline-west 机位，
  P3 为画面主体）；
- lod：东侧远观四球并排、档位雾中递退可读；
- post：后处理色差/暗角效果明确可见（独立于 day 的 post=on 基线）；
- flags：双旗 posed 姿态（旋转定姿态）入镜完整，无蒙皮剔除缺失。

### 与 macos-metal overlay 的关系（如实记录）

10 条捕获的 canonical 与 macos-metal overlay 像素均非逐字节一致
（fnv1a64 全部不同）——跨后端/跨设备像素分歧是 overlay 机制的存在理由，
**跨设备像素一致性不在获准比较域**；loose 统计包络两宿主高度接近
（day luma 211.6 vs 211.7、stddev 29.5 vs 29.1 等），内容构图逐项等价
（亲眼看图比对）。canonical 集合是唯一判定基准；overlay 只服务各自平台。

### 已知限制（沿用，未在本条目处理）

- 夜景地面/展台整体曝光偏亮（共享夜景色板既有行为，§5 夜景段末修正项）
  维持现状进入 canonical 基线——是否修正是 owner 决策点；若修正需
  GOLDEN_REBASELINE 重锁全部含夜景的基线（night/stress 及像素含夜景
  调色板的捕获）；
- 夜雾色与 zenith 天幕色系未统一（§5 同一修正项）；
- 本条目全部判定为同机自比对；跨后端分歧的判定链归 browser/android
  lane（各自 overlay 与已记录的平台分歧条目）。

---

## 2026-09-29 条目（晚）：v1 基线集随设计 v2 退役预告（owner 决策）

owner 窗口冒烟后否决「同屏全显」核心概念，庭院设计改版为 **v2 分区激活**
（docs/showcase/05 v2、ADR-0031 修正案）：默认与判定路径每次只显示一个
分区的展品，全景同屏降级为 opt-in 浏览模式，不承担 golden 判定。

本文件上述全部 dated 条目（2026-09-28 macos-metal overlay、2026-09-29
canonical）作为 v1 实现的历史记录**保留不改写**；其证明的基线集
（`fixtures/golden/showcase/` 顶层 canonical + `macos-metal/` +
`android-vulkan/` 两套 overlay）对应的是已退役的 v1 全景捕获矩阵，
**不构成 v2 设计的验收**。v2 的 12 捕获矩阵基线走 V2-3
GOLDEN_REBASELINE 重捕，v1 基线集届时删除，验收另起 dated 条目。
v1 判定模式修正史（day ROI 收敛、outline-west 机位、hybrid 宽松+p3 ROI）
的技术教训由 v2 继承的部分已写入设计文档 §9 v1 注记。

---

## 2026-09-29 条目（v2）：分区激活矩阵 12 捕获 canonical 基线验收

> **日期**：2026-09-29 ｜ **阶段**：docs/showcase/05 §12 V2-3（v2 基线捕获与验收）
> **宿主**：Linux + NVIDIA GeForce RTX 5090（Vulkan），驱动 NVIDIA 615.71.09，
> 单进程串行（canonical 参考宿主）
> **机制**：V2-1/V2-2 实现（zone 参数 + 分区激活 builder + 机位 6→10 +
> 12 捕获规格）后，`GOLDEN_REBASELINE=1 showcase -- courtyard
> --capture-spec all`；v1 基线集（canonical + macos-metal/android-vulkan
> overlay）已按 §12 V2-3 先行删除，本条目的 12 条基线是唯一现行集合。

### 判定闭环证据（写基线 → 同机重捕 → 判定，12/12 PASS）

| 捕获 | zone／机位 | 判定 | 结果 |
|---|---|---|---|
| courtyard-trio | trio／center-trio | strict | PASS（逐字节） |
| courtyard-ramp | p1／ramp | strict | PASS（逐字节） |
| courtyard-outline-hull | p2／outline | strict | PASS（逐字节） |
| courtyard-outline-hybrid | p3／outline-west，**Standard** | **strict** | PASS（逐字节） |
| courtyard-lighting | p4／lighting | strict | PASS（逐字节） |
| courtyard-flags | p5／flags（posed） | strict | PASS（逐字节） |
| courtyard-stage | stage／stage | strict | PASS（逐字节） |
| courtyard-lod | lod／lod-side | strict | PASS（逐字节） |
| courtyard-dusk | p4／lighting，dusk | loose | PASS（luma 188.7 / stddev 34.1 / chroma 42.8） |
| courtyard-night | lamps／lamps，night | loose + rois[lamps] | PASS（luma 125.1 / stddev 65.5 / chroma 28.3；lamps ROI max_delta=0 fail_ratio=0，矩形 (0,253,512,138) 投影自动计算） |
| courtyard-post | trio／center-trio，post=on | strict | PASS（逐字节） |
| courtyard-stress | lamps／lamps，night，crowd=1000 | loose + non-black | PASS（luma 124.4 / stddev 65.1 / chroma 29.2） |

### hybrid 噪声验证（v2 核心验收点）

courtyard-outline-hybrid 在分区激活后的纯净背景（P3 方块堆 + 环境壳，
无灯阵/无关分区）下 **strict 整图逐字节稳定**（捕获-重捕 max_delta=0）——
v1 记录在案的背景灯球/墙沿剪影噪声（逐运行 2–4px 翻转、Dawn/naga
2488px 系统分歧的前提）结构性消除，§9 v2 噪声口径的 strict 目标达成，
**未动用 loose+p3 ROI 回退预案**。v1 的「hybrid 一律宽松+ROI」口径与
「Dawn/naga 判定放宽」议题随之作废（后者在 browser lane v2 复跑时按
纯净背景重新评估，不预设立场）。

### 亲眼看图验收（逐张复核 canonical PNG）

- 分区隔离成立：每条捕获只含聚焦分区展品 + 环境壳（空展台画框在场）；
  v1 全景里的灯阵/角色/LOD 球不再串场（flags 机位曾被 mid LOD 球冲突
  的取景问题随分区隔离结构性消失）；
- 新机位（ramp/lighting/stage/lamps，§7「实施时验证」）经本轮捕获锁定：
  ramp 4 球横排 ramp 行色带可读；lighting 3 球 key light ±Z 对照明暗
  分明（显式光照对照恢复像素级）；stage 角色 NPR cel 分带特写可读；
  lamps 机位夜望灯阵 emission 带清晰；
- dusk（P4 视角暖紫调色板）、post（色差/暗角明确）、stress（千灯成带）
  复核通过；
- 夜景地面/展台偏亮维持现状入基线（owner 决策点，§5 夜景段末；
  night luma 125.1，灯阵视角下比 v1 全景夜景略暗）。

### 已知限制（沿用）

- 本条目全部判定为同机自比对；browser/android 的 v2 证据（overlay
  重播种 + lane 复跑）归各自主 host，产生前 coverage 两列维持 Pending
  （v1 证据对应已退役全景帧，不继承）；
- 夜景曝光修正（owner 决策点）若执行需重锁 night/stress 两条基线。

## 2026-09-30 条目（v3）：展亭模式 12 捕获 canonical 重捕与验收

**背景**：owner 2026-09-30 决策——「在同一个位置根据不同的 preset 展示
不同的内容」，v2 的庭院九分区空间布局改为展亭模式（全部内容以中央
平台为唯一舞台，按 zone 档位轮换装卸；W1–W3 增量装卸机制承载）。
overview 浏览模式、podium 展台环、minimap、场景内点击聚焦随 v3 退役
（同位必穿模/无空间导航意义）。12 条捕获规格 id 与判定口径不变，机位
（每内容一台取景机位）与摆位全部重排，故 canonical 基线整体重捕。

**流程**：布局断言先冻结（scenes metadata 落带/区域包含/机位映射测试
全绿）→ 删旧基线 → `GOLDEN_REBASELINE=1 STYLIZED_REQUIRE_GPU=1` 重捕
12 条 → 不带 rebase 复跑 **12/12 PASS**（strict 9 条逐字节 + night/stress
loose + dusk loose，同 v2 口径）→ 逐张亲眼看图验收。

| 捕获 | 内容/机位 | 判定 | 复核 |
|---|---|---|---|
| courtyard-trio | trio／center-trio | strict | PASS（逐字节）；斜上 hero 机位读三排对照 |
| courtyard-ramp | p1／ramp | strict | PASS（逐字节）；4 球横排 ramp 行色带清晰 |
| courtyard-outline-hull | p2／outline | strict | PASS（逐字节）；宽度 0/2/8 梯度正面可读 |
| courtyard-outline-hybrid | p3／outline-west，**Standard** | strict | PASS（逐字节）；收紧取景后方块堆 hull+内边缘清晰 |
| courtyard-lighting | p4／lighting | strict | PASS（逐字节） |
| courtyard-flags | p5／flags（posed） | strict | PASS（逐字节）；双杆条带上平台 |
| courtyard-stage | stage／stage | strict | PASS（逐字节）；角色立台面基座、全身入镜 |
| courtyard-lod | lod／lod-side | strict | PASS（逐字节）；侧斜视读带——近球平滑/远球低模面化可见 |
| courtyard-dusk | p4／lighting，dusk | loose | PASS |
| courtyard-night | lamps／lamps，night | loose + rois[lamps] | PASS（lamps ROI 投影随新机位自动重算） |
| courtyard-post | trio／center-trio，post=on | strict | PASS（逐字节） |
| courtyard-stress | lamps／lamps，night，crowd=1000 | loose + non-black | PASS |

**亲眼看图复核要点**：展亭隔离成立（每帧只有当前内容 + 画框：地面/
铺装/环墙/平台，空展台已移除）；平台承载全部对照组无溢出（p1 横排
±1.33m < r2.2）；lod 轴微偏西不穿灯柱列（lamps/lod 不同档不共存）。

**已知限制（沿用）**：同机自比对口径不变；browser/android 的 v3 overlay
重播种归各自主 host（本机 BLOCKED 口径不变）；夜景曝光维持现状入
基线（owner 决策点仍未拍板——v3 夜景为灯阵视角，曝光观感随布局变化，
该议题需在 v3 基线上重新评估）。

**v3 重捕漂移记录（如实）**：首轮重捕（12:15）后、gpu lane 复跑时
courtyard-trio 出现确定性 ±1 绿通道带状差异（max_delta=9 / mean 0.0094 /
psnr 68dB，墙面与天空带），位置无几何位移（相机/内容未变）。根因定位：
重捕与复跑之间 scene-runtime 有注释/测试级源码编辑，重编译改变了
inlining 使着色路径产生 ulp 级浮点差——strict 逐字节判定对构建产物敏感
（v2 时代码冻结故未暴露）。处置：12 条从最终树整体重捕，两个独立进程
复跑均 12/12 逐字节 PASS 后冻结。**运维口径**：strict 基线重捕后，
判定验证必须与重捕共用同一构建（中间不要夹任何源码编辑，包括注释）。

**进程上下文敏感性（v3 验收发现，如实记录）**：courtyard-trio 的像素受
「同进程内先跑过哪些捕获」影响（墙面/天空带 ±1 绿通道，max_delta=9/
mean 0.0094/psnr 68dB；几何无位移）。批量（单进程 12 连跑）与隔离
（每规格一进程）两种上下文产生不同结果；其余 11 条两种上下文一致。
二分定位：单个前置规格不触发、前后半区各自不触发、全集触发——累积性
效应，根因未继续深挖（疑似驱动/进程级着色状态累积）。**判定合同以此
钉死：golden 基线的捕获与判定都以 lane 的隔离上下文（每规格独立进程）
为准**；`--capture-spec all` 的批量路径只是开发便利路径，不作判定依据。
trio 基线已按隔离上下文重捕。

## 2026-09-30 条目（v4）：cel 特性补全——5 条新捕获基线验收

**背景**：cel shading 特性面调研（引擎参数族 vs 展台覆盖）后收编五个
高价值缺口为展亭新内容档（05 §4 各节；既有 12 条规格与基线零变动，
lod 档特性降级经查证已在演示，不动）。

| 捕获 | 内容 | 判定 | 复核 |
|---|---|---|---|
| courtyard-rim | rim：off / 白强 / 暖弱 | strict | PASS（逐字节）；边缘光对照可读 |
| courtyard-shadow | shadow：默认 / 硬高阈 / 软低下限 | strict | PASS（逐字节）；硬软阴影边界分明 |
| courtyard-palette | palette：默认灰 / 暖 / 冷阴影 | strict | PASS（逐字节）；三档阴影染色分明 |
| courtyard-metal | metal：平色 / matcap / +三色+spec ramp | strict | PASS（逐字节）；递进可读（首版 matcap 基底对照太弱，改递进后重捕） |
| courtyard-cutout | cutout：棋盘格镂空双卡 | strict | PASS（逐字节）；镂空剪影清晰 |

**流程**：5 条全部按 v3 合同以隔离上下文（每规格独立进程）捕获
（`GOLDEN_REBASELINE=1` + `--exact` 逐条）；gpu lane 复跑 **17/17 PASS**
（strict 15 + night/stress 宽松，同前口径）；逐张亲眼看图验收。
browser/android 的 v4 捕获归各自主 host（本机 BLOCKED 口径不变）。

## 2026-09-30 条目（夜景曝光修正 + lane 覆盖缺口修正）

**夜景曝光修正（owner 拍板，2026-09-30）**：根因 = 夜景只降 ambient、
key light 仍全白（view.rs 合同：illuminance 固定 1.0，光强由颜色携带）。
修正 = `courtyard_night` 的 key_light_color 调暗偏冷（0.22/0.26/0.42），
前景地面 luma 196.6→109.0（46% 日间），灯阵成为画面主光源。courtyard-
night / courtyard-stress 两条基线按隔离上下文重捕并亲验（夜景氛围成立、
千灯带可读）。

**lane 覆盖缺口修正**：scene-runtime 的 14 条 GPU 证据测试
（compiler/driver/journal/reconfiguration_evidence）原不在任何 lane
（gpu lane 机制此前只跑 stylized-integration-tests crate）。suites.json
的 gpu 键扩展支持 `crate::binary` 形态并全部登记；task_runner 身份校验
同步放行。此后这些测试随 gpu lane 强制。

**登记即揪出腐烂**：登记进 lane 后首轮复跑暴露 4 条随 v2/v3/W1 腐烂的证据
测试（此前无 lane 强制无人运行）——compiled_courtyard_recipe 的配方机位
停在 v2 坐标（更新到 v3 center-trio）、deform_strip 的 flags.0 目标在分区
激活下不在场（测试改钉 p5 档，经 CourtyardSettings 注入——zone 是视图
作用域参数，配方 parameters 不支持）、portable_session_upgrades 的 hybrid
预期停在内容重建语义（W1 起为附件 live 换，改 Applied）、unenabled_device
的「Stage A 未请求 Bindless」前提在本机失效（快照默认启用；改为从快照显式
摘除特性，测拒绝合同本身，宿主无关）。四条修复后各自实测绿。
