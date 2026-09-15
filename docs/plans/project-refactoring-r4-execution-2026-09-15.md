# R4 执行记录

## 范围与结果

完成[计划 v2](project-refactoring-plan-2026-09-15.md)的 R4a–R4e，分别验收。
机器证据见[本批验证](../quality/refactor-r4-validation-2026-09-15.json)。R5/R6
尚未执行；没有创建 Git commit、发布包、覆盖作者场景或接受新图片基线。

## R4a：失败恢复与真实源到资产构建

新增 `tools/assets/attempt.py`，将内容键与尝试 ID 分开。失败或中断后再次执行
原 build 命令，会创建同一内容键下的新尝试；已成功的候选验证摘要后复用。
原失败现场、日志和作者文件保留。没有跨阶段续跑状态机。

每阶段记录输入摘要、命令、日志、改变/新增/移除的产物和退出状态。完成清单
原子写入，临时清单不构成成功；`attempt.json` 不计入产物摘要。
具体契约见[资产构建恢复](../authoring/06-build-attempts.md)。

验证包含失败现场保留、完整新尝试、已成功缓存复用、篡改产物拒绝、阶段失败后的
部分产物记录、旧布局读取、未提交清单和输入摘要冲突。真实 Blender 5.2.1 LTS +
锁定 Gacha v3.6.4 完成最终新候选，八个进程阶段全部通过：导入、审计、初始参考
捕获、导出、编译、case、诊断与 matched 准备。审计的无效约束/driver/缺图均为 0。

最终新资产通过 asset-integration required 和六案例 character-reference required。
再次执行同一 build 命令返回同一路径，原尝试 journal 哈希不变。两次新构建的
全身/脸部 PNG 文件字节不同，但解码 RGBA 相同；最终图像已经目检。这与下文
冻结输入下的前后文件字节对比是不同证据，不混用。

## R4b：保留不同的输入校验边界

完成[接受域对照](../testing/03-capture-input-boundaries.md)，选择保留独立校验器。
reference 固定 case、相机、采样和 actual-setting 证据；inspector 面向实时配方
及自己的严格 lexical path/fingerprint 规则。两者没有合并成通用宽松 loader。

reference 的 package 校验函数正文与原实现逐字一致，仅移动拥有者；inspector
asset 校验实现未改。已有校验负例、默认 CPU 与五种独立 consumer 检查通过。
没有新增产品 crate 或把 capture schema 推进 runtime glTF 合约。

## R4c：reference 职责拆分

原 character.rs 的 1,003 行分成协调入口和八个私有模块：input、scene、frames、
calibration、diagnostics、metrics、cases、io。入口负责验证、构建校准场景和调用
六案例比较；case 循环与报告、独立指标、诊断 pass、采样/readback 各有拥有者。
原 `character::tests::empty_geometry_masks_cannot_pass` 随指标移到
`character::metrics::tests`，内容不变。没有修改 shader、阈值、采样或判定公式。

用相同冻结 matched 输入分别执行改动前后版本：62 张 PNG 的文件字节完全相同，
整个 reference report 也相同，六案例 parity 均 PASS。之后重新构建的新输入也
通过六案例匹配验证，因此不是只对缓存图片做报告生成。

## R4d：inspector 编排拆分

mod.rs 从 604 行收敛为 201 行 app 装配；options.rs 管参数，runtime.rs 管现场
生命周期、view 同步、拟合和 readiness。原 state、panel、capture、validation
继续拥有配方/UI、代际截图和回放/测量。公开 inspect 命令与调度顺序保持。

同一冻结资产的前后 inspector required 检查均通过，包括 reload、stale request
retirement 和 readiness，2 张截图文件完全相同。另执行相同的 orbit/light/zoom
扫描：每阶段 24 帧、两次回放、每视图 512²，144 个采样状态、288 张 PNG 全部
逐字节相同；重复帧、闭环和非空几何检查通过。该采样配置是明确的回归范围，
不冒充标准 120 帧/1024² 的 1,440 图采集，也不声称穷尽时间闪烁或性能资格。

## R4e：Python 可读性整理

针对本批资产编排文件展开多语句行和单行 guard，格式步骤用 AST 对照确认语义
不变；移除因职责迁移而失效的导入。恢复逻辑和原子提交属于独立行为修改，有
相应负例与真实构建证据。未为格式化引入新工具依赖，也未批量改写 Blender 脚本。

## 最终检查与范围限制

- CPU：Rust 166 passed / 33 ignored；Python 34 passed。
- Inspector feature：7 passed，all-target clippy 通过。
- Consumer：五配置通过；workspace 全功能 clippy、fmt 和文档检查通过。
- 前后共 352 张 PNG 文件相同；冻结 reference 报告相同；最终新资产六案例 PASS。
- 原 source lock、fixture、schema、WGSL 和作者源场景保持，未放宽任何阈值。

本轮没有重新执行完整第一方 GPU suite、性能测量、远程 CI 或 Git-free 全隔离
重建。局部 GPU 对比、新资产接入、当前静态 upstream parity 和 CPU/静态工程
检查分别记录；R6 的最终隔离闭环仍待执行。新构建只针对现有受支持静态变体，
不建立完整 rig、effect、动画控制或旧 Gacha 兼容义务。
