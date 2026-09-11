# Gacha Setup v3.6.2 行为契约 v1（Genshin Reference Family）

> **Status:** Draft（pending ADR-0007 / ADR-0012，未经维护者确认前不构成 Oracle 语义）
> **框架：** 本文档按 docs/reference/04-upstream-contract-upgrade-and-golden.md 的行为契约格式填充 v3.6.2 的具体内容。
> **证据：** `tools/blender-reference-export/contracts/expected-signature.genshin-v3.6.2.json`，revision `gacha-setup-v3.6.2-069e51f3fd89`，提取自 PaoloESAN/gacha-setup v3.6.2（commit `069e51f3fd89cdc66b4c2741798ede11a5fb16b5`）包内三个 reference blend，提取环境 Blender 5.2.1 LTS。
> **配套：** 导出侧契约见 docs/reference/06-export-contract-blender-gltf-npr-v1.md。

## 1. 证据能力与边界

签名数据逐组记录：节点组名称、bl_idname、node/link 计数、interface socket（名称/方向/类型）、interface 与 node-types 的 SHA-256。因此：

- 签名**能证明**：某节点组存在、其对外接口（socket 名称/类型/方向）、拓扑摘要未变。
- 签名**不能证明**：节点组内部的具体数学（如描边宽度公式、ramp 采样曲线、tonemap 曲线）、纹理所绑定的具体图片与通道语义。

凡是依赖“不能证明”部分的行为描述，一律标记 **待核实（needs inspection）**，不得凭上游名称猜测语义。

## 2. 行为契约总表

fidelity 标记：`Exact` / `Approximation` / `Intentional Difference` / `Unsupported`（定义见 docs/reference/04-upstream-contract-upgrade-and-golden.md）。

| Reference Area | Upstream Behavior（证据） | bevy-stylized Representation | Fidelity |
|---|---|---|---|
| Shadow Ramp | §3.1 | Ramp/Profile data（authored ramp 纹理 + profile 常量） | Approximation |
| ILM/Lightmap | §3.2 | Control texture decode（ILM 通道语义） | Approximation |
| Face | §3.3 | Face SDF 路径（`_NPR_FACE_SDF` / `_NPR_FACE_SIDE` + face SDF 纹理） | Approximation |
| Normal | §3.4 | Hero/Profile normal path（标准 normal map + 导出切线） | Approximation |
| Metal | §3.5 | Hero metallic module（MatCap） | Approximation |
| Outline | §3.6 | Outline profile（颜色/分类 Approximation；几何生成机制 Intentional Difference） | 混合，见 §3.6 |
| Lighting | §3.7 | Authored Lighting backend（key light + palette） | Approximation（palette）；Intentional Difference（取色轮 rig） |
| Post | §3.8 | Reference/Post profile（bloom 参数化 + display transform） | Approximation |

输入来源词汇（对每个 Exact/Approximation 项记录）：`Texture Channel` / `Vertex Attribute` / `Object/Material Custom Property` / `Profile Constant` / `View/Frame State` / `Derived Instance State`。

## 3. 分域明细

### 3.1 Shadow Ramp — Approximation

上游证据（main 角色签名）：

- 节点组 `Body Shadow Ramp` 与 `Hair Shadow Ramp`（各 3 nodes / 3 links，interface：输入 `Vector`，输出 `Color` + `Alpha`）——ramp 以纹理采样形式存在，采样坐标由外部输入。
- 顶层组 `HoYoverse - GI Body`（201 nodes / 344 links）的 `------ RAMPS OPTIONS ------` 段 socket：`Use Shadow Ramp`、`Shadow Ramp Width`、`Shadow Transition Range` 1–5、`Shadow Transition Softness` 1–5、`Daytime Shadow Color` 1–5、`Nighttime Shadow Color` 1–5——即 5 档材质分类 × 昼夜双套阴影色 + 每档过渡范围/软度。
- metadata_map：`Use Shadow Ramp` → `bevy_stylized.genshin.body.use_shadow_ramp`；`Shadow Ramp Width` → `bevy_stylized.genshin.body.shadow_ramp_width`；`Global Material Properties.Day/Night` → `bevy_stylized.genshin.global.day_night`。

bevy-stylized 表示：ramp 纹理 + 每档过渡参数进入 profile 常量；昼夜切换由 view/frame 状态驱动。

- 输入来源：`Texture Channel`（ramp 纹理）、`Profile Constant`（metadata_map 常量）、`View/Frame State`（Day/Night）。
- 待核实（needs inspection）：`Body Shadow Ramp` 的具体采样坐标构造与昼夜色混合公式（签名只证明接口，内部数学未提取）。

### 3.2 ILM/Lightmap — Approximation

上游证据：

- `HoYoverse - GI Body` 输入 socket `Lightmap Color`（NodeSocketColor）与 `Lightmap Alpha`（NodeSocketFloat），以及 `Use Lightmap AO`、`Use Vertex Color AO` 开关。
- 节点组 `Genshin Material IDs - Color` 与 `Genshin Material IDs - Float`（各 39 nodes / 48 links）：以 `Lightmap Alpha` + `Use Color/Float 2–5` 在 5 套材质常量间选择——即 lightmap 的某通道承担 material-class 选择语义。
- 节点组 `Color Multipliers`（10 nodes / 18 links）：同样以 `Lightmap Alpha` + `Use Color 2–5` 选择 `Color 1–5`。

bevy-stylized 表示：ILM/control 纹理按声明的 Texture Semantic（Discrete Classification 不得因压缩漂移，见 docs/authoring/03-authoring-schema-runtime-packing.md）解码；5 档材质分类映射 profile 常量组。

- 输入来源：`Texture Channel`（lightmap 各通道）、`Profile Constant`（5 档常量）。
- 待核实（needs inspection）：lightmap R/G/B/A 各通道的精确语义分配（AO / specular 强度 / material id / ramp 偏移的具体通道归属）；签名只证明 `Lightmap Alpha` 参与材质分类。

### 3.3 Face — Approximation

上游证据：

- 顶层组 `HoYoverse - GI Face`（45 nodes / 76 links）：输入 `[Loli/Boy/Girl/Male/Lady]`（原型选择）、`Face Blush Color`、`Face Blush Strength`、`Face Shadow Softness`、3 套昼夜阴影色。
- 节点组 `Face Factor`（144 nodes / 164 links）：输入 `[Loli/Boy/Girl/Male/Lady]`、`Lit Fac`、`Shaded Fac`、`Softness`、`Shadow Position`、`Catch Shadows`；输出 `Soft Factor`、`Factor`、`Shadow Alpha`。
- 节点组 `Face Shadow`（3 nodes）：`Vector` → `Face Shadow Color` + `Face Shadow Alpha`（阈值图采样形态）。
- 节点组 `Genshin Face Lighting`（14 nodes）：`Lit Factor` / `Sharp Lit Factor` + ambient/sharp/soft 光色与阴影色。
- GeometryNodeTree `Light Vectors`（23 nodes / 46 links）：输出 `lightDir`、`headForward`、`headUp`——上游用场景物体（Head Origin / Head Forward / Head Up）传递头部方向，即 doc 04 所称 direction transfer。
- metadata_map：`[Loli/Boy/Girl/Male/Lady]` → `bevy_stylized.genshin.face.archetype`；`Face Blush Color` → `bevy_stylized.genshin.face.blush_color`；`Face Shadow Softness` → `bevy_stylized.genshin.face.shadow_softness`。

bevy-stylized 表示：Face SDF 路径——`_NPR_FACE_SDF` / `_NPR_FACE_SIDE` 顶点属性（见 docs/reference/06-export-contract-blender-gltf-npr-v1.md §4）或 face SDF 纹理 fallback；头部前向/上向由 runtime 骨骼/实例状态提供，不依赖场景标记物体。

- 输入来源：`Texture Channel`（face SDF/阈值图）、`Vertex Attribute`（`_NPR_FACE_SDF`、`_NPR_FACE_SIDE`）、`Profile Constant`（archetype/blush/softness）、`Derived Instance State`（头部朝向）。
- 待核实（needs inspection）：上游 `Face Shadow` 阈值图与 bevy-stylized face SDF 的数学对应关系；`Face Factor` 内部 144 节点的合成公式。

### 3.4 Normal — Approximation

上游证据：

- 节点组 `Dot Creation`（38 nodes / 41 links）：输入 `Normal Map`、`Normal Map Strength`、`Offset`；输出 `Normal`、`NdotV`、`NdotL`、`NdotH`——所有光照/描边/镜面共享同一组点积。
- 节点组 `Optimized Normal Map`（31 nodes）：`Normal Map` + `Strength` → `Normal` 与专用输出 `For NdotV only`。
- 节点组 `Recalculate Z-Coord of Expanded Normal Map`（10 nodes）：`Expanded Normal Map` → 重建 Z 分量——即上游存在 packed/expanded normal 行为。

bevy-stylized 表示：Hero/Profile normal path——标准切线空间 normal map；导出合同强制 tangents（docs/reference/06-export-contract-blender-gltf-npr-v1.md §3），保证切线基确定。

- 输入来源：`Texture Channel`（normal map）、`Profile Constant`（strength）。
- 待核实（needs inspection）：`Expanded Normal Map` 的打包格式（通道布局、Z 重建公式）；`For NdotV only` 与完整 `Normal` 的差异优化内容。

### 3.5 Metal — Approximation

上游证据：

- 节点组 `Genshin Metallic`（41 nodes / 49 links）：输入 `Metallic Factor`、`Metallic Brightness`、`Metallic Map Tile Scale`、`Metallic Dark/Light Color`、`Metallic Shadow Multiply Color`、`Lit Factor`、`Normal`。
- `HoYoverse - GI Body` 的 `------ METALLIC OPTIONS ------` 段：`Enable Metallics?`、`Metallic Matcap Brightness`、`Metallic Matcap Tile Scale`、`Metallic Matcap Dark/Light Color`、`Metallic Matcap Shadow Multiply Color`，及 `MT Specular Color`、`MT Sharp Layer Color`、`Metallic Specular Shininess/Scale/Attenuation/Sharp Layer Offset`。
- 节点组 `Genshin Specular`（79 nodes / 87 links）：`Use Specular Ramp?` 开关、non-metallic（Color/Intensity/Multiplier/Shininess）与 metallic（Color/Sharp Layer Color/Intensity/Shininess/Scale/Attenuation/Sharp Layer Offset）双路参数；输入含 `NdotH`、`Lightmap Color`、`Metallic Factor`。
- metadata_map：`Enable Metallics?` → `bevy_stylized.genshin.body.enable_metallics`；`Use Specular Ramp` → `bevy_stylized.genshin.body.use_specular_ramp`。

bevy-stylized 表示：Hero metallic module——MatCap 纹理 + 暗/亮色 + shadow multiply；镜面双路并入 spec ramp / 常量参数。

- 输入来源：`Texture Channel`（MatCap、spec ramp、lightmap）、`Profile Constant`（metadata_map 常量）。
- 待核实（needs inspection）：MatCap 采样坐标（view-space normal 直接采样还是有额外变换）；`Metallic Specular Attenuation` 的作用点。

### 3.6 Outline — 混合（颜色/分类 Approximation；几何机制 Intentional Difference）

上游证据（两层机制）：

1. **Shader 层**（main 角色）：节点组 `Genshin Outlines`（9 nodes / 22 links）与顶层组 `HoYoverse - GI Outlines`（7 nodes / 22 links）：`Outline Color 1–5`（按 `Use Material 2–5` 分类选择）、`Gamma`、`Ambient/Light Colour` 参与调色；顶层 outline 材质另有 `Outline Tint`、`Outline Saturation`、`Outline Intensity`、`Use Face Shader`。
2. **几何层**（outline 角色签名）：GeometryNodeTree `HoYoverse - Genshin Impact Outlines`（115 nodes / 133 links）：输入 `Outline Thickness`、`Screen Space`（Bool）、`Z-Depth Offset`、`Use Vertex Colors?`、`Camera`，按 Hair/Body/Face/Dress/Dress 2/Other 六类材质分别指定本体与 Outline 材质，另有 `Clamp Min/Max Distance` + 距离倍率。

metadata_map：`Outline Color 1` → `bevy_stylized.genshin.outline.color_1`；`Outline Intensity` → `bevy_stylized.genshin.outline.intensity`；`Global Material Properties` 的 `Outline Tint/Saturation/Intensity` → `bevy_stylized.genshin.global.outline_*`。

bevy-stylized 表示：

- 颜色/分类/调色语义 → Outline profile 常量（**Approximation**）。
- 几何生成机制 → **Intentional Difference**：runtime 不复刻上游 GN 修改器，采用 bevy-stylized 自有 outline 架构（V1 基线为 hull outline，方向见 ADR-0008 草案）；导出合同因此禁止 runtime 网格携带该 GN modifier（docs/reference/06-export-contract-blender-gltf-npr-v1.md §2）。

- 输入来源：`Profile Constant`（outline 颜色/强度/分类）、`Vertex Attribute`（顶点色描边权重，对应 `Use Vertex Colors?`）、`View/Frame State`（camera 距离缩放）。
- 待核实（needs inspection）：上游 `Outline Thickness` 的具体宽度公式与 `Screen Space` 模式的屏幕空间等宽实现；`Clamp Min/Max Distance` 的曲线形状。

### 3.7 Lighting — Approximation（palette）/ Intentional Difference（取色轮 rig）

上游证据：

- 节点组 `Genshin Lighting`（15 nodes）与 `Genshin Face Lighting`（14 nodes）：`Lit Factor` + `Ambient Colour`、`Sharp/Soft Light Colour`、`Sharp/Soft Shadow Color`——sharp/soft 双档光色与双档阴影色的 palette 语义。
- 节点组 `Apply Light Modifications`（7 nodes）：`Tint`、`Saturation`、`Intensity` 后处理光色。
- 节点组 `Global Material Properties`（15 nodes，纯输出接口）：集中输出 `Ambient Colour`、`Sharp/Soft Lit Colour`、`Sharp/Soft Shadow Colour`、`Shadow Position`、`Catch Shadows`、`Day/Night`、`Rim Lit/Shadow`、`Outline Tint/Saturation/Intensity`、Fresnel 参数——上游的全局 palette 中心。
- GeometryNodeTree `Light Vectors`：从场景物体（`Light Direction` object + 9 组 `* Color Wheel` / `* Color Picker` 对象 + Rim 尺寸滑块）推导 `lightDir` 与全部 palette 色——Blender-only 的交互式取色 rig。
- 附加效果组：`Genshin Fresnel`（13 nodes，`NdotV` + `Use Fresnel/Color/Power/Scale`）、`Depth-based Rim`（26 nodes，`Tint Lit/Tint Shadow` + `Scale`）、`Set Depth`（25 nodes）。
- metadata_map：`Ambient Colour` → `bevy_stylized.genshin.global.ambient_color`；`Use Fresnel`/`Fresnel Color`/`Fresnel Power`/`Fresnel Scaler` → `bevy_stylized.genshin.global.fresnel_*`；`Rim Lit`/`Rim Shadow` → `bevy_stylized.genshin.global.rim_*`。

bevy-stylized 表示：Authored Lighting backend——NPR key light + global palette 作为 view/frame 数据；取色轮/滑块 rig 为 Blender authoring-time 装置，**不移植**（Intentional Difference），其产出值经 metadata_map 进入 profile。

- 输入来源：`Profile Constant`（palette）、`View/Frame State`（key light 方向、Day/Night）。
- 待核实（needs inspection）：`Genshin Lighting` 内 sharp/soft 两档的合成权重；`Depth-based Rim` 对深度缓冲的依赖方式（EEVEE 特定）。

### 3.8 Post — Approximation

上游证据（post 角色签名，CompositorNodeTree）：

- 顶层组 `HoYoverse - Post Processing`（4 nodes / 6 links）：输入 `Bloom Threshold`、`Bloom Intensity`、`Bloom Size`。
- 节点组 `Genshin Bloom`（25 nodes / 34 links）：同样三个参数。
- 节点组 `Genshin Tonemapper`（18 nodes / 17 links）：仅 `Image` 输入输出，无参数接口。
- `Compositing Nodetree`（post blend 内 5 nodes / 4 links）。
- Reference 色彩管理观测值：display_device=sRGB、view_transform=Standard、view_look=None、sequencer_colorspace=sRGB（记录于 docs/reference/reference-toolchain.toml 与各 manifest）。

bevy-stylized 表示：Reference/Post profile——bloom 以 Threshold/Intensity/Size 三参数参数化；display transform 对齐 sRGB/Standard 基线。

- 输入来源：`Profile Constant`（bloom 三参数）、`View/Frame State`（display transform 选择）。
- 待核实（needs inspection）：`Genshin Tonemapper` 内部曲线（18 节点，无参数接口，需逐节点检查）；`Genshin Bloom` 的核形状与 `Size` 的具体含义。

### 3.9 V1 明确 Unsupported（超出 8 域，依据签名中的实际存在）

- **眼部 / 瞳孔**：main 角色 required materials 含 `HoYoverse - Genshin Pupil`、`HoYoverse - Genshin New Pupil`、`HoYoverse - Genshin Sandrone pupil`，且存在节点组 `PupilaTodo`（21 nodes / 28 links）。V1 的 8 个 Reference Area 不含眼部域 → V1 **Unsupported**，留待后续版本评估。
- **Alpha/Emission 通道切换**：节点组 `Genshin Alpha`（57 nodes，`0 = Emission, 1 = Transparency`、`Transparency Cutoff`、`Emission Tint/Strength`）与 `HoYoverse - GI Body` 的 `------ ALPHA OPTIONS ------` 段。V1 以 `_NPR_EMISSION` 与标准 glTF alpha mode 承载其子集；完整切换语义 **待核实（needs inspection）**，未核实前不作为 Exact 承诺。

## 4. metadata_map（上游输入 → `bevy_stylized.genshin.*`）

正本见签名 JSON 的 `metadata_map.main`（20 条）；outline / post 角色的 map 当前为空（待补充）。汇总：

| 上游 Group.Socket | bevy_stylized key |
|---|---|
| `Global Material Properties.Day/Night` | `bevy_stylized.genshin.global.day_night` |
| `Global Material Properties.Ambient Colour` | `bevy_stylized.genshin.global.ambient_color` |
| `Global Material Properties.Use Fresnel` | `bevy_stylized.genshin.global.use_fresnel` |
| `Global Material Properties.Fresnel Color` | `bevy_stylized.genshin.global.fresnel_color` |
| `Global Material Properties.Fresnel Power` | `bevy_stylized.genshin.global.fresnel_power` |
| `Global Material Properties.Fresnel Scaler` | `bevy_stylized.genshin.global.fresnel_scaler` |
| `Global Material Properties.Outline Tint` | `bevy_stylized.genshin.global.outline_tint` |
| `Global Material Properties.Outline Saturation` | `bevy_stylized.genshin.global.outline_saturation` |
| `Global Material Properties.Outline Intensity` | `bevy_stylized.genshin.global.outline_intensity` |
| `Global Material Properties.Rim Lit` | `bevy_stylized.genshin.global.rim_lit_color` |
| `Global Material Properties.Rim Shadow` | `bevy_stylized.genshin.global.rim_shadow_color` |
| `HoYoverse - GI Body.Use Shadow Ramp` | `bevy_stylized.genshin.body.use_shadow_ramp` |
| `HoYoverse - GI Body.Shadow Ramp Width` | `bevy_stylized.genshin.body.shadow_ramp_width` |
| `HoYoverse - GI Body.Enable Metallics?` | `bevy_stylized.genshin.body.enable_metallics` |
| `HoYoverse - GI Body.Use Specular Ramp` | `bevy_stylized.genshin.body.use_specular_ramp` |
| `HoYoverse - GI Face.[Loli/Boy/Girl/Male/Lady]` | `bevy_stylized.genshin.face.archetype` |
| `HoYoverse - GI Face.Face Blush Color` | `bevy_stylized.genshin.face.blush_color` |
| `HoYoverse - GI Face.Face Shadow Softness` | `bevy_stylized.genshin.face.shadow_softness` |
| `HoYoverse - GI Outlines.Outline Color 1` | `bevy_stylized.genshin.outline.color_1` |
| `HoYoverse - GI Outlines.Outline Intensity` | `bevy_stylized.genshin.outline.intensity` |

注意：这些 key 是**导出桥产出的 namespaced metadata 键**，其 profile schema 归属（`[profile].genshin_schema`）在 docs/reference/reference-toolchain.toml 中仍 PENDING；key 集合本身不构成 schema 承诺。

## 5. 待核实清单（汇总，升级与 Golden 工作前须逐条关闭或显式接受）

1. Shadow ramp 采样坐标构造与昼夜色混合公式（§3.1）。
2. Lightmap R/G/B/A 通道语义分配（§3.2）。
3. Face 阈值图 ↔ face SDF 的数学对应；`Face Factor` 合成公式（§3.3）。
4. `Expanded Normal Map` 打包格式与 Z 重建（§3.4）。
5. MatCap 采样坐标与 `Metallic Specular Attenuation` 作用点（§3.5）。
6. Outline 宽度公式 / `Screen Space` 实现 / 距离 clamp 曲线（§3.6）。
7. `Genshin Lighting` sharp/soft 合成权重；`Depth-based Rim` 深度依赖（§3.7）。
8. `Genshin Tonemapper` 曲线；`Genshin Bloom` 核与 `Size` 含义（§3.8）。
9. `Genshin Alpha` emission/transparency 完整切换语义（§3.9）。

待核实条目的核查方式：在锁定的 Blender 5.2.1 LTS 上打开 reference blend 逐节点检查，结果按 docs/reference/04-upstream-contract-upgrade-and-golden.md 的流程回填本表并提升合同修订号。

---
