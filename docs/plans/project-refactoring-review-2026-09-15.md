# 全项目重构方案修订与复核记录

## 结论与范围

本次对[调研报告](../research/project-refactoring-audit-2026-09-15.md)和[实施计划 v2](project-refactoring-plan-2026-09-15.md)进行三轮“检查—修订—复核”。终轮在事实依据、阶段依赖、条件性范围和验收一致性方面没有发现仍未处置的实质性问题，可以作为后续实施依据。

此结论针对方案质量；源码缺陷 F01/F02/F06 仍待 R1 实施，不能将“审查项已修订”记成“程序缺陷已修复”。本轮没有修改运行时代码、CI 或基线，没有重新执行 GPU/Blender/性能资格验证。也不声称证明了全仓不存在未知缺陷。

后续已实施 R0/R1/R1b，实际源码修复和验证见[执行记录](project-refactoring-execution-2026-09-15.md)；以下保留方案审查时的处置状态。

## 第一轮：修正上一轮四项发现

| 审查项 | 证据与问题 | 修订 | 处置状态 |
|---|---|---|---|
| V01 旧 PASS 残留 | dev.py GPU 分派在失败前不更新 gpu.json；临时探针复现旧成功报告残留 | 调研补 F06；R1 增 run_id、运行开始状态、失败终态、旧子报告拒绝、消费者迁移 | 方案已修订；源码待实施 |
| V02 CI 过晚 | 原 R6 接入 inspector 门禁，但 R4 已改 inspector | 新增 R1b，在对应重构前验证 feature 与 consumer；R6 只复核迁移 | 方案已修订 |
| V03 材质拆分缺乏必要性证据 | 原 G6/R5 将按层拆模块设为必做，调研仅支持候选建议 | 改为职责审查与有收益才实施的试点，可保留原结构 | 方案已修订 |
| V04 R4 验收捆绑 | 恢复、bundle、reference、inspector、格式共用整条 Blender/GPU 退出条件 | 分成 R4a–R4e，分别列依赖和检查；最终统一闭环留 R6 | 方案已修订 |

## 第二轮：检查修订产生的边界与遗漏

| 审查项 | 问题 | 修订 |
|---|---|---|
| V05 摘要不变要求过宽 | 源码重构会改变 build key/source digest，不能要求所有哈希不变 | R3 区分运行时载荷与构建来源摘要 |
| V06 回放验收不足 | reload/stale request 不能验证轨迹重复、闭环或光照更新时序 | R4d 补 realtime scan；测量路径变化补 measure |
| V07 CLI 退出契约模糊 | usage 和资源阻塞均可退出 2；旧代码还透传子进程退出码 | 明确 usage、原因码、原始退出码与调用方迁移；缺本次报告不能读旧报告 |
| V08 mock 不足以证明新解析器 | Rust nocapture 真实输出可能与模拟文本不同 | R1 必须以真实 GPU suite 和 synthetic import 验证解析；缺资源保留 BLOCKED |
| V09 CI 环境前提缺失 | consumer 使用 offline，inspector 开启 X11/Wayland | R1b 补冷缓存依赖准备与系统构建前提，远程执行状态单独记录 |
| V10 恢复方案过度扩张 | 阶段续跑可能引入过重状态机 | R4a 优先完整新 attempt，跨阶段续跑需独立证据 |
| V11 共享抽取仍被暗中强制 | 目标图和 G5 不应先于接受域比较决定共享实现 | R4b 允许有依据地保留不同实现，目标图同步限定 |
| V12 文档导航重复 | 再增加一份 current summary 容易形成多处事实来源 | R2 优先更新现有 README/ARCHITECTURE 导航 |

## 第三轮：一致性收敛

- 修复验证选择表与 R1/R4a 不一致：GPU runner 需要实机输出验收；纯格式不自动要求全套 Blender。
- 修复 R5 只读审查也被 GPU 基线阻塞的问题；仅代码试点要求对应 GPU 前提。
- 明确 R1b 的 CPU 门禁可在 R1 GPU 子项缺资源时独立推进，后者仍不能标为完成。
- 区分固定 case/输入身份与每次运行变化的 run_id、源码摘要，消除“所有捕获身份不变”的歧义。
- 保持旧阶段编号以便追溯，执行顺序明确包含提前的 R1b，移除 R6 首次接 CI 的旧表述。
- 当前命令使用明确 matched case 路径和不同输出目录；尚未实现的 inspector CPU 新入口不伪装成现有命令。
- 限定串行规则适用本机工作区/设备，远程独立 runner 不被错误限制；GPU CI 准确称为 self-hosted 手动工作流。

## 复现与验证

复核读了 `tools/dev.py`、`tools/project.py`、`tools/checks/isolated_job.py`、`tools/checks/realtime.py`、consumer 工具、showcase feature 和 Rust CI，并对照当前架构与 realtime 契约。

以下只读探针用临时输出和 mock 子进程复现 F01/F02/F06；不访问真实 GPU，也不写项目验证报告。它应在项目根运行。现有缺陷修复后预期结果需要改变，因此它是本次证据脚本，不是永久通过测试。

```sh
python3 - <<'PY'
import contextlib, io, json, sys, tempfile
from pathlib import Path
from subprocess import CompletedProcess
from unittest.mock import patch
sys.path.insert(0, 'tools')
import dev
from project import Project

with tempfile.TemporaryDirectory() as directory:
    project = Project()
    project.output = Path(directory)
    probes = [
        ('wrong-identity', 'test wrong_test ... ok\n'
         'test result: ok. 1 passed; 0 failed; 0 ignored;', 0, 1),
        ('mixed-errors', 'required GPU unavailable\n'
         'test result: FAILED. 0 passed; 2 failed; 0 ignored;', 101, 2),
    ]
    for name, output, code, expected in probes:
        with patch.object(dev.subprocess, 'run',
                          return_value=CompletedProcess([], code, output)):
            with contextlib.redirect_stdout(io.StringIO()):
                try:
                    dev.run_tests(project, name, ['mock'], expected, {})
                except RuntimeError:
                    pass
        result = json.loads((project.output / (name + '.json')).read_text())
        print(name, result['status'], result['failed'], result['blocked'])
    project.report('gpu', {'status': 'PASS', 'marker': 'previous-run'})
    with patch.object(dev, 'Project', return_value=project), \
         patch.object(sys, 'argv', ['dev.py', 'check', '--lane', 'gpu', '--required']), \
         patch.object(dev, 'run_tests', side_effect=dev.CheckFailed('current failure')):
        try:
            dev.main()
        except dev.CheckFailed:
            pass
    print('stale-summary', json.loads((project.output / 'gpu.json').read_text()))
PY
```

本轮实际输出：wrong-identity 为 PASS；mixed-errors 为 BLOCKED、failed=0、blocked=2；stale-summary 为旧 PASS/previous-run。以上确认调研的风险表述，未证明当前真实 GPU case 丢失或下游曾误用报告。

文档检查使用 `python3 tools/dev.py check --lane docs`。此外单独检查三份文档的全部本地链接（包括非 Markdown 目标）、空白与代码块、问题编号和工作包引用，避免仓库 linter 的覆盖范围被误解为内容审查。

本轮复核完成后，下一步是实施计划中的 R0/R1；无需通过继续增加文档来替代实现。实施中发现新的具体问题时，应更新证据和对应工作包，不改写本次检视的历史范围。
