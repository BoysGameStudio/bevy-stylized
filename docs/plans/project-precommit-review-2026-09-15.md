# 提交前审查与分组建议

## 结论

审查覆盖整轮改动和未跟踪的新模块，重点复核报告状态、构建恢复、编译失败、
截图生命周期、依赖边界及是否扩大 API。发现四项问题，均已修复并完成针对性验证。
渲染/facade 的可执行语句与公开导出保持；本次不是渲染功能扩展。

这里的“审查完成”不代表不存在未知缺陷。此前完整隔离结果保留其原源码身份；
本轮额外验证范围见下表，没有把旧记录改写成新源码的全量隔离结果。
没有创建 commit、暂存改动、发布包或接受新基线。

## 发现与处置

| 编号 | 优先级 | 触发与影响 | 修正与验证 |
|---|---|---|---|
| PC01 | 高 | GPU registry 为空时仍运行 synthetic import，整条 GPU lane 可返回 PASS | 执行前验证非空 registry 及所有 binary/case 项；临时 CLI 回归确认退出 1 且不启动测试进程 |
| PC02 | 高 | verify_run 接受错误 schema 或空 files map，损坏的完成清单可被当作有效缓存 | 校验 schema、非空文件映射、inputs 类型、相对路径/边界和 SHA 格式；损坏清单拒绝，真实已完成候选仍通过 |
| PC03 | 高 | `A B` 与 `A_B` 归一化到同一 material 文件，后者覆盖前者；原版编译器同样存在 | 规划完成后统一检查输出路径；材质冲突或不同字节冲突拒绝发布；共享 embedded texture 的相同字节合并为一项 |
| PC04 | 中 | R4 移动 Runtime/Options 后，inspector 截图源码摘要漏掉新模块 | module-sources/v2 覆盖全部 inspector 模块，递归目录清单测试防止漏项；实机截图摘要与源码清单一致 |

PC03 已用保存的重构前 executable 复现，因此属于既有缺陷修复，不归因于模块
拆分。PC04 是此次重构引入的 provenance 遗漏，之前图片等价测试不能发现这种
问题。PC01/PC02 是本轮承诺的非空执行和缓存验证边界缺口，不是图像误差问题。

编译碰撞修复保持既有拷贝纹理命名冲突规则；相同 embedded image 的重复输出
才合并，不把不同来源的命名冲突悄悄改成自动重命名。它新增了错误诊断，也可能
减少共享 embedded texture 的重复 manifest 条目，应作为明确行为修复提交。

## 验证范围

| 检查 | 本轮结果 |
|---|---|
| CPU lane | Rust 168 passed / 33 ignored，Python 37 passed |
| strict profile CLI | 16 passed，包括材质碰撞、共享纹理、不同内容同名冲突 |
| GPU required | 20 passed，另 synthetic import 1 passed |
| Inspector feature | 8 passed，包含完整模块 provenance 测试 |
| 实机 inspector/reload | PASS；PNG 与 R4 冻结输入结果逐字节一致，v2 源码摘要正确 |
| 正常编译输入前后对比 | 三个真实输入共 43 个输出文件仍逐字节相同 |
| 缓存兼容 | 既有真实成功构建通过加强后的校验 |
| clippy / fmt / 文档 | PASS |

本轮没有改变 shader、fixture、source lock 或 runtime schema，也没有重复 Blender
导入、完整隔离或性能测量。此前 R6 隔离和 R4 图像证据仍适用于各自记录的源码；
本轮通过正常输出等价、真实 GPU/截图与新负例验证这四项增量修复。
机器摘要见[提交前验证](../quality/precommit-validation-2026-09-15.json)。

## 建议提交分组

以下是依赖有序的提交建议，不是已经创建或逐组 checkout 验证过的 commits。
同一文件横跨组时应按 hunk 暂存；每组形成后执行对应检查，不能只依赖当前整树
结果。尤其不能仅使用 `git commit -am`，否则会漏掉未跟踪的新 Rust/Python 模块。

| 顺序 | 建议主题 | 主要范围 | 验证 |
|---|---|---|---|
| 1 | 可靠任务结果与持续门禁 | project/task_runner/dev、checks、Python runner tests、CI、任务结果契约；先不含 PC01/PC02 新 hunk | CPU、inspector-cpu、consumers、资源负例 |
| 2 | 编译器内部职责拆分 | compile.rs、compile/input/textures/plan/output、main.rs 共用检查；先不含 PC03 collision hunk/新增测试 | CLI、schema、consumer、正常产物差分 |
| 3 | 可恢复资产构建 | assets/build、attempt、attempt tests、构建恢复文档 | Python 恢复测试、真实构建与接入；PC02 加强校验留第 7 组 |
| 4 | 开发捕获工具职责 | reference character 私有模块、inspector options/runtime、相关输入边界文档；先不含 PC04 provenance hunk | reference/inspector、352 图差分和回放检查 |
| 5 | 测试分组与 lint 清理 | morph/profile_lighting/sampling_camera/support、suites、迁移表；showcase 测试模块搬移与 Cargo lint 删除一起提交 | CPU、GPU exact identity、全功能 clippy |
| 6 | 当前文档与历史证据 | README/ARCHITECTURE、provisional 历史副本与当前导航、skills、仅注释的 render/facade 修正、阶段报告 | 文档链接与保护文件检查 |
| 7 | 提交前发现的行为与 provenance 修复 | PC01 registry guard、PC02 manifest guard、PC03 collision guard 和回归测试、PC04 source list/policy/test、本审查记录 | 本轮表列检查 |

分组 1 中隔离调用方使用最终测试名清单和现有标准入口；分组 5 迁移 suite 与测试
文件必须同时纳入，不能只移动测试文件。分组 3/4 的新目录也必须纳入文件清单。
文档存在跨组链接时，将对应链接 hunk 留到目标文件所在组或最后文档组。

完整改动文件清单位于 `target/stylized/precommit-review/file-inventory.json`，
包含已跟踪和未跟踪文件。整理提交时以实时 git status 为准，避免漏收后续修订。
