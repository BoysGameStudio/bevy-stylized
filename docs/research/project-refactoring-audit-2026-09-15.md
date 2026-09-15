# 全项目清理与重构调研

## 结论

项目已经具备合理的产品边界和可执行验证链，下一轮应围绕**验证结果可信、职责清晰、契约唯一、证据可追溯**开展增量重构。保留六个产品 crate；优先修复验证编排和文档事实漂移，再整理资产编译、开发应用和渲染内部模块。没有证据支持重写整个渲染后端或重新拆分 workspace。[1][2]

本报告区分代码事实、执行证据和重构建议。后续复核补充 F06 并修正实施顺序，见[审查记录](../plans/project-refactoring-review-2026-09-15.md)。复杂文件只是检查入口，不能单凭行数认定需要拆分。具体阶段、依赖和退出条件见[重构目标与实施计划](../plans/project-refactoring-plan-2026-09-15.md)。

后续实施已处理 R1 的验证器问题及 R1b 门禁，见[执行记录](../plans/project-refactoring-execution-2026-09-15.md)。本文保留调研基线上的发现与原测试结果。

## 1. 调研边界与基线

检查日期为 2026-09-15，源码基线为 `5d723b4da35cf9e9b143432f8dbce48b4d03657b`；开始时 Git 工作区干净。检查覆盖根契约、Cargo/CI、六个产品 crate 的入口及关键路径、showcase/inspector、reference/regression、Python 资产与检查入口、测试 harness、schema 和当前/历史验证记录。采取全仓文件与符号盘点、关键实现阅读、CPU 实跑和验证器模拟输入探针；不是对全部代码的逐行正确性证明。

本轮未运行 Blender、GPU 回归、实时性能或隔离重建，也未重新接受图片基线。相关结论引用已有日期化记录，不能当作本轮复验。旧版 Gacha 证据仅用于解释历史；当前支持目标来自 `assets/sources/gacha-authoring.json`，锁定 v3.6.4，归档 SHA-256 为 `7042aeeaac488ac60515773a1414bd643175f91c950bba2a05b2e6d826744321`。[3]

### 规模快照

统计仅包含基线 Git 跟踪文件。行数含注释和空行，不是复杂度分数。

| 指标 | 数量 |
|---|---:|
| 跟踪文件 | 351 |
| Rust / Python / WGSL 文件 | 87 / 41 / 7 |
| Markdown 文件 | 127 |
| workspace 成员 / 默认产品成员 | 13 / 6 |
| crates 下所统计文本行 | 14,517 |
| apps / labs 下所统计文本行 | 3,014 / 3,554 |
| tests / tools 下所统计文本行 | 6,384 / 3,679 |
| docs 下所统计文本行 | 14,913 |

这里“所统计文本”指 `.rs`、`.py`、`.wgsl`、`.md`。文档体量说明事实维护也应纳入重构，不能推导出应删掉多少文档。

## 2. 当前架构与应保留的成果

| 边界 | 实际职责 | 调研判断 |
|---|---|---|
| core | descriptor、capability、预算、profile trait、LOD/governor 数据与算法 | 保留与渲染器/AssetServer 分离 |
| render | 材质、shader、instance、view、outline、LOD、post | 保留内部 backend 边界，先整理职责 |
| gltf | 纯元数据层 + 可选 Bevy 注册 | 是数据契约复用的合理归属 |
| profile-genshin | 家族语义解释和映射 | 不把角色名称规则推进 core |
| tools | validate/inspect/compile/schema | 保持无 GPU 编译路径 |
| facade | re-export、能力生命周期、插件与可选材质接入 | 保持使用入口，避免成为工具库 |
| apps/labs/tests | 交互检查、参考对照、测量与验证 | 共用基础设施，保留不同验证目的 |

依赖图检查本轮报告 13 个 workspace 成员、6 个默认成员、无边界错误。静态独立性检查也通过；这仅证明静态规则，隔离执行是另一项资格。[1][4]

已有值得保留的机制包括：源包哈希验证、编译候选目录、材质覆盖检查、显式 glTF 注册、共享材质生命周期、capture readiness、缺失 baseline 的失败路径，以及工程独立性与上游对齐分离。重构应保住这些行为，不重新实现一套同名框架。[5][6][7]

产品不承担动画播放、IK、表情状态机或物理解算；消费外部已经求值的几何和实例数据。动态质量、实例表和高级描边已经存在，不能因它们出现在早期 V2 计划里就当成死代码删除。[2]

## 3. 已确认的问题

### F01：测试覆盖按总数验收，缺少身份核对（高优先级）

`tools/dev.py::run_tests` 从 Rust 文本摘要累加 passed/failed；`tests/suites.json` 保存测试名，但 GPU 入口只读取数组长度。源码没有在该入口核对实际测试名集合。因此“漏掉 A、加入 B、总数不变”不能被这层门禁识别。[4]

本轮在临时目录中 mock 子进程返回 `wrong_test` 成功和“一项通过”的摘要，设 expected=1；当前函数返回 PASS。这是验证器行为复现，不表示当前真实 GPU suite 已漏测。应建立 suite、binary、case identity 的精确集合比较，并保留原始日志。

### F02：错误类别混合，可能掩盖真实失败（高优先级）

同一函数只要发现任一资源缺失文本，就将全部 failed 计入 blocked，并将 failed 清零。探针输入“两项失败 + required GPU unavailable”得到 failed=0、blocked=2。若日志同时包含资源失败和真实断言失败，聚合结果会丢失差异。[4]

CLI 最外层又把 `OSError/ValueError/RuntimeError/KeyError` 大类都映射为 BLOCKED。`tools/assets/build.py::run_job` 对任意非零退出抛 RuntimeError，意味着构建逻辑错误也可能显示成资源阻塞。例外类型本身不能决定项目验收状态：Python 的 ValueError 只表达参数值不合适。[8]

建议单独定义资源不可用、输入契约失败、执行失败和内部错误；逐 case 保留状态。聚合采用 FAIL 优先于 BLOCKED，但同时保留 blocked 数量；缺少必需 GPU/资产/baseline 仍必须是 BLOCKED，不能退化为 PASS。

### F03：文档发现边界和最终计数不一致（中优先级）

`tools/checks/check_docs.py` 前部已经通过 Git 文件清单或有排除规则的遍历限制文档范围，但结尾打印数量又调用 `ROOT.rglob('*.md')`。这会重新扫描缓存/下载目录，数量也不等于实际被检查的文档集合。[9]

应使用同一份 `MARKDOWN_FILES` 做检查和计数。现有链接检查重点是 Markdown 目标，不能发现大量 Rust/Python 位置和功能状态漂移。通过文档 linter 不等于所有文档内容正确。

### F04：活动导航、旧计划和日期化证据容易混淆（高优先级）

`independent-project-execution.md` 保留 9 月 14 日约 20.96 dB 的历史失败；9 月 15 日实时验证 JSON 已报告当前 v3.6.4 限定静态角色 parity PASS。两者日期和范围不同，不构成“应抹掉旧失败”的理由。[3][10]

`static-character-reference-plan.md` 仍有继续运行旧 3.6.2 回归的计划文字；`02-reference-toolchain-lock.md` 的示例仍围绕 3.6.2，尽管标明历史日期。应在当前导航明确它们的适用性，避免误读成现行必需输入。[11]

`03-provisional-inventory.md` 留有迁移前测试路径，且同时存在“StorageTable 声明未实现”和实例表实现条目。render/facade 模块注释仍以 It-3b/Cel 切片描述已经包含 Hero、表通道和高级描边的实现；Cargo 根注释还用退休 legacy crate 解释全局 lint allowance。[12]

这些问题应通过“当前摘要 + 原日期记录 + supersession 链接”解决。不能直接批量改写日期化报告，也不能把未决 ADR 自动标成 Accepted。

### F05：标准 CPU/CI 未明确覆盖 inspector feature（高优先级）

`check --lane cpu` 执行 workspace 默认 feature 测试；inspector 是 showcase 的可选 feature。Rust CI 的 feature matrix 覆盖产品组合，但未列出 showcase inspector，也未调用独立 consumer lane。已有隔离记录执行过 inspector-unit，因此这是持续门禁与已有本地证据之间的缺口，不能说 inspector 从未测试。[3][13]

Cargo feature 会按依赖图发生合并，workspace 构建不能代替独立 consumer 证明。官方 resolver 文档也说明 resolver 3 调整的是 Rust 版本兼容选择默认值，不是取消 feature 合并。[14]

### F06：失败执行会留下上一轮成功汇总（高优先级，复核补充）

GPU lane 只在全部子检查成功后写 `gpu.json`；开始时不标记本次运行，失败时也不更新该汇总。`Project.report` 使用固定名字替换文件，没有运行标识。复核在临时目录预置旧 `gpu.json` PASS，mock 当前 `run_tests` 抛出 CheckFailed，执行真实 `dev.main` 分派后，旧汇总仍为 PASS。[4][17]

这不表示 CLI 返回了成功，也不证明已有消费者曾误判；风险在于读取固定汇总路径的消费者无法仅靠 status 区分本轮和旧轮。R1 应覆盖开始/终止/中断、run_id、源码与输入身份、旧子报告拒绝、写入失败和消费者迁移。失败报告和成功报告同等重要。

## 4. 结构性热点与建议

### 4.1 资产编译：明确中间结果与写盘边界

`compile.rs` 为 820 行，`compile_path` 从 278 行延伸至 683 行左右，涵盖 validate、再次读取 GLB、sidecar/directive 解析、profile 覆盖检查、纹理处理、descriptor 生成、候选写盘和 manifest。其 fail-closed 与 staging 机制已经存在，重构应保留。[5]

建议内部流程为 `读取 → 规范化/校验 → 构建编译计划 → 执行纹理转换 → 写候选包`。计划类型拥有诊断、材料覆盖和待输出项，文件提交模块只负责原子候选发布。不将纯 glTF 解析搬进 facade，不为了函数变短引入没有独立职责的层。

### 4.2 资产准备：显式当前和历史入口，提供失败后恢复路径

`tools/assets/build.py` 用 pack 分支组织当前角色和旧 reference 构建；当前角色流程包含导入、审计、捕获、导出、Rust 编译与 matched case 生成。它有内容寻址与已完成产物校验，但同一 key 留下未完成目录后直接拒绝再建。[6]

建议把步骤清单、输入摘要和阶段状态分离；为失败候选提供明确的新尝试目录或安全恢复语义。重试不得要求改 recipe 内容来绕开未完成目录，也不得自动删除作者场景。历史 reference 保留显式 opt-in 路径，不参与当前必需输入。

### 4.3 reference 与 inspector：共享运输和身份，不共享参考答案

`labs/reference/src/character.rs` 为 1,003 行，主 run 约 370 行，包含输入身份、场景构建、捕获、校准、区域指标和诊断。inspector 已拆成多个模块，但 `mod.rs` 仍为 604 行；两边均读取资产与 sidecar、处理哈希及路径。[7][15]

可共享严格的纯 bundle 读取、输入身份数据、请求代际和就绪基础设施。应先比较两边接受域：reference 要求固定 case/采样/校准，inspector 接受交互配方；不能简单用一个宽松 loader 覆盖两者。

Blender 真值、engine reference 与 production 的着色和指标推导不能合并成同一实现。采样分辨率、MSAA、色彩转换、warm-up、generation 和 stale request 拒绝均是行为契约。

### 4.4 render：按数据所有权分解，避免提前统一材质

character/hero/material 同时包含参数布局、descriptor 映射、Material specialization 和同步系统。可按这些职责组织私有模块，保留已有公开类型与 re-export；Character 的表通道与 Hero 的家族参数应保持显式差异。[16]

调研不足以认定相似材质代码都可共用。泛型超级材质、shader 全面模板化、自定义 render graph 替换以及“更底层必然更快”均缺少当前需求和测量依据。系统顺序尤其应保留：动画后 outline 同步、transform 后 face 输入、tag 写入前实例更新。

### 4.5 测试、工具与文档清理

`morph_skinning.rs` 达 902 行，却同时包含变形、Genshin AO/ramp/material-band、采样和相机语义测试。按行为拆成测试 binary 有助于定位失败，但必须与 suite identity 迁移一起进行。[4]

Python 文件存在大量分号串联和单行控制流，尤其在 CLI、资产编排和报告路径。分阶段格式整理可降低阅读成本，但格式提交应与错误语义修改分开，且工具版本/入口需要明确。当前没有证据要求为了格式化引入大型 Python 工程框架。

不能按“当前无引用”直接删除 labs、fixture 或 shader：还需核对 ADR、嵌入路径、GPU suite 和 opt-in 命令。缓存清理只应针对明确可再生的候选输出，并先生成清单。

## 5. 本轮验证与证据限制

| 检查 | 本轮结果 | 解释 |
|---|---|---|
| `python3 tools/dev.py check --lane cpu` | PASS，退出 0 | 包含 docs、依赖图、静态独立性、schema、Python 与 Rust |
| Python unittest | 12 passed | 不是 Blender 实机测试 |
| Rust 汇总 | 166 passed / 0 failed / 33 ignored | ignored 不能算 GPU 通过 |
| 验证器错误身份探针 | 当前返回 PASS | 复现 F01；mock 子进程，无真实 GPU 测试 |
| 验证器混合失败探针 | 当前返回 BLOCKED，failed 清零 | 复现 F02；需要保留逐项失败 |
| GPU / 图片回归 | 本轮未运行 | 现有 9 月 15 日记录为 20/20 GPU 通过 |
| 工程隔离 / 当前上游 parity | 本轮未运行 | 日期化记录分别 PASS，限定静态角色范围 |
| 性能 | 本轮未运行 | 既有 instrumented 双视图数据不等于发布 FPS/预算 |

CPU 原始日志本轮位于 `/tmp/stylized-refactor-cpu.log`，属于临时产物，不是项目执行依赖。摘要可从标准命令重新产生。新文档添加后另跑 docs lane 验证。

现有实时记录明确把 temporal visual 标为 REQUIRES_REVIEW，draw calls 为 UNAVAILABLE；不能因采集成功推导出所有视角无闪烁或已经达到发布性能目标。[3]

## 6. 优先级判断

先做 F01/F02/F06，并在重构相关模块前补齐 F05 持续门禁；F04 文档校正可独立推进：它们决定后续重构是否能被可靠判定。再拆编译和开发工具链，最后处理 GPU 敏感内部代码。文件数量、删减比例、crate 数量和“关闭多少 ADR”均不作为成功指标。

重构完成的核心证据应是：标准入口结果可信；现有输入输出与视觉行为保持；当前 source/feature/case 身份可追溯；一处契约变更只需修改明确的拥有者；所有缺失资源和未资格范围仍可见。

## 来源

除外部官方文档外，以下均为上述源码基线的仓库一手材料。代码位置用符号定位以降低重构后行号漂移；日期化材料按其原日期和 scope 使用。

1. [仓库与 crate 架构](../architecture/03-repository-and-crate-architecture.md)、[Cargo workspace](../../Cargo.toml)。
2. [目标和范围](../architecture/01-goals-scope-and-three-axis-model.md)、[目标架构](../architecture/02-target-architecture.md)。
3. [2026-09-15 实时与隔离验证](../quality/realtime-validation-2026-09-15.json)、[当前源锁](../../assets/sources/gacha-authoring.json)。
4. [任务入口](../../tools/dev.py)，`run_tests/main`；[suite 清单](../../tests/suites.json)。
5. [编译器](../../crates/bevy-stylized-tools/src/compile.rs)，`compile_path/compose_ilm/pending_descriptor`。
6. [资产构建](../../tools/assets/build.py)，`build/run_job/verify_run`。
7. [角色参考](../../labs/reference/src/character.rs)，`package/run/calibrate`；[capture harness](../../tests/harness/src/capture.rs)。
8. Python 官方，[Built-in Exceptions](https://docs.python.org/3/library/exceptions.html)，2026-09-15 查阅。
9. [文档检查器](../../tools/checks/check_docs.py)，`project_markdown_files` 与最终统计。
10. [独立化执行记录](../plans/independent-project-execution.md)。
11. [静态角色旧计划](../plans/static-character-reference-plan.md)、[reference 工具链说明](../reference/02-reference-toolchain-lock.md)。
12. [Provisional 清单](../release/03-provisional-inventory.md)、[facade](../../crates/bevy-stylized/src/lib.rs)、[render 入口](../../crates/bevy-stylized-render/src/lib.rs)。
13. [Archived Rust CI](../history/remote-ci-retired-2026-09-15/rust.yml)、[showcase feature](../../apps/showcase/Cargo.toml)、[独立 consumer](../../tools/checks/consumers.py)。
14. Rust/Cargo 官方，[Dependency Resolution](https://doc.rust-lang.org/cargo/reference/resolver.html)，2026-09-15 查阅。
15. [inspector 输入](../../apps/showcase/src/inspector/asset.rs)、[inspector 编排](../../apps/showcase/src/inspector/mod.rs)。
16. [Character](../../crates/bevy-stylized-render/src/character.rs)、[Hero](../../crates/bevy-stylized-render/src/hero.rs)、[Cel](../../crates/bevy-stylized-render/src/material.rs)。

17. [报告写入](../../tools/project.py)，`Project.report`。
