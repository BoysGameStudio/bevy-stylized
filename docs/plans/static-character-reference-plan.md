# 静态角色同条件渲染对比实施计划

> 版本：v2，2026-09-14。状态：用户已授权执行；验证与剩余资格见执行报告。
> 依据：当前源代码、独立化实现审查及既有验收规范。本文件保留批准时的目标与出口条件。

## 1. 目标与完成定义

建立一条可重复运行的 Blender/Gacha → 静态资产 → Bevy Stylized 对比流程。
两端读取同一份有版本和来源记录的 capture case，先验证输入一致，再评价输出。
输出应足以回答：差异来自导出、捕获条件、profile 映射还是 renderer 算法。

交付分为两个里程碑，不混用完成状态：

- M1：对比基础设施完成，输入可核对、捕获可重复、指标与失败原因真实可复现。
- M2：声明范围内的角色视觉资格通过。只有所有必需 case 和适用语义门槛均通过，
  才能声明 M2；M1 完成但指标失败时，明确交付 M1 和 M2 的具体阻塞项。

维持静态 `genshin-static-v2` 范围：同一 frame、原始 rest 求值几何、当前已声明的
mesh/material 集合；不新增动画播放、表情生成、骨架重定向、IK、角色物理和特效
编排。当前配方排除的效果网格、描边、metallic/specular-ramp/normal/emission
不为降低误差而继续删减，也不在本轮悄悄恢复。变形 fixture 的回归照常保留。

## 2. 已知起点

1. 最近实现审查：160 Rust CPU、7 Python、12 required GPU、自制与外部角色各
   1 项接入测试通过；25 GPU/环境测试在 CPU lane 明确 ignored。
2. 修改后的完整隔离验证尚未重跑。隔离 Cargo target 已与宿主分开，需要验证
   host → isolation → host 的完整往返，不能以旧报告代替。
3. Blender 用正交取景，Bevy 接入截图用透视自动取景；背景的相同数值采用了不同
   色彩空间。现有角色图不具备直接像素验收条件。
4. Gacha 的 Light Direction 独立于新增 Sun，当前 capture 缺少实际光照求值证据。
5. 现有合成 reference 使用锁定的 Gacha 3.6.2 shader；角色 authoring 使用 3.6.4。
   两条 case 保留独立身份，不能相互替换或复用旧 signature。
6. 合成 reference 的既有约 20.96 dB FAIL 保留；不把它当作新角色 case 的测量结果。

## 3. 架构与文件归属

复用现有目录和六个产品 crate，不新增角色控制 crate。

| 位置 | 拟承担的变更 |
|---|---|
| `schemas/authoring/` | 新增开发/验证用 capture-case schema；不冒充 material/view runtime schema |
| `tools/blender/` | 作者场景求值审计、case 生成与 Blender 捕获；保留原始场景 |
| `tools/assets/` | case 与 authoring/runtime/compiled 产物的身份连接、缓存输入和输出完整性 |
| `labs/reference/` | 新增静态角色 case 执行路径，复用正式材质接管与 profile；现有合成路径保留 |
| `tests/harness/` | 捕获就绪状态、比较尺寸检查、报告和诊断复用；不放 Odette 专属逻辑 |
| `tests/integration/`、`tests/python/` | 数据转换、失败路径、资源就绪和渲染回归 |
| `tools/dev.py` | 暴露显式 case 的捕获/检查入口，缺资源不自动下载 |
| `assets/local/` | 第三方场景、纹理、case 及派生 reference 产物 |
| `target/stylized/` | 本次运行的候选图、指标、日志及可视化报告 |
| `docs/reference/`、`docs/quality/` | 契约说明、验证结果及限制；不纳入第三方图像 |

若确需新增 runtime 渲染参数，再单独证明现有 Material/Instance/View 无法表达，
同步 specification、schema、ADR/traceability 与测试；不能为了截图把临时参数
塞进 core，或在 lab 中复制一套“比较专用”的生产着色器。

## 4. 分阶段实施

### P0：冻结输入和验证现有缓存边界

工作：

- 记录工作树相关源码摘要、Rust/Bevy/Blender/Gacha 版本、shader/recipe/source
  manifests、GPU/backend/driver；不以 latest 替代已锁定输入。
- 检查准备好的原始包及已有产物哈希，按新构建 key 生成候选；修改文件不得沿用
  旧成功 manifest。检查 manifest 所列依赖实际存在，拒绝越界路径与缺失纹理。
- 确认新隔离 target 与宿主 target 路径不同；最终完整往返验证安排在 P6，避免
  每个小改动都重跑整个 Blender 链路。
- 输出 case 来源清单，分别标识 3.6.4 角色线与 3.6.2 合成 reference 线。

出口：本轮输入可追溯且不存在旧缓存假成功。原始包或工具缺失时报告 BLOCKED，
继续能够独立完成的纯 schema/测试工作，不绕到兄弟项目取资产。

### P1：定义共享 capture case

采用显式 schema/version 和 canonical 内容摘要，至少包含：

| 数据组 | 必需字段和约束 |
|---|---|
| 身份 | case id、配方/作用范围、source/authoring/runtime/compiled manifests 的摘要、生成器版本 |
| 几何 | frame、mesh/material 集合、求值状态、单位、源到 runtime 变换、静态几何审计摘要 |
| 相机 | 世界位置、朝向/up、projection、正交可见宽高、near/far、512×512、像素比例 |
| 光照 | NPR 控制输入及最终求值语义、坐标空间、palette、day/night、ambient；物理光另列 |
| 输出 | 线性背景、alpha/compositing 规则、曝光语义、display/view transform、gamma、输出格式 |
| 采样 | 两端具体 AA/采样、抖动、收敛策略与容差政策；不把不同算法同名参数当作等价 |
| 运行条件 | 请求和实际 Surface Class/Tier、backend、工具版本要求、超时及就绪条件 |
| 验证 | 必需 case 列表、metrics 版本、mask 方法与身份、阈值、资格状态 |

原则：作者端只生成一次相机数值，Bevy 直接转换读取，禁止再次按自己的 Aabb 自动
取景。Blender→glTF 的 `(x,y,z)→(x,z,-y)` 需用自制轴标 fixture 验证，不能再次
变换已经转换过的 GLB。正交 scale 对应宽高的规则须显式编码。

将“请求设置”和“两端实际生效设置”分别记录；比较前检查二者，不能把同一份请求
复制两次当作实际一致的证据。路径相对 case 所在目录解析，身份使用内容摘要，
不依赖开发者绝对路径。

出口：schema 校验及负例测试通过；错误版本、NaN/非法相机、坐标空间缺失、摘要
不符或几何范围不符均在像素比较之前失败。

### P2：求值光照并校准相机、背景与输出

先用本项目自制的轴标、非对称几何、法线球和灰阶/色块，减少角色纹理干扰。

1. 跟踪 Gacha Light Direction → Light Vectors/属性 → shader 的实际消费路径，
   区分 Euler 旋转、方向向量、空间和正负号；不能直接 normalize Euler 或猜测局部轴。
2. 在原始场景的隔离副本中记录 evaluated mesh/属性和相关 driver 求值。必要时
   用只读诊断输出核验 shader 所消费的值；诊断输出与正式 beauty 输出分开。
3. 用正面和左右两侧光照输入验证映射方向；相机和角色姿态固定。物理 Sun/world
   是否影响本配方输出，要通过单变量实验确认，不能删掉无法解释的贡献来强行对齐。
4. 区分颜色背景与环境照明，验证线性背景到 sRGB 字节的结果；曝光明确是 EV
   还是线性倍率，记录 gamma/look/compositor/alpha 规则。
5. 校验屏幕轴、投影比例、静态几何与 near/far，不做事后缩放、裁切或图像配准。
6. 明确 AA 差异；必要时另设无 AA 的几何诊断 case，但不能拿诊断 case 替代正式
   beauty case，也不能事后扩大排除边缘以提高分数。

出口：坐标与颜色校准可重复；光照映射有求值证据。无法证明某项映射时，该项
alignment 为 BLOCKED/UNQUALIFIED；可继续诊断，但不得声明同条件 parity。

### P3：让两端执行同一个 case

Blender：读取 case、应用设置、等待 depsgraph 求值、核对场景/材质/纹理身份，
输出实际生效设置及 reference。只写新候选，不保存覆盖作者源 blend。

Bevy：在 `labs/reference` 使用真实 GLB、严格编译的 sidecar、正式材质接管和
显式 view；case 负责验证场景设置，不修改库的默认应用行为。保持现有资产接入
测试的职责，不把它扩成角色专用参考实验室。

捕获必须等待资产和依赖纹理加载完成、材质覆盖准确、pipeline 可用且 warm-up
完成；固定若干次 `app.update()` 不是唯一就绪证明。为稳定检测设置有限超时，
加载失败、shader 错误和缺纹理不能生成“成功但不完整”的比较结果。

输出：两端 beauty PNG、实际设置 JSON、输入/输出摘要、GPU/工具记录、日志。
补充尺寸一致检查：宽高必须逐项相同，不能只比较 RGBA 字节长度。

出口：先完成正面光的 full-body 和 face 两个 case；分别重复捕获两次，检查同
工具链复现，再扩展为两视角 × 三光照方向，共六个必需 beauty case。
面部 closeup 是预先定义的相机，不是从渲染后图像裁切出的局部。

### P4：建立可信的比较与归因报告

- 输出原图并排查看、闪烁/切换视图、绝对差异图、全图指标，以及预先定义的
  body/hair/face/pupil ROI 诊断。ROI 不替代全图验收，不能删除高误差区域。
- 沿用现有 Tier-2 阈值与算法版本，保留数值和 violation，不改旧 case 的判定。
- 审计旧“shadow IoU”：它以各图中位亮度分割，只是亮度区域代理，在多色角色上
  不能视为真实阴影。旧算法仍按原名/版本留存，新报告显式标注其代理性质。
- 新角色的前景和阴影语义诊断应由各端几何覆盖、材质/光照因子诊断获得，且不改
  正式 beauty shader 行为。不能由一端的结果生成另一端 ground truth。
- 新 mask 算法必须有独立版本、范围和自制已知真值 fixture 测试；空 foreground、
  双空 shadow mask 不得仅凭 IoU=1 获得角色视觉资格。真实无阴影的 case 必须预先
  声明，并用必需的侧光 case 保证阴影能力有覆盖。
- 若当前 shader 无法可靠导出光照因子，先交付带限定说明的诊断报告；角色阴影
  资格保持未验证，不伪造通过或静默改为不适用。

出口：每项差异有可观察的分层证据。报告分别显示 input alignment、capture、
repeatability、legacy numerical metrics、semantic qualification、overall parity。
任何必需 case 未执行或任一必需资格项未验证，都不能得到 overall PASS。

### P5：按归因修复静态渲染差异

固定顺序：

1. 几何/坐标/取景与输出色彩条件；
2. 纹理 decode、UV/翻转、采样和 packed 通道；
3. profile 参数映射、palette/ramp、面部基向量与 SDF；
4. 已被最小 fixture 证实的 renderer 算法差异。

每项修复先形成能失败的最小证据，再修改所属层，复测受影响 case 和原有回归。
不同时改变相机、曝光与多个 shader 参数；不按角色名字写分支；不把光照补偿
写入共享材质掩盖 view 错误。保持 Material/Instance/View 分离与材质共享。

若问题需要配方之外的材质效果、完整 DCC 编译或大规模 shader 重构，输出其最小
复现、责任层和后续单项计划。本阶段不能为追逐全绿而无限扩大范围或放宽门槛。
M1 可完成，M2 必须保留真实失败。跨设备和性能资格不由此次单设备结果推导。

### P6：完整验证、隔离往返与交付

代码稳定后串行执行：

- fmt、Clippy、文档/schema/依赖检查及相关 CPU/Python 测试；完整 CPU lane。
- required GPU lane、动态 morph/outline 回归、自制角色及 Odette 接入；测试数量
  来自实际 suite，新增 case 同步登记，不能因过滤器零执行而成功。
- 两端同工具链重复捕获，六个角色 case 的 diagnostic 与 strict 行为。
- 旧 3.6.2 合成 reference 回归继续运行，结果独立记录，不被新角色图替代。
- 完整 Git-free、空资产缓存、断网隔离构建与渲染，使用独立 Cargo target；完成后
  回到主工作区复跑原先失败的 manifest-path 测试及关键 glTF 路径测试，核验缓存
  未污染宿主。Cargo 依赖缓存可复用，不把空资产缓存表述成空编译缓存。
- 负例：缺 GPU、缺 baseline/texture、身份不符、相机设置不符、shader/加载失败、
  超时、缺少 case、不同宽高以及空 mask。参数或指标失败与资源不可用分别报告。

strict 命令建议保留现有语义：0=所有必需门槛通过，1=条件/指标验证失败，2=资源
不足或无法完成验证。diagnostic 可在成功产出报告后返回 0，但 JSON 必须保留真实
parity 状态。隔离工程 gate 检查 strict 与实际报告一致，而不是要求历史 FAIL 永远失败。

最后报告关联本次实际源码和产物摘要，区分工程独立性、图像回归和上游 parity。
不提交 Git、不发布、不接受新 Golden；这些不属于此次计划的完成条件。

## 5. 数值门槛

既有门槛不放宽，来源为
[Golden criteria](../reference/08-golden-tolerance-and-cross-layer-criteria.md)。

| 类别 | 门槛 |
|---|---|
| 同工具链复现 | max ≤ 2/255，mean ≤ 0.5/255，PSNR ≥ 48 dB |
| 跨渲染全图 | mean ≤ 4/255，PSNR ≥ 30 dB |
| 非边缘最大误差 | ≤ 16/255，保留现有边缘算法及其版本 |
| 剪影 IoU | ≥ 0.99 |
| 阴影 IoU | ≥ 0.95；角色语义资格必须有可信 mask，不能仅靠旧亮度代理 |

新增几何/颜色校准容差和 semantic mask 判定在 P2/P4 根据自制 fixture、像素精度
和既有规范明确并冻结，必须在正式角色比较之前完成；不能看完角色误差后反向选值。

## 6. 交付和实施顺序

交付：共享 capture schema/验证器、上游求值审计、两端执行入口、两视角三光照
候选与 provenance、分层可视报告、最小回归、隔离往返结果、真实失败清单。
新增 CLI 的确切语法随 P1/P3 定义，文档必须标注新命令与已有命令，不能复制尚未
实现的命令当作现在可运行的使用说明。

执行依赖：P0 → P1 → P2 → P3 → P4 → P5 → P6。单个失败不得跳过其依赖关卡；
允许继续处理不依赖阻塞项的校验器和自制 fixture。首个可检视成果是两个正面光
case 的真实同条件候选与报告，不先制作庞大的角色测试矩阵。

[计划自审记录](static-character-reference-plan-review.md)列出了 v1→v2 修正项。
