# Courtyard 捕获基线验收记录（macos-metal overlay）

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
