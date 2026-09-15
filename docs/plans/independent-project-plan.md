# Bevy Stylized 独立项目重构计划

> 状态：v4，用户已授权执行；实际进度见 independent-project-execution.md。
> 本文是实施提案，不是已完成的重构，也不修改现有 ADR 的状态。
> 基线：a47af6c，2026-09-14。
> 依据：[调研](independent-project-research.md)；[审查记录](independent-project-review.md)。

## 1. 要达到的独立性

在没有 Tenacity checkout、环境变量、源码、导出器、游戏 GLB 和缓存的环境中，
项目能够独立构建、生成自制资产、执行 CPU/GPU 验证，并从直接来源准备一个明确
支持的 Genshin 角色用例，完成 Blender 制作、导出和 Bevy 接入。

**活动目录中移除 legacy 和 Tenacity 工作流，不以改名字或复制旧加工产物替代解耦。**
历史迁移记录、必要署名和原始来源保留，限定为非规范、非执行的证据。运行时继续
依赖 Bevy，Genshin authoring 继续依赖明确锁定的 Gacha/Blender/直接资产来源；
“独立”不等于重新实现这些上游工具。

保留六个产品 crate、既有公开名称，以及 Surface Class × Execution Tier ×
Validation Mode 和 Material/Instance/View 三层模型。覆盖独立化必需的资产桥、
材质接入、测试和目录治理；不扩展到全部 Gacha 游戏、所有效果复刻、Tenacity
19 个动作重建、全面 GPU-driven 改造或未经决策的 1.0 发布。

区分两项结论：

- **工程独立完成**：本计划 I01–I12 验收成立，选定用例完全不读取 Tenacity。
- **视觉资格/发布完成**：额外满足对应上游 parity、平台、许可证和 V1 release gates。
  已知视觉未达标仍显示未达标，不因独立化改变标签或判据。

## 2. 建议确认的架构决策

| ID | 提案 | 理由与约束 |
|---|---|---|
| D01 | 保留一个 Cargo workspace、六个产品 crate | 产品边界已基本成立；避免无收益拆 crate |
| D02 | 活动源码移除两个 legacy 包及 odette-reference-lab | 不再把迁移期游戏 renderer 当项目永久架构 |
| D03 | Blender 独立行为参考 + 历史批准基线回归 + 合成语义断言 | 避免自验证；旧 engine 图像只保留历史迁移证据 |
| D04 | 自制 fixtures 为公共必需验证，外部 Genshin pack 显式准备/资格验证 | 默认验证不能依赖第三方模型或另一个游戏 |
| D05 | 源资源、创作工程、运行时包、捕获证据分层 | 任何结果可以追溯输入和生成过程 |
| D06 | 运行时 shaders 留在 render crate 内 | 保证嵌入路径和 package 自包含，不新增根 shaders 副本 |
| D07 | 纯 glTF 契约与 Bevy 注册 feature 分离 | tools 不必加载 Bevy 渲染依赖链来解析文件 |
| D08 | repo 内技能和文档为真源，个人安装可选 | 没有个人 Codex 配置也能操作项目 |
| D09 | 缺必需输入、GPU、基线或 coverage 必须失败 | 杜绝假绿色和参数降级替代真实用例 |
| D10 | 保留 publish=false，独立化与公开发布分别批准 | 不替用户决定许可证和注册表发布 |

D02/D03 需要修改现有迁移架构和 ADR-0007 的方案/证据要求，并同步 ADR-0014、
架构索引及 TRACEABILITY。用户确认本计划授权实施这些提案；仍须在具体变更中
给出 Decision/Evidence/Consequences，不自动把其他未决 ADR 全部标 Accepted。

## 3. 最终目录与所有权

```text
bevy-stylized/
  Cargo.toml / Cargo.lock / rust-toolchain.toml
  README.md / ARCHITECTURE.md / CONTRIBUTING.md / AGENTS.md / THIRD_PARTY_NOTICES.md
  crates/
    bevy-stylized/                  # facade / integration / examples
    bevy-stylized-core/             # domain descriptors, capability, schema types
    bevy-stylized-render/           # render modules and src/shaders
    bevy-stylized-gltf/             # pure contracts plus optional Bevy adapter
    bevy-stylized-profile-genshin/  # family interpretation, no Bevy render internals
    bevy-stylized-tools/            # validate / inspect / compile / schema export
  apps/showcase/                    # showcase package
  labs/
    reference/                     # reference-comparison package, no legacy
    benchmarks/                    # benchmarks package, reusable benchmark scenes
    regression/                    # regression-lab package
    spikes/gpu-driven/             # gpu-driven-spike package
  tests/
    harness/                       # golden-harness package, no product dependencies
    integration/                   # new stylized-integration-tests package
  tools/
    dev.py                         # documented orchestration entry
    blender/                       # import, audit, export, capture and shared modules
    assets/                        # fetch, prepare, manifest and provenance tools
    checks/                        # dependency, path, schema, docs, independence checks
  schemas/
    authoring/                     # source/export/attribute/toolchain contracts
    runtime/                       # generated snapshots of Rust runtime schemas
  assets/
    sources/                       # source manifests and file digests only
    recipes/                       # authoring, packing, capture and case recipes
    local/                         # ignored raw/authoring/runtime per external pack
  fixtures/
    synthetic/                     # self-made generators and small portable test inputs
    golden/                        # accepted first-party PNGs and provenance
  docs/
    architecture/ authoring/ reference/ adr/ quality/ performance/ release/
    history/                       # non-normative source/migration history
    plans/                         # proposals and reviews
  .agents/skills/                   # repository-owned workflow skills
  .github/workflows/
  target/                          # ignored compilation, temporary reports and captures
```

这是最终布局，不创建没有消费者的空目录。保留现有有效的 docs 子目录，非必要不
搬动全部文档；优先修复当前入口和权威链。产品单包 tests 留在所属 crate，最小
Rust examples 留在 facade/examples。根 tests/integration **必须有 Cargo.toml**，
不能假定虚拟 workspace 会自动执行根 tests。暂不新增独立 outline/post/assets 包。

### 3.1 完整迁移映射

| 现有位置 | 目标/处置 | 同步责任 |
|---|---|---|
| 六个 crates/bevy-stylized* | 原地保留 | 仅改职责边界、feature、元数据 |
| crates/dev/showcase | apps/showcase | manifest、捕获输出、场景路径、CI |
| crates/dev/benchmarks | labs/benchmarks | regression/showcase 调用和报告路径 |
| crates/dev/regression-lab | labs/regression | baseline 路径、任务参数和 matrix |
| crates/dev/reference-comparison | labs/reference | 先去 legacy、输入显式化，再移动 |
| crates/dev/gpu-driven-spike | labs/spikes/gpu-driven | 修正当前 ../../target 路径错误 |
| crates/dev/golden-harness | tests/harness | 所有 dev 依赖及 provenance/readback 路径 |
| render 中跨 crate/文件/GPU 集成测试 | tests/integration | 纯模块测试原地保留，移除 render dev→facade |
| crates/dev/odette-reference-lab | 删除活动包 | 用例职责迁入通用 reference/integration，旧证据归档 |
| crates/legacy | 删除活动目录 | 退出账本达标后移除；Git 历史和 notices 保留 |
| tools/blender-reference-export | tools/blender | 脚本拆模块，所有命令与相对 imports 同步 |
| tools/check_docs.py | tools/checks/check_docs.py | dev 入口、CI、文档和测试同步 |
| tools 下契约 JSON | schemas/authoring | writer/reader/import test 全部同批切换 |
| fixtures/reference | fixtures/synthetic/reference-scene | generator、测试、manifest、示例同批改路径 |
| fixtures/golden-local | 分拣后迁移 | 自制批准基线→fixtures/golden；第三方→assets/local；诊断→target |
| docs/migration 旧执行计划和 Tenacity review | docs/history | 保留原始内容并加历史标记，当前索引改指新规范 |
| 个人 blender-gacha / bevy-stylized 技能 | .agents/skills 同名目录 | repo 版本为真源；个人副本显式同步、无外部必需链接 |
| .kimi-code/mcp.json | 可选本地集成说明/配置模板 | 核实用途，锁工具版本；构建/authoring 不依赖其存在 |

搬迁先做行为等价变更，后做路径 move；每个 move 同步 Cargo manifests、
include_str/include_bytes、embedded asset URI、脚本 imports、CARGO_MANIFEST_DIR
用法、workflow path filters、文档、skill references。旧路径短期包装入口若需要，
只能在过渡阶段存在并在最终检查删除。

## 4. 依赖边界与 Feature 模型

| 包/角色 | 允许的项目 normal/build 依赖 | 禁止 |
|---|---|---|
| core | 无其他产品包；保留必要的 bevy_math/color/reflect、serde | 完整 Bevy renderer、profile、工具、角色数据 |
| render | core | facade、gltf/profile 反向依赖、labs/fixtures、角色名 |
| gltf pure | core | Bevy 引擎类型；其独立 engine feature 例外 |
| gltf 的 bevy-integration feature | core + Bevy 注册/loader API | profile-specific shader 实现 |
| profile-genshin | core | bpy、Bevy render internals、Tenacity |
| tools | core + gltf pure + optional profile-genshin | Bevy renderer、facade、harness |
| facade | core + render/gltf/profile 的已声明 optional edges | labs、测试、Blender、资产下载 |
| harness | 外部 Bevy/image/serde 等 | 产品 crate、任何一个具体角色 |
| apps/labs/integration | 显式所需产品/harness/benchmark 包 | legacy、外部游戏 workspace |

gltf 拆分要先把 attributes.rs 中 MeshVertexAttribute/VertexFormat 与纯字段规范
分开，再加 engine feature；tools 使用 default-features=false。对已有 facade
默认功能保持兼容。**不承诺 facade --no-default-features 是纯 CPU core**：当前
capability 模块仍可能需要 bevy_render。真正纯领域使用 core；如将来重构该点，
需要单独 API 依据。

Genshin 的术语和算法可以作为显式 family 支持存在。profile 拥有来源参数、
材质角色和模式选择；render 实现可复用 shader 模式，不读取角色名、文件路径或
Tenacity 常量。不能以“零 Genshin 字符串”作为架构合格标准。

守卫使用完整 cargo metadata resolve 图，遍历 normal/build 的传递边；按每个
产品和 feature 组合检查，另外检查 dev 边界与所有 path.realpath。运行在没有
workspace dev-feature 统一影响的外部 consumer 中，验证 minimal/default/
profile 等组合；维护白名单角色表而非容易漏包的黑名单。

## 5. 独立资产与版本管理

### 5.1 三条资产线

- **公共合成线（必需）**：自制几何、合成 ILM/ramp/face-SDF、单位/变换和 shader
  语义场景；生成脚本、少量便携运行时文件、已批准 baseline 入库。
- **公共骨骼角色线（必需）**：本项目自制 skinned character/面部 morph fixture，
  覆盖多材质、骨骼朝向、动画、LOD、实例与重新加载。禁止依赖 Mixamo 下载或
  Tenacity 19 动作才能通过必需验证。
- **外部 Genshin 线（显式资格任务）**：原始来源直接获取，锁 character ID、variant、
  channel 和 53-file 等实际清单；归档原始包，独立 recipe 生成创作与 runtime。
  原始/派生第三方艺术资源 local-only，公共仓库只保存引用和非艺术证据。

### 5.2 输入、输出和路径

每项 manifest 至少含 source ID/version/URLs、逐文件 SHA-256、toolchain/shader
摘要、recipe revision、所选材质/对象/动作、units/forward、输入依赖和结果清单。
下载请求只发生在 prepare/fetch 阶段，compile/test 不暗中联网取资产。

CLI 显式路径优先于项目配置；项目配置中相对路径相对配置文件，不相对调用者 cwd。
默认缓存为 assets/local，允许显式 cache root，但测试用独立空缓存。依赖项必须
经声明的 manifest 解析，不自动向上搜索兄弟目录。禁止同一次准备混合不同 variant/
channel；支持固定本地镜像作为下载不可用时的输入，哈希仍严格校验。

生成键包含输入哈希、工具版本、recipe 和设置哈希。临时目录完成后验证再原子提交，
失败不覆盖既有产物；保留 failed-run 日志和 manifest。动态下载 URL/etag 不替代
内容哈希。跨进程 `.blend`/PNG 文件字节变化与像素/语义变化分别记录，不虚构
完全 byte-identical 的保证。

### 5.3 加工数据的逐项重建

| 产物 | 独立制作责任 | 验证 |
|---|---|---|
| body/body01/hair ILM、ramp | 直接原始 PNG + 显式语义索引，不猜名称顺序 | 源 hash、RGBA 通道、色彩空间、使用材质列表 |
| MetalMap、Specular Ramp、Face02 Shadow | 原始包逐项引用 | feature 开关与实际 shader 槽位匹配，不支持就报缺口 |
| face_lightmap、face_shadow | 原始 FaceSDF 与锁定 Gacha shader 图像/生成步骤 | 明确是复制上游文件还是项目生成，记录公式/输入/输出和 face 参考图 |
| pupil composite | 项目自有合成 recipe，依据来源节点行为 | 每个输入贴图/参数、颜色空间、像素与瞳孔特写 |
| runtime GLB | 本项目 runtime-only export staging | 无 widgets/烘焙上游 hull，skin/morph/材质/属性和坐标正确 |
| 动作/物理 | 项目自制测试动作；外部按 case 声明 | 不要求 Tenacity animator 或 Kawaii 链，未实现功能不默认为支持 |

制作不能简单复制 Tenacity 现成 GLB/packed maps 或照搬其游戏导出器。已有本地
产物可在过渡期做一次对照证据，但不能成为最终必需输入或 rebuild 来源。

### 5.4 版本资格策略

保留当前 v3.6.2 reference 资格线，另建 v3.6.4 candidate；选择最终 supported
版本要执行 source/signature/导入/材质/rig/render qualification。旧 lock 是
reference artifact 的证据，不自动证明 v3.6.2 对完整 Odette 原始包导入完全正确。
不强制本次完成升级，也不为逃避 v3.6.4 问题静默降级。

至少一个外部 case 独立完成 source→Blender→runtime→Bevy。可以明确选择
“原始静态身体/脸部”等支持子集，但必须在 manifest、捕获范围和报告中一致标识；
保留原始完整模型，完整效果裙/rig 的未达标结果继续显示，不能用删部件后的图冒充
完整角色通过。若该支持子集也存在实际材料/骨架错误，修复或阻塞其资格，不通过
修改预期掩盖。

## 6. Authoring 到运行时的正式边界

| 数据 | 真源 | Writer → Reader | 版本/检查 |
|---|---|---|---|
| 领域 material/instance/view descriptors | core Rust 类型 | tools/schema export → runtime schema snapshots/consumers | 生成差异 gate，兼容测试 |
| glTF extras/运行时 sidecar envelope | gltf pure 类型 | tools/adapters → gltf pure reader | schema ID + material-name 映射 + 冲突检测 |
| authoring source/manifest/export settings | schemas/authoring JSON | assets/Blender tools → validators | 显式 schema version、必需字段 |
| `_NPR_*` 属性表 | schemas/authoring 一份标准表 | generator → Rust/Blender adapter | accessor/domain/type 与 GPU 注册实测一致 |
| reference node signature | 锁定 upstream artifact 的经审查记录 | inspector → bridge verification | source digest、topology/socket 变化资格 |
| profile ground truth | 实际源材质/rig 的求值记录 | Blender exporter → profile-genshin 解释器 | static/instance/view 归属及未映射字段 |
| compile packing instructions | tools 可序列化类型 | case/profile adapter → compiler | 独立命名空间，不能代替 material descriptor |

不把所有 JSON 交给同一个无类型字典。schema 导出优先复用 Rust 序列化定义；若需
新增生成依赖，在 P4 评估并锁定，禁止同时手写两份同等权威格式。

profile-genshin 输出纯数据映射结果（material + 现有 instance/view 描述和明确
profile 参数）；tools 的 profile compile 检查完整性并生成 bundle 清单。GLB
embedded extras 和 .npr.json 一致时接收，冲突显式失败。旧 bridge sidecar 通过
版本化 adapter 迁移，不以重命名文件充当转换。

facade/integration 作为可选模块负责 scene-ready 后的材质 adoption、贴图句柄、
重载与卸载、实例/视图初始状态；必要 renderer 构造 API 保持 profile-neutral。
core 不引入 AssetServer/Handle。compile 不对 GPU 句柄或 Blender 路径负责。

通用 compile 兼容默认 Cel 的行为可以保留，但 profile strict 模式要求每个预期
材质角色与必需输入映射；unknown feature/texture/palette 输出具名错误或声明的
unsupported 状态，不凭编译成功标“Genshin 完成”。

拟新增 CLI 形态为 `compile --profile genshin --strict`（实际当前 CLI 尚不支持），
对应 tools 的 optional profile-genshin feature；未编译该能力时返回明确错误，
不能回退普通 compile。最终语法在 P4A/P4B 的 CLI 测试与文档中一次确定。

## 7. Reference 与 legacy 退出

最终有两种渲染器来源、三种证据：

1. **Upstream reference**：独立 Blender/Gacha 工具链生成，输入固定，作为 family
   行为依据。不得用当前 production 结果反向补成真值。
2. **Approved production baseline**：先前批准的本项目结果，用于回归；不能冒充
   upstream parity。基线必须有来源、case、toolchain/device 和时间身份。
3. **Synthetic semantic oracle**：公式边界、通道、数据传输、几何/生命周期和
   可解释图像断言，与具体游戏无关。

历史 legacy engine PNG/JSON 在迁移阶段用于 delta 说明，完成移交后成为历史
证据，不再现场编译 legacy。不能为了维持“三层”措辞再造一个依赖 production
同路径的伪独立 renderer。

### 7.1 退出覆盖账本（P0 建、P5 验、P6 执行）

逐个列出现有 legacy tests、odette lab cases、comparison pair 和 production
外部角色测试，记录：原目标、输入、结果、独立替代测试 ID、是否保留行为、
已知 delta 与证据路径。分类处置：

- 可复用语义：迁入新产品/集成测试，不携带游戏名和 renderer 源依赖。
- 数据特定用例：迁入声明式 asset/case manifest，来源改为独立 recipe。
- 只测旧实现内部机制：作为历史退役，有理由和替代范围，不能算新覆盖。
- 尚无替代的必需质量目标：阻塞 P6，不能删除后减少分母。

至少覆盖 base/ramp/ILM、face、hair、metal、outline 生命周期、post、alpha/cull/
prepass、skin/morph、实例/LOD/环境及 glTF 顺序。每项由实际现有测试清单核对，
不凭章节名认定覆盖。批准且能再分发的自制基线搬迁前后像素不变；第三方历史
图像不移入公开 fixtures/golden。

## 8. 验证状态、命令和 CI

以下为拟实现的命令接口，不表示现在已经存在：

```text
python3 tools/dev.py check --lane cpu
python3 tools/dev.py assets prepare --pack synthetic
python3 tools/dev.py assets prepare --pack genshin-odette --manifest PATH
python3 tools/dev.py check --lane gpu --required --pack synthetic
python3 tools/dev.py check --lane asset-integration --required --pack genshin-odette
python3 tools/dev.py check --lane reference --required --pack genshin-odette
python3 tools/dev.py golden capture --case CASE --out CANDIDATE
python3 tools/dev.py golden compare --case CASE --baseline BASELINE --candidate CANDIDATE
python3 tools/dev.py golden accept --candidate CANDIDATE --baseline BASELINE
python3 tools/dev.py check --lane independence --required
```

不让 dev.py 重写 Rust 功能；它负责参数、root/cache/output 解析和组合调用现有
工具。生产 CLI 仍是 bevy-stylized。golden accept 是明确改变接受基线的动作，
默认 compare/capture 都不修改旧基线。

| 情形 | 单项状态 | Required lane 结果 |
|---|---|---|
| 结果符合契约/容差 | PASS | 成功 |
| 输入/语义/图像/coverage 不合格 | FAIL | 非零 |
| 已请求但缺 GPU、下载源、模型或工具 | BLOCKED | 非零 |
| 已请求但缺接受基线 | BLOCKED | 非零；不能自动新建后当通过 |
| 未请求的外部可选 case | NOT_REQUESTED | 不计入 expected/passed，不伪装执行 |
| 交互诊断中主动跳过 | SKIP + 原因 | 若 lane 标 required，非零 |

退出码建议 0=所请求验收全部通过，1=断言/契约失败，2=输入/环境/blocked。
现有报告器可保留诊断模式，但 --required 必须把 passed=false 转成非零。报告含
expected/executed/passed/failed/blocked 数量；缺失 case、GPU tier 被降级、缺纹理
返回 None，都不能沿用原 case ID 并判通过。

验收集合在 P0 固定，并随新增 case 有显式版本：

- independence 集合要求 CPU、自制 GPU regression、外部选定 case 的
  asset-integration 全部成功，并验证 reference 报告存在、完整且结果被如实汇总。
- asset-integration 检查来源/映射/场景加载/实际捕获和具名功能断言，不使用
  upstream pixel parity 的 PASS 标签。
- reference 集合检查上游视觉判据；当前已知差距会使该命令失败，并阻塞对应
  family parity/V1 资格。independence 汇总不能将它重标为 PASS 或删掉该 case。

此区分允许工程解耦完成同时明确报告 parity 未完成；若失败的是资产映射、
必需功能断言或 public regression，则独立化本身也不得完成。

**CPU CI**：图/路径/schema/docs、fmt/clippy、纯 CPU tests、feature/外部 consumer。
GPU/外部模型测试以明确的 ignored/独立 binary 或任务分组不在 CPU lane执行；
未执行清单是可观察的，不能出现隐式 graceful skip 的全绿宣称。

**GPU CI/本地 required task**：同一命令严格要求 adapter 和自制 baseline。
有可信 GPU runner 才配置 required CI；未部署时报告“本地合格、CI 未配置”，
不能承诺仓库已经具备持续 GPU gate。外部 Genshin lane 独立准备 pack，只对明确
请求的 qualification 运行。公开 PR 不直接进入具备本地资产/权限的未隔离 GPU
runner；使用受信任 revision 的 dispatch/审核执行策略。

**Performance**：目录搬迁不改变性能预期；shader/实例/adoption 行为变更才跑
相同设备、分辨率、实例数和质量设置的对照。缺 baseline cells 必须暴露并阻塞
要求覆盖的任务。benchmark 不与其他 GPU 工作并发。

## 9. 实施拆分、前置和退出条件

| 阶段 | 具体工作与变更边界 | 前置 | 可检视交付/退出条件 |
|---|---|---|---|
| P0 决策与覆盖冻结 | 资产/测试/依赖图清单；D01–D10 与 ADR 变更草案；保存既有基线身份和已知失败 | 用户确认计划 | 无未知的必需测试消费者；覆盖账本含 legacy 退出条件 |
| P1 项目路径与检查入口 | dev.py 路径/配置解析；完整依赖图/realpath 检查；修正 target 定位；文档扫描 scope/history 规则；default-members | P0 | 从仓库外 cwd、带空格路径运行；错误外部路径负例失败；暂时 allowlist 含到期阶段 |
| P2 自制资产与准备机制 | source manifests、下载/本地镜像、哈希校验、原子缓存；合成和 skinned fixture generators | P1 | 空项目缓存生成两类公共 fixtures；无网络 test；故意坏 hash/漏纹理失败 |
| P3 独立 Blender 管线 | 分模块 import/audit/capture/export；runtime-only staging；face/pupil 独立 recipe；版本 qualification | P2 | 一个声明范围的外部角色 source→Blender→GLB/maps，provenance 完整；无 Tenacity 制作工具；错误签名仍拒绝 |
| P4A 纯契约/CLI | gltf pure/Bevy adapter 分离；schema 单一真源和生成；tools CPU-only 路径 | P2 | tools normal/build 图无 bevy_render；旧 metadata 兼容和冲突/未知版本测试；CLI coverage 报告 |
| P4B Profile 与接入 | profile mapper、typed bundle；facade integration scene ready/reload；data domains 分离 | P3 + P4A | 独立生成包在 Bevy 出图；材质角色/纹理/实例/视图可审计；不从 Tenacity 读取参数 |
| P5 严格参考和测试 | upstream/production/semantic 三类证据；missing/skip/compare 状态；自制接受基线；迁移覆盖账本 | P4B | public GPU regression、外部 asset-integration 与 coverage 通过；reference 已执行并如实保留 parity 失败，按 §8 分集合验收 |
| P6A Legacy 退出 | 先切断所有消费者，再移除 legacy 和旧 odette lab；ADR/traceability 生效 | P5 账本无缺替代必需项 | cargo 图不含旧包；无活动源依赖；历史证据可追溯 |
| P6B 目录 move | 按 §3.1 搬 apps/labs/harness/tests/tools/fixtures；同批更新文档链接、CI filters、嵌入路径；不改算法 | P6A | 对照 inventory 无遗漏；移动前后 self-made Golden 像素一致；所有命令从任意 cwd 有效 |
| P7 文档/技能/CI/分发 | README 快速路径、CONTRIBUTING、schema/asset/reference 文档；repo 技能；最终 CI 门禁和操作说明；源码分发演练 | P6B | fresh-source 操作手册有效、文档检查只扫项目文件、个人技能可缺失 |
| P8 隔离总验收 | 在无 Tenacity 可访问的源副本中执行 I01–I12，汇总记录与剩余视觉/发布状态 | P7 | 独立性报告完整，临时 allowlist/旧包装器清零 |

P4A 可在 P3 的资格工作期间独立推进；其余依赖顺序不能用目录移动提前掩盖。
每一行可拆为多个小 PR，但每个 PR 必须有输入、变更文件、验证结果和回退范围。
遇到上游 bug，先确认 tag/source/复现，再做最小显式适配或版本选择；不能无限扩大
为全部 shader/rig 系统改造，也不能宣称被阻塞的 case 已完成。

### 9.1 行为与数据迁移回退

变更前记录对应 commit、lock、recipe 和 baseline 摘要。旧接受基线只读，候选写
新目录；schema reader 在必要兼容窗口读旧版本，writer 只写新版本。目录 move
可单独回退；若 schema/映射失败则回到上一个产物版本，不将新 shader 结果盖到旧
provenance。最终移除 legacy 前保留历史 Git 对象和声明，不新建永久子模块依赖。

## 10. 独立性总验收矩阵

| ID | 测试/交付 | 合格标准 |
|---|---|---|
| I01 | 全 workspace 依赖及文件访问检查 | 所有 normal/build 及声明 dev 依赖可解释；无 Tenacity path/env/git dependency、越界 include、隐藏 symlink；不误禁本 crate 的 CARGO_MANIFEST_DIR 合法用法 |
| I02 | 无兄弟仓库的干净源演练 | 临时容器/沙箱只挂项目源和必要工具缓存，Tenacity 不可访问；不是仅 unset env |
| I03 | 缓存隔离 | 新 assets/local 和 target artifact 区，所有源有 manifest；无对之前 target 结果的依赖 |
| I04 | 公共资产重建 | 自制几何与 skinned fixture 可生成、结构等价，声明的像素/语义复现判据成立 |
| I05 | 外部资产独立准备 | 至少一个显式 Genshin case 从直接来源或 hash 验证镜像生成；没有旧游戏 GLB/packed maps 输入 |
| I06 | typed 导出与接入 | source→runtime manifest→Bevy adoption；角色/贴图映射 complete，缺字段/错版本/冲突负例失败 |
| I07 | Reference 独立性 | upstream 不调用 production 来构造真值；legacy 无活动源码；覆盖账本完整 |
| I08 | strict GPU 与基线 | 缺 GPU/接受基线失败，比较失败非零，正常 synthetic Golden 通过；不自动 rebaseline |
| I09 | 路径/目录/运行方式 | 从任意 cwd、空格路径、无 Git 元数据源码包运行；CLI 显式 root 和 provenance revision 可用 |
| I10 | feature 与外部消费者 | 产品单独 minimal/default/profile 组合编译，tools pure 图无 renderer；不能依赖 dev feature union |
| I11 | 文档与可操作性 | 新贡献者只读本仓库文档即可完成公共流程；无个人 skill/MCP/绝对路径必要依赖；历史索引不再规范性引用旧计划 |
| I12 | 回归与交付边界 | 移动前后已接受自制结果不变；其他视觉 delta 逐项说明；源码归档可独立构建，许可/registry 状态如实记录（不宣称发布许可） |

隔离验收先允许只用于 Rust/Blender 工具的已验证安装或预取依赖，不能挂入旧项目
艺术产物。source prepare 可联网；之后必需公共 test 阶段禁外部模型下载。
操作系统/驱动限制按当前支持矩阵记录，不据 Linux 本机结果宣称跨平台 Supported。
独立验收的 Blender 配置属于隔离进程，不修改用户正在使用的偏好与场景。

## 11. 文档、技能和发布界限

当前 README 更新为真实 build/test/authoring 路径；ARCHITECTURE 只做索引与不变量。
新规范承担当前责任；Tenacity migration、旧对抗评审和来源比较记录移入 history，
保留事实和署名。历史内容允许 Tenacity 字符串，但不能被脚本解析作为默认路径。
checker 使用 Git tracked 或显式受控文件清单，包含当前新增待检文档，排除 target、
assets/local、第三方下载和 .git，避免扫描整个磁盘树。

两技能归入 .agents/skills 并移除当前执行中的 Tenacity 示例，具体 Odette 资料
转为直接来源与自有 recipe。个人技能副本只作为可选安装；通过版本摘要检查或明确加载位置避免同名旧副本
继续引导到 Tenacity，不假设宿主会自动选择新版本。AGENTS.md 路由到本仓库
工作约定和 skill；安装/同步是可选操作，核心任务不依赖它。维护脚本和公开文档可不通过任何 AI 工具执行。

发布准备只做元数据、包文件范围、内部 version+path、shader 包含和源码分发
检查，不改变 publish=false，不选许可证、不上传模型或发布 registry。
若执行 .crate 验证，内部尚未发布包必须使用明确的本地 registry staging 或
记录缺少 registry 版本这一阻塞，不能以 --no-verify、workspace path patch 或
普通 cargo check 冒充包验证。没有 .git 的源码归档也能构建/验证，provenance
使用随包提供的 revision，不能在 runtime 强制调用 git。

## 12. 风险、结束条件与检视重点

最大风险是缺失的独立资产转换能力，而不是目录数量；P3/P4B 是关键路径。
第二风险是删除旧 oracle 导致覆盖丢失，由 P0/P5/P6 的覆盖账本阻止。
第三风险是把已知视觉失败当成工程独立完成的隐含豁免，由状态矩阵和分别报告防止。
第四风险是仓库目录变化破坏嵌入、包边界和 CI，由 move-only 验证与外部 consumer
演练阻止。

**实施结束条件**：I01–I12 全部有证据，活动 Tenacity/legacy 依赖为零；选定外部
支持 case 已独立运行；其 parity 是否达标单独报告；没有到期的过渡 allowlist 或
必要操作仍指向个人目录。若必需项 BLOCKED，实施不能宣称全部完成。

本次请重点检视 D02/D03（legacy 退出与参考体系）、§3 目录、§5 资产线和
§9 关键路径。计划不留下这些基本方向给实施时临时猜测；确认后再进行实际重构。
