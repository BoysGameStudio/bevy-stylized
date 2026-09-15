# R0、R1 与 R1b 执行记录

## 范围

实施[重构计划 v2](project-refactoring-plan-2026-09-15.md)的 R0、R1、R1b。
R2–R6 尚未执行。本批不改变产品 Rust/WGSL、runtime schema、源包锁、作者场景
或已接受的图片基线，也没有创建 Git commit 或发布包。

状态：R0、R1、R1b 的本地实施与适用验证完成；远程 CI 未运行。

## R0：基线与迁移

基线 revision 为 `5d723b4da35cf9e9b143432f8dbce48b4d03657b`，开始时已有三份
未跟踪的调研/计划/审查文档，已保留。351 个原跟踪文件的完整哈希、测试清单与
公共导出文本快照写入 `target/stylized/refactor-r0/baseline.json`。
[持久摘要](../quality/refactor-r0-2026-09-15.json)保留其哈希、原工作区状态、suite
身份以及 fixture/schema/source 锁摘要。完整快照是审计产物，不是项目运行依赖。

| 原拥有者 | 新拥有者/处置 | 保留的行为 | 验证 |
|---|---|---|---|
| dev.py 的 run_tests/异常处理 | task_runner.py + dev.py 分派 | required suite、原始退出码、禁止隐式 rebaseline | 身份/混合失败负例、真实 libtest/GPU |
| Project.report 固定文件 | Project 的运行目录、状态生命周期与 report reader | 原子 JSON 写入、兼容固定名便利副本 | 成功后失败/中断、错 run/source、旧子报告、写失败 |
| dev.py GPU_TESTS + suites 长度 | tests/suites.json 单一 binary/case 清单 | 原 20 个 GPU case + synthetic import | 精确 inventory 与逐项执行 |
| docs checker 最终 rglob | 使用已检查集合计数 | 原文档与 ADR 校验 | docs lane |
| isolated_job 只核对退出码 | 父调用方分配 run_id 并检查对应报告 | 隔离负例与当前静态角色范围 | 调用方协议测试；完整隔离未复跑 |
| 默认 CPU 未开启 inspector | 新 inspector-cpu lane | 默认 CPU 不要求窗口/外部角色 | 7 个 feature 单测、all-target clippy |
| 本地 consumer 工具 | 显式 --fetch + CI 调用 | 五个独立消费者、纯依赖闭包检查 | 五配置实际通过 |

## R1：结果可信性

- 必需测试先比较真实 ignored inventory，再按 exact identity 单独执行；摘要数量
  与身份必须一致，零执行和异常输出不能通过。每个 case 保留独立日志。
- 资源 panic 在 case 内识别，混合结果同时保留 failed/blocked，FAIL 优先。
- RUNNING 在任务开始时发布；失败、阻塞和可处理的中断写终态。旧 PASS 不能通过
  本次 run_id 的 reader。不可处理的终止留下 RUNNING，不构成资格。
- 普通校验、输入、子进程、I/O 和内部错误不再统一记为 BLOCKED；明确资源缺失
  使用 ResourceBlocked。usage 的退出 2 与资源阻塞必须结合报告区分。
- 标准任务日志与子报告进入运行目录；主报告只列本次子报告。隔离调用方在子进程
  启动前保存运行标识和源码摘要，并检查本次终态，不能只看固定名 JSON。

第一次真实缺 GPU 负例暴露 Rust panic 输出中的线程编号，模拟文本未覆盖这个
格式。已加入带编号 panic 回归，修复后真实负例返回 BLOCKED/退出 2。第一次错误
分类的报告仍保留为 `refactor-missing-gpu`，不重写其失败结果。

完整对外行为见[任务结果契约](../testing/02-task-results.md)。运行身份只补充任务
归属，不能代替现有 asset/case 的依赖哈希、领域验证和上游对齐判据。

## R1b：持续门禁

新增 `python3 tools/dev.py check --lane inspector-cpu`：执行 inspector library
测试及该 feature 下 all-target clippy。Rust CI 安装 X11/Wayland 开发依赖后调用
此入口，再调用 `check --lane consumers --fetch`。后者显式为每个独立 consumer
准备依赖，再执行 offline 检查；省略 fetch 时，缺依赖缓存会报告 BLOCKED。

Cargo/GPU 在本机串行。GPU CI 仍是既有 self-hosted 手动工作流；本批未运行远程
CI，也未配置新 runner。本地通过不替代远程运行证据。

## 验证结果

最终机器摘要见[本批验证记录](../quality/refactor-validation-2026-09-15.json)。
记录包含源码身份、run ID、实际计数和范围。首轮与最终复验分别留档，不覆盖旧结果。

| 检查 | 结果 |
|---|---|
| CPU lane | PASS：Rust 166 passed / 33 ignored；Python 28 passed |
| GPU required | PASS：20/20，另 synthetic import 1/1 |
| Inspector CPU | PASS：7/7，feature all-target clippy 通过 |
| Independent consumers | PASS：五配置；显式 fetch 与缓存模式均执行 |
| 真实缺 GPU | BLOCKED/退出 2：20 个 GPU case + synthetic import，无失败计数被掩盖 |
| 真实 libtest 混合负例 | 按预期 FAIL：1 个 baseline panic 为 BLOCKED、1 个断言为 FAIL；使用临时 Rust fixture，不移除真实 baseline |
| 文档、Python 语法、差异空白、CI YAML/入口 | PASS |

缺 GPU 的早期错误分类和修复后的结果分别留档；临时 Rust fixture 只验证真实
libtest 输出与分类，不能替代图像 baseline 本身的资格证据。

工程边界/CPU、第一方 GPU 回归分别报告。完整 Git-free 源到 Blender 重建、当前
上游 parity 和性能本轮未复跑；产品/fixture/schema 哈希保持，既有日期化资格
记录保留原范围。R6 的完整隔离闭环仍是后续工作，不提前声明完成。
