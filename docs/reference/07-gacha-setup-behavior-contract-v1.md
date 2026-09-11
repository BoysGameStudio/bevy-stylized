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

**2026-09-12 补充**：`tools/blender-reference-export/inspect_node_math.py` 已将三个 reference blend 全部节点组的内部数学（节点类型/参数/socket 默认值/逐 link 连接）全量转储为 JSON，§3 各域的“待核实”条目已据此逐条核实（见 §3 各节与 §5）。注意该转储是一次性取证手段，**不**进入签名/导出合同——上游升级时仍以签名 diff 触发复核，内部数学变更须按 docs/reference/04-upstream-contract-upgrade-and-golden.md 的流程重新核实并提升本文档修订号。

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
- ~~待核实（needs inspection）~~ **已核实（2026-09-12，逐节点转储，`tools/blender-reference-export/inspect_node_math.py`）**：
  - `Body Shadow Ramp` / `Hair Shadow Ramp` 组本体是**纯纹理采样器**（`Group Input.Vector` → `ShaderNodeTexImage`（label `Body Shadow Ramp (sRGB) (Channel Packed)`）→ `Color`/`Alpha`），坐标完全由外部构造。**注意：vendor blend 中这些纹理节点的 image 为 None**——ramp 贴图由 Setup Wizard 在导入角色时注入（运行时数据，非 shader 本体内容）。
  - 采样坐标（`HoYoverse - GI Body` 内，以 Day 为例）：X = `1 − (ShadowPosition − LitFactor_AO) / (ShadowPosition × (vcol_G × 2) × ShadowRampWidth)`，链：`Ramps Subtract -- NdotL`(=`Shadow Position` − `Light Mix -- AO`) → `Ramps Divide -- Shadow Pos` → `Ramps Divide -- Shadow Width`(÷ `Ramps Multiply -- Shadow Width` = `Ramps Green.Green`（顶点色层 `Col` 的 G 通道 × 2.0）× `Shadow Ramp Width`) → `Ramps Subtract -- Shadow Width`(1−x) → `Ramps Combine XY -- Day.X`。其中 `Light Mix -- AO` = lerp(0, NdotL×0.5+0.5, `Smoothstep -- AO`(edge0=0.01, edge1=0.4))——**光照项为 half-lambert（`Multiply -- NdotL` ×0.5 → `Add -- NdotL` +0.5），并经 AO 门控**。
  - 采样 Y（行选择）= `0.05 + 0.1×((6 − RampID) − 1)`，Night 直接用该值（贴图下半行），Day = Night + 0.5（贴图上半行）；RampID ∈ {1..5} 由 `Ramp IDs`（`Genshin Material IDs - Float` 实例）从 `Lightmap Alpha` 阈值分带选择：≤0.2→1，(0.2,0.4]→2，(0.4,0.6]→3，(0.6,0.8]→4，>0.8→5（`>=`/`<=`/`&&` 辅助组，带 `Use Material 2–5` 开关）。
  - Body/Hair 与昼夜混合：`Ramps Mix Day` = lerp(`Ramps Day Body`, `Ramps Day Hair`, `Body / Hair`)，`Ramps Mix Night` 同理；`Light Mix -- Ramps` = lerp(Day, Night, `Day/Night`)。
  - **异常发现**：`HoYoverse - GI Body` 内的 `Ramps Cast Shadows` 节点在 Blender 5.2.1 加载为 **NodeUndefined**（原类型无法解析；同类还有 Face Factor 的 `Shader Info.002`（输出 `Cast Shadows`）与 `Depth-based Rim` 的 `Curvature` 节点）——cast-shadow 调制路径在锁定环境中为死路径（输出默认 0），It-1b 捕获的 upstream Golden 亦反映该状态。

### 3.2 ILM/Lightmap — Approximation

上游证据：

- `HoYoverse - GI Body` 输入 socket `Lightmap Color`（NodeSocketColor）与 `Lightmap Alpha`（NodeSocketFloat），以及 `Use Lightmap AO`、`Use Vertex Color AO` 开关。
- 节点组 `Genshin Material IDs - Color` 与 `Genshin Material IDs - Float`（各 39 nodes / 48 links）：以 `Lightmap Alpha` + `Use Color/Float 2–5` 在 5 套材质常量间选择——即 lightmap 的某通道承担 material-class 选择语义。
- 节点组 `Color Multipliers`（10 nodes / 18 links）：同样以 `Lightmap Alpha` + `Use Color 2–5` 选择 `Color 1–5`。

bevy-stylized 表示：ILM/control 纹理按声明的 Texture Semantic（Discrete Classification 不得因压缩漂移，见 docs/authoring/03-authoring-schema-runtime-packing.md）解码；5 档材质分类映射 profile 常量组。

- 输入来源：`Texture Channel`（lightmap 各通道）、`Profile Constant`（5 档常量）。
- ~~待核实（needs inspection）~~ **已核实（2026-09-12，逐节点转储）**，lightmap 通道语义分配（`HoYoverse - GI Body` 内 `Group Input.Lightmap Color` / `Lightmap Alpha` 的全部下游消费）：
  - **R → 金属遮罩**：`Metallics Red`（ShaderNodeSeparateColor）→ `Metallics < -- 0.9`（GREATER_THAN，阈值 0.9）。
  - **G → AO**：`AO Green` / `AO Green.001`（SeparateColor.Green）→ `AO Mix -- Lightmap`（factor = `Use Lightmap AO`）；另 `Light Green.Green` → `Light <`（GREATER_THAN，阈值 0.95）参与光照门控。
  - **B → 高光抑制**：`Genshin Specular` 内 `Separate RGB.001.Blue` → `Math`(SUBTRACT：1.03 − B) 作为高光强度乘性因子；同组 `Separate RGB.002.Red`（R）亦乘入 non-metallic 高光支路（`Math.007`）。
  - **A → 材质分带 ID**：`Lightmap Alpha` → 各 `* IDs`（`Genshin Material IDs - Color/Float` 实例），阈值分带见 §3.1。
  - 补充：**顶点色层 `Col`**（ShaderNodeVertexColor，layer_name="Col"）——R 通道 = vertex AO（`AO Col` → `AO sRGB -> Linear` → `AO Red` → `AO Mix - -Vertex`，factor = `Use Vertex Color AO`），G 通道 = ramp 阴影宽度缩放（`Ramps Col` → `Ramps Green`，见 §3.1）。

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
- ~~待核实（needs inspection）~~ **已核实（2026-09-12，逐节点转储）**：
  - `Face Shadow` 组为纯阈值图采样器（`ShaderNodeTexImage`，image=`Avatar_Tex_Face_Shadow.png`，label `Face Shadow (Non-Color) (Channel Packed)`）。
  - `Face Factor` 内部公式：`Face_Lightmap[.001–.004]` 五张原型 SDF 阈值图（Loli/Boy/Girl/Male/Lady，对应 5 个 `Avatar_*_Tex_FaceLightmap.png`）经 `Mix.003/004/005` 链按 archetype 阈值（GREATER_THAN）选择；方向项由 `headright = normalize(cross(headForward, headUp))`（`矢量运算.001`/`矢量运算.002`）+ 光向归一化 + DOT_PRODUCT（`Vector Math.001` 等）+ `Flip Lightmap dir`（左右翻转）构成；阈值比较为 **smoothstep(center−Softness, center+Softness, sdf)**（`Group.002`=`Smoothstep` 组，edge0/edge1 = `Math.020`/`Math.021` = center±`Softness`）——即"方向值 vs SDF 图阈值"的经典 Genshin 脸部阴影，与 bevy-stylized face SDF 路径数学同构（softstep 阈值比较），对应关系成立。
  - `Shadow Alpha` 由 `Face_Shadow` 纹理 **G 通道**（`Separate RGB.Green`）× `Map Range`（From 0.71–0.8 → To 0–1，LINEAR）调制。
  - 注意：`Catch Shadows` 支路经 `Shader Info.002`（输出 `Cast Shadows`）——该节点在 Blender 5.2.1 为 NodeUndefined，支路为死路径（见 §3.1 异常发现）。

### 3.4 Normal — Approximation

上游证据：

- 节点组 `Dot Creation`（38 nodes / 41 links）：输入 `Normal Map`、`Normal Map Strength`、`Offset`；输出 `Normal`、`NdotV`、`NdotL`、`NdotH`——所有光照/描边/镜面共享同一组点积。
- 节点组 `Optimized Normal Map`（31 nodes）：`Normal Map` + `Strength` → `Normal` 与专用输出 `For NdotV only`。
- 节点组 `Recalculate Z-Coord of Expanded Normal Map`（10 nodes）：`Expanded Normal Map` → 重建 Z 分量——即上游存在 packed/expanded normal 行为。

bevy-stylized 表示：Hero/Profile normal path——标准切线空间 normal map；导出合同强制 tangents（docs/reference/06-export-contract-blender-gltf-npr-v1.md §3），保证切线基确定。

- 输入来源：`Texture Channel`（normal map）、`Profile Constant`（strength）。
- ~~待核实（needs inspection）~~ **已核实（2026-09-12，逐节点转储）**：
  - `Recalculate Z-Coord of Expanded Normal Map`：Z = `sqrt(1 − X² − Y²)`（`Math.002`/`Math.003` POWER 2 → `Math.001`(1−…) + `Math.004` ADD → `Math.005` SQRT → `Combine XYZ.Z`）——即 XY 双通道打包、Z 重建的标准 expanded/packed normal 格式。
  - `Optimized Normal Map`：`Influence.001` = lerp((0.5,0.5,1), 贴图 texel, `Strength`) → ×2−1（`Vector Math.004` SCALE 2.0 → `Vector Math.003` SUBTRACT (1,1,1)）→ 以 `Tangent`/`Bi-Tangent`/`Normal` 三节点手工组装 TBN 矩阵（三个 Separate XYZ + 三个 Combine XYZ + 三个 DOT_PRODUCT）变换到世界空间 → `Mix`（factor=`Geometry.Backfacing`）做背面翻转（×−1）。输出差异：**`Normal` = 背面翻转后**，**`For NdotV only` = 翻转前**（`Reroute.001` 直接引出）——优化内容即 NdotV 不需要背面修正。

### 3.5 Metal — Approximation

上游证据：

- 节点组 `Genshin Metallic`（41 nodes / 49 links）：输入 `Metallic Factor`、`Metallic Brightness`、`Metallic Map Tile Scale`、`Metallic Dark/Light Color`、`Metallic Shadow Multiply Color`、`Lit Factor`、`Normal`。
- `HoYoverse - GI Body` 的 `------ METALLIC OPTIONS ------` 段：`Enable Metallics?`、`Metallic Matcap Brightness`、`Metallic Matcap Tile Scale`、`Metallic Matcap Dark/Light Color`、`Metallic Matcap Shadow Multiply Color`，及 `MT Specular Color`、`MT Sharp Layer Color`、`Metallic Specular Shininess/Scale/Attenuation/Sharp Layer Offset`。
- 节点组 `Genshin Specular`（79 nodes / 87 links）：`Use Specular Ramp?` 开关、non-metallic（Color/Intensity/Multiplier/Shininess）与 metallic（Color/Sharp Layer Color/Intensity/Shininess/Scale/Attenuation/Sharp Layer Offset）双路参数；输入含 `NdotH`、`Lightmap Color`、`Metallic Factor`。
- metadata_map：`Enable Metallics?` → `bevy_stylized.genshin.body.enable_metallics`；`Use Specular Ramp` → `bevy_stylized.genshin.body.use_specular_ramp`。

bevy-stylized 表示：Hero metallic module——MatCap 纹理 + 暗/亮色 + shadow multiply；镜面双路并入 spec ramp / 常量参数。

- 输入来源：`Texture Channel`（MatCap、spec ramp、lightmap）、`Profile Constant`（metadata_map 常量）。
- ~~待核实（needs inspection）~~ **已核实（2026-09-12，逐节点转储）**：
  - MatCap 采样坐标**不是**裸 view-space normal，而是显式重建相机基：`up = normalize(WORLD(CAMERA(0,1,0)) − WORLD(CAMERA(0,0,0)))`（`Vector Transform.002`/`Vector Transform.001`，POINT CAMERA→WORLD），`right = normalize(cross(Geometry.Incoming, up))`（`Vector Math.007`/`Vector Math.008`），UV = 法线与该基的点积经 `Combine XYZ.001` → ×(`Metallic Map Tile Scale`, 1, 1)（`Vector Math.011`）→ ×0.5 + 0.5（`Vector Math.012`/`Vector Math.013`）→ 采样 `MetalMap`（image 由 wizard 注入；label `Metallic Matcap (sRGB) (Channel Packed)`）。语义上等价于 view-space normal 的 xy 映射 + tile 缩放（Approximation 成立），但实现为手工基向量投影。
  - `Metallic Specular Attenuation` 作用点：`Genshin Specular` 内 `Group Input.Metallic Specular Attenuation` → `Reroute.026/027` → `Mix.009`（blend=MULTIPLY，factor = `Math.014` = `Lit Factor` 减 `Sharp Layer Offset` 链）——即**对 metallic sharp-layer 高光项的乘性衰减，按 lit factor 门控**。

### 3.6 Outline — 混合（颜色/分类 Approximation；几何机制 Intentional Difference）

上游证据（两层机制）：

1. **Shader 层**（main 角色）：节点组 `Genshin Outlines`（9 nodes / 22 links）与顶层组 `HoYoverse - GI Outlines`（7 nodes / 22 links）：`Outline Color 1–5`（按 `Use Material 2–5` 分类选择）、`Gamma`、`Ambient/Light Colour` 参与调色；顶层 outline 材质另有 `Outline Tint`、`Outline Saturation`、`Outline Intensity`、`Use Face Shader`。
2. **几何层**（outline 角色签名）：GeometryNodeTree `HoYoverse - Genshin Impact Outlines`（115 nodes / 133 links）：输入 `Outline Thickness`、`Screen Space`（Bool）、`Z-Depth Offset`、`Use Vertex Colors?`、`Camera`，按 Hair/Body/Face/Dress/Dress 2/Other 六类材质分别指定本体与 Outline 材质，另有 `Clamp Min/Max Distance` + 距离倍率。

metadata_map：`Outline Color 1` → `bevy_stylized.genshin.outline.color_1`；`Outline Intensity` → `bevy_stylized.genshin.outline.intensity`；`Global Material Properties` 的 `Outline Tint/Saturation/Intensity` → `bevy_stylized.genshin.global.outline_*`。

bevy-stylized 表示：

- 颜色/分类/调色语义 → Outline profile 常量（**Approximation**）。
- 几何生成机制 → **Intentional Difference**：runtime 不复刻上游 GN 修改器，采用 bevy-stylized 自有 outline 架构（V1 基线为 hull outline，方向见 ADR-0008 草案）；导出合同因此禁止 runtime 网格携带该 GN modifier（docs/reference/06-export-contract-blender-gltf-npr-v1.md §2）。

- 输入来源：`Profile Constant`（outline 颜色/强度/分类）、`Vertex Attribute`（顶点色描边权重，对应 `Use Vertex Colors?`）、`View/Frame State`（camera 距离缩放）。
- ~~待核实（needs inspection）~~ **已核实（2026-09-12，逐节点转储 outline 角色的 GN 组 `HoYoverse - Genshin Impact Outlines`）**：
  - 宽度公式：`offset = Normal × (Outline Thickness × 0.01) × vcol × distFactor`（`Math.001` MULTIPLY(thickness, 0.01) → `Vector Math.008/007/002` 链 → `Set Position.Offset`）；`vcol` 为顶点色 RGBA 打包权重（`Use Vertex Colors?` 开关，关闭时默认 0.08 灰）。
  - `Screen Space` 实现：`distFactor = 1`（对象空间等宽）或 `clamp(|Position − Camera.Location|)`（`Vector Math.003` DISTANCE）——即按相机距离线性放大，达到屏幕空间近似等宽；由 `Mix`（factor=`Screen Space`，A=1.0，B=距离）选择。
  - 距离 clamp 是**硬钳制、无平滑曲线**：`Clamp Min Distance`（Bool）开启时 `Math.002` MAXIMUM(d, `Min Distance Multiplier`)；`Clamp Max Distance`（Bool）开启时 `Math.003` MINIMUM(d, `Max Distance Multiplier`)（`Switch.002`/`Switch.003` 选择）。
  - 另有视线方向推挤项：`normalize(Position − Camera.Location) × vcol_B × (Z-Depth Offset × 0.01)`（`Vector Math.010/006/012` + `Math` MULTIPLY(Z-Depth Offset, 0.01)），用于把描边壳往相机方向推以控制深度交叠。

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
- ~~待核实（needs inspection）~~ **已核实（2026-09-12，逐节点转储）**：
  - `Genshin Lighting` 的 sharp/soft **不是加权混合，而是串联相乘**：`Multiply [SHADOW].002`（[SOFT MIX]）= lerp(`Soft Shadow Colour`, `Soft Light Colour`, `Lit Factor`)；`Multiply [SHADOW].001`（[SHARP MIX]）= (`Lit Factor` > 0.5) ? `Sharp Light Colour` : `Sharp Shadow Color`（`Math` GREATER_THAN，阈值 0.5）；最终 `Color = base × softMix × sharpMix × Ambient Colour`（`Multiply [LIGHT]` → `Multiply [SHADOW]` → `Multiply [AMBIENT]`，全为 MULTIPLY blend）。
  - `Depth-based Rim` 的深度依赖：`Camera Data.View Z Depth` → `Math.004`（0.1 / Z）作为深度归一因子；另有 `Curvature` 节点（输入 Samples/Sample Radius/Thickness/Scale，输出 `Scene Rim`）——该节点在 Blender 5.2.1 为 NodeUndefined（见 §3.1 异常发现），curvature rim 支路在锁定环境中为死路径，仅剩深度缩放生效。

### 3.8 Post — Approximation

上游证据（post 角色签名，CompositorNodeTree）：

- 顶层组 `HoYoverse - Post Processing`（4 nodes / 6 links）：输入 `Bloom Threshold`、`Bloom Intensity`、`Bloom Size`。
- 节点组 `Genshin Bloom`（25 nodes / 34 links）：同样三个参数。
- 节点组 `Genshin Tonemapper`（18 nodes / 17 links）：仅 `Image` 输入输出，无参数接口。
- `Compositing Nodetree`（post blend 内 5 nodes / 4 links）。
- Reference 色彩管理观测值：display_device=sRGB、view_transform=Standard、view_look=None、sequencer_colorspace=sRGB（记录于 docs/reference/reference-toolchain.toml 与各 manifest）。

bevy-stylized 表示：Reference/Post profile——bloom 以 Threshold/Intensity/Size 三参数参数化；display transform 对齐 sRGB/Standard 基线。

- 输入来源：`Profile Constant`（bloom 三参数）、`View/Frame State`（display transform 选择）。
- ~~待核实（needs inspection）~~ **已核实（2026-09-12，逐节点转储 post blend）**：
  - `Genshin Tonemapper` 内部曲线为**固定有理函数（filmic rational）**，无参数接口：`f(x) = x·(1.36·x + 0.047) / (x·(0.93·x + 0.56) + 0.14)`，`clamp_result=true`（链：`Mix`(x×1.36) → `Mix.001`(+0.047) → `Mix.002`(×x) 为分子；`Mix.005`(x×0.93) → `Mix.004`(+0.56) → `Mix.003`(×x) → `Mix.006`(+0.14) 为分母；`Mix.007` DIVIDE）。
  - `Genshin Bloom`：阈值 = `Mix`（SUBTRACT，`Image − Bloom Threshold`，clamp）；核 = **4 级链式可分离 Gaussian Blur**（`Blur`/`Blur.004`/`Blur.007`/`Blur.009`，Type=Gaussian、Separable），`Size` 为各级半径缩放因子——基础半径 16/32/64/128 像素 × `Bloom Size`（`Vector Math.001/.002/.003/` 无后缀，SCALE）；相邻级经两次 50/50 `MIX` 金字塔合并，×`Bloom Intensity`（`Mix.004` MULTIPLY）后 ADD 回原图（`Mix.005`）。

### 3.9 V1 明确 Unsupported（超出 8 域，依据签名中的实际存在）

- **眼部 / 瞳孔**：main 角色 required materials 含 `HoYoverse - Genshin Pupil`、`HoYoverse - Genshin New Pupil`、`HoYoverse - Genshin Sandrone pupil`，且存在节点组 `PupilaTodo`（21 nodes / 28 links）。V1 的 8 个 Reference Area 不含眼部域 → V1 **Unsupported**，留待后续版本评估。
- **Alpha/Emission 通道切换**：节点组 `Genshin Alpha`（57 nodes，`0 = Emission, 1 = Transparency`、`Transparency Cutoff`、`Emission Tint/Strength`）与 `HoYoverse - GI Body` 的 `------ ALPHA OPTIONS ------` 段。V1 以 `_NPR_EMISSION` 与标准 glTF alpha mode 承载其子集；完整切换语义 **已核实（2026-09-12）**：`Use Alpha` 为总开关（`Mix.001` factor）；alpha test = `Math.040` LESS_THAN(`Diffuse Alpha` − 0.03, `Transparency Cutoff`)（`Math.039` 减 0.03 偏置）；模式选择 `0 = Emission, 1 = Transparency` 把 alpha-test 因子路由到 emission 支路（模式 0，`Mix.002` → `Emission Factor` 输出）或 transparency 支路（模式 1，`Mix.003` → `Transparency Factor` 输出，经 `Mix Shader` 与 `ShaderNodeBsdfTransparent` 混合）。仍不作为 Exact 承诺（输出因子在 `HoYoverse - GI Body` 顶层的最终合成未纳入本次核实范围）。

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

核查方式（原定）：在锁定的 Blender 5.2.1 LTS 上打开 reference blend 逐节点检查。**2026-09-12 已执行**：`tools/blender-reference-export/inspect_node_math.py` 对三个 reference blend 的全部节点组做了全量转储（节点类型/参数/socket 默认值/逐 link 连接），逐条核实结果如下：

1. ~~Shadow ramp 采样坐标构造与昼夜色混合公式（§3.1）~~ → **已核实**：X = 1 − (ShadowPosition − LitFactor_AO)/(ShadowPosition × 2·vcolG × RampWidth)；Y = 0.05+0.1×(5−RampID)，Day 行为上半（+0.5）Night 下半；half-lambert + AO 门控；lerp(Day,Night,Day/Night)。**附带发现：ramp 贴图由 wizard 运行时注入（blend 内 image=None），且 cast-shadow 节点为 NodeUndefined 死路径。**
2. ~~Lightmap R/G/B/A 通道语义分配（§3.2）~~ → **已核实**：R=金属遮罩（>0.9），G=AO（另 >0.95 光照门控），B=高光抑制（1.03−B），A=材质分带 ID；顶点色层 `Col`：R=vertex AO，G=ramp 阴影宽度。
3. ~~Face 阈值图 ↔ face SDF 的数学对应；`Face Factor` 合成公式（§3.3）~~ → **已核实**：5 原型 SDF 图阈值选择 + head 基向量方向项 + smoothstep(center±Softness) 阈值比较，与 face SDF 路径数学同构；`Face Shadow` 纹理 G 通道调制 shadow alpha。
4. ~~`Expanded Normal Map` 打包格式与 Z 重建（§3.4）~~ → **已核实**：Z=sqrt(1−X²−Y²)；手工 TBN + 背面翻转；`For NdotV only` = 翻转前法线。
5. ~~MatCap 采样坐标与 `Metallic Specular Attenuation` 作用点（§3.5）~~ → **已核实**：显式相机基投影 ×0.5+0.5（非裸 view normal）；attenuation 乘性衰减 metallic sharp-layer 高光、lit factor 门控。
6. ~~Outline 宽度公式 / `Screen Space` 实现 / 距离 clamp 曲线（§3.6）~~ → **已核实**：width = 0.01×thickness×vcol×distFactor；Screen Space = 相机距离线性缩放（硬钳制 min/max，无平滑曲线）；另有视线方向 Z-Depth 推挤项。
7. ~~`Genshin Lighting` sharp/soft 合成权重；`Depth-based Rim` 深度依赖（§3.7）~~ → **已核实**：sharp/soft 串联相乘（soft=lerp by LitFactor，sharp=0.5 阈值二选一），最终 ×ambient；`Depth-based Rim` 依赖 `Camera Data.View Z Depth`（EEVEE 屏幕深度），curvature 支路 NodeUndefined 死路径。
8. ~~`Genshin Tonemapper` 曲线；`Genshin Bloom` 核与 `Size` 含义（§3.8）~~ → **已核实**：tonemap = 固定有理曲线 x(1.36x+0.047)/(x(0.93x+0.56)+0.14)；bloom = 4 级可分离 Gaussian（16/32/64/128 × Size）金字塔 50/50 合并，subtract 阈值，×Intensity 后 ADD。
9. ~~`Genshin Alpha` emission/transparency 完整切换语义（§3.9）~~ → **已核实**（组内逻辑）；顶层合成未纳入本次范围，保留 Approximation 定位。

**新发现问题（须进入决策流程）**：上游 blend 依赖至少两个在 Blender 5.2.1 不可解析的节点类型（`Cast Shadows` 输出型 Shader Info、`Curvature`），在锁定环境中为死路径——upstream Golden 与"完整上游行为"之间存在该固有偏差，任何未来以"修复死路径"为目标的升级都属 docs/reference/04 的受控迁移（分类 C/D）。

---
