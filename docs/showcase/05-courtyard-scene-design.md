# 庭院展台（Courtyard）单场景设计

> **Status:** 设计 v3 已落地（2026-09-30：owner 决策「在同一个位置根据
> 不同的 preset 展示不同的内容」——展亭模式：全部内容以中央平台为唯一
> 舞台，按 zone 档位轮换装卸；W1–W3 增量装卸机制承载。overview 浏览模式、
> 展台环、minimap、场景内点击聚焦随 v3 退役）。v2（分区激活）与 v1
> （同屏全景）的实现与基线是 dated 历史：各阶段落地记录保留在 §12
> 状态列，验收保留在验收记录 dated 条目，均不改写。
> ADR-0031 决策记录：**Accepted（2026-09-30 owner 拍板）**。
>
> **交接状态（2026-09-30，v3 已落地）：** 展亭模式全链完成——内容全部
> 上中央平台（lod 轴向视线递退、lamps 灯环不变）、机位表 9 台（panorama
> 退役）、catalog zone 枚举去 overview、keymap 去 Tab（`previous_zone`
> 记忆机制同退）、导览脚本 9 站收尾回 trio、minimap/pick 模块删除、
> canonical 基线 12/12 重捕验收（逐张亲验，见验收记录 v3 条目）、
> cpu/gpu/docs 三 lane 绿。增量装卸（W1–W3）与 UI（U1–U3）在 v3 下
> 语义不变。v2/v1 为 dated 历史。
> 续作起点（按优先级）：① browser/android v3 overlay 重播种 + lane 复跑
> （macOS 侧，coverage 两列 Pending 待此）；② browser 交互 V2 / Android
> 交互 V3（v1 §12 阶段 6/7，未批准不动工）。
> 待 owner 确认项：交互输入链路的物理确认（窗口已开，←/→ 内容循环、
> ↑/↓ 时刻、G 导览、C 捕获、H 演示、鼠标手势）。夜景曝光已修正
> （2026-09-30，§5 夜景段末）；ADR-0031 已 Accepted（2026-09-30）。

## 0. 决策记录（已拍板）

| 决策点 | 结论 |
|---|---|
| 场景数量 | 1 个合成场景 `courtyard`，退役现有 18 个展示场景 |
| 显示模式（**v2，2026-09-29 owner 拍板**；**v3 修正 2026-09-30**） | **内容档轮换**：每次只展示当前内容档的展品（画框常驻）；v2 的分区激活演进为 v3 展亭模式（同一平台轮换装卸）；overview 同屏浏览随 v3 退役（同位必穿模），浏览全部内容归导览模式（G） |
| 回归结构（**v2 修正**） | 分区激活后每帧单一主体：整图精确判定为主力（行业规范形态），ROI 双层判定机制保留在 harness（夜景灯阵等暗部场景继续使用）；v1 的「全景宽松 + 分区 ROI」随 v1 矩阵退役 |
| 中央展示体 | 判定特写用球（基线纯净）；角色台放 reference GLB 做真实效果展示（混合方案） |
| S4 资产门控 | GLB 打包进所有 lane；v2 起依赖面收窄——只有 stage 分区捕获在资产缺席时 BLOCKED，其余分区捕获不依赖 GLB（v1 的「缺席=整场景 BLOCKED」随全景退役解除） |
| 默认状态 | **v2：zone=trio** + day + hull 描边 + crowd=100 + post off + flags rest |

## 1. 调研依据（2026-09-28 调研，关键来源）

**合成场景不承担回归职责是业界共识**：Unity Graphics Tests（一场景一用例一基线；gallery 类 Material Samples 是教学资产，与测试分离）、Unreal（Content Examples 是展示关卡；截图判定用 local/global 双层误差 + 同机位多基线）、three.js/Bevy（200+ 隔离小 example 即展示即测试）、Skia Gold（每天 50 万+ 小图逐张 triage）、Babylon.js（每测试独立 playground + 每像素阈值 + 失败像素比双层门限）。没有任何主流渲染器把核心回归押在少量合成大场景上。

**内容选型有先例**：UTS2/UTS3 sample 场景与本项目特性一一对应（AngelRing=hair 高光、MatCapMask=metal matcap、LightAndShadows=光照对照、Firefly=夜景自发光）；shaderball 统一几何 + 中性灰环境 + 固定机位是行业标准方法论（USD StandardShaderBall）；昼夜切换用光照参数驱动 + 离散预设（Unreal Sun and Sky、Flat Kit presets）；夜景灯阵 = 自发光材质夜间激活（Unity day/night 教程标准做法）。角色特性在角色本体上演示（UTS 的 AngelRing 在 Unity-chan 头部，不在 shaderball 上）——本设计以「球判定 + GLB 展示」混合方式兼顾基线纯净与该惯例。

**资料缺口（如实声明）**：「many small scenes vs one composed scene」无正面辩论文献，结论从各家做法反推；「环形展台」「三分区景深」作为设计教条无直接来源（电影构图原则可迁移，博物馆仅有脊柱动线的间接依据），本文档将其降级为构图手法而非规范引用；米哈游的 demo 场景组织无公开资料。

## 2. 概念与构图手法

环形展示庭院（v3 展亭化）：中央平台是唯一舞台——角色材质三对照组
（主角）与各特性内容档（细节）在同一位置轮换展示；远景 LOD 轴与实例
灯阵保持空间特性（氛围与规模）。v1/v2 的「周围展台环各演示一个特性」
随 v3 退役（展台环移除，内容全部上平台）。

**内容档轮换（v3，2026-09-30 owner 拍板；v2 分区激活的演进）**：庭院的组织是一个场景，呈现是一个展台——每个视图只展示当前内容档的展品（中央平台同一位置轮换装卸）；画框（大地面、广场铺装、环形矮墙、中央平台、基座）常驻。overview 同屏浏览随 v3 退役（同位必穿模；浏览归导览模式 G）。v2 记录的红利：v1 记录在案的三类跨区污染——hybrid 屏幕空间描边的背景灯球/墙沿噪声、panorama 下 lamps ROI 全域交叠、灯阵雾效对远景判定的穿透（v1 隔离承诺边界，修正史见 §9 v1 注记）——随背景简化结构性消除。

防杂乱手法（v1 为全景构图所立；v2 下第 3 条退为布局理由而非同屏构图约束）：

1. **同几何、变材质**——材质对照统一用球，差异只来自材质（shaderball 方法论）；
2. **统一展台形制**——展台同一尺寸同一中性灰，是画框不是展品；
3. **三区景深**——前景展台环 / 中景中央平台 / 远景 LOD 轴 + 灯阵（电影构图手法迁移）；
4. **色彩纪律**——场景固有色低饱和，饱和度只出现在被演示的材质上。

## 3. 布局

坐标系：Y 向上；展亭机位族在南（+Z）望北（−Z）。单位：米。

**v3（2026-09-30 owner 决策，展亭模式）**：全部展品内容以中央平台为唯一
舞台，按 zone 档位轮换装卸（同一位置，不同 preset 不同内容）；v2 的
展台环与分区摆位退役。布局图只余画框与环境：

```
                 北 (−Z)
       · · · 实例灯阵（北半环 r8.5–11，纵深 ≤38m）· · ·
          [LOD₄ r6.4 d≈62]   [LOD₃ r3.8 d≈36]
              [LOD₂ r1.7 d≈16.6]
                [中央平台·唯一舞台（内容轮换）]
              [LOD₁ r0.75 d≈7.3]（平台西南前景）
               矮墙 r13（南留门）
            hero 机位族（南望北；lamps/lod 各有取景机位）
                 南 (+Z)
```

| 元素 | 位置 | 尺寸 | v3 语义 |
|---|---|---|---|
| 大地面 | 原点中心 | 半径 60 圆盘，深灰，接收阴影 | 常驻（环境壳） |
| 广场铺装 | 原点中心 | 半径 12 圆盘，抬高 0.02，浅灰（描边可读底） | 常驻 |
| 环形矮墙 | 半径 13 | 高 1.2、厚 0.4，四段弧墙，南留门 | 常驻 |
| 中央平台 | 原点 | 半径 2.2、高 0.3 圆台 | 常驻（**唯一舞台**） |
| LOD 轴 | 平台西南起向北递退（微偏西，避开灯柱列） | 四球递退 | **lod 档展品** |
| 实例灯阵 | 北半环 r8.5–11 网格 | 灯柱 + 灯球，密度参数控制 | **lamps 档展品** |

**已验证的相机约束**（v1 panorama 取景证据，随 panorama 退役仅作 dated
历史保留）：golden 捕获 512×512 为 1:1 画幅，垂直 fov 45° 时水平视角同
为 45°；v1 eye=(0, 3.4, 16.0) 全景方案的侧台边缘裕量约 0.5°。v3 起
全景机位退役，本段数值不再构成合同。

## 4. 分区详设

每区标注：内容 → 演示特性 → 回收的旧场景。**v3 语义（2026-09-30）**：
「分区」= 展台内容档位——所有内容的展品部分都生成在**中央平台**上
（lod 轴向视线递退、lamps 灯环除外，二者是空间特性展品）；各小节标题
中的方位坐标是 v1/v2 的 dated 摆位记录（不改写），当前坐标单一源是
`labs/scenes/src/metadata.rs`（AABB 表）与 builder 代码。

### 中央平台：三对照组（球，判定用）

> 对抗性检视修正（S3）：原「三联像」每特性一球会丢失旧场景的像素级状态对照
> （face 三阈值梯度、hair/metal 的 off/on），违反「不删减覆盖」。改为三对照组，
> 7 球三排，全部面朝南，状态与旧场景一一对应。

三排横列（排轴平行 X，从前到后）：每球立于高 0.7 基座（顶面 y=1.0），球心 y=1.32（face 排）/ 1.40（hair、metal 排）。**排距 ±0.9（PC-2 实施修正）**：文档原排距 ±0.6 下中/后排同列球心距 0.6 < 半径和 0.8（几何重叠），以「球心不重叠」断言为准放宽；球心 y 与 x 坐标不变。

| 排 | 组 | 球（x 坐标，z 为排位置） | 状态 | 回收 |
|---|---|---|---|---|
| 前（z=+0.9） | face（Character + face SDF） | −0.75 / 0 / +0.75，r=0.32 | face_threshold = 0.2 / 默认 / 0.8 | face（三阈值梯度） |
| 中（z=0.0） | hair（Character + ILM） | −0.5 / +0.5，r=0.40 | hair_specular off / on | hair（off/on 对照） |
| 后（z=−0.9） | metal（Hero + matcap + 金属 ILM） | −0.5 / +0.5，r=0.40 | matcap off / on | metal（off/on 对照） |

坐标单一源是 `labs/scenes/src/metadata.rs` 的 `COURTYARD_TRIO_BALLS`（含两两无重叠断言）。

三排并排同时回收 overview 的「三 surface class 对照」语义。face 的阈值同时也暴露在参数面（§8），但像素覆盖不依赖参数扫描——参数扫描的逐字节证据在动作层（coverage 已有 H2/H3 记录），golden 只管这三个固定状态的像素。

### P1 · ramp 展台（东，(5.5, 0, 0)）

4 小球（r=0.28）横排：左 1 解析 ramp，右 3 经 `profile_index` 选 3 行纹理 ramp。→ 回收 ramp。

### P2 · 描边 hull 展台（东南，(2.75, 0, 4.76)，近相机）

3 球（r=0.35）纵列，hull 宽度 0 / 2 / 8。近相机位使 2px 与 8px 差异在全景可读。→ 回收 outline。

### P3 · 描边 hybrid 展台（西南，(−2.75, 0, 4.76)）

方块堆（2 cuboid + 1 小球），hull + screen-space 内部边缘。**tier 门控**：会话不足 Standard 时该展台渲染为 hull-only（内容降级显式声明，不静默）；hybrid 只在专属 Standard 捕获中判定。→ 回收 outline-advanced。

### P4 · 光照对照展台（西，(−5.5, 0, 0)）

3 个 Hero 球横排：无覆盖 / key light 锁 +Z / 锁 −Z（`with_effect_light_dir`，已验证存在于 `crates/bevy-stylized-render/src/character.rs:142`，无 tier 门控，Portable 可用）。→ 回收 effects。

### P5 · 旗帜（西北，(−2.75, 0, −4.76)）

两根旗杆（高 2.6）挂双骨骼蒙皮条带旗面；姿态参数 rest / posed（root 旋转 0.5 rad，确定性定姿态）。→ 回收 animation。

### rim · 边缘光对照（V4，2026-09-30 调研收编）

3 球横排（Cel，底色压暗）：rim off / 白 rim 强 / 暖色 rim 弱。→ 收编
`StylizedCelParams::with_rim`（此前只有 golden_cel 测试覆盖，展台无）。

### shadow · 阴影形状对照（V4）

3 球横排（Cel）：默认（0.5/0.1/0.3）/ 硬高阈（0.7/0.02/0.15）/ 软低下限
（0.35/0.3/0.45）。→ 收编 `with_shadow`（展台此前只用默认阴影形状）。

### palette · 四色调色板对照（V4）

3 球横排（Character）：默认灰阴影 / 暖阴影（夕照）/ 冷阴影（蓝紫）。→
收编 `with_palette`（soft/sharp × lit/shadow 串联相乘，此前零展示）。

### metal · 金属三色 + spec ramp（V4）

3 球横排（Hero）：无 matcap 平色 / matcap 银灰 / matcap + 金属三色调
（dark/light/shadow）+ spec ramp 强。→ 收编 `StylizedMetalParams` +
`with_spec_ramp_intensity`（trio 此前只演示 matcap off/on）。

### cutout · alpha 镂空（V4）

双竖卡（Cel + 程序化棋盘格 alpha 纹理 + cutoff 0.5 + double_sided）。→
收编 `with_alpha_cutoff`（此前只有 masked_outline 测试覆盖）。

### P6 · LOD 轴（见 §6）

→ 回收 lod。展台环第 6 座（东北，(2.75, 0, −4.76)）按统一形制建成、暂无展品——「展台是画框不是展品」；LOD 轴的 mid 球从该座与 P1 之间的空档穿出。

### 角色台（北，(0, 0, −8)，资产门控）

加载 reference-scene GLB 角色，面朝南——角色特性在角色本体上的真实效果展示（行业惯例侧）。→ 回收 reference。

**NPR 化（2026-09-28 决策落地）**：角色经 NPR adoption 渲染而非 vanilla `StandardMaterial`——`WorldAssetRoot` 挂 `StylizedMaterialSet`（四个 FixtureMat* 按 glTF 材质名映射到 Character class 描述符，`base_color_factor` 取 GLB 内联 baseColorFactor，double_sided 按 GLB 声明，纹理表为空）；材质集数据归 builder（`labs/scenes/src/builders.rs::courtyard_reference_stage`），adoption 系统（`StylizedAssetIntegrationPlugin`）归各宿主插件装配（showcase headless/interactive、browser-showcase、android-smoke、scene-runtime 证据 App）。动因：vanilla PBR 在 Dawn/Metal 下渲染纯黑（host 限定上游差异，docs/platforms/SUPPORT_MATRIX.md 2026-09-28 条目），且 vanilla 路径本就不满足「真实 NPR 效果展示」意图。labs/reference 的六视角验收合同与 labs/reference 的编译角色管线不受影响（两者是独立合同）。

**S4 决策（2026-09-28 拍板）：GLB 打包进所有 lane。** reference-scene 资产经 HTTP asset source（browser）/ APK assets（android）随包分发（实施前置项，见 §12 阶段 0）。**v2 依赖面收窄（2026-09-29）**：分区激活后只有 stage 分区加载角色——`courtyard-stage` 捕获在资产缺席时 BLOCKED（不做空台降级，遵循「必需资产缺席是 BLOCKED 不是 PASS」）；其余分区捕获不依赖 GLB，v1 的「资产缺席 = 整场景捕获 BLOCKED」窗口期代价随之解除。

**角色台展品参数化（2026-09-29 owner 决策落地）**：新增 `stage_character` 参数（§8）。`fixture`（默认）= 上述合成 reference-scene 角色——golden 判定与证据测试内容**不变**（基线零变动、lane 资产面不变）；`odette` = 真 Genshin 角色（genshin-odette compiled static-v3，`assets/local/genshin-odette/compiled/static-v3`），经宿主注册的 `odette://` 资产源 + `.npr.json` sidecar 资产（`StylizedSidecarAsset`/`StylizedSidecarSource`，integration 插件统一注册，浏览器/APK 同源可加载）走同一 adoption 链路——**交互展示档，不进 golden**（第三方美术基线不可提交：ADR-0017 政策；golden-local 拆分已退役，不复活）。odette 资产缺席 = 加载失败并在 stderr 可见，不做空台降级；browser/android 宿主不注册 odette 源（维持 fixture）。

### 实例灯阵（北半环背景，密度参数）

灯柱 + 发光灯球，密度参数 0 / 100 / 1000。已验证的材质约束（2026-09-28 代码查证）：

- **灯球必须用 Character 材质**——Cel 无 emission 字段（`crates/bevy-stylized-render/src/material.rs:44-55`）；Character 有 `with_emission`（`character.rs:204`）；MeshTag 实例通道与 Character 兼容，1000 实例仍 1 次 draw；
- emission 是 per-material uniform，**灯阵同色**（无 per-instance emission 位段）；逐灯色彩变化不可行（**修正**：Character 材质无 ramp 通道，「profile_index 选 ramp 行」不适用于灯球；受光面暖黄经 base_color_factor 实现）；
- emission 在雾之前叠加，**灯阵纵深 ≤38m**（夜雾 end=40 留余量），超出部分消隐到雾色。

100 盏 = 常态背景氛围；1000 盏 = 压力负载（兼 performance 证据，性能测量走独立 lane 不进 golden）。→ 回收 stress + performance。

### 后处理

view 参数（vignette 0.5 + chromatic 0.004 + display transform），整帧开关，不占空间分区。→ 回收 post。

### 配色表（2026-09-28 美术方向锁定，PC-5 实机迭代）

首版实现把所有演示材质留成默认灰 + 紫蓝 matcap，实机检视判定为「全灰工地 + 噪音球」，本轮按 §2 色彩纪律锁定固有色（linear RGB）。判定语义（ramp 行数、阈值梯度、off/on、描边宽度、LOD 分带）不随配色变化。

| 对象 | base_color (linear) | 备注 |
|---|---|---|
| 外圈大地面 | (0.32, 0.34, 0.33) | 深灰绿收底 |
| 广场铺装 | (0.62, 0.60, 0.56) | 暖白石材 |
| 环形矮墙 | (0.48, 0.46, 0.43) | 暖灰 |
| 展台柱 / 中央平台 / 基座 / 角色台 | (0.70, 0.68, 0.64) | 暖白统一画框 |
| face 排 | (0.85, 0.72, 0.65) | 肤色底 |
| hair 排 | (0.16, 0.14, 0.18) | 深发色 |
| metal 排 / LOD 轴 | 银灰 matcap：中心 (0.90,0.92,0.95) → 边缘 (0.25,0.28,0.34) | courtyard 专用 `matcap_image_silver`；共享 `matcap_image` 逐字节未动（旧 metal/lod 基线不受影响） |
| 旗面 / 旗杆 | (0.75, 0.22, 0.20) / (0.25, 0.25, 0.27) | 砖红旗 |
| 灯球 / 灯柱 | (0.95, 0.80, 0.50) / (0.30, 0.30, 0.32) | 受光面暖黄经 base_color_factor（**修正**：Character 材质无 ramp 通道，§4 灯阵节「profile_index 选行」不适用于灯球）；emission 不变 |
| P1/P2/P4 展台球 | 中性白/浅灰 | 功能演示需要 |

## 5. 光照与时刻参数

沿用现有三套环境 preset 的**色板**（`labs/scenes/src/views.rs`），但雾范围按庭院景深独立调整为庭院专用 preset（`views::courtyard_day/dusk/night`；原 `environment_*` 保留服务旧 18 场景直至阶段 4 退役）：

| 值 | key light | 雾 | zenith | 备注 |
|---|---|---|---|---|
| day（默认） | (0.5, 0.8, 0.6) 白色 | 浅蓝灰 (0.58,0.68,0.84) **28–110** | 淡蓝 | 展台环/对照组不受雾染色；very_far 球半没 |
| dusk | 暖橙 (1.0,0.75,0.55) | 暖橙 (0.85,0.55,0.38) **14–70** | 紫红 | 中景可读、远景渐没 |
| night | 不变，day_night_factor=0 | 深蓝 (0.08,0.1,0.2) **10–48** | **需与雾色统一** | end=48 > 灯阵纵深 ~27m；LOD very_far 没入雾中属预期 |

（雾范围数值为 PC-5 实机捕获迭代锁定：初版直接沿用旧场景的 day 14–80 导致展台环以远全部洗白，2026-09-28 第二轮捕获验收修正。）

已验证的夜景修正项：雾是逐片元效果、不染天空，天空色由 zenith_tint 单独写清屏色（`crates/bevy-stylized-render/src/view.rs:258-260`）；现有 night preset 雾色与 zenith 不同色，远景消隐方向与天幕有可见色差——**实施时统一为同一色系后锁定基线**。夜景 non-black 契约安全（zenith 保底 + 灯阵 emission，全图均值远超阈值 1.0）。夜景下 LOD 远球没入雾中（fog end=48 < d=60.6）属预期，lod 捕获钉 day 不受影响。~~夜景地面/展台曝光偏亮~~ **已修正（2026-09-30 owner 拍板）**：根因 = 夜景只降 ambient、key light 仍全白（view.rs：illuminance 固定 1.0、光强由颜色携带），地面读到 ~83% 日间亮度；修正 = `courtyard_night` 的 key_light_color 调暗偏冷（0.22/0.26/0.42），地面落 ~46% 日间亮度（前景 luma 196.6→109.0），灯阵成画面主光源；courtyard-night/stress 两条基线随修正重捕（隔离上下文）。

## 6. LOD 轴验证坐标

阈值实测 mid=10 / far=25 / very_far=50（`crates/bevy-stylized-core/src/lod.rs:32-40`），距离 = 球心到相机眼点的欧氏长度。相对 panorama 机位 eye=(0, 3.4, 16.0) 解出的合法坐标（四带各中、离带边界 ≥20% 余量、球随距离放大保持角直径 ≈11.8°）。**下表坐标由 PC-2 实施时重解并锁定**（原表反推自已废弃的 eye z=11.5 方案，near 球实距 10.6m 会落入 mid 带；且原 mid 坐标 (2.2,−4.76) 与东北展台冲突）：

| 带 | 球心 (x, y, z) | 半径 | 实际距离 | 余量 |
|---|---|---|---|---|
| near | (−1.2, 0.67, 9.4) | 0.67 | 7.24 | 27.6%（前景下中部，完整入镜） |
| mid | (−4.2, 1.95, −2.5) | 1.95 | 19.0 | ≥23.9%（西侧，展台环内档） |
| far | (−6.0, 3.51, −18.0) | 3.51 | 34.5 | ≥30.9%（西北墙外） |
| very_far | (−8.0, 6.21, −44.0) | 6.21 | 60.6 | 21.2%（远景） |

单一源是 `labs/scenes/src/metadata.rs` 的 `COURTYARD_LOD_BALLS`（断言覆盖四带落位、余量与无重叠）；far 带起换低模（沿用 `StylizedLodMeshes.band_meshes[2/3]`）。**PC-5 修正记录**（2026-09-28 实机捕获检视）：初版轴穿中轴，far/very_far 在背景叠合遮挡角色台、near 球卡画框左下边缘——轴整体移至西侧轨道。布局变更必须先过断言。

## 7. 相机机位

**v3：机位与内容一一对应**——内容同在中央平台（展亭模式），机位是每
内容的取景机位（摄影师换位、展品不动）：平台内容共用南望北的 hero
机位族，lamps 用灯环广角机位，lod 用侧斜视沿轴机位。panorama 随
overview 退役。机位经捕获检视锁定（v3 实捕验证，见验收记录 v3 条目）。

| 机位 | eye | look | 用途 |
|---|---|---|---|
| center-trio（默认） | (3.2, 3.4, 5.2) | (0, 1.2, 0) | 三对照组判定（斜上角读三排纵深） |
| ramp | (0, 1.7, 5.2) | (0, 1.28, 0) | p1 ramp 判定（正对横排 4 球） |
| outline | (0, 1.7, 5.4) | (0, 1.35, 0) | p2 描边判定（正对读 0/2/8 宽度梯度） |
| outline-west | (0, 1.3, 3.4) | (0, 0.65, 0) | p3 hybrid 判定（方块堆矮小，取景收紧） |
| lighting | (0, 1.8, 5.6) | (0, 1.35, 0) | p4 光照对照判定 |
| flags | (0, 2.8, 7.0) | (0, 1.9, 0) | 蒙皮旗面姿态判定（条带高，取景偏高） |
| stage | (0, 2.0, 6.2) | (0, 1.45, 0) | 角色台 NPR 角色判定（全身入镜） |
| lod-side | (1.8, 2.4, 7.5) | (−2.2, 2.2, −20) | 侧斜视 LOD 轴（正对轴向会球体相叠） |
| lamps | (0, 2.6, 10) | (0, 1.8, −10) | 灯阵夜景判定（北望灯环） |
| rim | (0, 1.8, 5.4) | (0, 1.35, 0) | rim 边缘光对照（V4） |
| shadow | (0, 1.8, 5.4) | (0, 1.35, 0) | 阴影形状对照（V4） |
| palette | (0, 1.8, 5.4) | (0, 1.35, 0) | 四色 palette 对照（V4） |
| metal | (0, 1.8, 5.4) | (0, 1.35, 0) | 金属三色 + spec ramp（V4） |
| cutout | (0, 1.8, 4.6) | (0, 1.3, 0) | alpha 镂空卡片（V4，稍近读剪影） |

机位数据单一源是 `labs/scenes/src/metadata.rs` 的 `COURTYARD_CAMERA_PRESETS`
（v3 为 9 个 preset、v4 增 5 台共 14，center-trio 居首为默认；←/→ 交互
循环与捕获规格引用同表）。

## 8. 参数面

| 参数 | 取值 | 作用域 |
|---|---|---|
| **zone** | trio / p1 / p2 / p3 / p4 / p5 / lod / stage / lamps / rim / shadow / palette / metal / cutout | **展台内容档**（v3：内容在中央平台轮换装卸 + 机位联动；overview 随 v3 退役；v4 增 5 个 cel 特性档） |
| time_of_day | day / dusk / night | view preset |
| outline_mode | off / hull / hybrid | P2/P3 内容 + 相机附件（hybrid 需 Standard） |
| crowd | 0 / 100 / 1000 | 灯阵密度（只影响 lamps 分区展品） |
| post | off / on | 相机附件 |
| flags | rest / posed | 蒙皮姿态 |
| **stage_character（2026-09-29 新增）** | fixture / odette | 角色台展品（内容重建语义；fixture=golden 判定内容，odette=真 Genshin 角色交互展示档，见 §4 角色台段） |
| face_threshold | 0.0–1.0 / null | 中央前排 face 组 SDF 阈值（沿用现有参数 schema 机制） |

> 实施前置项（M1，**已落地**）：`ParameterKind::Enum(Vec<String>)` 变体已随
> PC-1 加入 catalog（`labs/scenario-model/src/catalog.rs`），含校验与单测；
> §8 的四个枚举参数即以该变体声明。
>
> 参数落点的稳定逻辑 ID（`SceneTarget`）随 builder 定义并进入 catalog 参数
> schema：face 组实例 `trio.face.0..2`、灯阵 `lamps`、旗帜 `flags.*`、
> 视图作用域 `main`；命名在代次内唯一（合同同 SceneTarget 既有约定）。

## 9. 回归结构（v3：一帧一主体，整图精确为主；ROI 机制保留）

**原则（v3）**：展亭模式下每条捕获的画面只含当前内容档展品 + 画框
（地面/铺装/矮墙/平台）——**一帧一主体**，整图精确判定成为主力，即
Unity「一场景一用例一基线」在单场景内的等效形态。宽松整图 + ROI 双层
判定机制**保留在 harness**（夜景灯阵等暗部场景继续使用）；tier/参数
差异仍走多基线分桶。v1 的「全景宽松 + 分区 ROI」与 v2 的「分区隔离」
随展亭模式演进为「档位轮换」（dated 历史不改写）。

### 捕获矩阵（v4，17 个捕获）

| 捕获 | zone／机位 × 参数 | 判定 |
|---|---|---|
| courtyard-trio | trio／center-trio，day | 整图精确（等效旧 face/hair/metal 基线，含全部 off/on 与阈值状态） |
| courtyard-ramp | p1／ramp，day | 整图精确（**v2 新增**：v1 由 day 全景 ROI 覆盖） |
| courtyard-outline-hull | p2／outline，day，hull | 整图精确 |
| courtyard-outline-hybrid | p3／outline-west，day，hybrid（**Standard 桶**） | 整图精确（v2 目标：噪声源结构性消除，实施时按本节噪声口径验证；若 host 噪声复现则回退整图宽松 + p3 ROI，机制保留） |
| courtyard-lighting | p4／lighting，day | 整图精确（**v2 新增**：v1 由 day 全景 ROI 覆盖；key light ±Z 显式对照回到像素级，v1 的显式→隐式降级恢复） |
| courtyard-flags | p5／flags，day，flags=posed | 整图精确 |
| courtyard-stage | stage／stage，day | 整图精确（**v2 新增**：v1 角色仅全景隐式覆盖；资产门控捕获——GLB 缺席 = 本捕获 BLOCKED） |
| courtyard-lod | lod／lod-side，day | 整图精确（LOD 分带；far 带起特性降级——matcap 等 hero extras 随 `StylizedLodPolicy` 削落，近球亮面/远球哑光的对比即可读证据） |
| courtyard-dusk | p4／lighting，dusk | 整图宽松（时刻覆盖；光照对照区同时是色板探针） |
| courtyard-night | lamps／lamps，night，crowd=100 | 整图宽松 + ROI：lamps（emission 判定；夜景暗部沿用 L2 区域阈值口径） |
| courtyard-post | trio／center-trio，day，post=on | 整图精确（对抗性检视 S1 结论沿用：post 必须有独立基线） |
| courtyard-stress | lamps／lamps，night，crowd=1000 | 整图宽松 + non-black；性能证据走 performance lane |
| courtyard-rim | rim／rim，day | 整图精确（V4 新增：rim off/白强/暖弱对照） |
| courtyard-shadow | shadow／shadow，day | 整图精确（V4 新增：阴影形状三档） |
| courtyard-palette | palette／palette，day | 整图精确（V4 新增：四色 palette 三档） |
| courtyard-metal | metal／metal，day | 整图精确（V4 新增：平色/matcap/金属+spec ramp 递进） |
| courtyard-cutout | cutout／cutout，day | 整图精确（V4 新增：alpha 镂空剪影） |

outline_mode=off 无独立捕获（L1 沿用）：hull 宽度 0 的球在 P2 内部分覆盖该状态，如实记录为部分覆盖。crowd=0 无独立捕获（lamps 区空阵不是演示状态）。v1 的 courtyard-day（全景默认态）随全景退役——day 默认态的像素覆盖由 trio/ramp/lighting 等 day 钉值捕获分担，无覆盖丢失。

单附件槽约束（实现备注，沿用）：`CameraRecipe` 只有一个附件槽，post 与 hybrid 不同时组合（v2 矩阵仍无此组合）；运行时同时请求时 hybrid 优先（ScreenOutline 先于 PostProcess），已在 builder 注释记录。

### 捕获规格（capture spec）schema（v2 增 zone 钉值）

每条捕获是一份数据记录——suites.json、lane 控制文件（`catalog-scene.txt` 的后继）、golden 目录三方都靠它寻址。**捕获 ID 只允许 `-`**（`_`↔`-` 派生约定：browser/android lane 从 `showcase_golden_<id 下划线形>` 测试名反解捕获 ID，见 §10 迁移清单）：

```
capture_id    捕获身份（如 courtyard-trio；对应测试名 showcase_golden_courtyard_trio）
camera        §7 机位 id（14 个 preset；v2 起 camera 与 zone 一一对应，校验一致性）
parameters    §8 参数集的完整钉值（v2 起含 zone 键；未钉参数取默认状态 §0；摄入时展开，无运行时隐式默认）
tier          tier 桶（缺省 Portable；Standard 桶只用于 courtyard-outline-hybrid）
judgment      whole_frame_loose | whole_frame_strict | { rois: [区域 id, ...] }
```

17 条捕获规格是 §9 矩阵的机读形式（v3 12 条 + v4 新增 5 条 cel 特性档）；规格是纯数据，新增捕获不改 harness 代码。

### golden.json 的 regions 扩展（schema 形状，沿用）

```jsonc
{
  "fixture": "showcase/courtyard-night", // 现有字段不变
  "png_fnv1a64": "…", "width": 512, "height": 512,
  "regions": [                            // additive 扩展；v2 起主要由夜景 lamps 使用
    { "id": "lamps", "rect": {"x": 0, "y": 0, "width": 0, "height": 0},
      "max_channel_delta": 0, "max_fail_ratio": 0.0 }
  ]
}
```

`rect` 由分区 AABB 投影自动计算（非手工标定）；`max_channel_delta` / `max_fail_ratio` 在基线验收时锁定并写入 acceptance 记录。缺省（无 regions 字段）= 整图判定，与现状兼容。

### ROI 判定规格（机制保留；v2 矩阵以整图精确为主）

- ROI **不手工标定**（M2）：从分区 3D AABB 经捕获相机投影自动计算屏幕矩形，捕获时写入 golden.json 的 `regions` 扩展字段；harness 断言同一捕获内 ROI 两两不重叠，防止跨区污染判定归属；
- ROI 内精确判定：per-pixel 每通道阈值 + 失败像素比双层门限（Babylon.js/Unity 模式），**阈值按区域分别锁定**（L2：夜景暗部相对噪声更高，不能全局一把尺），具体数值在基线验收时写入 acceptance 记录；
- 整图宽松判定：non-black + 全图 luma/chroma 统计包络（抓全黑、整体色差、大面积缺失）；
- 噪声源固定：AA 档位、曝光锁定（Unreal「Disable Noisy Rendering Features」对应做法）；个别持续 noisy 的区域才评估引入 FLIP（`pip install flip-evaluator`），不全局放宽；
- **布局即合同**：ROI 定义跟随分区几何自动更新；布局变更使基线像素失效，走显式 GOLDEN_REBASELINE。

### 分区 AABB 表（ROI 投影的数据源）

单一源是 `labs/scenes/src/metadata.rs` 的 `COURTYARD_REGION_AABBS`：trio / p1 / p2 / p3 / p4 / p5 / lamps / stage 共 8 区（v3 重写为平台摆位；stage 的 y 上界是角色高度的保守估计）。v3 的 zone 集合 = 这 8 区 + lod（lod 判定走整图，无 ROI 需求）。新增内容档 = 加 AABB 数据 + 捕获规格引用，不改 harness 代码。

### 噪声与播种口径（v2 修正）

- **screen-outline 剪影噪声（v1 记录在案的两类）来源消除**：v1 实测 hybrid 背景灯球/墙沿剪影处逐运行 2–4px 边缘翻转（host 深度次序竞争）、Dawn(Tint) vs wgpu/Metal(naga) 2488px 系统性边缘分歧——两者的共同前提是**背景里存在无关分区的内容**。v2 分区激活后 hybrid 视图背景只剩环境壳，噪声前提结构性消除。**实施口径**：courtyard-outline-hybrid 以整图精确捕获，逐运行逐字节稳定即锁定 strict；若 host 噪声在纯净背景下仍复现，回退整图宽松 + p3 ROI（判定模式数据随捕获规格，lane 工具按 sidecar regions 自动选择模式的既有链路不变），证据写入验收记录。v1 的「hybrid 一律宽松+ROI」口径随之作废（它是全景背景的应对，不是描边本体的性质）。同族噪声的重配置证据（`labs/scene-runtime` reconfiguration_evidence）以 ≤16 差异像素包络断言，不受影响。
- **overlay 播种的 regions 携带**（沿用）：平台 overlay 播种（GOLDEN_REBASELINE）写出 sidecar 时，regions 从既有 overlay sidecar 携带——分区 AABB × 捕获机位投影是平台无关的规格数据，不因平台重算；阈值继承首锁值，跨平台容差分歧走既有 overlay 评审流程，不放宽容差。

### 隔离承诺（v2 修正版）

分区激活后跨区穿透大幅消解：每帧只有聚焦分区展品 + 环境壳，影子/雾/光向的跨区污染面只剩环境壳（展台影子落在地面属帧内内容）。残余的全局因素是**视图层**（时刻色板、雾、曝光）——它同时影响所有捕获。故障定位语义简化为：单捕获红 = 该分区特性疑似；多捕获同红 = 优先怀疑视图层/光照/共享材质。v1 M3 记录的 lighting 显式→隐式降级由 courtyard-lighting 恢复为像素级显式对照。

### 成本表述（诚实版，v2 修正）

v2 矩阵 12 捕获（v1 为 10）：新增 ramp/lighting/stage 三条专属精确捕获；courtyard-day（全景默认态）一条退役；dusk/night/stress 三条由 panorama 改挂分区机位（lighting/lamps/lamps）。单帧负载下降（无全景大构图；stress 仍含 crowd=1000 灯阵）。「一帧多判」回到「一帧一判」——失去的是编排节省，换回的是与行业规范同形态的判定隔离（ADR Context 的行业证据在 v2 下不再是张力而是依据）；组织/浏览收益由导览模式（G）承接（v3 起 overview opt-in 退役）。

### v1 注记（历史，不改写）

v1 矩阵与判定模式的完整修正史——day 全景 ROI 集投影收敛 [trio,p1,p4]、outline-west 机位增设、hybrid 判定模式 strict→宽松+p3 ROI、蒙皮旗面剔除 Aabb + `NoAutoAabb` 陷阱（回归测试 `flag_strips_carry_overridden_culling_aabb`，v2 flags 分区同一机制继续适用）——记录在验收记录 dated 条目（`fixtures/golden/showcase.courtyard.acceptance.md`）。

## 10. 旧场景退役映射

| 旧场景 | 去向 | 旧场景 | 去向 |
|---|---|---|---|
| overview | 中央三对照组 | performance | crowd 参数 + lamps 分区（v1 曾记为 crowd=100 + 中央对照组） |
| ramp | P1 | outline-advanced | P3（Standard 桶） |
| lighting | 时刻参数 + P4 专属精确捕获（v1 曾降级为全场隐式 NdotL 覆盖；v2 恢复像素级显式对照，§9 v2 矩阵 courtyard-lighting） | lod | LOD 轴 |
| face / hair / metal | 中央三对照组（保留全部 off/on 与阈值状态） | environment ×3 | time_of_day |
| outline | P2 | effects | P4 |
| animation | P5 旗帜 + courtyard-flags 捕获（posed） | reference | 角色台（GLB 展示，全 lane 打包；S4 决策 2026-09-28；v2 起专属 courtyard-stage 捕获） |
| post | courtyard-post 捕获（post=on 独立基线） | stress | courtyard-stress 捕获 |

18 条旧基线（`fixtures/golden/showcase/*.png`）随旧场景退役删除，由 10 个捕获的基线集替代（v1；**v2 起再由 12 捕获矩阵替代**，§12 v2 阶段）；重捕走 GOLDEN_REBASELINE（需 GPU，缺席记 BLOCKED）；dated 验收记录（`*.acceptance.md`、SUPPORT_MATRIX）按惯例保留不改写，新验收记录追加。

退役时的线协议迁移清单（M5，易被低估，单列）：`tests/suites.json` 的 18 条 `showcase_golden_*` 身份 → 10 条捕获身份；`tools/checks/browser.py` 与 `tools/checks/android_catalog.py` 的 `showcase_golden_*` 派生逻辑同步；android `catalog-scene.txt` 控制文件的语义从「场景名」改为「捕获规格」（§9 schema）；`coverage.rs` 场景行改按捕获/分区组织；`tests/integration/tests/showcase_golden.rs` 的 18 个 `scene_case!` 重写；`README.md:20` 的 `-- overview` 示例更新；`CONTRIBUTING.md` 的 lane 描述同步；`docs/showcase/01` §380-409 的 12 族收敛表标注「由本文档（05）取代」。

## 11. 功能与交互设计（多平台）

> 依据 docs/showcase/01:426-443 的既有合同：输入映射成共同动作，鼠标/键盘与触摸
> 只是适配器；UI 参数来自目录；相同控件元数据、控制器与命令校验用于所有布局；
> 横竖屏/DPI 只影响显示布局；自由操作产生新 revision，自动重放不读取实时输入。
> 交互能力的落地顺序见 §12 分期；本节定义目标态。

### 11.1 命令层扩展（共享层，三平台共同前置）

现有 `SessionCommand` 只有 NextCase/PrevCase/SelectCase，承载不了参数面与机位。
扩展为（数据命令，不含输入语义；`CommandJournal` 录制/回放同步覆盖）：

| 命令 | 语义 | 内容语义 |
|---|---|---|
| `SetCameraPreset(preset_id)` | 切到 §7 机位之一（只换 eye/look/projection；**附件不由机位携带**，见下一行） | driver 新增「构建后替换相机配方」API（现状 CameraRecipe 只构建期生效，属前置缺口） |
| `SetCameraAttachment(...)` | 插入/移除/替换相机附件（post、ScreenOutline） | 由 outline_mode / post 参数驱动，与机位正交；与上一条共用 driver 的相机重建入口 |
| `CameraOrbit { yaw, pitch }` / `CameraPan { dx, dy }` / `CameraZoom { factor }` / `ResetView` | 自由相机 | 在当前机位上偏移；产生新 revision（01:429）；reset 回到当前 preset 的配方机位 |
| `SetParameter(key, value)` | 改 §8 参数 | 逐参数语义见 §11.2 |
| `Capture` | 捕获当前画面 | 走现有捕获路径（严格捕获仍用固定像素 target，与交互画面解耦）；交互模式下落盘到会话 run 输出目录（与 lane 捕获同一合同），文件名带捕获规格与时刻戳。**交互观察通道设计锁定（06 §9，2026-09-30 owner 接受）**：`Capture` 为 SessionCommand 变体 + driver 新增 observe 入口；交互捕获不参与 golden 判定，toast 文案明示「浏览快照，非判定证据」；实现归 U3 分期 |

### 11.2 参数的运行时语义分层

| 参数 | 语义 | 机制 |
|---|---|---|
| time_of_day | **live**：会话内换 view preset | 新增会话内视图 preset 切换（现状只有构建期 ViewPresetFn；切换时从基线起步应用，与场景切换的视图重置同一语义） |
| flags | **live**：Deform 动作 | 现有 driver 的 apply_deform 路径 |
| face_threshold | **live**：ApplyParams | 现有 entity scope 绑定（driver.rs 已实现） |
| post | **相机附件换**：插入/移除 PostProcess 附件 | 依赖 §11.1 的相机替换 API |
| outline_mode | **内容 + 附件换**：P2/P3 状态 + ScreenOutline 附件 | hybrid 需 Standard tier；不足时按既有 RefusalReason 结构化拒绝（UI 置灰 + 原因），可触发 ADR-0030 会话内升档 |
| crowd | **内容重建**：retire 当前代次 + 重建灯阵 | 走 driver 既有 switch 路径（同场景内重建，generation+1） |

> **W1–W3 已落地（2026-09-30，owner 需求「场景内容动态加载」，方案 A：
> 增量装卸 + 本表 live 路径）**：post/outline_mode = 相机附件 live 换
> （不重建、不换代次）；flags = live Deform（rotation_z 绝对设定；展品
> 不在场时只写设置、下次激活生效）；face_threshold = live ApplyParams
> （三落点同写——与重建语义同义，null 回 0.2/默认/0.8 梯度）；
> zone/stage_character/crowd = 分区展品**增量装卸**（`CourtyardBuckets`
> 分桶登记 + `courtyard_update` 按 settings diff 装卸 + driver
> `note_content_update` 换代次重登账本）——画框（地面/墙/展台/平台）与
> 观测相机实体跨代常驻，zone 切换先装后卸（补间落定才退役旧区展品，
> 06 §8 美术检视），代次 +1 与 `Switched` 语义不变。等价性证据：
> 增量巡场（zone 循环 + lamps 桶重装 + live 参数混合）后捕获与全新构建
> **逐字节一致**（`incremental_zone_walk_preserves_capture_identity`，
> GPU）——该证据揪出并修复一处 ulp 级相机姿态缺陷（W1 附件切换一度用
> orbit 姿态往返重建的呈现姿态，偏离配方机位末位字节致 2px 差；改取
> driver 应用配方的精确值）。

### 11.3 相机统一与分区聚焦

- **修正历史遗留**：窗口相机改为使用场景返回的 CameraRecipe（01:416-417「窗口和离屏不能另补不同机位」），交互画面与捕获画面同源；
- orbit/pan/zoom 共享实现：把 Inspector 的 `OrbitCamera`（yaw/pitch/distance，米制，distance>0.001、pitch±1.5 rad 校验）模式上移为共享层能力，Inspector 与庭院共用（**设计锁定**：06 §5——手势翻译层与键位翻译层同构、合成输入事件单测；2026-09-30 owner 接受 06；**已在 U1 落地**：庭院侧经 `mousemap` 翻译层发相机命令，控制器 CameraState 即共享 orbit 模型，Inspector 数据结构不动，见 §12 U1 行）；
- **分区聚焦（v2 语义扩展；v3 起点击聚焦退役）**：切内容档 =
  `SetParameter(zone, …)` 装卸该档展品（§4）+ 跳对应机位（§7 一一对应）。
  v2 的「点击/触摸展台 → 射线命中分区」（06 §5/§16 D2 手写 ray-primitive）
  曾在 U2 落地，**v3 退役**——同一位置下点展品 = 已是当前内容，无导航
  意义（pick/minimap 模块随之删除，不留死代码）；

### 11.4 UI 从目录派生

参数 schema（§8，含 M1 的枚举扩展）即控件元数据：`Enum` → 分段按钮/下拉、`F32/OptionalF32` → 滑杆、`Bool` → 开关。控件声明只有这一份；tier 门控参数在能力不足时置灰并显示结构化原因。01:443 的「相同控件元数据用于所有布局」由此落实——三平台布局不同，语义与校验同源。

**最小切片已落地（阶段 7，2026-09-28）**：PC 交互窗口的 egui 叠加层——左下常驻状态条（机位/时刻/outline 分段/post/crowd/flags）、分区图例（9 区，默认展开）、键位帮助（默认折叠）——以静态声明数据实现（tier 门控呈现已生效：hybrid 不足 Standard 时置灰并带原因）。**未落地**：schema 驱动的控件生成（参数面板从 catalog 参数 schema 自动装配分段按钮/滑杆/开关）、跨平台共享控件元数据——**设计锁定（06 §6，2026-09-30 owner 接受）**：右侧抽屉 + 控件从 catalog ParameterKind 自动生成（含 U32 步进滑杆 + presets 芯片、OptionalF32 「自动」勾选；分组 Scene/Rendering/Content 进 catalog 元数据），实现归 U2 分期。

### 11.5 平台适配矩阵

| 平台 | 输入翻译 | UI 壳 | 资产来源 | 主要缺口 |
|---|---|---|---|---|
| PC（winit） | 键盘（键位表）+ 鼠标（拖拽 orbit/滚轮 zoom/点击聚焦） | egui 侧栏（复用 Inspector 的装配模式；interactive feature 引入 bevy_egui——**已落地**：状态条/图例/键位帮助叠加层 + `EguiWantsInput` 输入门控） | filesystem | ~~交互模式现无 egui~~（阶段 7 已关闭）；orbit/pan/zoom/分区聚焦未接 |
| Browser（WebGPU） | 鼠标/键盘 + 页面控件 | HTML 面板绑定同一命令通道（页面桥模式现成：publish/inline_js + CDP 轮询） | HTTP | 宿主从「一次性捕获后停摆」改为驻留会话状态机。**注意分期**：URL 从 `?scene=` 扩展为捕获规格是阶段 4（lane 迁移）的必需项，不属交互 V2；V2 只加页面控件与交互驻留 |
| Android（GameActivity） | 单指拖 orbit / 双指 pinch zoom / 双指拖 pan / 双击 reset / 点按聚焦 | 折叠面板（egui on Android：有社区先例可行，见下；项目内 spike 验证，失败退回原生覆盖层） | APK assets（S4：GLB 打包进 APK——**已完成并验证**，见阶段 6） | ~~驻留会话模式~~（阶段 6 已落地：跑完不退出 + 触摸事件 logcat 证据）；手势分解、参数面板、应用内导入/导出桥接（01:378，实现度 0）维持缺口 |

> egui on Android 调研结论（2026-09-28）：egui 经 winit/GameActivity 在 Android 可运行且支持多点触摸；已知坑是软键盘（[android-activity#44](https://github.com/rust-mobile/android-activity/issues/44)，社区解法 [egui_virtual_keyboard](https://lib.rs/crates/egui_virtual_keyboard)）——**庭院参数面板只用按钮/滑杆/开关、不用文本输入，天然规避**。触摸事件消费顺序按 [egui#3210](https://github.com/emilk/egui/issues/3210) 模式处理：egui 面板优先消费，场景手势只吃未消费的触摸（bevy_egui 的 `EguiWantsInput` 机制现成）。残余风险（GameActivity 触摸与 winit 的接线质量，[Bevy Android 触摸社区记录](https://mevlyshkin.com/notes/bevy-android-input/)）由项目内 spike 兜底。

### 11.6 最小可行交互（V1 键位表）

交互 V1 不加 UI 框架，纯键位（PC 先行，验证命令层正确性）：

| 键 | 命令 |
|---|---|
| ← / → | **内容档循环**：trio → p1 → p2 → p3 → p4 → p5 → lod → stage → lamps（机位随内容联动，§7；v1 的「机位循环」语义由内容循环吸收） |
| ↑ / ↓ | time_of_day 循环（day → dusk → night） |
| O / P | outline_mode / post 切换（tier 不足时 stderr 打结构化拒绝） |
| 1 / 2 / 3 | crowd = 0 / 100 / 1000（按数量序助记） |
| Space | flags rest/posed |
| N | stage_character fixture/odette 翻转（2026-09-29 新增；odette 档需宿主 odette:// 资产源，不影响判定内容） |
| R | ResetView |
| C | Capture（浏览快照，非判定证据；U3 落地） |
| G | 导览模式（脚本化巡场，任意导航输入退出；U3 落地） |
| H | 演示模式（隐藏全部 UI，Esc/H 退出；U1 落地） |

（v2 的 Tab overview 开关随 v3 退役——同屏全显与展亭模式互斥。）

浏览器与 Android 在 V1 不接交互（浏览器保持一次性捕获、Android 保持 catalog 模式），交互分期见 §12。

### 11.7 跨平台一致性合同

同一命令序列在任意平台产生同一画面语义（平台差异只在输入翻译与显示布局）；命令录制/回放（CommandJournal 扩展到新命令变体）作为跨平台一致性的测试证据；严格捕获继续走固定像素 target，与交互窗口的分辨率/DPI 无关。

## 12. 实施阶段（概要）

| 阶段 | 内容 | 验收门槛 | 状态（2026-09-28） |
|---|---|---|---|
| 0 | 前置项：catalog `ParameterKind` 枚举扩展（M1）；GLB 打包进 browser/android 资产管线（S4）；`SessionCommand` 扩展 + driver 相机配方/附件构建后替换 API（§11.1）；共享 orbit 相机（§11.3） | cpu lane 全绿 + 新单测 | **已落地**（M1 Enum 变体在 PC-1 前置落地；S4 打包 browser 阶段 5 / android 阶段 6 验证；命令层 PC-3、相机统一 PC-4） |
| 1 | 场景构建：courtyard builder（分区 × 参数） | cpu lane + 布局/LOD 落带断言（沿用 metadata.rs 余量测试套路） | **已落地**（PC-2；§0 默认状态 = CourtyardSettings::Default） |
| 2 | ROI 判定扩展：golden.json regions + AABB 投影 + 不重叠断言 + labs/validation 区域 diff | cpu lane + harness 单测 | **已落地**（捕获规格 10 条 + golden.json regions + 区域 diff/整图包络；day ROI 集按投影证据收敛 [trio,p1,p4]） |
| 3 | 基线捕获与验收：10 捕获（GPU，GOLDEN_REBASELINE；布局冻结前先跑临时基线，L3） | gpu lane + acceptance 记录；GPU 缺席 = BLOCKED | **已落地**（macos-metal overlay 10/10 验收 + 两轮捕获修正/美术迭代 + outline-west/hybrid 判定修正；**canonical 基线已建**——2026-09-29 linux-vulkan 参考宿主 10/10 捕获验收，overlays 降为平台层，验收记录同日条目） |
| 4 | 旧场景退役：§10 迁移清单全量同步 | cpu + gpu + browser + android 四条 lane 全绿 | **已落地**（catalog=1、绑定=1、suites.json 18→10、cpu/gpu lane 全绿；browser 9/10 exact + outline-hybrid 平台分歧记录在案；android 10/10 PASS 模拟器） |
| 5 | 交互 V1：PC 键位表 + 窗口相机统一（§11.6、§11.3 第一条） | cpu lane + 交互冒烟 | **已落地**（PC-4 键位翻译层 + 窗口相机统一；P7 egui 叠加层最小切片——状态条/图例/键位帮助 + tier 置灰；C=Capture 维持暂缓；窗口冒烟待用户验证） |
| 6 | 交互 V2：browser 驻留会话 + HTML 参数面板（URL 捕获规格扩展在阶段 4 已落地） | browser-showcase lane + 手动验证 | **部分落地**（`?spec=` 捕获规格合同 + browser lane 判定链已落地并实跑；驻留会话 + HTML 参数面板未做） |
| 7 | 交互 V3：Android 驻留会话 + 手势 + 面板（egui spike 先行）+ 应用内桥接（01:378） | android-catalog lane + spike 结论记录 | **部分落地**（驻留会话模式已落地——跑完不退出 + 触摸事件 logcat 证据；egui 面板 PC 侧已验证装配模式；手势分解/参数面板/应用内桥接维持缺口） |
| 8 | 文档收尾：本文档标注落地，docs/showcase/02 更新，ADR-0031 转 Accepted（Evidence 补齐） | docs lane | **本轮**（ADR-0031 Evidence 已补齐；Accepted 属 owner 决策） |

阶段 1–4 交付「可捕获、可判定、可退役」的庭院；阶段 5–7 交付交互。**捕获与 lane 迁移不依赖交互层**——交互只是同一命令集的另一个驱动源。

### v2 阶段（2026-09-29 owner 决策改版：分区激活，不同屏全显）

上表 v1 各阶段的落地记录保留为历史（dated 验收记录不改写）；v1 的基线集
（canonical + macos-metal/android-vulkan overlay）与全景判定模式随 v1 矩阵
在 V2-3 退役重捕。窗口冒烟另修复交互路径三缺陷（窗口相机 Tonemapping、
egui 叠加相机清屏配置、PrimaryEguiContext 挂载，2026-09-29），
阶段 5 的「窗口冒烟待用户验证」在 V2-1 一并复验。

| 阶段 | 内容 | 验收门槛 | 状态（2026-09-29） |
|---|---|---|---|
| V2-0 | 设计改版：本文档 v2 + ADR-0031 修正（Context/Decision/Consequences）+ TRACEABILITY R-031 同步 | docs lane | **已落地** |
| V2-1 | 分区激活实现：`zone` 参数进 catalog schema（枚举含 overview）；builder 按激活分区生成展品（环境壳/画框常驻；lamps/stage/lod 展品条件生成）；机位表 6→10（ramp/lighting/stage/lamps 新增，panorama 归 overview，新机位实捕验证锁定）；键位 ←/→ 分区循环 + Tab overview；阶段 5 窗口冒烟复验 | cpu lane + 激活/机位断言（沿用 metadata 断言套路）+ 窗口冒烟 | **已落地**（CourtyardZone + zone 钉值全链（catalog/controller/capture-spec/keymap）；builder 条件生成 + overview 全量；机位 10 preset + zone↔机位映射断言；窗口冒烟经 BRP（bevy-copilot，127.0.0.1:15702 + 自定义方法 `showcase/session_command`，与键位同一 SessionCommand 通道）端到端驱动验证：trio→p2→overview→trio 切换中应用/窗口/egui/BRP 全程存活、窗口相机随分区联动、分区内容计数与画面截图逐项正确。**冒烟揪出并修复两个运行时缺陷**：① 场景账本基线钉在 `CaseDriver::new`，其后出生的宿主实体（PrimaryMonitor——OnMonitor 关系级联杀主窗口、bevy_remote 系统实体、egui 相机）被误退役——改首次构建前惰性快照（`labs/scene-runtime/src/driver.rs`）；② `built_recipe`/`applied_recipe` 不随代次作废，同步窗口内把上一代配方当新配方派生，窗口相机永久锁死——改 switch_scene 清空。两缺陷各有回归测试。物理按键链路待用户确认——Wayland 合成输入不可注入，翻译层有单测） |
| V2-2 | 捕获矩阵 v2：capture_spec.rs 改 12 条（zone 钉值 + camera↔zone 一致性校验）；suites.json / 集成测试 / lane 派生逻辑同步（§10 迁移清单同一机制）；hybrid 纯净背景 strict 验证（噪声复现则回退 loose+p3 ROI 并记录） | cpu lane + harness 单测 | **已落地**（12 规格 + camera↔zone 一致性测试；suites.json 12 身份；showcase_golden.rs 12 用例；lane 工具经 suites.json 派生无需硬编码同步；coverage.rs v2 真实状态——browser/android 证据不随帧变更继承回落 Pending；**hybrid strict 验证通过**：纯净背景逐字节稳定，未动用回退预案，Dawn/naga 放宽议题作废——browser v2 复跑时按纯净背景重估） |
| V2-3 | v2 基线捕获与验收：12 捕获 GOLDEN_REBASELINE（本机 canonical）；v1 基线集退役删除；验收记录追加 dated 条目 | gpu lane + acceptance 记录；GPU 缺席 = BLOCKED | **已落地**（12/12 PASS：strict 9 条（含 hybrid）逐字节 + night loose+lamps ROI + dusk/stress loose；v1 基线集已删；亲眼看图 12/12（分区隔离成立、新机位取景锁定）；验收记录 v2 条目） |
| V2-4 | lanes 全绿：cpu / gpu / docs；browser/android 的 v2 overlay 重播种与 lane 复跑归各自主 host（macOS 侧后续工作，模拟器证据不构成真机声明的口径沿用） | cpu + gpu + docs lane 绿 | **本机三 lane 全绿**（gpu 全量含 showcase_golden 12/12、cpu、docs，退出码 0）；browser/android v2 重播种待 macOS 侧 |
| V2-5 | 角色台展品参数（2026-09-29 owner 需求：庭院里看到真角色）：`stage_character`（fixture 默认 / odette）全链（catalog schema/controller/keymap N 键/builder 分支）；`.npr.json` sidecar 资产化（`StylizedSidecarAsset` + 加载器 + `StylizedSidecarSource` apply 系统 + `param_overrides` 宿主订正通道，归 `StylizedAssetIntegrationPlugin` 统一注册，三宿主零装配变更）；纹理前缀支持 `source://` 限定；交互宿主注册 odette:// 资产源（目录缺席不注册 + stderr 明示）；**golden/证据内容不动**（fixture 档，基线零变动，ADR-0017 第三方美术不提交基线） | cpu lane + 单测（catalog 校验/controller 重建/keymap 翻译/sidecar apply+override/纹理路径）+ gpu lane 零基线变动复跑 + 窗口冒烟（BRP 切 odette + 截图亲验） | **已落地**（单测全绿；cpu/gpu/docs 三 lane 退出码 0——**gpu 12/12 未重捕**，实证 golden 内容零变动；窗口冒烟 BRP 驱动 fixture↔odette 往返 + stage 截图亲验：odette 全身 NPR 渲染正确。**冒烟揪出并修复面部黑脸**：static-v3 静态烘焙导出 Face 网格节点恒等（顶点已最终朝向），sidecar 的 face 基向量是 Blender 原作者空间 (0,−1,0)，直接代入整面背光——经 param_overrides 覆盖 face_forward/up 为世界朝向 + effect_light_dir 正面来光（只动 Face 材质，全局 view 光照口径不变）。物理 N 键链路同 V2-1 物理按键待确认项） |
| U1 | 交互层一期（06 §13，owner 2026-09-30 接受 06 并批准动工）：toast 堆叠（右下，确认 2s 消退/拒绝琥珀色驻留，键位/鼠标/BRP 三驱动源共用）；Tab 上一分区记忆（keymap `overview_return` 参数 + 宿主会话记忆）；鼠标手势（新 `mousemap` 翻译层与 keymap 同构——左拖 orbit/右拖 pan/滚轮 zoom/双击 reset，Inspector 同手感常量，点击/拖拽位移阈值判定，EguiWantsInput 指针门控）；机位补间（控制器相机状态内 0.4s ease-in-out 球面插值，默认关——捕获/冒烟零行为变化，交互窗口默认开，`--no-camera-transition` 钉关；补间不进 journal/不产生 revision）；H 演示模式（隐藏全部 egui UI，Esc/H 退出，宿主本地状态）；状态条 building 指示（DriverStatus::Running 驱动） | cpu lane + 手势/翻译/补间单测（合成输入 + 显式 dt 推进）+ 窗口冒烟（BRP 驱动 + 亲眼看图）+ owner 物理鼠标确认 | **已落地**（单测全绿：keymap 5 + mousemap 7 + controller 补间 4（起跳无瞬跳/半途同源/终点落定/默认关闭/抢占/重建起跳）；cpu lane 退出码 0；窗口冒烟 BRP 驱动 + 截图亲验：切分区补间中间态与落定态逐帧正确、toast 确认/拒绝/驻留语义逐项可见。冒烟揪出并修复：BRP 路径绕过 desktop_tick 命令循环不落 toast——命令结果收敛到共用 helper，三驱动源同源。building 指示在本机重建过快（亚秒级）未捕到可见帧，代码路径有 controller 测试覆盖，如实记录。物理鼠标/H 键链路同 V2-1 物理按键待确认项——翻译层合成事件单测覆盖，winit 物理链路待 owner 确认） |
| U2 | 空间导航与参数面板（06 §13）：分区 dock（底部 9 分区芯片 + overview 开关，06 §3 Legend 退役、说明文案下移为芯片 tooltip；点击 = `SetParameter(zone)`，与 ←/→ 完全同命令）；minimap（右上，egui Painter 自绘俯视：点状分区取 `COURTYARD_REGION_AABBS` 中心投影、lamps 画北半环弧（不取 AABB 中心）、LOD 轴四球折线（出界径向截断 + 箭头）、当前机位视锥楔形（与状态条同源缓存）；激活高亮/overview 全亮；图形命中 = 点半径/弧带/线段距离阈值，不做像素级精确）；场景内点击聚焦（builder 随展品注册命中图元 `CourtyardHitMap`——球/圆柱/盒展品本体，不是分区 AABB（lamps AABB 覆盖北半场会误触发）；lamps 逐灯柱实例、stage 命中台基座/角色、画框展台常驻注册供跨分区导航；宿主用窗口相机 `viewport_to_world` 射线，未命中/同分区不动作）；schema 驱动参数抽屉（catalog `ParameterKind` → 控件选型单一源（`param_panel::widget_kind`：Enum ≤4 分段/>4 下拉、U32 步进滑杆（拖拽结束才提交——crowd 是内容重建语义）、Bool 开关、OptionalF32 滑杆+自动勾选）；catalog schema 增可选 `ui` 元数据（ParameterUiMeta：分组 Scene/Rendering/Content + crowd presets 芯片 [0/100/1000]，全域 0..=1000 仍合法）；tier 置灰经控制器 `evaluate` 同一判定 + 结构化原因 tooltip；Legend 退役） | cpu lane + schema 控件单测 + 射线命中断言 + 窗口冒烟 | **已落地**（2026-09-30：catalog 分组/presets 断言；pick 求交 4 单测（球/盒/圆柱/最近序）+ builder 注册命中断言（默认 13 条 = 画框 6 + trio 7、overview 236 条、lamps 逐灯命中、未命中不动作）；minimap 投影往返/点/弧/轴命中 4 单测；控件选型 + 取值映射 2 单测；cpu + gpu lane 退出码 0（**golden 12/12 未重捕**，命中表是纯资源注册、像素零变动）；窗口冒烟 BRP 驱动 + 截图亲验：dock/minimap/参数抽屉渲染正确，BRP 切 stage+odette 后 dock 芯片/minimap 激活点/机位楔形/抽屉下拉与分段/状态条五处联动一致，tier Portable 下 hybrid 置灰生效。场景内点击的 winit 物理指针链路同 V2-1 物理按键待确认项——射线拾取与命中注册有单测/断言覆盖，待 owner 物理确认） |
| U3 | 演示与捕获（06 §13）：导览模式（G 开关，宿主本地状态；脚本 = 数据 `tour::COURTYARD_TOUR`（9 分区循环序 ×4s + overview 收尾 6s 后结束），driver 落定时才推进驻留计时（构建耗时不吃驻留预算），任意导航输入退出，每条驻留命令照常走 SessionCommand 通道）；宿主 journal 接线（desktop_tick 命令循环与 BRP 入口同换 `command_recorded`——键位/鼠标/UI 队列/BRP 四驱动源同通道，会话级 `CommandJournal` 驻留内存）；`SessionCommand::Capture`（06 §9 落地：evaluate 仅在 driver Done 稳态受理（构建在途/失败 = DriverBusy 结构化拒绝），执行 = driver 新 `observe_adhoc` 入口（与动作图 Observe 同一读回通道，读回跨帧完成、发起即 Applied）；回执由宿主落盘 `target/stylized/showcase/interactive/`（gitignored，文件名带观测名/代次/实验 tick/时刻戳）+ toast 显示路径并明示「browsing snapshot, not golden evidence」；**不参与 golden 判定**；C 键 + 抽屉按钮 + BRP 同变体） | cpu lane + observe 通道测试 + 冒烟 | **已落地**（2026-09-30：controller Capture 门控单测（非 Done 拒绝）+ journal Capture 持久化往返 + tour 脚本推进/收尾单测；cpu + gpu lane 退出码 0（golden 12/12 未重捕）；窗口冒烟 BRP 发 capture → Applied + PNG 落盘 + 亲眼看图（512² 内容与窗口一致）+ 双 toast（applied + snapshot saved 路径）可见。重建在途拒绝的实况窗口本机过短（亚秒级重建）未捕到——门控路径有 controller 单测覆盖，如实记录。物理 G/C 键链路同既有物理按键待确认项） |
| W1–W3 | 场景内容动态加载（2026-09-30 owner 需求「动态加载，不要全部放在场景中」，方案 A：增量装卸 + §11.2 live 参数路径全量落地）：W1 post/outline_mode = 相机附件 live 换（不重建、不换代次）；W2 flags/face_threshold = 实体级 live（在场应用/缺席只写设置）；W3 分区展品动态装卸（`CourtyardBuckets` 分桶登记——差值登记法零耦合，`courtyard_update` 按 settings diff 装卸，driver `note_content_update` 换代次重登账本；画框 29 实体与观测相机实体跨代常驻；zone 切换先装后卸，补间落定才退役旧区） | 等价性证据（增量 == 全新构建逐字节）+ 单测 + 三 lane + 窗口冒烟 | **已落地**（`incremental_zone_walk_preserves_capture_identity`（GPU）：zone 巡场 + lamps 桶重装 + live 参数混合后捕获与单次运行**逐字节一致**、计数回基线、观测相机实体不变；controller 延迟退役/在场 live 路由单测、builders 装卸桶单测全绿；cpu/gpu/docs 三 lane 退出码 0，**golden 12/12 未重捕**（捕获路径不动）；窗口冒烟 BRP + 截图亲验：post live 即时生效（Applied 不换代次）、p5 切换画框连续无闪烁、stage 换 odette 只动角色台（gen 2→3）。**证据揪出并修复一处 ulp 级缺陷**：W1 附件切换一度用 orbit 姿态往返重建的呈现姿态写观测相机，偏离配方机位末位字节（逐字节证据差 2px）——改取 driver 应用配方的精确值。物理键鼠链路同既有待确认项） |

| V4 | cel shading 特性补全（2026-09-30 调研缺口收编，owner 批准）：5 个新内容档（rim 边缘光对照 / shadow 阴影形状三档 / palette 四色对照 / metal 金属三色+spec ramp 递进 / cutout alpha 镂空）——纯增量：5 builder 函数 + zone id/机位/区域表 +5 + catalog 枚举 +5 + 捕获规格 17 条 + dock/keymap/tour 同步；既有 12 条基线零变动（lod 档经查证特性降级已在演示——Hero+matcap 球经 StylizedLodPolicy 在 far 带削 matcap，不动）；新基线按 v3 合同隔离上下文捕获 | cpu lane + 布局断言 + gpu lane 17/17 + 逐张亲验 | **已落地**（cpu lane 退出码 0；5 条新基线隔离捕获 + 亲眼看图（rim 边缘光可读、shadow 硬软对照分明、palette 暖冷阴影分明、metal 平色→matcap→spec 递进、cutout 棋盘镂空清晰）；gpu lane 17/17；文档收尾同步） |
| V3-0..4 | 展亭模式改版（2026-09-30 owner 决策「在同一个位置根据不同的 preset 展示不同的内容」；方案与对抗性评估见 plans/patriot-big-barda-valkyrie.md）：全部内容上中央平台（p1/p2/p4 横排 + 基座、p3 台面堆、p5 杆带、stage 台基座、lod 轴西南起北伸微偏西、lamps 不变）；展台环/overview/panorama/minimap/点击聚焦退役（不留死代码：pick.rs、minimap.rs 删除，Tab 键释放，导览脚本 9 站收尾回 trio）；机位表 9 台重排（hero 机位族 + 内容取景机位）；区域 AABB 表重写；canonical 基线 12/12 重捕 | cpu/gpu/docs lane + 布局断言 + 逐张亲验 + 冒烟 | **已落地**（布局/落带/区域断言更新全绿；三 lane 退出码 0；基线 12/12 重捕并双复跑逐字节稳定——首轮重捕曾因重捕与验证之间夹了注释级源码编辑（重编译改变 inlining 致 ulp 级着色漂移）在 lane 复跑暴露 trio 漂移，整体重捕 + 双复跑后冻结，运维口径记入验收记录 v3 条目；窗口冒烟逐内容亲验取景） |

## 13. 明确不做

- **全景同屏不承担 golden 判定**（owner 决策 2026-09-29；v3 起 overview 浏览模式整体退役——同位必穿模，无基线、无 ROI、无判定职责的表述随之归档）；
- Inspector 不动（独立角色检视器，自带 provenance hash 机制；阶段 2-7 全程零改动）；
- benchmarks / labs/regression 不动（它们直接复用 builders 的共享 helper，不经 catalog 场景；退役后持续绿——共享 helper（build_strip/build_hero_1/build_portable_instances/ramp_image/matcap_image/ilm_image 等）全部保留并经回归验证）；
- labs/reference 的六视角验收合同不动（独立于展示场景；角色台 NPR 化不影响其编译角色管线）；
- dated 验收/测量记录不改写（新旧条目以追加为准）；
- 不为旧 18 场景 ID 留兼容层或别名（AGENTS.md：只支持当前版本；§10 退役为硬切）；同理不为 v1 退役的捕获 ID（courtyard-day）留兼容层——V2-2 硬切，旧 ID 在版本/catalog 门被显式拒绝。

交互分期的排序依据（V1 PC → V2 browser → V3 Android）：PC 交互宿主已存在、键鼠输入翻译最薄，命令层正确性在这里验证成本最低；browser 的页面桥/CDP 驱动模式现成，缺口集中在驻留状态机与页面控件，工作量可控；Android 未知数最多（手势分解、egui spike、驻留会话、应用内桥接、APK 资产管线），放最后让前两期先把命令集与参数面打磨稳定，避免三平台同时返工。
