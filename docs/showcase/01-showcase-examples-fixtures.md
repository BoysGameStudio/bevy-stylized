# Showcase、Examples 与 Fixtures

> **Status:** 职责与资产纪律继续有效；跨平台重构为 **Proposed design，完整目标架构评审版
> （2026-09-21）**。其开发层已大部落地，以下为合并后的实施与证据摘要（2026-09-22）。
>
> **已落地层：** `labs/scenario-model`（目录/配方/动作/会话/判定 schema、两层规范化、
> 身份 hash、合同向量、覆盖映射 `coverage`）；`labs/validation`（自 `tests/harness`
> 提取的纯 CPU diff/Tier-2 判定，阈值与数值行为不变）；`labs/scenes`（内容构建器、
> 绑定表、catalog 参数 schema、`SceneTarget` 稳定逻辑 ID）；`labs/benchmarks`（测量
> 实现与 workload runner，`workloads` 子命令 headless、不依赖 egui/Inspector）；
> `tests/harness` 证据层（中性 `CaptureStamp`、时钟域、GPU 证据测试）；
> `labs/scene-runtime`（非阻塞 CaseDriver、ExperimentClock、会话重配置事务、平台无关
> controller、配方编译器 compiler、命令日志 journal）。消费侧迁移：showcase 经 driver
> 驱动（交互 ←/→ 同会话切换）、tests/integration 经 `CaptureCycle`、labs/regression
> cell 生命周期经 driver、Android 宿主 `driver_host` 逐帧推进与设备端配方重放、
> Inspector 读回 stamp 对齐。
>
> **本地证据状态（2026-09-22，macOS/Metal 主机）：** showcase 17/17 捕获迁移前后逐字节
> 一致；A→B→A 切换捕获身份逐字节一致；tests/integration 19 个测试二进制 pre/post 逐项
> 一致（预存在失败的身份不变）；regression golden 子集逐字节一致；PC 与 Android
> （API-35 模拟器，-gpu host）对同一 outline 配方编译出相同的 recipe hash 与 Execution
> 域计划摘要，设备端捕获带完整 stamp、非黑帧，报告候选→原子发布；journal 录制后新鲜
> 会话重放末次捕获逐字节一致；scene-runtime 全栈 wasm32 编译证明通过。覆盖映射：PC
> 7 已有验证 / 10 仅展示，Android 1 已有验证 / 16 待实现，Browser 17 待实现。
>
> **未关闭项（所有者 / 阻塞条件）：** wasm-bindgen-cli 装配与实时 Chromium 运行
> （阻塞全量 webgpu lane）；浏览器宿主接入与 APK/浏览器资产字节闭包桥接（宿主
> 工作）；扩大"已有验证"覆盖格（终跑属合格 Linux GPU 主机；本 Metal 主机有预存在
> 限制：golden 增量、readiness 停滞、workloads GPU 时间戳读回失败）；控制协议传输
> 与 adb 取回编排进 dev.py（宿主工具）；交互式部分提交的完整撤销。
>
> **2026-09-22 更新：** 产品公共重配置 API 已交付（ADR-0030，Proposed；
> `bevy_stylized::reconfiguration`）——会话内 tier/class/fallback policy 对 Stage B
> 快照确定性重解析、原子应用、代次 + 变更通知、结构化拒绝；控制器的 tier 拒绝
> 先尝试会话内升档，设备级缺口才要求新 device 会话。
>
> 当前可执行入口见[运行手册](02-running-the-showcase-suite.md)。以上为本地证据，不构成
> V1 就绪或新平台合格声明；[SUPPORT_MATRIX](../platforms/SUPPORT_MATRIX.md) 不变。
> 本设计不改变既有 Golden、性能、参考验收规则。
>
> **2026-09-27 更新：** 覆盖映射闭合到 PC 18/18 验证（showcase golden 判定）+
> Browser 18/18 验证（`browser-showcase` lane：全目录经 CaseDriver 逐帧泵送
> 在浏览器执行；canonical host Chrome 153/Dawn/Vulkan 18/18 exact）。浏览器
> 宿主接入（R1 的遗留项）已由 `apps/browser-showcase` 交付；判定基线统一为
> `fixtures/golden/showcase/`（套件验证场景另有其专属捕获合同，不变）。
> macOS 诊断运行：`lod`（1 px 亚 LSB 非边缘舍入）与 `outline-advanced`
> （帧首行 3 px）未过 §5 envelope，如实记录为 FAIL，不构成缺陷证据；
> envelope 与阈值未变。

## 目标与边界

目标是统一的跨平台渲染实验与验证工作台：PC、Android、iOS 与浏览器使用同一
场景模型、资产语义、相机配方、动作、观测和验证规则。平台适配负责窗口/Surface、
输入、资源传输、生命周期和结果导出。PC 与移动端具备同等场景及诊断能力，按
真实设备能力说明可执行范围；现有 Android smoke 和浏览器入口均须纳入统一设计。
iOS 是目标平台，当前没有宿主；目标平台列表不能当作已经构建或验证的支持清单。

Examples 是最小公共 API 教程；Showcase 是交互展示、调试和测量入口；Fixtures
是稳定、可重复的验证输入。共享构建代码不会使展示截图自动成为 Golden，也不
使测试依赖最终第三方角色。公共 examples 不为复用而依赖开发场景库。

本轮只做调研、设计评审和文档修订，不实施、不运行原型、不删除资产、不改基准图。
设计以完整性、可组合性、可观测性、可验证性和单一所有权为准，不以实施成本、
最少 crate、最少改动或最短迁移路径裁剪目标。现有目录和初始化接口可以重构；
仍在使用的合同与职责必须完整转交。生产渲染仍属于六个产品 crate；工作台复用
其公共 API，确定性动作负责渲染实验，不扩展为游戏、IK 或物理引擎。

目标完成后，新增一个场景族只定义一次内容、参数和动作，即可交互检查、固定
时间回放、离屏捕获、执行共享断言和生成性能负载。PC 保存的完整配方可直接在
移动端重放，结果能追溯到相同内容及实际执行条件。无法执行时返回具体原因，
不能维护一份删减后的移动场景目录。

## 网络调研与方案评审

评审结论：共享内容、薄平台宿主、独立验证的方向成立；目标应覆盖从场景声明到
交互、执行、观测、判定和产物追踪的完整链路。取消“只新增一个共享库、先交付
最小原型、以当前接口能力封顶”的设计限制。下文区分已核对的事实、架构决策与
尚待实证的技术假设；文献与源码评审不构成 Android/WebGPU 运行证据，也不授权
本轮开始实施。未来实现必须满足文末的验收义务。

版本依据：本项目 `60981f921a44d625ca9539234628ce81a6e6de40`，Bevy 0.19.1，
本地引擎 `969ac3a3e98dae1fa4a56ecbba3a0e7f53be61b6`，当前解析的 wgpu 29.0.4、
winit 0.30.13、android-activity 0.6.1。在线 Bevy `main` 和 docs.rs `latest` 是
可变参考，部分 wgpu 页面已为 30.x；涉及 API 的结论另与本地锁定源码核对，
不因此升级依赖或将上游示例直接当本项目支持承诺。

### 一手资料及其适用范围

| 资料 | 查得的事实 | 对方案的影响 |
|---|---|---|
| [Bevy testbed](https://github.com/bevyengine/bevy/blob/main/examples/testbed/3d.rs) | 单 App 用状态进入场景，用 `DespawnOnExit` 清理实体 | 复用 Bevy 系统/状态机制，不开发第二套 ECS 或场景脚本引擎 |
| [Bevy loading screen](https://bevy.org/examples/games/loading-screen/) | 同时跟踪资产和 pipeline，并等待稳定窗口 | 保留分阶段 readiness；示例不是精确帧或 Golden 正确性的证明 |
| [Bevy headless renderer](https://github.com/bevyengine/bevy/blob/main/examples/app/headless_renderer.rs) | 关闭 Winit，使用专门 runner 和离屏输出 | headless 是宿主选择，不是可无条件套用到 Android 的主循环 |
| [winit 生命周期](https://docs.rs/winit/latest/winit/application/trait.ApplicationHandler.html) | Android Surface 依赖 resume，suspend 后失效；事件可能重复 | Surface/设备与场景生命周期分开；保留 Bevy/Winit 的底层所有权 |
| [wgpu Device](https://docs.rs/wgpu/latest/wasm32-unknown-unknown/wgpu/struct.Device.html) | WebGPU 的 `poll` 无阻塞推进作用；可用能力由设备创建请求确定 | 事件驱动读回；设备策略在启动前确定，不能切场景时补申请能力 |
| [Bevy ReadbackComplete](https://docs.rs/bevy/latest/bevy/render/gpu_readback/struct.ReadbackComplete.html) | 读回异步完成，结果带请求实体与数据 | 主世界 tick 不能冒充被捕获帧；需跨渲染边界的身份关联 |
| [AndroidAssetReader](https://github.com/bevyengine/bevy/blob/main/crates/bevy_asset/src/io/android.rs) | 通过 AssetManager 读 APK assets，无文件监听 | 不能把 PC 的 `std::fs/canonicalize` 校验器直接搬到 APK |
| [Bevy asset settings](https://github.com/bevyengine/bevy/blob/main/examples/asset/asset_settings.rs) | 同一资产后续加载设置不会覆盖首次设置 | 同路径不同 sRGB/sampler 不能当作相同缓存对象 |
| [Cargo features](https://doc.rust-lang.org/cargo/reference/features.html) | feature 会取并集；应设计为可叠加能力 | 窗口、验证、资产能力隔离；单独构建不能被 workspace feature 并集掩盖 |
| [Android 状态恢复](https://developer.android.com/topic/libraries/architecture/saving-states)与[应用文件](https://developer.android.com/training/data-storage/app-specific) | 进程死亡需持久化重建输入；应用有独立可写目录 | 保存小型配方/进度；APK 输入与可写报告分离 |
| [Android 功耗优化](https://developer.android.com/games/optimize/power) | 温度、负载和帧节奏影响持续性能 | 600 帧 GPU span 不等于手机稳态帧率，测量范围必须保持明确 |
| [WGSL 浮点规则](https://www.w3.org/TR/WGSL/#floating-point-evaluation) | 规定精度界限及允许的实现差异 | 同输入不保证所有 GPU 输出 bit-exact；保留按比较域定义的验收 |
| [RFC 8785 / JCS](https://www.rfc-editor.org/rfc/rfc8785) | 定义 JSON 规范编码，包括数字表示与 UTF-16 属性名排序；属于 Informational RFC | 语义展开后采用明确编码，不能把 Rust map 排序等同于跨语言规范化 |
| [WebGPU CTS 参数化示例](https://github.com/gpuweb/cts/blob/main/src/webgpu/examples.spec.ts) | 参数生成可组合用例；case 与共享执行的 subcase 有不同身份/隔离语义 | 统一生成矩阵和可定位 case；状态序列不能随意拆成独立测试，也不能为批量运行合并隔离单元 |
| [Bevy Time](https://docs.rs/bevy/0.19.1/bevy/time/struct.Time.html) | Real、Virtual、Fixed 各有职责，默认 Time 随 schedule 切换，Duration 精度为纳秒 | 单独定义实验时钟及到引擎时间的映射；固定步长不等于等待/着色器时钟都已受控 |
| [Bevy Globals 提取](https://github.com/bevyengine/bevy/blob/main/crates/bevy_render/src/globals.rs) | 从 Time 与 FrameCount 更新 shader 全局输入，本地引擎已核对 | 自定义实验 tick 还需明确连接到渲染侧时间/帧输入 |
| [Bevy RenderLayers](https://docs.rs/bevy/latest/bevy/camera/visibility/struct.RenderLayers.html) | 相机和实体的层相交决定可见性 | 不能据此推断全局 Resource、LOD 或材质写入获得隔离 |
| [wgpu 像素复制布局](https://docs.rs/wgpu/latest/wgpu/struct.TexelCopyBufferLayout.html) | encoder 纹理/缓冲复制的行距有 256 字节对齐要求 | 捕获格式、实际 stride 与紧密像素布局分别记录；已与本地 wgpu-types 29.0.4 核对 |
| [wgpu Queue](https://docs.rs/wgpu/latest/wgpu/struct.Queue.html) 与 [Buffer 映射](https://docs.rs/wgpu/latest/wgpu/struct.Buffer.html) | write_buffer 的传输在后续 submit 开始；映射中的 buffer 不能供 GPU 使用 | 分离上传/渲染/copy 的队列顺序和 CPU 读回缓冲生命周期；已与本地 wgpu 29.0.4 核对 |

上游 `bevy_ci_testing` 的名称不意味着必须运行远程 CI。其固定时间/截图实现可供
参考，但固定帧触发不满足本项目的资产身份、代次和严格逐用例报告，不整体替换
现有验证器。本仓库仍只做本地验证。

### 评审发现与处置

P1 表示全量迁移前必须解决，P2 表示应在相应模块迁移前明确。各项“修订”指设计
已调整，不等于实现或原型已通过。

| 编号 | 级别 | 初稿问题及具体后果 | 修订与验证要求 |
|---|---|---|---|
| R1 | P1 | 统一 runtime 未明确由谁推进 App；现有 harness 的 GPU probe 使用线程/block_on，readiness 有 `while app.update()`，不能直接作为 Android/Web runner | 共享非阻塞状态推进；事件循环归宿主；仅桌面 headless 包装允许 pump。验证恢复/读回时事件循环保持响应。**本地证据（2026-09-22）**：`labs/scene-runtime` 的 `CaseDriver` 为非阻塞状态机（宿主每帧推进一次，自身不 pump、不忙等），桌面 headless 包装是唯一泵送实现；`android-smoke` 的 `driver_host` 以 Update 系统逐帧推进 driver（无第二事件循环、无阻塞接收），API-35 模拟器 -gpu host 实测通过；浏览器宿主接入属后续工作 |
| R2 | P1 | 初稿允许任意 preset 同会话切换，但 `StylizedPlugin::finish` 只初始化能力结果；改 recipe 不会重建 Device 或上调已解析能力 | 定义 SessionPlan 和会话重配置事务；产品所有者补齐公共配置接口，设备能力变化由宿主重建设备会话。不能直接写 resolved 资源。**本地证据（2026-09-22）**：`labs/scene-runtime::session` 定义了重配置事务状态机与 `SessionHost` 接口；接管失败只清理候选、device loss 后旧回执不能命中新槽位等验收行经 fake host 单测验证；产品公共重配置 API 已落地（2026-09-22，ADR-0030 `bevy_stylized::reconfiguration`：快照重解析 + 原子应用 + 结构化拒绝，GPU 证据见 `labs/scene-runtime/tests/reconfiguration_evidence.rs`）。Device features/limits 变化仍需新 device 会话 |
| R3 | P1 | 原方案仅标记 generation/tick；现有 `CaptureRequest` 只有 entity/target，Probe 的 frame 在 Render Cleanup 自增，不能证明图像属于该 tick 或 GPU 已完成 | 定义跨提取/提交/读回的 CaptureStamp 和快照保留；单步屏障与连续流水线捕获均必须证明逐帧身份。**本地证据（2026-09-22）**：`tests/harness` 已落地中性 `CaptureStamp`（请求序列 + 主世界 update 序号 + 渲染帧镜像 + 首次完成接受），GPU 证据测试见 `tests/harness/tests/capture_stamp.rs`；实验 tick 映射已由 `labs/scene-runtime` 的 ExperimentClock 落地（2026-09-22） |
| R4 | P1 | 初稿让 APK 在一个进程内运行所有严格用例，弱化现有每用例独立进程合同；全局 PipelineCache 的错误会污染后续 readiness，资源和质量状态也会串场 | 既有严格 GPU gate 保留每用例新进程；共享会话只用于交互和明确的生命周期测试。失败用例不能阻止下一隔离用例被核对。**本地证据（2026-09-22）**：tests/integration 迁移到 `CaptureCycle` 后每用例仍是独立 App/driver 实例，19 个测试二进制 pre/post 逐项一致（含 8 个预存在失败的身份不变），严格 gate 的进程模型未动 |
| R5 | P1 | “同等级 bundle 检查”未定义跨平台字节来源；Inspector 用文件路径与 canonicalize，APK 无对应语义；检查后再加载也可能读到不同内容 | 逻辑路径校验与资源适配分离，固定内容身份，校验实际送入 loader 的字节；参考输入域继续独立 |
| R6 | P1 | 仅按路径或内容哈希复用贴图会忽略 sRGB/data 与 sampler；不同 bundle 都叫 character.glb 时还会串缓存 | 命名空间包含 bundle 身份，纹理绑定包含解释语义；冲突显式拒绝或使用已声明独立逻辑资产，不改色彩合同 |
| R7 | P2 | “删除根实体”不足以撤销全局设置和异步工作；设备失效时等待所有回调又可能无法退出 | 定义分区资源账本和有界退役；晚到事件隔离，超期或 device loss 由宿主撤销设备会话；覆盖失败中的部分构建 |
| R8 | P2 | 初稿可移植测试 library 仅说明目录迁移；当前 package 普通依赖含 tempfile/naga 等，许多测试自行构建 App 并同步捕获 | 分离可移植场景驱动、纯观测判定和主机工具依赖；保留桌面测试 ID/断言，独立检查各平台依赖图 |
| R9 | P2 | “配方 hash”未规定默认值、浮点、顺序与 schema；相同参数可能跨语言产生不同 ID，旧报告又可能被误收 | 语义编译与 JCS 编码分层，版本化 hash 域；主机核对 run/case/request/recipe/artifact 身份和原子完成记录。**本地证据（2026-09-22）**：`scene-runtime::compiler` 的计划摘要使用 Execution 域（JCS 载荷 = 配方 hash + RunSpec 字段 + 会话 hash），同一配方 → 同一摘要的稳定性与域分离有单测覆盖 |
| R10 | P2 | 以新增 crate 数量和最小原型约束架构，内容、执行、UI、判定仍可能聚集在一个共享包 | 按依赖方向与独立验证边界拆分开发层；现有文件位置不决定最终所有权 |
| R11 | P2 | 只列代表性设备和现有场景，无法说明组合覆盖缺口，也没有统一交互/恢复目标 | 增加完整目标平台、约束矩阵、动作回放、诊断控制和会话恢复合同；实测状态另记 |
| R12 | P1 | 多视图只规定相机/target 隔离；当前 view 设置与托管灯光全局同步，LOD 又读取所有 Camera，预览相机可能改变测试结果 | 明确实验分区、观察相机集合、独立光照/材质变更所有权；区分同场景多机位与独立 A/B 实验 |
| R13 | P1 | 逻辑 tick 未绑定引擎/着色器时钟、最终姿态同步及暂停后的期限；固定 seed 也未消除构建顺序影响 | 明确时钟域、稳定逻辑 ID、随机子流与提取前屏障，等待不推进实验状态，主机总期限独立计时。**本地证据（2026-09-22）**：harness 时钟域已定义（主世界 update 序号 / 渲染世界 Cleanup 帧计数 / 墙钟分列），"等待中的额外 update 不改变捕获 stamp↔帧绑定"已由 GPU 证据测试证明；`labs/scene-runtime` 的 ExperimentClock 只由 fixed_step 推进，泵送节奏不改变实验 tick/采样身份/像素的证据见 `labs/scene-runtime/tests/driver_evidence.rs`；随机子流与提取前屏障仍属 runtime 层后续工作 |
| R14 | P1 | 捕获有身份但缺完整像素合同；现有 RGBA8 与线性 HDR resolve 是不同路径 | 引入捕获描述符与比较域，固定格式/采样/转换链；去 padding 不改变数值，禁止隐式重采样或重复 sRGB 编码 |
| R15 | P1 | 配方/产物 hash 未锁定判定器、基线、mask/ROI 和阈值；同一图可能按不同规则被宣称同一 PASS | RunSpec 绑定独立 EvaluationSpec，计划冻结全部验收输入，报告区分捕获成功与验证成功 |
| R16 | P2 | 完整目录可能随 feature 消失；“所有组合”又未定义有限覆盖域及未实现项如何计数 | 元数据与构建器绑定分开编译；版本化有限 CoveragePlan，必需缺项不可归类为普通设备缺能力 |
| R17 | P2 | 幂等请求仅在进程存活时成立；恢复后无法区分已接受、已执行未落盘和未执行请求 | 明确持久化确认、恢复水位与结果提交；不保证外部副作用恰好一次，不自动重跑状态不明的严格用例 |

R2/R3/R4/R5/R6 均有本地直接证据：
[插件初始化](../../crates/bevy-stylized/src/lib.rs)、
[捕获请求](../../tests/harness/src/capture.rs)、
[全局 pipeline 探针](../../tests/harness/src/readiness.rs)、
[Inspector 输入](../../apps/showcase/src/inspector/asset.rs)、
[纹理接管](../../crates/bevy-stylized/src/integration.rs)。
这里没有声称已经观察到上述所有运行故障；这些是由现有实现与拟议迁移推导的风险。

源码依据还包括 [view 全局同步](../../crates/bevy-stylized-render/src/view.rs)、
[LOD 相机集合](../../crates/bevy-stylized-render/src/lod.rs)、
[最终姿态同步接口](../../crates/bevy-stylized-render/src/presentation.rs)、
[两种捕获路径](../../tests/harness/src/capture.rs)及
[既有比较标准](../reference/08-golden-tolerance-and-cross-layer-criteria.md)。
R12–R17 修正的是完整目标架构中的遗漏，不表示当前运行入口已被改造或新增故障已复现。

### 方案取舍

| 选项 | 结论 |
|---|---|
| Android 直接依赖当前完整 Showcase | 拒绝。当前 UI、benchmark、桌面 feature 与宿主装配耦合；应提取真正跨平台的应用层，再由各宿主装配 |
| 合并为一个包含所有平台/测试/工具的大 crate | 拒绝。路径减少但依赖和条件编译耦合增加，也削弱验证独立性 |
| 限定只新增一个 scenes crate，其余保留现有归属 | 不采用为目标约束。现有目录不能阻止模型、内容、执行、判定和交互分别获得清晰所有者 |
| 统一声明模型 + 场景组合 + 会话执行 + 共享交互 + 独立验证/测量 | 采用。每种职责有唯一实现，平台只承担真实差异，完整覆盖由同一模型派生 |

采用有类型、可序列化、可校验的场景声明与动作图，支持组合、参数扫描、自动
控件、保存和回放。模型约束实验语义，执行仍使用 Bevy ECS；不另建实体运行时或
复制材质 schema。扩展通过注册的构建器/动作/观测类型接入，每种扩展必须提供
schema、能力需求、确定性规则和生命周期所有权。动态加载任意原生代码不是统一
场景的必要条件；这属于语义与可复现边界，而非开发成本取舍。

## 当前问题与证据

| 当前所有者 | 问题 | 迁移方向 |
|---|---|---|
| [Showcase](../../apps/showcase/src/lib.rs) 与 ~~cases~~（2026-09-22 已迁 [labs/scenes](../../labs/scenes/src/lib.rs)） | ~~17 个注册项混合内容、相机与输出；交互入口另建窗口相机，没有沿用 case 的全部离屏配置和 tier~~（2026-09-22 已统一：内容/相机配方归 scenes，showcase 经 scene-runtime driver 同会话切换） | 场景返回内容与相机配方，宿主绑定输出；同一会话切换 |
| [Android](../../apps/android-smoke/src/lib.rs) | 独立程序化场景；角色模式通过 feature 切换并编译嵌入描述符 | 同一目录与运行时选择；feature 仅控制可编译能力 |
| [回归实验](../../labs/regression/src/lib.rs) | ~~`build_strip` 明确复制 Showcase 双骨骼条带，只为控制相机/姿态~~（2026-09-22 已统一：条带构建器归 scenes，regression 委托调用并保留各自姿态/采样参数） | 提取无相机的条带构建器，保留各自姿态与采样参数 |
| [benchmarks](../../labs/benchmarks/src/lib.rs) 与 [workloads](../../labs/benchmarks/src/workloads.rs) | ~~部分实例已复用，但内容与采集混合；workloads 依赖 Inspector 测量模块和 feature~~（2026-09-22 已归位：内容归 scenes，测量/workload runner 归 benchmarks，headless 运行） | 内容归场景库，测量归 benchmark 所有者并脱离 UI |
| [Inspector](../../apps/showcase/src/inspector/runtime.rs)、[Android 角色](../../apps/android-smoke/src/character.rs)、[浏览器](../../tests/browser/smoke.rs) | 各自装配角色、视图和接管完成条件 | 共享已验证输入的场景装配与就绪状态 |
| [测试 support](../../tests/integration/tests/support/render_scene.rs) 与 GPU 用例 | ~~多套 App/相机/等待循环~~（2026-09-22 已统一：11 个测试文件经 `CaptureCycle`，断言与身份不变） | 中性捕获探针保留在 harness；统一场景生命周期，断言留在测试 |
| [参考捕获](../../labs/reference/src/character/scene.rs) | 装配相似，但校准、输入和采样有独立合同 | 仅共享合同一致的装配，不合并参考与 Inspector 的输入验收 |

相似外观不等于相同负载：旧 `hero_1`、完整纹理 Hero、浏览器组合角色与 Android
角色的参数/相机/AA 不同。先保留显式 preset，不能按名字或截图接近就删除。

## 依赖与代码所有权

以下为目标开发层，不是当前目录声明。新增 package 均为 `publish = false`，数量
由依赖隔离和测试边界决定。产品 crate 不承担测试工作台职责；开发层可重新组织，
不能仅把旧目录移入一个公共包后继续保持原有耦合。

```text
PC / Android / iOS / browser bootstrap
                  │
                  ├── shared workbench UI/controller
                  │                     │
local CLI / QA / reference / regression ─┼── scene-runtime
                  │                     │       ├── scenes → 产品公共 API + Bevy
                  ├── validation        │       └── golden-harness → Bevy
                  └── benchmarks ───────┘

上述开发层共享 scenario-model 的数据合同；产品 normal/build 依赖不反向到开发层
validation 不依赖被测 scenes/runtime；UI、判定和测量不互相反向引用
```

| 目标所有者 | 唯一职责 | 依赖边界 |
|---|---|---|
| `labs/scenario-model` | 目录类型、配方/动作/观测/协议 schema，规范化编译、身份与约束展开 | 无 App、GPU、UI、文件系统；材质等产品语义引用现有纯数据合同，不复制字段定义 |
| `labs/scenes` | 注册场景及参数元数据；几何/纹理/蒙皮原语、内容组合和产品装配 | 元数据部分只依赖 model/产品纯数据；构建 feature 才引入产品渲染 API 与 Bevy；无 runner、UI、判定或测量 |
| `labs/scene-runtime` | 非阻塞 CaseDriver、会话重配置协调、场景所有权、动作调度、资产桥接、观测归一 | 依赖 model/scenes 与中性 harness；通过宿主接口请求设备操作，不持有平台事件循环 |
| `tests/harness` | 中性 pipeline 探针、捕获关联与 GPU 观测；纯 CPU 比较算法独立于 GPU feature | 不引用产品/场景类型；渲染依赖可隔离，纯图像比较不要求创建设备 |
| `labs/validation` | 对观测执行断言；独立期望、比较域、容差和失败解释 | 只依赖纯数据与 CPU 算法；不调用被测 builder 生成期望，不要求 App/Device |
| `labs/benchmarks` | 测量协议、采样器、统计与性能报告，固定 workload 配方 | 可调用 runtime/观测适配器；不依赖 UI，不负责生成第二套场景 |
| `apps/showcase` 共享应用层 | 平台无关的控制器、目录浏览、参数编辑、回放、图像/性能诊断面板 | 通过命令/观测接口调用 runtime；平台 bootstrap 与 UI crate/feature 分离 |
| 平台 bootstrap 与主机工具 | runner、窗口/Surface、设备建立/重建、生命周期、打包、传输、进程隔离 | 调用共享层；只含平台差异，不定义场景、相机、断言或性能负载 |

`scenario-model` 定义注册合同，`scenes` 的注册项把元数据与构建实现绑定为唯一目录；
schema、UI 表单和工具清单均从它导出。测试的必需集合仍归验证层，不能被目录
自动扩展或缩减。GPU 观测转为纯数据后交给 validation，判定逻辑可在设备执行，
也可在主机对取回的相同产物执行。测试 orchestrator 组合 runtime 与 validation，
二者不互相拥有。纯图像算法提取时保留原阈值和数值行为。

目录元数据必须可在不启用 renderer/glTF/UI 的构建中枚举；构建器绑定通过 feature
选择，但稳定 ID、参数 schema、需求和未编译原因不被条件编译删除。纯 model
依赖与目录导出不得间接拉入 GPU。schema 快照及平台可用性表是生成物，不维护
第二份手写目录；绑定表核对重复 ID、缺失实现和声明/编译能力是否一致。

`golden-harness` 接收中性捕获元数据，由 runtime 映射到工作台身份，避免反向依赖。
通用 headless App 装配可在 harness 复用，但 Stylized 插件配置由调用者提供。
共享的是非阻塞请求、状态与回执，不是桌面 probe/wait 循环。
Android 和 Web 保留各自 Bevy runner，通过系统逐帧推进 CaseDriver；主线程没有
忙等、阻塞 channel recv 或重复启动 EventLoop。原生 headless 包装可串行 pump
同一个 driver；装载、断言和写出任务不能阻塞平台生命周期事件。

`labs/benchmarks` 接收现有测量实现和 workload runner；`labs/regression` 保留
扫描矩阵、既有 gate 和报告解释。Showcase 仅提供测量入口，测量不依赖 egui、
Inspector 或桌面窗口。`labs/reference` 保留输入验证、校准、固定六视角及指标。
Inspector 成为共享应用层的角色工作区；相机控制、回放和场景代次归共享 runtime。
目录、参数、快照比较和测量面板在 PC/移动端复用；布局和输入适配不另建业务逻辑。

X11/Wayland、GameActivity、桌面 UI feature 只出现在对应宿主及正确 target 的
依赖中。共享库默认不启动窗口、不要求外部角色；glTF 等能力显式启用。缺少
编译能力时，目录仍报告不可用及原因，不让场景静默消失。

## 场景、配方与验证身份

区分内容、执行请求和会话配置，不用一个名字混淆内容与验收：

1. `SceneId + preset` 决定内容，例如 `outline/hybrid`、`character/compiled`。
2. `SceneRecipe` 完整确定内容组合、相机、视图、材质参数、资产清单哈希、种子、
   动作图、分辨率、AA、请求 tier 与必需效果。展开默认值后才能保存或比较。
3. `RunSpec` 引用配方，选择交互、捕获、断言或测量及步数/采样规则。测试函数 ID、
   Golden 路径、性能 target ID 继续由各自验证层管理；需要判定时引用不可变
   `EvaluationSpec`，其中包含判定器版本、期望/基线/ROI/mask 的实际内容 hash、
   比较域、阈值、适用条件与所需观测。这些输入不由场景构建器生成。
4. `SessionPlan` 固定 backend/device features/limits、初始化 tier 上限、插件集、
   资产 source、instrumentation 和 runner 类型；其 hash 进入执行身份。

`SessionPlan` 是可重复配置，`session_id` 是一次实际 App/Device 生命周期，二者
不能互换。同计划重建设备也必须产生新 session_id。generation 标记实验内容的
一次实例化，revision 标记其语义编辑，sample_id 标记一次被批准的时间/历史
采样；恢复或重新采样同 tick 不复用 sample_id。ViewId 在 ExperimentId 内唯一，
request_id 标记一次具体读回。所有回执保留这些作用域，不能单凭 Entity、tick
或一个跨场景复用的计数接受结果；计数耗尽必须结束所属生命周期，不能回绕。

配方编译器把参数、组合引用和动作图展开成不可变 `ScenePlan`，再结合 RunSpec、
SessionPlan 生成可审查的执行计划。组合节点使用稳定逻辑 ID，明确实体、材质、
灯光、view 和动作的作用域；参数继承只在编译阶段展开，不留运行时隐式默认。
动作包含施加参数、变形/移动、等待就绪、固定步进、观测与检查点；按依赖关系和
稳定次序执行，拒绝循环依赖、悬空引用和未声明的同 tick 写冲突。时间轴使用
整数 tick 与有理步长。运行时 Entity/Handle 不进入可保存配方。

同一 SessionPlan 内支持合法场景切换及公共 API 定义的运行时重配置；跨计划由
会话协调器执行“停止接收 → 排空/作废请求 → 保存配方检查点 → 重建 → 校验 →
恢复”的事务。不能直接修改 resolved 资源冒充重新初始化。产品 policy 可动态
调整而设备已具备必要能力时，应由产品所有者提供完整公共重配置接口；Device
features/limits 改变则必须建立新 Device。宿主可以重建 App/设备会话，严格隔离
用例必须新进程；不把“所有修改都要求用户手工重启”作为长期交互设计。

产品侧已提供公共重配置接口（2026-09-22，ADR-0030
`bevy_stylized::reconfiguration`：对 Stage B 快照重解析并原子更新，Device
features/limits 变更仍需新 device 会话，由宿主重建）。底层设备/Surface 重建
接口缺失时，改动归引擎或产品所有者，场景层只协调其生命周期。Portable
baseline 保留声明的 device 限制，不能为了切换方便统一申请最高能力；不同
SessionPlan 的结果不能混为相同执行条件。

同一配方在 PC/Android 上不能按操作系统改物体数、材质、灯光或相机。配方身份
排除资产绝对根路径；执行身份另加 run ID、源码/lock、设备/驱动/backend、实际
启用能力、resolved tier、fallback 和输出信息。不同修订可对同一负载做 A/B，
但不得省略执行身份。

目录以 Rust 注册定义为唯一源，派生版本化 schema、JSON 清单和 UI 元数据；用户
保存的配方是实例数据，不是第二份参数定义。支持有类型的声明式组合及动作图，
无需为每个组合新增 Rust 场景文件。未知类型、重复 ID、非法范围、不适用参数
及未声明的资产依赖在执行前失败；配方不携带任意代码脚本。

身份计算分成两层，不能只写“排序 JSON 后 hash”：

- **语义规范化：** model 展开默认值、组合和 preset revision，验证单位/坐标/
  路径，按字段合同确定有序列表与集合。浮点先映射到声明的 f32/f64 精度，再
  规范编码；拒绝非有限值。需要检查特殊位模式的测试使用显式整数/十六进制
  测试向量，不把 NaN 作为普通 JSON 数值。seed、tick 等 64 位整数编码为规范
  十进制字符串，避免 JSON 数值消费方丢失精度。
- **字节规范化：** 对已展开模型采用 RFC 8785 JCS，使用其数字表示、属性排序
  和 UTF-8 编码；不把 Rust `BTreeMap` 的排序视为 JCS，也不追加未声明的 Unicode
  归一化。SHA-256 的输入包含合同种类、schema/compiler 版本与规范 payload，
  将 recipe/session/execution/artifact 的 hash 域明确区分。

JSON 在进入普通 map 或 schema 校验前拒绝重复属性名；未知字段按版本合同
处理，不能静默丢弃后计算相同 hash。有序动作/视图列表不排序；集合字段才按
声明规则排序去重。普通数值中的正负零按合同等同并规范化；需要区分其位模式
的测试使用显式位向量。hash 输入的种类/版本/载荷须有无歧义编码边界，不以
裸字符串相接代替结构化编码。配方中的资产标识绑定完整依赖清单，不含清单
自身的 hash 字段；排除自引用后再由外层记录其摘要。

Python/UI 传输规范字节与 hash，接收方按同一协议核对；跨语言实现必须通过
相同合同向量，不自行改写默认值和浮点。schema 或语义编译规则改变使用新版本，
拒绝未知版本；需要转换时生成新配方和新身份，不静默解释旧配方。seed 同时记录
RNG 算法/版本，固定 seed 不是跨 CPU/GPU 位级一致性的承诺。

源码/lock 身份仍按现有任务合同计算，不直接用 Git HEAD 代替含未提交变更的
运行源码。执行计划另绑定构建参数、编译能力、实际安装包/可执行产物及上述
EvaluationSpec。更换阈值、ROI、mask、基线或解码器均改变判定身份；产物 hash
证明内容相同，不证明其通过了正确的规则。跨设备比较先核对获准的比较域，
未获准的跨设备容差不能从已有 same-toolchain 或 Blender 规则推导。

现有 `tests/suites.json` 仍是必需测试身份的唯一清单。场景型用例在测试实现中
引用 `SceneId/preset`；ABI、预算、schema 等合同测试不必伪造成可视化场景。
覆盖检查证明旧用例仍存在、每个场景引用可解析。多个断言引用同一场景是正常
复用。不能让被测 builder 生成期望结果：解析期望、烘焙形状和 Blender 参考仍
独立于运行时构建，防止同一错误两边抵消。

跨平台用例拆成 setup/动作、观测和纯 check：驱动归 runtime，判定归 validation。
现有 `tests/integration` 保留测试注册与桌面薄包装，测试名、断言语义及必需身份
不变；Android QA 宿主调用同一驱动与判定，不在 APK 中运行 libtest 或另写简化
通过条件。文件写出、进程控制、临时目录和仅主机 shader 工具留在宿主/test-only
依赖。普通交互与 QA 装配可选择能力，但不以 feature 建立第二份验证实现。
注册的实现 ID 与必需清单核对，不创建第二份必需集合。无法在设备执行的合同
检查继续在主机验证，报告执行范围。
可移植用例接收已创建的运行上下文，不能内部再创建 App/Device 或直接读主机
文件。GPU 观测在设备侧进行；图像断言可在设备或取回后由相同 check 实现执行，
结果注明执行位置。资源缺失与断言失败使用结构化错误，panic/进程崩溃仍由主机
记 FAIL，不能靠 catch_unwind 或日志关键词把驱动错误当正常跳过。

### 统一控制与结果传输

PC CLI、应用内选择器和本地自动任务使用同一版本化控制协议：发现目录、协商
可执行能力、提交 RunSpec、查询状态、取消、取得观测/产物。未来 `tools/dev.py`
增加目标适配，负责构建、打包/安装、启动、取回和中断识别；本文没有新增可执行
命令。控制协议与传输解耦，不把 adb 文件复制本身当成场景 API。

每条请求包含协议版本、run/request ID、序号和预期 session/revision；重复提交
同 ID 返回该请求的当前状态而不重复执行，载荷不同则拒绝，过期 revision 不能
修改新场景。取消停止后续动作并排空异步任务；沿用任务结果合同，已处理的中断
记 FAIL 并注明原因，未能写出则保留 RUNNING，由主机补充失败/中断证据。

每个 session 只有一个控制者分配接受顺序；UI/CLI/平台桥接提交意图，不各自推进
runtime。去重在再次执行 revision 前置检查之前完成，因此查询一个已完成请求
不会因当前 revision 已前进而重新执行或误报过期。并发编辑使用预期 revision
比较提交，冲突明确拒绝；不得按线程到达时机静默覆盖另一条已接受修改。

协议区分“收到”“持久化接受”“执行中”“完成”。可恢复的接受确认必须先持久化
规范请求及其 hash；检查点记录已应用命令水位，完成记录绑定全部产物及判定。
崩溃后无法判断是否已执行的请求记录为不确定/中断原因，不能凭 request ID 自动
再执行并宣称恰好一次。交互恢复可从合格检查点重放纯状态动作；截图发布、报告
写出等副作用用独立产物 ID 去重。严格运行中断仍保留失败证据，重试建立新 run，
不得改写原结果；已完成请求查询可取回已提交产物，不重新运行测试。

传输包括桌面进程内/本地 IPC、Android 应用桥接、浏览器消息与下载适配。Android
debug QA 可通过 adb `run-as` 访问应用专属输入/输出目录；正式包必须有可用的
应用内导入/导出桥接，不能依赖 debug 权限。交互与 QA 共用协议和执行层，QA
是否启用由装配策略明确。请求只引用受控输入，输出归各自 `runs/ID`；APK assets
是只读输入。当前 MainActivity 没有实现这些桥接，属于待实现目标。

## 目录收敛与旧场景去向

下表为拟定目标。preset 保留真实差异；合并的是构建实现，不是删减覆盖。
测试最小 preset 与展示组合分开，展示布局变化不能改变已锁定测试配方。

| 统一场景族 | 参数 / preset | 接收当前内容 |
|---|---|---|
| `overview` | class 对照、可选 Standard、显式布局 | overview、Android 程序化、浏览器 class 样本；先保留原布局 |
| `ramp` | analytic / texture、行、bands、昼夜、smoothing | ramp、Cel Golden、profile ramp 输入 |
| `lighting` | authored / physical / hybrid、光向、palette | lighting、physical material、光照扫描、effects 光照基覆盖 |
| `face` | SDF、阈值/侧向、外部 head basis | face、Character Golden、head-pose 输入 |
| `hair` / `metal` | 各自参数、贴图和开关 | hair、metal、profile 高光/金属输入 |
| `outline` | hull / screen / hybrid、宽度、alpha class | outline、outline-advanced、alpha/prepass 输入 |
| `deformation` | two-bone / morph / skin+morph；rest / posed / deterministic motion | animation、regression strip、morph_skinning、extended_skin |
| `post` | 各效果开关及 enable/remove/restore 动作 | post、post_lifecycle、Hero post workload |
| `environment` | day / dusk / night | 三个展示项变为一套内容、三组 view 参数 |
| `lod` | 距离、阈值、效果策略 | lod；保留各距离带及 class 不降级断言 |
| `instances` | class、数量、布局、逐实例数据、churn | stress、performance 组合、benchmark crowd、batch/table/precision 输入 |
| `character` | first-party reference / synthetic rig / compiled bundle；可组合 class 样本 | reference、Inspector、Android 角色、浏览器组合、参考六视角与扫描 |

九个标准 workload 全部保留：Hero 1/3、Character 32、Portable 100/500/1000、
Hero skin+morph、Hero outline、Hero post。它们改为共享场景的固定配方，不产生
九套独立 builder。旧四组 benchmark 与 42-cell sweep 的合同保留；只有证明
配方、采集器与调用方等价，才删除完全重合的 runner。`performance` 展示画面
本身不等于这些测量结果。

现有 16 组、41 个 GPU 用例全部纳入核对；另核对 synthetic-import、asset-integration、
Inspector 重载、参考六视角、realtime 扫描、浏览器比较及 `tests/current-contracts.json`
引用。GPU-driven spike 留在实验 lab，仅复用合同相同的原语，不成为平台正式场景。
“所有场景”不意味着把所有内部合同和研究实验都放进移动菜单。

## 相机、操作与运行生命周期

Builder 不再接收可任意启动的 App，而通过受限构建上下文操作内容 World，返回
`SceneInstance`：根实体、拥有的句柄、动作绑定、相机配方、结构期望。共享 runtime
安装一次系统，按当前实例执行；禁止每次切换重复注册系统。宿主提供输出 target，
共享 runtime 按配方创建相机；窗口和离屏不能另补不同机位。Inspector 多视图是
显式配方，不是隐含第二相机。

内容根、view 根和异步工作分别登记 owner generation。层级清理可复用 Bevy
state-scoped despawn；根之外的 hull/诊断副本、camera target、材质 handle、场景
observer 等仍需明确所有权。会话维护默认 view、ClearColor、质量 override 和
post 状态；退役时恢复，不能指望 despawn 自动撤销 Resource。通用系统仅注册一次，
晚到接管/读回结果必须核对 owner。资源回收检查关注活跃 owner/handle 与有界
缓存稳定，不要求驱动分配量立即回到零，也不为“清理”手工删除仍被渲染使用的资源。

输入映射成共同动作：选择场景/preset、orbit、pan、zoom、旋转样本、调光、reset、
单步/播放/暂停、capture、reload。鼠标/键盘与触摸只是适配器。UI 参数来自目录；
PC 侧栏、移动折叠面板可布局不同，参数语义一致。横竖屏/DPI 只影响显示布局；
严格捕获用固定像素 target，必要时留边。自由操作产生新 revision，自动重放
不读取实时输入。

共享应用层包含完整的场景工作区，不把移动端做成只有“选择/旋转”的演示器：

| 工作区 | 共同功能与约束 |
|---|---|
| 目录与配方 | 按场景族/效果/能力检索，编辑有单位和范围的参数，组合实例/灯光/view，保存规范配方；不可用项可见且有原因 |
| 时间轴与回放 | 录制语义动作，固定步进、检查点、回到指定 tick、参数扫描；撤销/重做重放配方变更，不保存 Entity |
| 材质与结构 | 按逻辑对象选择，查看 class、材质绑定、纹理语义、骨骼/Morph、bounds、outline 接管和 owner；调用同一诊断接口 |
| 图像诊断 | 多 view、A/B、差值/叠加、像素与 ROI 检查、声明的中间观测；显示比较域、色彩空间及容差来源 |
| 能力与测量 | requested/resolved 能力、fallback、pipeline/readiness、CPU/GPU 采样及内存观测；不可获得的指标明确标记 |
| 结果与追溯 | 查看运行阶段、失败原因、配方/资产/build 身份，导入/导出可重放输入与产物 |

相同控件元数据、控制器和命令校验用于所有布局。参数变更提交为事务，新 revision
形成一致快照；未验证的中间输入不能直接写入 renderer。严格捕获/测量通过
execution lease 固定配方，UI 可以继续导航和查看结果，修改需产生后续请求。
窗口实时大小、触摸采样率和刷新率不成为隐式实验参数。诊断观测通过 renderer
公共接口暴露，缺失接口由产品所有者补齐，不在 UI 中复制渲染状态推导。

### 实验分区与多视图隔离

`ExperimentId` 表示一份可变实验状态，`ViewId` 表示该实验的一个观察输出。
同一角色的六机位可以共享姿态/材料；修改灯光、材质、质量或时间的 A/B 是两个
实验分区。只创建第二台相机或添加 RenderLayers 不能获得后者的隔离。

当前 `StylizedViewSettings` 会同步托管灯光和被选中的相机；`MarkedCameras` 只是
选择写入对象。`sync_lod` 又按所有带 Camera 的实体计算最近距离，包括没有作为
实验观察者声明的相机。因此独立 view 环境和预览不干扰 LOD 目前都不是现成能力。

目标产品接口须显式绑定 view 环境与实验光照组，LOD 绑定声明的观察者集合和
聚合策略；原用例保留其原语义，新配方不得依赖“当前恰好存在的全部 Camera”。
预览、UI、检查器及禁用相机只有被配方列为观察者才影响实验质量。材质可只读
共享，A/B 静态参数变更采用独立绑定/写时分离；实例数据、灯光、后处理历史、
全局质量和时间各有分区所有者。删除其中一侧不得改变另一侧的绑定或历史。

执行计划检查每个系统/资源是否具备分区能力。无法隔离的 App/RenderWorld 全局
状态必须放入独立执行上下文；设备策略不同则进一步使用独立 SessionPlan。
共享 UI 合成这些结果，不以同 World 多相机模拟已隔离实验。宿主保持单一事件
循环；严格用例继续使用其进程隔离合同，性能 A/B 串行测量，不能在同时渲染的
两个分区之间直接作单负载性能比较。验收必须包含增加/移除预览相机及改变 A 侧
灯光/LOD/材质后，B 侧规范输入和对应合格像素结果不变。

### 时间与调度合同

实验时钟、宿主真实时间、渲染提交序号和 GPU 完成序号分别记录。实验时钟以
`tick × 有理步长` 定义，映射到 Bevy 纳秒 Duration 时从绝对 tick 计算并采用
版本化舍入规则，不能逐帧累加截断后的 1/60 秒。着色器时间、delta、随机噪声帧
及 temporal history 步数均由同一实验规则导出；宿主为装载/读回运行更多 update
不得推进它们。引擎默认 Globals 会提取 Time 与 FrameCount，必须在拥有者处建立
明确的实验输入接口，不能只插入自定义时钟后假定所有消费者自动改用它。

实验动作按逻辑 ID 稳定排序；随机流按对象/动作的稳定键派生，并记录算法版本，
不依赖 Entity 分配、HashMap/查询遍历或异步加载完成次序。动作 → ApplyDeferred →
源姿态传播 → 产品最终姿态同步 → Render 提取构成显式屏障。可复用现有
`configure_presentation_schedule`，但它不会替宿主推进时间或传播源骨骼姿态；
同一实验步不能既执行 native 同步又额外同步一次。没有顺序依赖且结果可交换的
系统仍可并行，不要求整个 ECS 单线程。

现有接口每 App 只接受一个 host presentation schedule；实验分区由该阶段统一
调度，不能逐场景重复注册不同 schedule。只有获准的实验渲染样本才推进历史与
噪声序列；重复展示上一张结果或为读回推进宿主不得额外更新 temporal history。

阶段超时注明采用活动时间还是单调真实时间；自动运行另有主机总期限，包含
挂起、失联和重建，设备不推进 App 也能由主机结束。交互暂停冻结实验 tick；
自动严格用例的意外挂起按中断处理，只有声明的生命周期测试允许恢复继续其
预期序列。恢复后不追赶后台经过的时间，重建运行时钟映射和必要历史；不能
一边无限暂停阶段计时、一边永远保持 RUNNING。

```text
Select → Validate → Loading → Adopting → Warming → Ready
                                                   │
                                   Interact / Step / Capture / Measure
                                                   │
                                     Retire → Select 或退出
```

上图是正常路径。任一阶段的失败、取消或超时都进入终止处理：固定首个主因和
阶段 → 停止接收新动作 → 撤销发布资格 → 有界清理 → 写出终态。后续清理错误
作为附加诊断保留，不覆盖主因，也不把已记录 FAIL 变为 BLOCKED 或 PASS。
验证失败时可能尚无实例；装载/接管失败时可能只有部分实体/任务，两者都必须
进入同一所有权清理路径。同一有效 session 内，候选在隔离分区达到 Ready 后才
提交替换，提交前再次核对其基准 revision；失败清理候选并保留旧已提交版本。
跨 session 重建或设备已失效时不保证旧运行实例可恢复，UI 显示失败及历史快照，
不能把历史画面标为当前活跃结果。两条路径都不允许半新半旧状态对外发布。

候选准备不等于旧版本已退役。旧版本只能按其原身份对外可见；成功提交替换后，
再撤销旧版本的发布资格并启动退役。用户取消候选时仅清理候选，不顺带删除旧
版本。需要独占资源而无法保留旧版本的替换在计划中明确为重建，不伪装成事务
回滚已得到保证的热更新。

- Validate 检查能力、资产身份和输入合同。缺必需资源是 BLOCKED，损坏资产/配置
  是 FAIL；未声明为负向测试的 shader/pipeline 编译失败是 FAIL，不能作为正常
  “不支持”探测。预期拒绝/故障用例按下文的显式断言合同判定。
- Ready 要求场景依赖加载、准确接管覆盖且无错误、所需纹理、有效 bounds、预期
  效果组件和连续就绪 render 帧。update 次数或非黑像素不能代替完整覆盖。
  依赖集合来自 ScenePlan 与 renderer 的实际 pipeline/接管观测；不能只看全局
  pending 数为零，尚未请求的必需管线也要核对。构建器的结构声明只用于驱动
  readiness，不充当独立验证的期望答案。
  装载、接管、warm、读回与退役各有明确截止时间；等待 shader import 是暂态，
  持续等待超过截止时间为失败并保留诊断，不能无限阻塞任务。
- 扫描/变形用配方中的离散 tick、步长和 seed；资源等待不推进逻辑 tick。
  状态应用/传播后，将捕获绑定到该 revision/tick 的渲染及读回；主世界更新不等于
  GPU 呈现。迁移旧动画用例须保留采样姿态，单独核对时序。
- Capture、Reload、切换携带 generation/revision。退役代次停止新请求，旧 GPU
  请求在期限内排空但不能发布为当前结果；只释放自身实体/句柄，不能清空共享 Assets。
- Android 暂停时停止时间和测量；恢复检查 Surface/资源并重新 warm。进程重建
  从持久化配方恢复，不复用旧 Entity/Handle。中断测量不能拼接采样或报告完成。

窗口生命周期与上图正交。窗口宿主处于 Unavailable/Suspended 时保有配方，
不得提交依赖失效 Surface 的工作；Android 应用挂起同时暂停其整个实验。
无窗口的离屏会话将 Surface 标为 NotRequired，不把“没有窗口”误判为无法运行。
App 活动状态、Device 有效性和 Surface 就绪分别检查，恢复只重建实际失效部分。
Surface 创建/销毁交给 Bevy/Winit，场景库不得额外
保留 RawWindowHandle/Surface。Activity onPause 不等于已收到 Surface 销毁通知；
重复 suspend/resume 必须幂等。恢复后复用可存活的内容或重建代次，二者都重新
检查就绪。目标包含交互会话的 device loss 恢复：持久化配方/检查点和命令序号，
作废原 session 的 GPU 请求，由宿主重建后从确定状态重放；不承诺复用原设备对象
或保持呈现无间断。普通严格用例遇到 device loss 必须失败，不能恢复后擦除失败；
专门的恢复测试则明确预期故障、恢复边界和判定。恢复后的性能样本重新 warm，
不得拼接到原采样窗口。
正常退役可等待合法的在途完成；device loss、回调缺失或退役超期时，由宿主
撤销整个设备会话并隔离其回执，不再等待不可能完成的 GPU 工作。CPU 任务可取消
则取消，否则解除结果发布资格并限制驻留；不可终止的宿主任务由主机结束进程。
资源实际释放仍通过引擎/wgpu 所有权完成，不手工销毁被其他活跃实验持有的对象。
本地引擎通过移除窗口 raw handle、渲染提取时删除 surface 来处理 Android 暂停；
回调/渲染线程时序是否符合平台约束仍待实测，不能靠场景库另造一套生命周期修补。

捕获协议新增中性 `CaptureStamp`：run/case/session/experiment/generation/revision/
sample/tick/view/request，并绑定 ScenePlan 与 CaptureDescriptor 的 hash。
主世界只声明待捕获状态，stamp 随提取进入 render world；在对应 target 的渲染/
copy 提交阶段绑定 request，回执保留该 stamp。GPU readback 完成后才可发布图像。
现有 `ReadbackComplete` 本身没有 tick，不能在回调时补写“当前 tick”冒充原身份。
协议同时支持两类捕获，使用同一身份和判定规则：

| 捕获模式 | 执行规则 |
|---|---|
| 单步屏障 | 固定 revision/tick，完成全部声明 view 的读回后再推进，适合独立静态样本及确定性调试 |
| 连续流水线 | 按固定时间线产生多个在途快照，每个快照的状态、target/copy 与 stamp 一一对应；按身份归并异步回执，不能用完成次序推测采样次序 |

提取后的逻辑快照不可变；物理 GPU 资源可以按已证明的队列顺序复用。必须保证
对应上传 → 渲染 → copy → 下一次覆盖的顺序，不能在同一次 submit 前用两个姿态
的 write_buffer 覆盖同一区间，却把两次渲染声称为不同姿态。跨实验共享可变缓冲时
使用独立区间、版本或明确有序的 GPU copy。提交顺序不等于 CPU 已取得像素：
读回槽位要等映射成功、消费并 unmap 后才能回池，映射中或待消费不能再次供 GPU
写入。不能为简单实现把每帧全设备等待当作流水线捕获合同。
多 view 同属一个逻辑样本，完整集合到齐才可发布。每个逻辑请求只登记其指定
copy 的结果；重复传输仅接受同一内容，额外底层 copy 不冒用同一请求身份。
过期 session/generation 的回执仅排空。队列按请求数与总字节双重设限，
确定性离线捕获在快照入队前施加背压，逻辑时间暂停、平台事件继续推进；实时
观察模式可以丢帧但必须列出缺口，严格序列不能因此 PASS。测量模式不通过阻塞
改变负载来保证截图，捕获与性能采样窗口分离。

每个请求必须归结为已交付、已取消或失败的内部终态；这些不是新增的顶层任务
状态。相同 request 的重复回执只有在描述符和内容一致时可去重，冲突则失败。
接收过期回执只回收其自身资源，不恢复已注销的请求或占用新 session 的槽位。
队列预算同时覆盖旧代次排空、解码/编码任务及待写出数据；控制、取消和生命周期
通知有独立推进通道，不能因图像队列满而被阻塞。

终态发布前核对已登记请求均已结算，并关闭该执行的生产入口。终态后的取回/
重传不重新启动生产；若此时发现身份或内容矛盾，追加完整性故障证据并使资格
核验失败，不覆盖原始记录或将矛盾当作另一张可选择的正确图。

两类模式都必须覆盖 pipelined rendering；以提取/提交身份确认，而不是猜延迟
一帧或等待固定 update 数。Probe 的
Render Cleanup 计数只代表 render schedule 进度，不是 GPU fence 或像素正确性。
动态效果的 temporal history、自动曝光、质量 governor、随机动画与时钟都必须
显式冻结或以配方驱动；只固定主相机还不够。UI 显示用相机与严格捕获目标分离，
不让按钮/字体、窗口合成或显示缩放改变 Golden 输入。

需要 temporal history 的场景，配方规定 reset、预热时间线与捕获起点；跳到任意
tick 时必须恢复合格检查点或从 reset 重放。不能只设置目标姿态就声称得到相同
历史状态。检查点若未包含完整 GPU 历史则标记为逻辑检查点，恢复后重新生成历史。

### 像素与观测合同

CaptureStamp 只证明来源关联；另需不可变 `CaptureDescriptor` 定义被读回的
数据。描述符包含实验/view、输出阶段、extent、mip/layer/aspect、纹理格式、
sample count/resolve、色彩空间/传递函数、alpha 语义、行方向，以及从渲染值到
比较像素的完整转换链。实际 copy 的 offset/stride 与紧密布局分开记录。尺寸、
乘积溢出、字节长度、资源 usage 与格式能力在分配/解码前校验。

现有默认捕获为 `Rgba8UnormSrgb` 的紧密 RGBA8；参考离线采样另走 `Rgba16Float`，
按线性光求平均后裁剪并编码一次 sRGB。这两个 profile 保留各自名称、版本与
基线身份，不能被“统一捕获”改成同一路径。HDR 观测保留浮点原值；诊断 PNG
只是声明转换后的派生产物，不能取代 HDR 原始观测。NaN/Inf 像素不能靠 clamp
隐藏；特殊值测试须声明对应的独立判定。

MSAA 输出先按声明规则 resolve 到可复制目标；深度、法线、mask 和最终颜色是
不同观测类型，不按 RGBA8 任意解释。GPU copy 的行 padding 只做布局剥离，不能
改变像素；不因输出尺寸不同自动 resize，不以平台 framebuffer 格式决定严格
比较域。RGBA/BGRA、线性/sRGB、预乘/直通 alpha 的转换必须显式且版本化。基线
比较优先使用规范解码像素 hash，同时保留文件 hash；PNG 压缩/元数据变化不等于
像素变化。错误格式、缺失 view、截断读回和重复身份冲突均不可成为合格观测。

验证层先校验完整观测集合，再执行 EvaluationSpec。mask/ROI 及预处理来自独立
合同；任何从实测图派生的诊断区域都不得自行缩小必需断言范围。指标报告区分
有限数值、合法特殊结果和无效结果，例如零误差 PSNR 可编码为有标签的正无穷，
不写非法 JSON Infinity，也不把缺失/null 当作比较通过。捕获完成、指标计算完成
和全部适用标准通过是不同事实，任务及领域结果继续分别记录。

## 资产复用与验证边界

程序化与第一方 synthetic fixture 是默认内容；第三方角色为显式本地 bundle，
保持 ignored，不进入产品默认依赖。PC 目录、APK assets、浏览器资源路径只是
同一逻辑 asset ID 的传输位置。GLB、NPR/view sidecar、buffer、贴图和可选 focus
的完整清单与哈希绑定，不能只检查三个文件存在。打包只复制依赖闭包，不复制
整份 authoring 目录；失败/未完成的编译产物不能作为输入。

取消 Android 特有的编译期描述符嵌入：描述符和贴图从同一 bundle 经 AssetServer/
资源适配器加载，选择场景不再重编 Rust。打包前验证并生成资源目录，启动后验证
清单与加载结果。未打包角色在目录中显示原因，程序化场景仍可运行；严格选择
尚未提供的外部角色须 BLOCKED，不能偷偷显示球体。计划已承诺打包而实际漏项
则属于构建/打包错误，按 FAIL 处理。

**共享资源传输和装配，不合并不同输入验收。** 遵守
[参考与 Inspector 输入边界](../testing/03-capture-input-boundaries.md)：参考 package
验证器保留路径、校准、六视角和采样合同；Inspector 保留严格 bundle/recipe
规则。各自验证后生成共享运行时输入。Android/browser 的普通 compiled bundle
采用 Inspector 同等级完整性检查，不能成为宽松绕过入口。公共材质接管继续调用
`StylizedAssetIntegrationPlugin` / `StylizedMaterialSet`，不复制转换或 shader。

共享运行时接收带验证域/验证器版本/清单 hash 的不可变已验证输入，不靠一个
普通路径或布尔 `validated` 标志跨越边界。域适配器说明 glTF scene/node/primitive
选择、坐标/单位、材质映射、相机来源和采样规则。当前 Inspector 的 scene 0
计数语义不能被共享 loader 自动改成 glTF default scene；参考域继续使用它的
声明范围。跨域复用只传递双方共同认可的数据，不能把某域的通过印章转给另一域。

校验分两步：纯策略先验证逻辑路径/依赖清单/描述符；异步资源适配再提供实际
字节并校验哈希。PC 适配保留 canonical containment，APK/浏览器使用受控资源
命名空间，不伪造 `canonicalize`。送入 loader 的字节必须就是已校验字节，可用
验证 reader 包装或已核验的不可变快照；不能先 hash 原目录再从可变路径重新读。
每次 bundle 更新创建新内容身份，热重载不原地改写正在执行的 bundle。大资产
允许逐文件异步校验与释放临时副本，不能强制同时在内存保留整个包的多份字节。

打包需要记录逻辑 URI 到传输位置的映射，并检查目标文件系统/容器能否无歧义
表示它们；大小写或编码别名冲突必须显式报告，不得加载“找到的第一个”文件。
此检查归平台打包适配，不偷偷收紧参考或 Inspector 的原输入域。若需要改名或
格式转换，生成完整重写引用的新 bundle 身份；严禁改字节后沿用原清单 hash。

物理缓存与逻辑绑定分开：`bundle hash + 相对路径` 隔离同名包；纹理身份还包含
解码角色、sRGB/data、sampler、mip 等设置。同字节不同解释不能合成一个 Image
handle；如需多个逻辑 URI 指向同一字节，由清单显式声明，避免重复文件。现有
材质接管的路径前缀必须使用该 namespace；不得绕过它提前以默认颜色空间加载
纹理。不能表达的角色冲突拒绝输入，不能静默改贴图语义。

原语只在输入语义一致时复用。sRGB/data 格式、顶点布局、尺寸、法线、UV、joint
weights、sampler 必须显式参数化或保留独立 preset；不能用到处修改的万能默认
材质掩盖差异。

会话资源预算覆盖活跃内容、候选 revision、旧代次排空、贴图解码、GPU 读回和
待写出产物的总峰值，并记录预算来源及实际占用。缓存只能回收无活跃拥有者的
条目；持续超预算必须有界结束，不能通过无限延迟写出维持运行，也不能静默
减少实例、纹理质量或捕获尺寸。声明的设备限制不足按前置资源合同分类，合法
请求运行中的 OOM/写出失败保留为执行失败；这不是新增的性能合格预算。

## 能力、捕获与性能政策

统一指同一配方可执行、可解释，不保证每台设备支持所有效果。宿主初始化时
申请 device policy，创建后以实际启用的 features/limits 交给现有 resolver；
不新增场景层 renderer。Surface Class 不随 tier 降级。

交互可接受已注册 fallback，但 UI/报告显示 requested/resolved tier 和缺失效果。
严格用例声明 required effects：hybrid 缺 screen edge 不能按 hull-only 通过；
GPU timestamp 缺失不能用 CPU 时间代替。声明的外部能力/资源缺失是 BLOCKED；
构建、实现及有效能力下的渲染错误是 FAIL。必选项不能自动跳过，非选中项不加入通过计数。

这里的 BLOCKED 指声明正确的执行请求缺少外部前提，不包括遗漏用例、构建器
未实现、错误打包、schema/目录不一致或本应编入 QA 包的 feature 缺失。后者在
要求这些能力的验收中是 FAIL；目标规划中尚未实现的功能保持未完成，不能借
“设备不支持”消除覆盖义务。能力检查还包含捕获格式/usage、AA resolve、尺寸
limits 和 instrumentation，而不仅是产品 tier。

负向测试须在 EvaluationSpec 声明目标错误类型、阶段、作用域和后置条件。
例如缺能力后的明确拒绝、合法 fallback 或故障注入恢复，可以在断言全部符合
时通过；实际测试设备/输入缺失仍是 BLOCKED。意外 shader 错误、进程崩溃或超时
不能被宽泛的“期望失败”吞掉。UI 只可显示负向用例通过，不能将故障渲染本身
标为图像通过。现有基于日志的资源分类规则仍按任务合同执行，未来结构化协议
不能借迁移扩大正常跳过范围。

预期进程终止的测试由独立控制者判定：被注入故障的子执行仍如实记录失败/中断，
外层用例只有核对精确故障、报告身份、资源边界及恢复后置条件后才能 PASS。
不能让已崩溃的进程自行证明成功，也不能将其子执行 FAIL 擦除。计划在运行前
声明这种父子关系，普通用例不能在看到失败后临时改成负向用例。

### 目标平台与组合覆盖

以下是设计目标域，须逐格建立构建、运行、语义和性能证据；不是当前支持声明。
后端适用性由目标 OS、引擎编译能力及实际设备约束判断，不按平台名硬编码画质。

| 目标宿主 | 待覆盖的后端/运行域 | 需独立证明的差异 |
|---|---|---|
| Windows PC | D3D12、Vulkan；交互与 headless | Surface/离屏、输入、驱动、设备重建 |
| Linux PC | Vulkan；交互与 headless | 窗口系统、无窗口 runner、真实 GPU 与软件诊断 |
| macOS PC | Metal；交互与 headless | 生命周期、离屏、读回、实际能力 |
| Android | 真机 Vulkan、模拟器 host Vulkan；软件 Vulkan 为诊断域 | 触摸、APK 资产、Surface 暂停/恢复、进程重建、热状态 |
| iOS | Metal，待建立宿主 | 平台事件/资源、应用恢复、捕获与导出 |
| Browser | WebGPU，按实际浏览器/适配器区分 | 异步资源/读回、页面生命周期、主线程与 worker 边界 |

GLES/WebGL 等兼容后端需先有单独的产品支持合同和能力映射，再进入可执行矩阵；
它们不是 Vulkan/WebGPU 失败时自动替换的等价路径。矩阵保留“不在合同域”及其
理由，与“在域但设备/依赖缺失”的 BLOCKED 区分。Android 真机性能单列，模拟器
不代表手机功耗、温度或帧预算。

覆盖规划由同一目录和验证合同生成，目标是解释全部覆盖义务，而非仅选择当前
机器能运行的样本：

- 离散维度包括场景族/preset、Surface Class、Execution Tier、效果组合、变形、
  view/AA、资产通道、动作序列和目标后端。按语义约束展开所有有效的声明组合；
  每个排除条件必须有合同依据，不能以运行成本或缺设备删掉必需组合。
- 连续参数不能穷举，用合同定义边界、临界值两侧、等价区间和可重放的生成测试。
  除图像基线外，验证有明确前提的不变量及变换关系，例如 enable/remove/restore
  的状态恢复、实例排列不改变各自材质语义。关系的期望必须由独立验证层定义。
- 单因素、交互组合、时间序列与平台一致性分别报告。pairwise 或代表性子集可供
  日常诊断，但不能冒充全矩阵完成；旧必需测试与新覆盖义务均有可定位 case ID。
- 生成计划先于执行固化并带 hash，逐项显示未实现、未执行或已执行证据；这些是
  规划元数据，不另建一套任务 PASS/FAIL/BLOCKED 状态。无断言的展示没有验证覆盖。

`CoveragePlan` 必须定义有限域：本版枚举值、数值采样集、组合深度/实例规模、
动作序列长度、目标平台/设备类别及约束版本。所谓完整展开只针对这个显式域，
不声称穷尽无限的用户配方、驱动和硬件。边界来自合同、算法分段和设备 limits，
不能为了缩短运行而偷偷缩域；新增功能/边界同步扩展计划并保留缺口。生成过程
自身也需独立核对预期基数、约束理由及旧用例映射，不能让错误生成器同时决定
用例集合与“覆盖完成”的答案。

场景数量不是覆盖率。报告须从“合同/效果 → 配方 → case → 平台执行 → 判定/产物”
追踪缺口；用例减少只有在证明语义包含关系且原合同所有者同意后才能发生。

捕获保留 scene/preset、展开 recipe/hash、源码/lock、完整资产身份、generation/tick、
分辨率、投影、AA、色彩/曝光、实际设备/backend、能力解析及实际完成帧。同设备/
驱动/工具链重现、已获准的跨设备/backend 比较、Blender parity 分别判定。沿用现有容差与 exact 合同，
不因重构放宽阈值；缺少合格比较规则只能报告诊断结果。

测量和捕获共享场景，但采样窗口分开。保留 warm-up、计时帧数、重复次数和 GPU
sample 完整性要求，截图/编码在计时之外。报告区分 CPU update、GPU render-graph
span、allocator 计数和真实呈现帧率，不用实例数推算 draw calls。只比较同设备、
backend、配方、采集器语义和质量合同。更改计时范围须新身份，旧数据不自动成为
新 baseline；规则见[性能所有者](../performance/targets.md)。

目标观测覆盖 CPU 阶段、GPU pass/span、帧间隔分布、管线/批次/实际 draw 统计、
资源分配与驻留、装载和接管耗时、持续运行热状态；各指标注明来源、单位、采样
范围、精度与观测开销。按数据适用性报告分位数、离散程度和重复测量不确定性，
不以单一平均 FPS 代替。设备不提供某指标时标为不可得，不能用推算值冒充实测。
无 instrumentation 的呈现性能与开启诊断的测量分开，新增采样合同不得覆盖旧
workload/target 的数值身份。跨设备可描述同一负载的差异，不直接套用同设备回归 gate。

执行隔离分为两种明确模式：

| 模式 | 会话与资源政策 |
|---|---|
| 交互及专门的切换/重载测试 | 同一 SessionPlan 下共享 App/Device；记录并恢复场景自有状态。全局 pipeline 错误后要求新会话，不把旧错误当作当前场景正常阻塞 |
| 严格语义/GPU gate | 保留桌面逐用例进程合同；Android 主机串行重启应用进程，每次一个必需用例。测试内部明确要求的状态序列仍在一个用例里执行 |

设备进程无法安全 `fork`/自建第二个 EventLoop 时，由 adb 适配器重启，不在 app
内部模拟进程隔离。主机逐条核对启动和退出；失败不吞掉剩余用例。进程隔离不
等于驱动 shader cache 清空，性能必须另外记录 warm/cold/instrumented 范围。
其他目标平台的宿主也须声明并证明隔离级别；新页面或 Worker 不等同于新进程。
既有 browser 比较保留自身合同，不能因共用 driver 就宣称满足桌面逐进程 gate。
目标平台若无法提供某用例要求的隔离，保留该覆盖义务并报告 BLOCKED。
benchmark 保持自身声明的重复/采样合同，不能因统一 runner 改成未经说明的新流程。

结果通过[任务合同](../testing/02-task-results.md)汇总 RUNNING/PASS/FAIL/BLOCKED。
Android 输出可取回的结构化报告和目标图像；logcat/窗口截图仅作辅助，不能凭
接管日志判定图像通过。每个结果绑定 run/request/case、recipe/session hash、
源码/lock/build 身份、实际设备、各阶段结果和带哈希的 artifact 清单。主机同时
核对安装构建与运行报告身份，缺报告、旧报告、身份错配、失联或异常退出不能 PASS。
先写候选文件，再原子发布完成记录；写出失败保留 RUNNING/失败证据，不先发 PASS。
每次启动只接受当前请求，不从 convenience 文件推断通过；取回目录不可覆盖旧 run。

完成标记是整个产物集合的提交点：先完成所有必需文件和清单，主机取回后核验
其字节/大小/hash，再接受对应领域结果。原子重命名不单独证明掉电后的持久性；
平台须声明提交保证，恢复时重新校验。传输可分块和续传，但部分文件、目录存在
或设备侧 PASS 日志都不是主机完整验收。取消与完成竞争由同一控制者串行裁决：
已提交终态不被迟到取消改写，已接受取消不能随后提交 PASS。

整批结果与运行前冻结的预期集合核对，键包含 case、配方、目标执行条件和重复
序号；不能从已有产物反推应执行集合。拒绝重复、额外、缺失和空的必需集合，
继续统计每个计划项的已执行/未执行及原因；FAIL 优先于 BLOCKED。重试属于新 run，
原失败始终可见，不从多次运行中挑选通过结果拼成一次完整成功。诊断子集保持
选择范围，不能覆盖完整矩阵结论。

同一 validation 实现也需要各执行目标的数值合格证据：解码精度、归约顺序、
特殊值及阈值边界用共同合同向量核对。设备判定和主机复算必须引用同一
EvaluationSpec 与规范观测；不一致时报告判定错误，不能选择有利结果或扩大
容差。原指标的数值路径和比较规则保持不变，新增判定后端单独取得资格。

## 原有实验覆盖目标的保留

下列原规范目标继续保留，迁移逐项标注“已有验证 / 仅展示 / 待实现”。它们不是
当前实现声明，也不是本次结构重构顺带实现全部新效果的范围。

| 场景族 | 原覆盖要求 |
|---|---|
| ramp | 1/2/3/5 bands、256 authored ramp、smoothing、day/night atlas、360° 光照、时间稳定性 |
| lighting / environment | Physical/Authored/Hybrid、key light、physical influence、palette、environment；日出/白天/日落/夜/雨参考 preset |
| face | 前后左右上下光向、Generic/Gacha、mirrored right、face correction、hard/soft factor |
| hair | tangent/shift/power/threshold/softness/mask、光照/相机运动、蒙皮/镜像 |
| metal | MatCap/specular ramp/material class、相机/光照旋转、packed normal、day/night |
| outline | 球/方块/凹面/蒙皮/Morph/hair cards/薄几何/大小尺度，1/2/4/8/24 px，screen-space hull/source-local hull/internal edge |
| deformation / lod | skin、morph、组合、极端姿态、快速运动、mesh swap、LOD、bounds |
| post | HDR/bloom/vignette/distortion/chromatic/display transform/tonemap/prepass，显示 resolved camera pipeline |
| instances | 100/300/500/1000，outline/face/hair/post 组合；帧时间、draw calls、材质/可见实例、内存、tier/bindless（不可用指标显式标记） |
| reference | 固定角色/姿态/相机/光照/曝光/背景/分辨率；Golden 验收独立于展示和测量 |

最小自动 fixture 仅用自制、CC0 或明确可再分发资产。现有 first-party reference、
synthetic character、extended-skin、已接受 Golden 保留。原建议球、方块、凹面、
face-SDF head、two-bone、morph cube、hair strip 可用确定性原语表达，不为文件名
重复导出几何。Blender 作者源与独立期望资产不能因可生成运行时几何而删除。

## 研究结论、未决证据与完成条件

本节规定完整目标的证明义务，不是本轮执行计划，不以“最小原型通过”代替整体
设计验收。架构评审与运行证据分开：现有资料足以否定阻塞式跨平台 runner、
仅靠 tick 猜测读回帧、路径级贴图去重和把当前初始化接口当作最终能力边界；
不能据此宣布新捕获协议、设备恢复或新平台已经可用。

### 尚待实现和实证的关键问题

| 问题 | 已知事实与目标决定 | 尚缺的证据 / 责任所有者 |
|---|---|---|
| 事件推进与 Surface 时序 | winit 的生命周期约束明确；宿主拥有事件循环，runtime 非阻塞 | Bevy 当前 Android 回调/渲染线程时序、暂停中读回及重复恢复的实测；引擎/平台宿主负责 |
| 会话重配置与 device loss | Device 请求能力不能原地增加；目标采用重配置事务与检查点重放 | ~~产品公共重配置 API~~（2026-09-22 已交付，ADR-0030）、宿主设备重建入口、各平台失败恢复语义；引擎/宿主负责 |
| 精确帧与连续捕获 | 当前读回没有 tick；目标 stamp 跨提取/提交/完成，快照不可变 | 多在途（不同 tick/同 tick 多 target）、重复完成、取消、编辑后新鲜内容的身份与像素一致性已由 `tests/harness/tests/capture_stamp.rs` 本地证明（2026-09-22，Metal）；乱序强制构造、多 view 组合与 temporal history 仍缺；harness/runtime 负责 |
| 资产身份与缓存 | APK 非文件系统，同路径加载设置可能复用首次值 | 实际 loader 字节核验、同名包隔离、多语义绑定、重载与资源稳定；资产适配/runtime 负责 |
| 共享 UI 与声明模型 | 场景语义可与输入/布局分离；模型是目录/控件/回放的共同来源 | 全部旧场景能无丢失表达，触摸/键鼠操作得到等价配方，事务/撤销可重放；model/content/UI 负责 |
| 参数化覆盖与独立判定 | CTS 展示参数生成与 case/subcase 的组织方式，不证明本项目覆盖完整 | 旧身份到新配方映射、约束理由、独立期望与未实现组合清单；validation/合同所有者负责 |
| 规范身份与控制协议 | JCS 解决规范编码，不解决语义默认值或执行一致性 | 规范化向量、协议幂等/取消/崩溃、正式包导入导出、产物闭包；model/宿主工具负责 |
| 可移植依赖与观测开销 | 当前 integration 和 Inspector 存在主机/UI 耦合 | 各目标独立依赖图、纯判定无设备、采样来源及扰动测量；各开发库/benchmark 所有者负责 |
| 实验分区 | 当前全局 view 和全 Camera LOD 不等于独立多实验 | 预览/光照/材质/LOD/时间相互不干扰，跨分区资源清理正确；产品/runtime 负责 |
| 时钟与像素 | Bevy 默认 Time/FrameCount 与自定义 tick 不自动一致；既有两类 capture profile 不等价 | harness 时钟域已定义、"额外 update 不改变捕获绑定"已本地证明（2026-09-22）；实验调度/舍入/随机流、奇数行宽/格式/resolve/色彩向量仍缺；runtime/harness 负责 |
| 判定与恢复身份 | 图像 hash 不能标识判定规则，进程内去重不能证明崩溃后的恰好一次 | 冻结 EvaluationSpec、负向用例边界、日志确认/产物提交故障窗口；validation/协议所有者负责 |

这些缺口不会被“保持旧实现更便宜”关闭，也不能靠扩张场景库职责绕开。若证据
否定某项机制，应修改对应机制及合同，保留跨端同义、可复现、独立判定的目标。
当前没有执行上述实验，不能把待验证项计为通过。

### 完整目标的验收矩阵

以下事件序列用于检验各合同的交界，不是已执行测试记录。每项都要求保留事件
身份、确定的终态及无串场的观测，不能只检查程序没有崩溃。

| 事件序列 | 必须得到的结果 |
|---|---|
| 同 session 的 A 已提交 → 候选 B 接管失败/被取消 | A 保持原身份；B 有失败/取消记录并只清理 B；不得把 A 的图发布为 B |
| A 的读回在途 → B 成功提交 → A 回执晚到 | A 已失去发布资格；晚到回执仅释放 A 的资源；B 的请求数、像素和终态不变 |
| 请求在途 → device loss → 同计划建立新 session → 旧回执到达 | 原严格执行失败；新 session 身份不同；无需等待旧 GPU 才能终止，旧回执不能命中新槽位 |
| 在同 tick 对两个独立实验或两次历史重放采样 | experiment/sample/view/request 共同区分输出；不能只靠 tick 或 plan hash 合并 |
| 图像队列已满 → 取消；或完成提交与取消同时到达 | 控制仍能推进；唯一控制者决定先后，已接受取消不得提交 PASS，已完成不被迟到取消改写 |
| 设备写完报告 → 主机只取回部分文件/发现 hash 不符 | 主机不接受通过；续传完成且身份/全部产物校验成功后才可核验领域结论 |
| 同一观测在设备和主机判定结果不同 | 保留两份结果并报告判定错误；不选择较宽松的一方，不调整基线或容差 |
| 必需矩阵中一项失败、一项缺资源、另有未执行项 → 重试部分通过 | 原运行聚合 FAIL 并列出全部缺口；重试是新 run，不能拼接成原运行 PASS |

| 验收域 | 完成条件 |
|---|---|
| 清单与所有权 | 17 展示项、16 组/41 GPU 用例、9 workload、旧四组 benchmark、42 sweep cells、参考六视角、Inspector/扫描/browser/Android 全部有明确去向；每种构建/相机/动作/判定有唯一所有者 |
| 声明与身份 | 旧场景完整映射；组合展开、默认值、约束、动作顺序、schema 与 hash 合同明确；非法/循环/冲突输入拒绝；跨语言合同向量相同；基线/ROI/mask/判定器身份锁定 |
| 跨端交互 | 同配方同参数语义，目录与诊断功能一致；保存/重载/录制/撤销/回放保持逻辑身份；显示布局不改变严格捕获 |
| 帧与时序 | 两姿态及连续时间序列可逐帧核对；延迟/乱序回执、reload、cancel、多 view、history 重放不串帧；pipelined 与单步模式均满足身份合同；不同等待/update 次数和异步到达顺序不改变实验状态 |
| 实验隔离与像素 | 添加预览或修改 A 侧不改变 B 侧；LOD 观察集合、光照、材质和历史显式归属；非对齐行宽、RGBA/BGRA、线性/sRGB、HDR resolve 和异常字节输入符合捕获 profile |
| 生命周期 | A→B→A、反复 reload、后台恢复、进程死亡、device loss、跨 SessionPlan 切换按合同结束/恢复；无重复系统、残留 owner 或持续增长的场景资源 |
| 资源 | PC/APK/browser 同一 first-party 及合格角色 bundle 的实际字节闭包一致；同名不同包、颜色空间/sampler 冲突、缺图/损坏/hash 错配得到准确结果 |
| 判定与隔离 | 同观测使用同一断言；FAIL、BLOCKED、未执行完整核对；shader/断言/进程故障不污染下一隔离用例；原 Golden、browser exact、parity 规则不削弱 |
| 控制与产物 | 请求幂等、过期拒绝、取消、超时、丢失连接、写出失败可追踪；在接受确认、检查点、产物提交各边界终止后恢复无误报；构建/安装/运行/取回身份一致；残留报告不能充当新结果 |
| 测量 | 固定 workload/采集器合同保留；headless 无 UI 依赖；指标来源、缺项、扰动、热状态和重复性可解释；截图/恢复不混入采样窗口 |
| 平台与覆盖 | 目标域全部枚举；有效离散组合及连续参数覆盖义务可追踪；实际构建/运行/判定证据分开；设备缺失/未实现不计为完成 |
| 依赖与清理 | 各宿主/展示/QA feature 独立验证；六个产品 normal/build 依赖不反向；所有旧调用/注册/导出/资产职责转交后才删除重复实现 |

未来迁移需先建立旧合同到目标所有者的完整映射，再按依赖关系转交职责；模型
与接口定义先于消费者替换，实际等价证据先于删除。这里不规定以成本为依据的
分期，也不授权开始实现。迁移期间的适配在职责转交后删除，不保留两套永久路径。
旧结果保留原身份、日期和数值；source hash 与模块清单必须覆盖全部共享库，
防止迁出模块后漏计源码变化。

删除前核对 Cargo、入口、`tests/suites.json`、`tests/current-contracts.json`、Python
runner、独立 consumer 模板、Gradle 打包、source snapshot/identity、exporter 和文档。
若 Android 路径更名，同时迁移生成物排除规则，不能把 Gradle 输出当源码。
当前应用 ID 可保留以便原位升级，不承载场景逻辑。

未来实际验证仍遵守仓库运行纪律：

- CPU/架构/文档与最小 feature 检查，各目标独立构建；共享库不拉桌面后端，
  六个产品 normal/build 依赖仍与开发层隔离。
- 原必需 GPU 断言、Golden、资产接管与独立 consumer；分阶段运行受影响检查，
  完成后核对完整清单，不能用 Showcase non-black 代替。
- PC/Android 相同已选 recipe 的加载、动作与捕获；来回切场景、pending capture
  时重载、后台恢复、缺失/损坏资源、shader 错误路径；无残留实体、多余相机/系统
  或持续增长的场景自有资源，异步资源允许有界排空期。
- measurement、parity、image regression 分项报告；同设备迁移前后测量。Cargo/GPU
  批次串行且 `CARGO_INCREMENTAL=0`，仅本地执行，不启用远程 CI。

验收保留两种资源事实：主动 owner/实体/句柄应完成回收；驱动/引擎缓存可稳定
驻留但必须有记录。计时采样期间 UI 展示、资源 hash、截图、写出和生命周期事件
不得改变负载；被中断的采样重新开始。原有已失败用例仍单列，迁移后没变坏不等于
全部质量门已通过。本轮没有新增跨设备容差、GPU 百分比 gate 或绝对帧预算。

本轮设计文档只检查链接、命令引用与变更范围，不运行 Cargo/GPU，不接受新基准。
