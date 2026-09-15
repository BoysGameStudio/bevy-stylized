# 独立项目计划审查记录

本轮为同一执行者的分轮自审，没有声称由独立代理或外部维护者审查。
审查只修改调研/计划文档，没有实施重构。v1 快照保存在本地
`target/tmp/independence-plan-review/plan-v1.txt`；关键差异在本记录中保留。

## 第一轮：对草案 v1 的反例审查

| ID | 严重度 | 发现及反例 | 必须修订的内容 |
|---|---|---|---|
| R1 | Critical | 仅宣称移除 legacy，没有映射其测试覆盖；直接删除会掩盖旧行为回归 | 加 legacy 退出覆盖账本、证据保存和前置 gate |
| R2 | Critical | 3 层参考改为 2 渲染器后缺少独立性和 ADR 决策规则 | 明确 upstream/历史 baseline/数学断言职责，改变 ADR 0007 等的正式提案 |
| R3 | High | P3 未选资格版本；直接使用已知失败 v3.6.4 可能悬空 | 保留 v3.6.2 资格线，v3.6.4 候选单独验证；不自动换锁 |
| R4 | High | 只列 packed maps 名称，没有逐项制作责任 | 原始来源/上游生成/项目变换分类，pupil 与 face 逐项 hash、通道、参考图 |
| R5 | High | 图里 tools 未依赖 profile，却要求其调用 profile mapping | 补 optional profile edge，保持 core/profile 纯数据和 engine 边界 |
| R6 | High | runtime/schema authority 过于笼统，可能新增第二份手维护规范 | 明确每类 schema 的 writer/reader/真源和生成校验方向 |
| R7 | High | “无 Tenacity”仅等同于不设置 env，缓存/软链接仍可偷读 | 加干净源副本、隔离文件系统和独立空缓存的实际负例验收 |
| R8 | High | phase 粗粒度；move 与行为变更耦合、测试定位仍用 ../ | 完整目录映射、显式路径接口、各 PR 工作/退出/回退 |
| R9 | High | required GPU/optional external 退出码和缺基线仍不具体 | 明确状态矩阵、命令语义、case coverage 和 CI 分层 |
| R10 | High | 发布包验证受 publish=false、registry 未发布内包影响 | 区分源码独立分发、包就绪检查、真实发布；不以 --no-verify 伪造通过 |
| R11 | Medium | 新技能仅存在个人目录，项目还依赖机器上的知识 | repo-owned skills 为真源，个人安装为可选副本，文档本身必须可独立使用 |
| R12 | Medium | profile-neutral/feature 策略可能错误承诺 facade 无 render 依赖 | CLI pure feature 明确拆注册类型；最小消费者独立验证；core-only 使用 core |

修订动作：v2 修正依赖边；随后在 v3 中展开资产、参考、schema、阶段及验收表，
保留全部 R1–R12 的关闭证据索引。没有用增加等待用户决定的空占位来回避反例。

## 第二轮：对展开后的 v3 计划复审

| ID | 严重度 | 发现 | v4 修订/关闭位置 |
|---|---|---|---|
| R13 | High | P7 才改文档扫描和 CI 路径，P6 move 期间可能使验证断裂 | §9：扫描/history 规则前移 P1；move 同批更新链接/CI filters |
| R14 | Critical | required reference 失败与“工程独立允许已知 parity 未达标”存在阶段退出歧义 | §8：independence/asset-integration/reference 分集合，判据和失败归属明确；P5 退出重写 |
| R15 | Medium | 同名个人旧技能可能仍优先生效，引入旧路径 | §11：repo 真源、摘要/加载位置检查，不假定宿主选新版本；新增 AGENTS 入口 |
| R16 | Medium | “源码可独立交付”可能被误读为已解决许可 | I12/§11：只证明源码归档可独立构建，不宣称公开分发许可 |
| R17 | Medium | strict profile compile 描述没有说明功能未编译时的路径 | §6：拟定 profile/strict CLI、optional feature 缺失明确报错，不回退默认 Cel |
| R18 | Low | 路径守卫可能过度禁止所有 CARGO_MANIFEST_DIR | I01：只禁越界和隐藏外部依赖，保留合法 crate 内资源定位 |

v3 原稿本地快照：`target/tmp/independence-plan-review/plan-v3.txt`。
v4 逐项吸收上述修订，没有删除旧视觉失败，也没有把 BLOCKED 当完成。

## 最终一致性复审

| 检视维度 | 结论与证据 |
|---|---|
| 每个已发现活动依赖有处置 | 调研 C01–C08 → 计划 §3.1、§5、§7、P3/P4/P6；包括 Rust、Python、资产、CI 和技能 |
| 六产品与目标 workspace 一致 | §3/§4；保留六产品，harness 和 integration 明确 dev 包；无未定义 normal edge |
| 删除前覆盖不会丢失 | §7.1/P0/P5/P6A；缺替代的必需测试阻塞删除，旧实现内部测试具名退役 |
| 动态下载和本地缓存不会掩盖耦合 | §5.2、I02/I03/I05；直接来源、逐文件 hash、空缓存、禁挂 Tenacity |
| 参考不循环自证 | §7 三种证据职责；production baseline 只证明回归，upstream parity 独立 |
| 失败/缺资源行为可实现 | §8 状态表、任务集合、expected/executed coverage；缺 GPU/基线有负例 |
| 版本候选没有被误标资格通过 | §5.4；v3.6.2 artifact 与完整角色资格区别，v3.6.4 仍 candidate |
| schema 不形成双真源 | §6 真源/生成方向/reader table；profile→tools edge 已加入 |
| 目录搬迁中间状态可验证 | §3.1、P1、P6B；搬迁同批改引用，行为改变另做 |
| 发布承诺与依赖现实一致 | §11；publish=false 保留，未注册内部包不靠 no-verify 假通过 |
| 用户检视前没有执行迁移 | 本轮新增三份计划/调研文档；未改 Cargo、产品源码、CI 或资产接受基线 |

第一轮关闭索引：R1→§7.1；R2→D03/§7；R3→§5.4；R4→§5.3；
R5→§4/§6；R6→§6；R7→I01–I05；R8→§3.1/§9；R9→§8；
R10→§11；R11→§11；R12→§4/I10。

结果：当前没有已知未关闭的 Critical/High **计划缺陷**。这不等于实施已经完成，
也不保证未来不会发现新的技术问题。已知 importer/资产映射/visual parity/runner/
license 问题均有具名阶段、状态和退出条件；计划给出建议方向，等待用户检视。


文档检查补充证据：审查快照最初以 Markdown 存在 gitignored target 中，现有
checker 仍把其中相对链接当项目文档校验并报错。快照改存纯文本后检查通过；
checker 本身未在计划阶段修改。P1 的受控文档扫描修正因此有本轮实际反例。
