# 全项目重构最终验收

## 结论

[计划 v2](project-refactoring-plan-2026-09-15.md)的 R0–R6 已完成本地实施和适用
验收。R5 按条件性目标保留现有渲染实现，依据见[职责审查](../rendering/05-renderer-ownership-review.md)。
本批未创建 Git commit、发布包、覆盖作者源场景或接受新图片基线。

[最终机器证据](../quality/refactor-final-validation-2026-09-15.json)分别记录：

| 维度 | 结果 | 范围 |
|---|---|---|
| 工程独立性 | PASS | Git-free、带空格路径、空项目资产缓存、断网；只挂原始源包、工具链和设备 |
| 第一方图像/语义回归 | PASS | 20 个必需 GPU case + synthetic import 1 个 |
| 当前上游 parity | PASS | 锁定 Gacha v3.6.4、现有静态变体、六个 matched case |
| 性能 | SMOKE_ONLY | 隔离内 60 帧 instrumented 检查；不建立发布预算或优化声明 |
| 远程 CI | 未执行 | 工作流已接入并通过本地对应检查，未新增/运行远程 GPU runner |

后续[提交前审查](project-precommit-review-2026-09-15.md)补充四项边界修复与验证；
本记录保留 R6 完成时的源码与隔离范围。

## R5：有依据地保留渲染边界

审查了材质参数、descriptor 映射、specialization、事件同步、instance/table、
view/LOD、face input 和 outline 生命周期。Cel→Character→Hero 已复用转换，表
材质具有不同的存储绑定。局部重复不足以证明新 trait 或更细目录层次有净收益。
因此没有启动无依据的 renderer 拆分，也没有预先选择新的 Material backend。

仅修正两处与实际 enable_prepass 返回值矛盾的注释。shader、uniform、binding、
公开导出和运行时语句保持。Open ADR、平台和精度限制仍保留，不因验收而自动关闭。

## R6：测试分组与清理

原 morph_skinning 中的十一项分为四项变形/姿态、五项家族光照、两项采样/相机。
测试函数名、测试正文和场景/capture helper 的行为保留；helper 放到测试专用模块。
[suite](../../tests/suites.json)同时更新，必需 GPU 身份总数仍为 20，逐项关系见
[迁移清单](refactor-test-migration-2026-09-15.json)。原独立化 coverage ledger 的
preserved morph/skinning 名称与 outline replacement 路径仍有效，无需改写历史。

[清理处置表](project-refactoring-cleanup-2026-09-15.md)记录被替代的入口/重复清单、
归档、移除的豁免，以及必须保留的 labs、skills、fixture、原始资产和失败候选。
不以删除比例或文件长度衡量成果。

## 最终隔离与返回检查

通过标准 `tools/dev.py check --lane independence --required` 执行完整隔离。
其中重新准备原始源包、生成第一方 fixture、运行 CPU、资源负例和正常 GPU suite，
重新构建 Blender 角色，并完成 asset integration、inspector、scan/measure smoke、
诊断 reference 和 strict reference。不是挂载之前加工完成的角色来替代重建。

隔离 CPU 为 Rust 166 passed / 33 ignored、Python 34 passed；inspector feature
7 项通过。缺 GPU 时 20 个主 suite case 均 BLOCKED；缺 Cel baseline 时 18 passed /
2 BLOCKED，均正确退出 2，没有把真实失败清零或写成 PASS。基线负例只移动隔离
副本中的文件并恢复，宿主接受基线哈希不变。

隔离报告分别保留工程、图像、parity 状态。失败步骤先记录原始退出码，再核对其
运行标识和终态；已测得的 parity 不再被后续失败覆盖成“未执行”。完整副本摘要
保存在 snapshot_sha256，与任务的有界 source_sha256 分开，避免元数据覆盖。

返回宿主后 CPU、五个独立 consumer 配置及 workspace/all-targets/all-features
clippy 再次通过；fmt 和文档检查通过。最终证据的任务源码摘要与宿主一致。
隔离完成后仅更新完成记录等文档，完整副本摘要保留实际执行时的内容。

## G1–G8 验收表

| 目标 | 处置 | 核心证据 |
|---|---|---|
| G1 结果可信 | 实现并验证 | R1 负例、真实 libtest、run ID、最终资源负例与精确 case 身份 |
| G2 当前契约入口 | 实现并验证 | README/ARCHITECTURE 导航、当前清单、完整历史副本、文档检查 |
| G3 编译职责 | 实现并验证 | 四个私有模块；R3 43 个输出文件及错误诊断前后等价 |
| G4 构建恢复 | 实现并验证 | 同键新尝试、原子完成清单、失败保留、完整新构建和缓存复用 |
| G5 开发应用边界 | 实现并验证；共享 loader 经评估保留独立实现 | R4 接受域对照、职责拆分、352 张 PNG 和 report 等价 |
| G6 渲染职责 | 经评估保留实现 | R5 已有组合与调度/生命周期审查；最终 GPU 验证 |
| G7 持续门禁 | 定义与本地对应检查完成 | inspector-cpu、consumer、GPU 手动工作流；远程状态单列 |
| G8 有据清理 | 完成处置与核对 | 清理表、测试迁移、shader/fixture/schema/source 锁保护 |

各阶段详细证据：[R0/R1/R1b](project-refactoring-execution-2026-09-15.md)、
[R2/R3](project-refactoring-r23-execution-2026-09-15.md)、
[R4](project-refactoring-r4-execution-2026-09-15.md)。历史失败记录保留原日期和范围。
本次完成的是工程重构，不等同于关闭发布资格、支持完整 rig/effect 或所有 GPU 后端。
