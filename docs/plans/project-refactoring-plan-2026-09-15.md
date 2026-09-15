# 全项目重构目标与实施计划

## 1. 目标与状态

本计划依据[全项目调研](../research/project-refactoring-audit-2026-09-15.md)，面向独立化完成后的可维护性和验证可靠性。状态为**实施方案 v2；R0–R6（含 R1b）已完成本地实施与适用验证**。最终结果见[整轮验收](project-refactoring-final-2026-09-15.md)。分阶段结果见[R0/R1/R1b 执行记录](project-refactoring-execution-2026-09-15.md)、[R2/R3 执行记录](project-refactoring-r23-execution-2026-09-15.md)与[R4 执行记录](project-refactoring-r4-execution-2026-09-15.md)。审查闭环见[修订与复核记录](project-refactoring-review-2026-09-15.md)。它承接独立项目计划 v4 的边界，不改写其执行历史，也不扩展旧版 Gacha 兼容义务。

总目标：在保持现有受支持行为的前提下，使数据解释、资产编译、运行时渲染、开发检查和验收报告各有明确拥有者；让任何一次修改都能通过可追溯、不会误报的标准命令验收。

### 固定边界

- 保留六个产品 crate 和当前 product/app/lab/test 依赖方向。
- 保留 Surface × Tier × Validation、Material/Instance/View 的正交关系。
- 产品可在没有 sibling checkout、个人路径、AI 客户端或 Blender MCP 的环境执行文档化工作流。
- 当前 Gacha 接受目标只取已资格化的最新稳定精确锁；基线为 v3.6.4。旧 3.6.2 只保留历史 opt-in 证据。
- 不把动画、表情控制、IK、物理或大型编辑器加入产品范围。
- 不提交 Git commit、不发布包、不覆盖已接受 baseline、不覆盖作者源场景。
- 不以清理为由放宽误差阈值、删掉失败 case 或默认接受 ADR。

## 2. 可验收重构目标

| 编号 | 目标 | 完成证据 |
|---|---|---|
| G1 | 验证结果忠实表达执行行为 | case 身份精确匹配；混合失败保留 FAIL/BLOCKED；旧 PASS 不可冒充本次结果；零执行和日志损坏不能 PASS |
| G2 | 当前契约有唯一入口 | 每个域明确 authority；当前资格摘要链接日期化证据；历史要求不会进入默认验收 |
| G3 | 资产编译职责分离 | 输入解析、语义校验、编译计划、纹理处理、候选写盘可以分别检查；外部格式保持 |
| G4 | 当前资产构建可诊断、可恢复 | 步骤状态、输入摘要、失败路径明确；中断后可新建尝试，源文件不被覆盖 |
| G5 | 开发应用复用必要基础设施 | bundle 接受域与 capture 生命周期明确；有证据支持才共享实现；参考计算保持独立 |
| G6 | render 内部拥有者明确 | 参数布局、映射、specialization、同步职责可定位；有收益才拆分；公开导出、调度、绑定和视觉行为保持 |
| G7 | CI 覆盖实际产品使用路径 | 默认 CPU、inspector 可选 feature、独立 consumer、GPU required 的职责明确 |
| G8 | 清理有处置依据 | 每个移除项有引用/嵌入/测试/历史处置记录；没有按体积批量删除资产 |

不设“代码减少 30%”或“所有文件低于 300 行”之类指标。长函数应围绕独立的副作用、状态转换和失败边界拆分；只包含紧密数据定义或有价值测试的长文件可保留。

## 3. 目标结构

以下为职责图，新增目录名是候选命名，不是已存在路径。

```text
tools/dev.py：参数解析和任务分发
  ├─ task runner：进程、日志、结果分类、报告
  ├─ checks：docs / graph / suites / consumers / isolation
  └─ assets：prepare / build stages / capture case / reports

tools crate：CLI → validated input → compile plan → candidate writer
gltf crate：纯 metadata + 可选 engine 注册；bundle identity 是否抽取由 R4b 决定
profile crate：家族解释 → descriptor
facade：插件 / capability 生命周期 / 可选场景接入
render crate：明确 params / mapping / pipeline / sync 的拥有者，模块拆分待试点评估

tests/harness：readiness / request identity / readback / image primitives
labs/reference：独立参考流程、校准、区域判据
apps/showcase：交互状态、UI、回放和诊断呈现
```

不新增万能 common crate。纯 bundle 逻辑放入 gltf 前，应确认属于运行时运输格式而非 Blender capture schema；后者继续由 labs/工具拥有。harness 保持产品中立，不吸收角色 profile 解释。

## 4. 阶段与工作包

### R0：锁定行为与迁移清单

**依赖：** 无。**优先级：** P0。

1. 记录源码 revision、工作区差异、源锁、suite 名集合、公开导出和 generated schema 摘要。
2. 以本轮 CPU 166 passed / 33 ignored、Python 12 passed 为观察基线，而非永久硬编码数量。
3. 为后续改动建立“原位置 → 新拥有者 → 关联契约 → 测试身份 → 证据”清单。
4. 记录现有 GPU、独立性和 v3.6.4 静态 parity 的日期与 scope；实际渲染改动前在可用设备复验相关基线。

**退出条件：** 可以明确说明一项行为将由哪个目标模块和测试继续承担；缺失资源明确 BLOCKED。只做文档清理不强制运行 GPU。

### R1：验证入口与结果模型

**依赖：** R0。**优先级：** P0。**对应：** F01/F02/F03/F06、G1。

1. 定义 `CheckResult/CaseResult` 等内部结果模型：schema、run_id、lane、case identity、状态、原因码、原始子进程退出码、源码/输入摘要、日志引用、执行/缺失计数。
2. 区分资源缺失、契约错误、子进程失败和内部错误；已进入任务执行的 CLI 退出约定为成功 0、失败 1、必需资源阻塞 2。参数解析 usage 错误保留 argparse 的退出 2 和 usage 文本；自动消费必须结合报告原因码，不能仅把退出 2 解释成资源缺失。迁移所有读取旧子进程退出码的调用方，报告保留原始退出码。
3. 用 suite 清单选择并核对实际测试身份。先使用锁定工具链可支持的列表/执行格式，不预设 nightly JSON test harness；任何解析不完整都不通过。
4. GPU binary 列表与 expected identity 使用同一来源，避免 dev.py 和 suites.json 双重维护。
5. 聚合保留混合状态；FAIL 优先，同时报告 BLOCKED。只日志含关键词不得清空所有 failed。
6. 每次执行分配 run_id，开始运行先发布对应的未完成状态，再运行任务；最终报告只汇总本次子报告。失败与阻塞也必须写入终态。历史报告保留在独立运行目录，当前入口不得指向旧 PASS。中断留下的未完成状态不可用于资格声明；当前状态写入失败时，不执行任务并以失败退出。
7. 读者必须匹配本次 run_id、源码/输入身份与终态；父调用方应在执行前持有预期 run_id，未获取本次身份或没有报告时一律不能接受旧结果；禁止只读取固定路径的 status。捕获 SIGKILL 等无法完成清理的情形靠未完成状态拒绝资格，不承诺总能写出终态。实现从标准 dev.py 入口覆盖到其报告消费者；避免引入通用任务调度框架。
8. 把进程运行、日志落盘和报告构造从 CLI 分派分离。修复 docs 统计重复扫描；保留参数和入口，明确文档化上述结果契约的修正。

**必要负例：** 同数量错误 case、缺 case、重复 case、零测试、摘要损坏、assertion 与缺 GPU 混合、构建非零、缺 baseline、非法 schema、已有候选目录、先成功后失败/阻塞/中断、错 run_id、旧子报告、报告写入失败。

**退出条件：** 上述负例分类正确；原有成功路径保持；CPU 全通过；真实 GPU required 和 synthetic import 至少运行一次，证明新身份解析器处理真实输出；缺 GPU 则该 GPU 子项 BLOCKED。报告消费者及隔离负例同步迁移，不能只有 mock 通过。这里新增测试是为了覆盖已复现缺陷，不为纯搬文件逐行复制测试。

### R1b：提前建立持续门禁

**依赖：** R1 的 CPU/报告接口部分；GPU 解析复验不阻止独立 CPU 门禁建设，但保持其 BLOCKED 状态。**优先级：** P0。**对应：** F05、G7。此编号保留旧阶段对应关系，不表示可延后到 R6。

1. 在 tools/dev.py 增加明确的 inspector CPU 子任务并接入 Rust CI；底层执行已有的 `cargo test --locked -p showcase --features inspector --lib -- --test-threads=1`，同时检查该 feature 下 bin/相关 target 的编译和 clippy。
2. 在改 Cargo/编译器/bundle 之前建立独立 consumer 门禁。现有 consumers 使用 `--offline`，CI 必须先准备依赖，验证新的独立锁解析所需缓存，不假定 workspace fetch 足够；冷缓存失败应被准确报告。
3. 明确 inspector 的 X11/Wayland 等构建前提并配置 CI 系统依赖；CPU 单测不要求实际窗口/GPU。保留默认 CPU 的无 Blender/外部角色资产性质。
4. 工作区和设备上的 Cargo/GPU/性能任务串行；不同隔离 CI runner 可以并发。GPU CI 是 self-hosted 手动工作流，未配置或未执行不能记为远程通过。

**退出条件：** 本地对应 feature 单测、编译/lint、consumer 检查通过；CI 工作流定义已验证且调用同一入口。远程运行状态单独记录为已验证或未执行，不要求为完成可逆代码工作新增 runner。R3、R4b/R4c/R4d、R5 开始前必须满足对应本地门禁。

### R2：现行文档与治理清理

**依赖：** R0；可在 R1 后落地。**优先级：** P0。**对应：** F04、G2。

1. 优先补齐现有 README/ARCHITECTURE 导航，链接 source lock、最新资格记录、support matrix 与未决 ADR；不复制全部规范。
2. 给旧计划添加适用性/替代关系，保留原正文和结果；历史 3.6.2 命令明确 opt-in。
3. 更新 facade/render 注释与 provisional 清单的当前实现描述；将旧迭代标签留在历史说明，不作为当前功能定义。
4. 清理迁移前测试路径，检查 Cargo lint allowance 的真实使用；仅移除经 clippy 验证不再必要的豁免。
5. 项目技能仅负责导航和步骤，引用当前契约；不复刻另一套支持矩阵。
6. 给重要源码/脚本引用与当前锁建立有界一致性检查；避免靠关键词禁止合理的历史记述。

**退出条件：** docs lane PASS；从 README/ARCHITECTURE 可找到当前范围，且不会误将 9 月 14 日失败或旧 Gacha lane 当作现行必需 gate。ADR 状态维持原值，除非另有明确决策。

### R3：编译器与纯数据契约

**依赖：** R1、R1b。**优先级：** P1。**对应：** G3。

1. 将 `compile_path` 分为输入读取、统一校验、profile/sidecar 合成、计划构建、纹理转换和候选输出。
2. 使用显式内部编译计划表示输出文件与依赖，集中重复名字、材料覆盖、路径和冲突检查。
3. 读取与解析复用以减少重复操作，但不绕过现有 validate 语义。
4. 保留 glTF/extras/sidecar 的优先级与冲突拒绝，不将 generic default 编译误报为 profile 转换。
5. 保留新目录写入、失败不产出成功包、manifest/hash 校验；校验故障不能被误分类为环境阻塞。

**退出条件：** CLI 测试及 pure consumer 通过；同输入的运行时载荷（GLB、纹理、descriptor/view）材料覆盖、文件集合、哈希与 schema 保持；源码变更导致 build key 和 provenance/source digest 改变是预期行为，应列明，不要求这些摘要与旧值相同。若不可避免改变序列化字节，必须解释字段等价和 hash 影响，作为独立契约变化处理，不能藏在重构里。

### R4：资产步骤与开发工具（分包验收）

以下工作包分别完成、分别记录状态；不因一个包缺 GPU 而阻止无关的纯数据或文档工作。

| 工作包 | 依赖 | 具体目标 | 退出条件 |
|---|---|---|---|
| R4a 资产构建恢复 | R1、R3 | 当前/历史步骤显式化；保存阶段输入输出和失败证据；新增 attempt 语义 | 临时目录中断/重试/产物哈希负例通过；新候选 source→Blender→compile→audit→asset-integration 完成；若捕获/导出行为受影响，再跑当前 character-reference |
| R4b bundle 边界 | R1、R1b | 逐字段比较 inspector/reference 接受域；只共享相同路径/哈希/材料身份基础逻辑 | 非法路径、hash、版本和覆盖负例；pure consumer；两边已有合法/非法输入接受行为保持；实际加载受影响时补 asset-integration/inspector |
| R4c reference 编排 | R1、R1b；若采用新 bundle，再依赖 R4b | case、scene、capture、calibration、metrics/report 明确职责；保持参考计算独立 | CPU 校验通过；同一冻结 case 跑六项当前 character-reference required；诊断、采样、区域结果和输入/case 身份与改动前一致（run_id 与源码摘要按本次更新） |
| R4d inspector 编排 | R1、R1b；若采用新 bundle，再依赖 R4b | 主模块装配、交互状态、同步、代际、回放和报告职责明确 | feature 单测/lint、reload/stale request 通过；改动涉及回放/调度/readback 时，按 realtime 扫描契约验证重复帧、闭环、光照时序；测量路径变化才补性能 |
| R4e Python 格式 | R1；在相关逻辑改动之外单独执行 | 多语句行展开、格式统一；保持 Python 3.11+ | Python 语法/单测与对应 CLI smoke；只改 Blender 脚本格式时至少做语法检查，改变求值/导入行为才需实机验收 |

R4a 优先使用完整的新尝试目录，保留旧失败目录；跨阶段续跑只有输入/输出摘要和副作用边界足以验证时才引入，不能把简单恢复扩展成工作流引擎。原成功缓存校验、内容寻址和作者场景保护必须保留。

R4b 可以得出“不抽取”的结论，但必须列出接受域差异与各自拥有者；不把不同的 Blender case 和运行时 bundle 合并成一个宽松 schema。新增纯逻辑仍遵循现有 crate 边界。

R4c/R4d 不必等待 R4a 重建，可以用先验证身份的既有冻结输入。缺少适用的 Blender、源包或 GPU 时，只将相应必需检查记为 BLOCKED，不声明该工作包完成。最终新源码构建闭环由 R6 负责。

### R5：渲染职责审查与条件性试点

**依赖：** 职责审查只需 R0；实际代码试点需 R1、R1b 及 R0 对应 GPU 基线。**优先级：** P2。**对应：** G6。不强制依赖无关的编译器重构。

1. 记录参数布局、descriptor 映射、Material specialization、同步系统的拥有者和变化理由。先区分长文件的测试/数据体积与实际职责耦合。
2. 只有存在具体维护问题（例如同一语义须多处修改、独立副作用妨碍验证）时，选一个材质做小范围私有模块试点；保留公开路径和 re-export。
3. 比较试点前后该问题是否消除、导航是否更清楚、额外类型/间接调用是否值得。无净收益则保留原结构，记录理由；Character/Hero 不自动推广。
4. 标注并保持 instance/face/outline/view 调度约束；检查已有移除组件、reload、共享材质和外部姿态测试的覆盖。调度修复属于单独行为变更，不混入文件迁移。
5. shader 仍只在 render crate 嵌入；纯组织性重构保持 WGSL 字节、binding、uniform layout 和 specialization key。`capability.rs/governor.rs` 采用相同的证据门槛。

**退出条件：** 拥有者、问题和处置明确；“保留现状”有理由即可完成职责审查。若实际改代码，CPU、schema、consumer、相关 GPU golden/预算通过，公开路径保持，无未解释视觉差异；涉及每帧工作、分配、上传或 draw 时再测对应性能。不能以“无需拆分”为由跳过已确认缺陷，也不能为满足阶段名而拆文件。

### R6：测试组织与清理收尾

**依赖：** R1、R1b、R2、R3、R4a–R4e、R5 的审查及适用实现。**优先级：** P1/P2。**对应：** G7/G8。

1. 将变形测试和 profile/采样语义测试分组；保持每个旧 case 的迁移关系，suite identity 精确核对。
2. 复核 R1b 门禁随文件与 case 迁移同步更新，不把持续验证首次接入留到收尾。
3. 保持 Cargo 构建和 GPU/性能运行串行。self-hosted GPU runner 未就绪不能写成远程 CI 已通过；不同隔离 CI runner 不共享本机串行限制。
4. 形成移除清单：失效 wrapper、重复导航、已替代内部 helper、无用途生成物分别处置；任何历史署名和源资产不纳入自动删除。
5. 全量收尾运行 CPU、consumer、当前 GPU/资产/参考与隔离链；每条结果关联 run_id、源码、工具、输入与设备。既有 isolation lane 包含工程与 parity 检查，机器报告仍必须保留各维度结果；前面已运行且身份一致的检查可复用，隔离内复验用于证明隔离性质。

**退出条件：** G1–G8 均有证据；所有必需 case 被执行；没有通过更新 baseline 掩盖回归；最终文档准确列出未资格平台与未决 ADR。

## 5. 验证选择表

| 修改范围 | 必需检查 |
|---|---|
| 文档/技能导航 | `python3 tools/dev.py check --lane docs` |
| CLI/结果模型/suite | CPU + 对应失败分类/身份负例；GPU runner 改动加真实 GPU required/synthetic import 验证 |
| Cargo 边界/公共导出 | CPU + architecture + consumers + 对应 feature |
| compiler/schema/profile | CPU/schema + CLI 契约 + 候选一致性；映射变化加 GPU/当前参考 |
| Blender/资产步骤 | R4a 新候选 build + audit + asset-integration；捕获/导出行为变化加当前 character-reference；纯格式按 R4e |
| inspector/capture | feature 单测 + inspector required + readiness/reload；回放/时序变化加 realtime scan，性能采集变化加 measure |
| 材质/shader/调度 | CPU + GPU required + 对应图像/语义/预算；有运行成本变化再测性能 |
| 项目隔离闭环 | 当前 source packs + independence required |

以下均为当前已存在的命令；R1b 的 inspector-cpu 入口现已实现。BUILD 为本次 build 输出，当前 matched case 为 BUILD/character-matched/case.json，两个 NEW_* 为不同的新目录：

```sh
python3 tools/dev.py check --lane cpu
python3 tools/dev.py check --lane inspector-cpu
python3 tools/dev.py check --lane consumers
python3 tools/dev.py check --lane gpu --required
python3 tools/dev.py check --lane asset-integration --required --asset BUILD/compiled/character.glb
python3 tools/dev.py check --lane inspector --required --asset BUILD/compiled/character.glb --out NEW_INSPECTOR_OUTPUT
python3 tools/dev.py check --lane character-reference --required --case BUILD/character-matched/case.json --out NEW_REFERENCE_OUTPUT
python3 tools/dev.py check --lane independence --required
```

每次候选使用不同的新输出目录。当前 GPU lane 同时运行 synthetic import；历史 reference lane 不加入默认当前资格验收。

## 6. 执行粒度与风险控制

建议顺序：R0 → R1 → R1b → R2/R3 → R4 分包 → R5 审查及条件性试点 → R6。R2 和 R5 的只读职责审查可独立提前，R4b/c/d 不依赖资产重建；此处表示依赖允许，不授权并行运行本机 Cargo/GPU。各包保持可单独审阅：结果模型、suite 身份、持续门禁、文档、编译计划、候选输出、资产恢复、bundle、reference、inspector、格式、render 职责、最终收尾。这里的工作包是差异与验证单位，不授权创建 Git commit。

高风险是测试结果误报、输入规则变松、capture 时序漂移、渲染调度改变和参考实现失去独立性。对应控制分别是负例、输入接受域对比、generation/readiness 测试、GPU 回归和独立参考计算保留。

每个工作包只做一个行为边界；纯搬移、格式清理、缺陷修复和新能力分开。失败时停止推进该包，保留候选和日志，回退本包修改；不对工作区使用破坏性整树重置。未受影响检查无需反复全跑。

本阶段不提供伪精确工期。R1 完成后，可根据实际模块拆分成本与 GPU/Blender 周转时间估算；执行顺序和退出条件比未经测量的天数更可靠。

## 7. 最终交付与完成定义

最终交付包括模块迁移清单、当前架构导航、标准任务说明、版本化验证结果以及逐项验收表。报告固定拆分：

1. **工程独立性**：静态边界、consumer、隔离重建。
2. **图像回归**：已有受支持场景与已接受第一方 baseline。
3. **当前上游 parity**：精确 Gacha/source/case 范围，不推导完整 rig/effect 支持。
4. **性能证据**：设备、分辨率、采样口径和实际测量；采集 PASS 不等于预算资格。

所有目标记录为“实现并验证”或“评估后保留现状（仅限 G5 的共享抽取、G6 的结构拆分等条件项）”；记录理由与现有验证覆盖，不把条件项未实施写成测试 PASS。只有适用的必需检查都通过，且 G1–G8 有可复核证据，才宣布整轮重构完成。BLOCKED、未执行与未资格项必须保留。当前 R0–R6 和最终隔离闭环已完成；R5 保留实现的处置有审查依据，远程 CI 未运行，性能仅有本批烟雾检查。
