# 庭院展台（Courtyard）单场景设计

> **Status:** Implemented 2026-09-28（阶段 0–5 已落地，6–7 部分落地（详见 §12 状态列）；ADR-0031 决策记录，
> Accepted 属 owner 决策。本文档为唯一设计依据；§12 阶段表逐项标注落地状态，
> 与实现冲突处以代码与验收记录为准）
>
> 本文档替代 18 个平铺展示场景的组织方式：合成一个「庭院展台」场景，
> 全部渲染特性作为分区同屏出现；golden 回归采用 ROI 分区双层判定
> （方案 A），不承担整图精确判定。
>
> **交接状态（2026-09-29）：** 阶段 0–5 全部落地、6–7 部分落地（§12 状态列），
> cpu/gpu/docs/browser/android 五条 lane 全绿（明细见 §12 与验收记录），
> 实现已提交等待 owner 验收。续作起点（按优先级）：
> ① canonical 基线捕获——需 Linux/Vulkan 参考宿主，现任基线为 macos-metal 与
> android-vulkan overlay（§12 阶段 3，BLOCKED 记录在案）；
> ② browser 交互 V2（驻留会话 + HTML 参数面板，§11.5/§12 阶段 6）；
> ③ Android 交互 V3（手势分解 + 参数面板 + 应用内桥接，§12 阶段 7）。
> 待 owner 拍板项：ADR-0031 转 Accepted；browser outline-hybrid 的 Dawn/naga
> 分歧是否放宽判定（§12 阶段 4）；夜景曝光偏亮是否修正（§5 夜景段末）。

## 0. 决策记录（已拍板）

| 决策点 | 结论 |
|---|---|
| 场景数量 | 1 个合成场景 `courtyard`，退役现有 18 个展示场景 |
| 回归结构 | 方案 A：全景宽松整图判定 + 分区 ROI 精确判定（Unreal local/global 模式） |
| 中央展示体 | 判定特写用球（基线纯净）；角色台放 reference GLB 做真实效果展示（混合方案） |
| S4 资产门控 | GLB 打包进所有 lane，成为庭院必需依赖；缺席 = BLOCKED，不做空台降级 |
| 默认状态 | day + hull 描边 + crowd=100 + post off + flags rest |

## 1. 调研依据（2026-09-28 调研，关键来源）

**合成场景不承担回归职责是业界共识**：Unity Graphics Tests（一场景一用例一基线；gallery 类 Material Samples 是教学资产，与测试分离）、Unreal（Content Examples 是展示关卡；截图判定用 local/global 双层误差 + 同机位多基线）、three.js/Bevy（200+ 隔离小 example 即展示即测试）、Skia Gold（每天 50 万+ 小图逐张 triage）、Babylon.js（每测试独立 playground + 每像素阈值 + 失败像素比双层门限）。没有任何主流渲染器把核心回归押在少量合成大场景上。

**内容选型有先例**：UTS2/UTS3 sample 场景与本项目特性一一对应（AngelRing=hair 高光、MatCapMask=metal matcap、LightAndShadows=光照对照、Firefly=夜景自发光）；shaderball 统一几何 + 中性灰环境 + 固定机位是行业标准方法论（USD StandardShaderBall）；昼夜切换用光照参数驱动 + 离散预设（Unreal Sun and Sky、Flat Kit presets）；夜景灯阵 = 自发光材质夜间激活（Unity day/night 教程标准做法）。角色特性在角色本体上演示（UTS 的 AngelRing 在 Unity-chan 头部，不在 shaderball 上）——本设计以「球判定 + GLB 展示」混合方式兼顾基线纯净与该惯例。

**资料缺口（如实声明）**：「many small scenes vs one composed scene」无正面辩论文献，结论从各家做法反推；「环形展台」「三分区景深」作为设计教条无直接来源（电影构图原则可迁移，博物馆仅有脊柱动线的间接依据），本文档将其降级为构图手法而非规范引用；米哈游的 demo 场景组织无公开资料。

## 2. 概念与构图手法

环形展示庭院：中央角色材质三对照组（主角），周围展台环各演示一个特性（细节），中轴线引向远景 LOD 轴与实例灯阵（氛围与规模）。防杂乱手法：

1. **同几何、变材质**——材质对照统一用球，差异只来自材质（shaderball 方法论）；
2. **统一展台形制**——展台同一尺寸同一中性灰，是画框不是展品；
3. **三区景深**——前景展台环 / 中景中央平台 / 远景 LOD 轴 + 灯阵（电影构图手法迁移）；
4. **色彩纪律**——场景固有色低饱和，饱和度只出现在被演示的材质上。

## 3. 布局

坐标系：Y 向上，默认相机在南（+Z）望北（−Z）。单位：米。

```
                 北 (−Z)
       · · · 实例灯阵（北半环 r8.5–11，纵深 ≤38m）· · ·
                 [LOD₄ r6.21  d=60]
               [LOD₃ r3.51  d=34]
   [P5 旗帜]      [角色台 (0,0,−8)]       [P1 ramp]
  [P4 光照对照]    [LOD₂ r1.71 d=16.5]   [P2 描边hull]
        [P3 描边hybrid]  [中央平台·三对照组]  [LOD₁ r0.67 d=6.5]
               矮墙 r13（南留门）
              默认相机 (0, 3.4, 16.0)
                 南 (+Z)
```

| 元素 | 位置 | 尺寸 |
|---|---|---|
| 大地面 | 原点中心 | 半径 60 圆盘，深灰，接收阴影 |
| 广场铺装 | 原点中心 | 半径 12 圆盘，抬高 0.02，浅灰（描边可读底） |
| 环形矮墙 | 半径 13 | 高 1.2、厚 0.4，四段弧墙，南留门 |
| 中央平台 | 原点 | 半径 2.2、高 0.3 圆台 |
| 展台环 | 半径 5.5，60° 均布 6 座 | 0.8×0.8×1.0 方柱，顶面 y=1.0 |
| 角色台 | (0, 0, −8) | 半径 1.5、高 0.5 圆台（资产门控区） |
| LOD 轴 | 见 §6 验证坐标表 | 四球递退 |
| 实例灯阵 | 北半环 r8.5–11 网格 | 灯柱 + 灯球，密度参数控制 |

**已验证的相机约束**（2026-09-28 本地计算）：golden 捕获 512×512 为 1:1 画幅，垂直 fov 45° 时水平视角同为 45°。eye=(0, 3.4, 16.0)、look=(0, 1.3, −2) 时展台环全部入镜（侧台边缘 21.95°，半角 22.5°，裕量约 0.5°；实施时若需更稳取 z=17–18）。更早的 eye z=11.5 方案在 1:1 下侧台出画、P2 被裁，已废弃。

## 4. 分区详设

每区标注：内容 → 演示特性 → 回收的旧场景。

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

### P6 · LOD 轴（见 §6）

→ 回收 lod。展台环第 6 座（东北，(2.75, 0, −4.76)）按统一形制建成、暂无展品——「展台是画框不是展品」；LOD 轴的 mid 球从该座与 P1 之间的空档穿出。

### 角色台（北，(0, 0, −8)，资产门控）

加载 reference-scene GLB 角色，面朝南——角色特性在角色本体上的真实效果展示（行业惯例侧）。→ 回收 reference。

**NPR 化（2026-09-28 决策落地）**：角色经 NPR adoption 渲染而非 vanilla `StandardMaterial`——`WorldAssetRoot` 挂 `StylizedMaterialSet`（四个 FixtureMat* 按 glTF 材质名映射到 Character class 描述符，`base_color_factor` 取 GLB 内联 baseColorFactor，double_sided 按 GLB 声明，纹理表为空）；材质集数据归 builder（`labs/scenes/src/builders.rs::courtyard_reference_stage`），adoption 系统（`StylizedAssetIntegrationPlugin`）归各宿主插件装配（showcase headless/interactive、browser-showcase、android-smoke、scene-runtime 证据 App）。动因：vanilla PBR 在 Dawn/Metal 下渲染纯黑（host 限定上游差异，docs/platforms/SUPPORT_MATRIX.md 2026-09-28 条目），且 vanilla 路径本就不满足「真实 NPR 效果展示」意图。labs/reference 的六视角验收合同与 labs/reference 的编译角色管线不受影响（两者是独立合同）。

**S4 决策（2026-09-28 拍板）：GLB 打包进所有 lane。** reference-scene 资产成为庭院的必需依赖，browser lane 走 HTTP asset source、android lane 打包进 APK assets（实施前置项，见 §12 阶段 0）。资产缺席时整个场景构建失败（BLOCKED），不做空台降级（遵循「必需资产缺席是 BLOCKED 不是 PASS」）；该 lane 的庭院捕获整体标记 BLOCKED 而非跳过通过。已知代价：资产管线未就绪的 lane 在窗口期内无庭院覆盖——这是打包完成前的显式状态，不是静默缺口。

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

已验证的夜景修正项：雾是逐片元效果、不染天空，天空色由 zenith_tint 单独写清屏色（`crates/bevy-stylized-render/src/view.rs:258-260`）；现有 night preset 雾色与 zenith 不同色，远景消隐方向与天幕有可见色差——**实施时统一为同一色系后锁定基线**。夜景 non-black 契约安全（zenith 保底 + 灯阵 emission，全图均值远超阈值 1.0）。夜景下 LOD 远球没入雾中（fog end=48 < d=60.6）属预期，LOD ROI 判定只挂 day 机位。PC-5 验证另确认：夜景地面/展台整体曝光偏亮（共享夜景色板的既有行为），与色系统一一并留阶段 3 基线锁定时处理。

## 6. LOD 轴验证坐标

阈值实测 mid=10 / far=25 / very_far=50（`crates/bevy-stylized-core/src/lod.rs:32-40`），距离 = 球心到相机眼点的欧氏长度。相对默认相机 eye=(0, 3.4, 16.0) 的合法坐标（四带各中、离带边界 ≥20% 余量、球随距离放大保持角直径 ≈11.8°）。**下表坐标由 PC-2 实施时重解并锁定**（原表反推自已废弃的 eye z=11.5 方案，near 球实距 10.6m 会落入 mid 带；且原 mid 坐标 (2.2,−4.76) 与东北展台冲突）：

| 带 | 球心 (x, y, z) | 半径 | 实际距离 | 余量 |
|---|---|---|---|---|
| near | (−1.2, 0.67, 9.4) | 0.67 | 7.24 | 27.6%（前景下中部，完整入镜） |
| mid | (−4.2, 1.95, −2.5) | 1.95 | 19.0 | ≥23.9%（西侧，展台环内档） |
| far | (−6.0, 3.51, −18.0) | 3.51 | 34.5 | ≥30.9%（西北墙外） |
| very_far | (−8.0, 6.21, −44.0) | 6.21 | 60.6 | 21.2%（远景） |

单一源是 `labs/scenes/src/metadata.rs` 的 `COURTYARD_LOD_BALLS`（断言覆盖四带落位、余量与无重叠）；far 带起换低模（沿用 `StylizedLodMeshes.band_meshes[2/3]`）。**PC-5 修正记录**（2026-09-28 实机捕获检视）：初版轴穿中轴，far/very_far 在背景叠合遮挡角色台、near 球卡画框左下边缘——轴整体移至西侧轨道。布局变更必须先过断言。

## 7. 相机机位

| 机位 | eye | look | 用途 |
|---|---|---|---|
| panorama（默认） | (0, 3.4, 16.0) | (0, 1.3, −2) | 全景：对照组 + 展台环 + LOD 轴 + 灯阵一框读完 |
| center-trio | **(3.5, 4.0, 4.0)** | **(0, 1.25, −0.2)** | 三对照组特写：face/hair/metal 精确判定（P3 实捕验证锁定：3/4 高位斜视角使 7 球三排分离可读） |
| outline | (4.2, 1.8, 6.8) | (2.75, 1.4, 4.76) | P2 描边判定（只覆盖 P2——P3 不在画框内是初版设计疏漏，P3 由 outline-west 覆盖） |
| outline-west | (−4.2, 1.8, 6.8) | (−2.75, 1.4, 4.76) | P3 hybrid 判定（outline 的镜像，P3 方块堆为主体；PC-5 捕获检视修正） |
| lod-side | **(70, 7, −8)** | **(−4.6, 2.5, −17.3)** | 侧视 LOD 轴（P3 实捕验证锁定：东侧远观，四球档位带 ≥16px 边距并排可读；初版 eye (7,3,−10) 只拍到最近一颗球） |
| flags | **(0, 2.4, −0.4)** | **(−2.75, 2.0, −4.76)** | 蒙皮旗面姿态判定（P3 实捕验证锁定；原 eye (−4.6,2.0,−1.6) 与西移后的 mid LOD 球冲突——eye 位于球体内，整帧被球面占据） |

机位数据单一源是 `labs/scenes/src/metadata.rs` 的 `COURTYARD_CAMERA_PRESETS`（6 个 preset，panorama 居首；←/→ 交互循环与捕获规格引用同表）。

## 8. 参数面

| 参数 | 取值 | 作用域 |
|---|---|---|
| time_of_day | day / dusk / night | view preset |
| outline_mode | off / hull / hybrid | P2/P3 内容 + 相机附件（hybrid 需 Standard） |
| crowd | 0 / 100 / 1000 | 灯阵密度 |
| post | off / on | 相机附件 |
| flags | rest / posed | 蒙皮姿态 |
| face_threshold | 0.0–1.0 / null | 中央前排 face 组 SDF 阈值（沿用现有参数 schema 机制） |

> 实施前置项（M1，**已落地**）：`ParameterKind::Enum(Vec<String>)` 变体已随
> PC-1 加入 catalog（`labs/scenario-model/src/catalog.rs`），含校验与单测；
> §8 的四个枚举参数即以该变体声明。
>
> 参数落点的稳定逻辑 ID（`SceneTarget`）随 builder 定义并进入 catalog 参数
> schema：face 组实例 `trio.face.0..2`、灯阵 `lamps`、旗帜 `flags.*`、
> 视图作用域 `main`；命名在代次内唯一（合同同 SceneTarget 既有约定）。

## 9. 回归结构（方案 A：ROI 分区双层判定）

**原则**：一次渲染，多层判定。整图只抓整体崩坏；精确判定按分区 ROI 独立断言，故障隔离粒度 = 特性级。同一捕获的 tier/参数差异走多基线分桶（Gold「multiple approved images」/ Unreal「Add As Alternative」模式），不放宽容差。

### 捕获矩阵（10 个捕获）

| 捕获 | 机位 × 参数 | 判定 |
|---|---|---|
| courtyard-day | panorama, day, hull, crowd=100, post=off, flags=rest | 整图宽松 + ROI：对照组 / P1 / P4（见下注） |
| courtyard-dusk | panorama, dusk | 整图宽松（时刻覆盖） |
| courtyard-night | panorama, night, crowd=100 | 整图宽松 + ROI：灯阵 emission |
| courtyard-trio | center-trio, day | 整图精确（等效旧 face/hair/metal 基线，含全部 off/on 与阈值状态） |
| courtyard-outline-hull | outline, hull | 整图精确 |
| courtyard-outline-hybrid | outline-west, hybrid（Standard 桶） | 整图宽松 + ROI：p3 精确（见下注；tier 门控捕获） |
| courtyard-lod | lod-side, day | 整图精确（LOD 分带） |
| courtyard-post | panorama, day, post=on | 整图精确（对抗性检视 S1：post 必须有独立基线） |
| courtyard-flags | flags, day, flags=posed | 整图精确（对抗性检视 S2：posed 姿态覆盖） |
| courtyard-stress | panorama, night, crowd=1000 | 整图宽松 + non-black；性能证据走 performance lane |

outline_mode=off 无独立捕获（L1）：hull 宽度 0 的球在 P2 内部分覆盖该状态，如实记录为部分覆盖。

单附件槽约束（实现备注）：`CameraRecipe` 只有一个附件槽，post 与 hybrid 不同时组合（本矩阵本就无此组合）；运行时同时请求时 hybrid 优先（ScreenOutline 先于 PostProcess），已在 builder 注释记录。

### 捕获规格（capture spec）schema

每条捕获是一份数据记录——suites.json、lane 控制文件（`catalog-scene.txt` 的后继）、golden 目录三方都靠它寻址。**捕获 ID 只允许 `-`**（`_`↔`-` 派生约定：browser/android lane 从 `showcase_golden_<id 下划线形>` 测试名反解捕获 ID，见 §10 迁移清单）：

```
capture_id    捕获身份（如 courtyard-day；对应测试名 showcase_golden_courtyard_day）
camera        §7 机位 id（panorama / center-trio / outline / outline-west / lod-side / flags）
parameters    §8 参数集的完整钉值（未钉参数取默认状态 §0；摄入时展开，无运行时隐式默认）
tier          tier 桶（缺省 Portable；Standard 桶只用于 courtyard-outline-hybrid）
judgment      whole_frame_loose | whole_frame_strict | { rois: [区域 id, ...] }
```

10 条捕获规格是 §9 矩阵的机读形式；规格是纯数据，新增捕获不改 harness 代码。

### golden.json 的 regions 扩展（schema 形状）

```jsonc
{
  "fixture": "showcase/courtyard-day",   // 现有字段不变
  "png_fnv1a64": "…", "width": 512, "height": 512,
  "regions": [                            // 新增（additive）
    { "id": "trio", "rect": {"x": 0, "y": 0, "width": 0, "height": 0},
      "max_channel_delta": 0, "max_fail_ratio": 0.0 }
  ]
}
```

`rect` 由分区 AABB 投影自动计算（非手工标定）；`max_channel_delta` / `max_fail_ratio` 在首次基线验收时锁定并写入 acceptance 记录。缺省（无 regions 字段）= 整图判定，与现状兼容。

### ROI 判定规格

- ROI **不手工标定**（M2 修正）：从分区 3D AABB 经捕获相机投影自动计算屏幕矩形，捕获时写入 golden.json 的 `regions` 扩展字段（additive 扩展；现有 golden.json 只有 fixture/hash/尺寸）；harness 断言同一捕获内 ROI 两两不重叠，防止跨区污染判定归属；
- ROI 内精确判定：per-pixel 每通道阈值 + 失败像素比双层门限（Babylon.js/Unity 模式），**阈值按区域分别锁定**（L2：夜景暗部相对噪声更高，不能全局一把尺），具体数值在首次基线验收时写入 acceptance 记录；
- 整图宽松判定：non-black + 全图 luma/chroma 统计包络（抓全黑、整体色差、大面积缺失）；
- 噪声源固定：AA 档位、曝光锁定（Unreal「Disable Noisy Rendering Features」对应做法）；个别持续 noisy 的区域才评估引入 FLIP（`pip install flip-evaluator`），不全局放宽；
- **布局即合同（修正版）**：ROI 定义跟随分区几何自动更新，无需手工重标定；布局变更使基线像素失效，走显式 GOLDEN_REBASELINE。布局冻结前先以临时基线跑通 ROI 机制，正式验收在布局冻结后进行（L3）。

**day 全景 ROI 集修正（P2 实施，2026-09-28，投影证据）**：§9 原列的 6 区在 panorama 机位下不可能两两不重叠——lamps 整环 AABB 投影覆盖全宽并与所有区交叠；p5↔trio 有 1.7px 细缝交叠；p2↔p1 有 30×9px 交叠。故 day 集收敛为 [trio, p1, p4]（投影矩形 (158,238,196×96)/(410,253,102×65)/(0,248,103×70)，两两零交叠，已固化为 harness 锚点测试）；被移出的三区由专属精确捕获覆盖（courtyard-night 的 lamps、courtyard-flags 的 p5、courtyard-outline-hull 的 p2），判定覆盖无丢失。

### 分区 AABB 表（ROI 投影的数据源）

单一源是 `labs/scenes/src/metadata.rs` 的 `COURTYARD_REGION_AABBS`：trio / p1 / p2 / p3 / p4 / p5 / lamps / stage 共 8 区（p3 含 hull 外扩；stage 的 y 上界 2.5m 是角色高度的保守估计）。新增分区 = 加 AABB 数据 + 捕获规格引用，不改 harness 代码。

**hybrid 判定模式修正（P3 验收，2026-09-28）**：hybrid 在 Standard 桶下整图 strict 判定暴露真实噪声——背景灯球/墙沿剪影处逐运行 2–4px 边缘翻转（crowd=0 对照捕获逐字节稳定，确认噪声源是屏幕空间边缘检测对背景深度次序的敏感性，非 P3 主体）。按「个别持续 noisy 区域不全局放宽」原则，hybrid 判定改为整图宽松 + p3 ROI 精确（p3 ROI 逐字节一致）。

**实施陷阱（捕获管线教训）**：蒙皮网格离原点部署时必须覆盖剔除 Aabb 并挂 `NoAutoAabb`——旗面条带的剔除 Aabb 按 bind 姿态（原点）计算，关节移到展台后非原点机位下被视锥剔除；且 `calculate_bounds` 的 `AssetChanged<Mesh3d>` 路径会在网格资产事件后重算覆盖手工 Aabb。回归测试：`flag_strips_carry_overridden_culling_aabb`（scenes）。

### 噪声与播种口径（2026-09-28 验收后锁定）

- **screen-outline 剪影噪声的判定口径**：screen-space 描边在背景灯球/墙沿剪影处存在逐运行 2–4 像素边缘翻转（host 噪声，深度次序竞争经屏幕空间边缘检测放大；Dawn(Tint) 与 wgpu/Metal(naga) 之间另有系统性边缘分歧——2026-09-28 browser lane 实测 2488 edge 像素，与 2026-09-23 记录的 Metal screen-outline 平台差异同类）。**hybrid 相关像素判定一律走「整图宽松 + ROI 精确」与噪声包络，不走整图 strict**——判定模式数据随捕获规格（`courtyard-outline-hybrid` = whole_frame_loose + rois[p3]），lane 工具（`tools/checks/browser.py`、`android_catalog.py`）按 sidecar regions 自动选择判定模式。同族噪声的重配置证据（`labs/scene-runtime` reconfiguration_evidence）以 ≤16 差异像素包络断言。
- **overlay 播种的 regions 携带**：平台 overlay 播种（GOLDEN_REBASELINE）写出 sidecar 时，regions 从既有 overlay sidecar 携带——分区 AABB × 捕获机位投影是**平台无关的规格数据**（矩形在 PC 播种时已由 harness 锚点测试锁定），不因平台重算；阈值继承首锁值（max_channel_delta=2、max_fail_ratio=0.005），跨平台容差分歧走既有 overlay 评审流程，不放宽容差。

### 隔离承诺的边界（如实声明，M3）

ROI 隔离只对**材质/内容局部变化**成立。阴影、雾、全局光向等**跨区效果**穿透 ROI 边界（旗杆影子可落入展台 ROI，光向变化使所有 ROI 同时变化）。旧 `lighting` 场景的侧向 key light 显式对照在庭院里降级为**隐式覆盖**（全场球面都是 NdotL 探针 + 整图宽松判定）——这是相对旧体系的一个真实覆盖性质变化，记录在案。因此故障定位语义是：ROI 红 = 该区域特性疑似；多 ROI 同红 + 整图红 = 优先怀疑跨区因素（光照/雾/视图层）。

### 成本表述（诚实版，M4）

本方案的收益在**组织、浏览与概念层级**，不在渲染成本：18 个微场景（每场 3–4 球）单次渲染成本极低，庭院 10 条捕获含 crowd=1000 夜景等重负载，总 GPU 时间未必低于旧体系。「一次渲染多处判定」省的是**场景切换与进程编排**，不是像素吞吐。

## 10. 旧场景退役映射

| 旧场景 | 去向 | 旧场景 | 去向 |
|---|---|---|---|
| overview | 中央三对照组 | performance | crowd=100 + 中央对照组 |
| ramp | P1 | outline-advanced | P3（Standard 桶） |
| lighting | 时刻参数 + P4 + 全场隐式 NdotL 覆盖（显式→隐式降级，见 §9 隔离承诺边界） | lod | LOD 轴 |
| face / hair / metal | 中央三对照组（保留全部 off/on 与阈值状态） | environment ×3 | time_of_day |
| outline | P2 | effects | P4 |
| animation | P5 旗帜 + courtyard-flags 捕获（posed） | reference | 角色台（GLB 展示，全 lane 打包；S4 决策 2026-09-28） |
| post | courtyard-post 捕获（post=on 独立基线） | stress | courtyard-stress 捕获 |

18 条旧基线（`fixtures/golden/showcase/*.png`）随旧场景退役删除，由 10 个捕获的基线集替代；重捕走 GOLDEN_REBASELINE（需 GPU，缺席记 BLOCKED）；dated 验收记录（`*.acceptance.md`、SUPPORT_MATRIX）按惯例保留不改写，新验收记录追加。

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
| `Capture` | 捕获当前画面 | 走现有捕获路径（严格捕获仍用固定像素 target，与交互画面解耦）；交互模式下落盘到会话 run 输出目录（与 lane 捕获同一合同），文件名带捕获规格与时刻戳 |

### 11.2 参数的运行时语义分层

| 参数 | 语义 | 机制 |
|---|---|---|
| time_of_day | **live**：会话内换 view preset | 新增会话内视图 preset 切换（现状只有构建期 ViewPresetFn；切换时从基线起步应用，与场景切换的视图重置同一语义） |
| flags | **live**：Deform 动作 | 现有 driver 的 apply_deform 路径 |
| face_threshold | **live**：ApplyParams | 现有 entity scope 绑定（driver.rs 已实现） |
| post | **相机附件换**：插入/移除 PostProcess 附件 | 依赖 §11.1 的相机替换 API |
| outline_mode | **内容 + 附件换**：P2/P3 状态 + ScreenOutline 附件 | hybrid 需 Standard tier；不足时按既有 RefusalReason 结构化拒绝（UI 置灰 + 原因），可触发 ADR-0030 会话内升档 |
| crowd | **内容重建**：retire 当前代次 + 重建灯阵 | 走 driver 既有 switch 路径（同场景内重建，generation+1） |

### 11.3 相机统一与分区聚焦

- **修正历史遗留**：窗口相机改为使用场景返回的 CameraRecipe（01:416-417「窗口和离屏不能另补不同机位」），交互画面与捕获画面同源；
- orbit/pan/zoom 共享实现：把 Inspector 的 `OrbitCamera`（yaw/pitch/distance，米制，distance>0.001、pitch±1.5 rad 校验）模式上移为共享层能力，Inspector 与庭院共用；
- **分区聚焦**：点击/触摸展台 → 射线命中分区 → 跳对应特写机位（§7）；中央对照组 → center-trio，P2/P3 → outline，LOD 球 → lod-side，旗帜 → flags，其他 → panorama。前置缺口（A4）：共享层无 mesh 射线检测能力，需引入 bevy_picking 或手写 ray-primitive 命中（分区都是球/盒，手写成本可控）；V1 键位表不依赖本条；

### 11.4 UI 从目录派生

参数 schema（§8，含 M1 的枚举扩展）即控件元数据：`Enum` → 分段按钮/下拉、`F32/OptionalF32` → 滑杆、`Bool` → 开关。控件声明只有这一份；tier 门控参数在能力不足时置灰并显示结构化原因。01:443 的「相同控件元数据用于所有布局」由此落实——三平台布局不同，语义与校验同源。

**最小切片已落地（阶段 7，2026-09-28）**：PC 交互窗口的 egui 叠加层——左下常驻状态条（机位/时刻/outline 分段/post/crowd/flags）、分区图例（9 区，默认展开）、键位帮助（默认折叠）——以静态声明数据实现（tier 门控呈现已生效：hybrid 不足 Standard 时置灰并带原因）。**未落地**：schema 驱动的控件生成（参数面板从 catalog 参数 schema 自动装配分段按钮/滑杆/开关）、跨平台共享控件元数据——维持后续阶段。

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
| ← / → | 机位 preset 循环（panorama → center-trio → outline → lod-side → flags） |
| ↑ / ↓ | time_of_day 循环（day → dusk → night） |
| O / P | outline_mode / post 切换（tier 不足时 stderr 打结构化拒绝） |
| 1 / 2 / 3 | crowd = 0 / 100 / 1000（按数量序助记） |
| Space | flags rest/posed |
| R | ResetView |
| C | Capture（捕获当前画面） |

浏览器与 Android 在 V1 不接交互（浏览器保持一次性捕获、Android 保持 catalog 模式），交互分期见 §12。

### 11.7 跨平台一致性合同

同一命令序列在任意平台产生同一画面语义（平台差异只在输入翻译与显示布局）；命令录制/回放（CommandJournal 扩展到新命令变体）作为跨平台一致性的测试证据；严格捕获继续走固定像素 target，与交互窗口的分辨率/DPI 无关。

## 12. 实施阶段（概要）

| 阶段 | 内容 | 验收门槛 | 状态（2026-09-28） |
|---|---|---|---|
| 0 | 前置项：catalog `ParameterKind` 枚举扩展（M1）；GLB 打包进 browser/android 资产管线（S4）；`SessionCommand` 扩展 + driver 相机配方/附件构建后替换 API（§11.1）；共享 orbit 相机（§11.3） | cpu lane 全绿 + 新单测 | **已落地**（M1 Enum 变体在 PC-1 前置落地；S4 打包 browser 阶段 5 / android 阶段 6 验证；命令层 PC-3、相机统一 PC-4） |
| 1 | 场景构建：courtyard builder（分区 × 参数） | cpu lane + 布局/LOD 落带断言（沿用 metadata.rs 余量测试套路） | **已落地**（PC-2；§0 默认状态 = CourtyardSettings::Default） |
| 2 | ROI 判定扩展：golden.json regions + AABB 投影 + 不重叠断言 + labs/validation 区域 diff | cpu lane + harness 单测 | **已落地**（捕获规格 10 条 + golden.json regions + 区域 diff/整图包络；day ROI 集按投影证据收敛 [trio,p1,p4]） |
| 3 | 基线捕获与验收：10 捕获（GPU，GOLDEN_REBASELINE；布局冻结前先跑临时基线，L3） | gpu lane + acceptance 记录；GPU 缺席 = BLOCKED | **已落地**（macos-metal overlay 10/10 验收 + 两轮捕获修正/美术迭代 + outline-west/hybrid 判定修正；canonical 基线未建 = BLOCKED 记录在案） |
| 4 | 旧场景退役：§10 迁移清单全量同步 | cpu + gpu + browser + android 四条 lane 全绿 | **已落地**（catalog=1、绑定=1、suites.json 18→10、cpu/gpu lane 全绿；browser 9/10 exact + outline-hybrid 平台分歧记录在案；android 10/10 PASS 模拟器） |
| 5 | 交互 V1：PC 键位表 + 窗口相机统一（§11.6、§11.3 第一条） | cpu lane + 交互冒烟 | **已落地**（PC-4 键位翻译层 + 窗口相机统一；P7 egui 叠加层最小切片——状态条/图例/键位帮助 + tier 置灰；C=Capture 维持暂缓；窗口冒烟待用户验证） |
| 6 | 交互 V2：browser 驻留会话 + HTML 参数面板（URL 捕获规格扩展在阶段 4 已落地） | browser-showcase lane + 手动验证 | **部分落地**（`?spec=` 捕获规格合同 + browser lane 判定链已落地并实跑；驻留会话 + HTML 参数面板未做） |
| 7 | 交互 V3：Android 驻留会话 + 手势 + 面板（egui spike 先行）+ 应用内桥接（01:378） | android-catalog lane + spike 结论记录 | **部分落地**（驻留会话模式已落地——跑完不退出 + 触摸事件 logcat 证据；egui 面板 PC 侧已验证装配模式；手势分解/参数面板/应用内桥接维持缺口） |
| 8 | 文档收尾：本文档标注落地，docs/showcase/02 更新，ADR-0031 转 Accepted（Evidence 补齐） | docs lane | **本轮**（ADR-0031 Evidence 已补齐；Accepted 属 owner 决策） |

阶段 1–4 交付「可捕获、可判定、可退役」的庭院；阶段 5–7 交付交互。**捕获与 lane 迁移不依赖交互层**——交互只是同一命令集的另一个驱动源。

## 13. 明确不做

- Inspector 不动（独立角色检视器，自带 provenance hash 机制；阶段 2-7 全程零改动）；
- benchmarks / labs/regression 不动（它们直接复用 builders 的共享 helper，不经 catalog 场景；退役后持续绿——共享 helper（build_strip/build_hero_1/build_portable_instances/ramp_image/matcap_image/ilm_image 等）全部保留并经回归验证）；
- labs/reference 的六视角验收合同不动（独立于展示场景；角色台 NPR 化不影响其编译角色管线）；
- dated 验收/测量记录不改写（新旧条目以追加为准）；
- 不为旧 18 场景 ID 留兼容层或别名（AGENTS.md：只支持当前版本；§10 退役为硬切）。

交互分期的排序依据（V1 PC → V2 browser → V3 Android）：PC 交互宿主已存在、键鼠输入翻译最薄，命令层正确性在这里验证成本最低；browser 的页面桥/CDP 驱动模式现成，缺口集中在驻留状态机与页面控件，工作量可控；Android 未知数最多（手势分解、egui spike、驻留会话、应用内桥接、APK 资产管线），放最后让前两期先把命令集与参数面打磨稳定，避免三平台同时返工。
