# R2 与 R3 执行记录

## 完成范围

已完成[计划 v2](project-refactoring-plan-2026-09-15.md)中的 R2 文档治理与 R3
编译器职责拆分。本批保留前一批 R0/R1/R1b 改动；R4–R6 尚未执行。
没有创建 Git commit、发布包、接受新图片基线或改动作者源场景。

## R2：当前契约与历史证据

README/ARCHITECTURE 现在直接导航到当前 source lock、日期化静态资格、后续任务
验证、支持矩阵与未决 ADR。旧独立化执行记录、静态角色计划和 reference 锁说明
增加适用性标注，旧 Gacha 要求保持历史 opt-in，不进入当前默认验收。

原 2026-09-12 provisional 清单逐字节保存在
[历史副本](../history/provisional-inventory-2026-09-12.md)。
[当前清单](../release/03-provisional-inventory.md)改为实际模块、已记录证据和未决
决策的导航，明确 StorageTable 已实现、Hero face transport 的限制以及 outline
现有同步行为。没有改变任何 ADR 状态或重写历史测量结果。

仓库技能的导出路径改为当前 schemas/authoring；当前角色 matched 对比与历史
reference lane 分开。facade/render 模块说明覆盖当前功能，去除旧切片和稳定 API
的误导描述。文档 checker 增加当前入口的源文件/schema 链接检查，以及 README
资格版本与源锁的一致性检查，范围有界，不检查历史示例的版本义务。

移除全局 items_after_test_module 豁免前，clippy 查明唯一仍触发的位置是 showcase
cases 中间的测试模块。将该模块原样移动到文件末尾后，workspace 全功能 clippy
通过，豁免得以移除。测试内容和运行时函数未改变。

## R3：编译职责与不变量

| 拥有者 | 职责 |
|---|---|
| compile.rs | 公共报告/入口、验证与最终发布门禁 |
| compile/input.rs | 指令 schema、sidecar、profile 解释和材料覆盖 |
| compile/textures.rs | 复制去重、文件命名、base-color 提取和 ILM 转换 |
| compile/plan.rs | 材质合并、有序诊断、完整待输出文件内容 |
| compile/output.rs | staging、hash、manifest 和候选目录发布 |
| main.rs 的 inspect_document | validate/inspect/compile 共用的已解析文档检查 |

compile 现在只读取、解析一次 glTF 主文件；外部纹理和 sidecar 仍由各自拥有者读取，
不宣称所有文件具有事务快照。诊断顺序、schema/字段、文件命名、纹理算法、profile
映射、CLI 退出码和错误时不发布候选的行为保持。没有新增公共 crate、依赖或特性。
完整职责约束见[架构说明](../architecture/03-repository-and-crate-architecture.md#compiler-internal-ownership)。

入口由 820 行缩为 157 行，四个私有模块分别为 199、221、246、61 行。总行数因
显式边界和导入略有增加；此次收益是拥有者和副作用边界清晰，不以总行数或提速验收。
资产构建的 compiler_inputs 使用递归源码清单，已核对四个新模块均纳入缓存键。
源码变化会使构建缓存失效，运行时包格式不变。

## 前后行为比较

在修改前保存原编译器 executable，随后对冻结输入运行原版和重构版，候选输出
分别写入 `target/stylized/refactor-r3` 下的新目录，未复用已有编译结果冒充重编译。

| 输入 | 输出文件数 | 结果 |
|---|---:|---|
| 第一方 reference-scene | 5 | 全文件哈希一致 |
| 第一方 synthetic character | 4 | 全文件哈希一致 |
| 当前静态角色、strict Genshin | 34 | 全文件哈希一致 |

包括 compile-manifest 在内共 43 个文件逐字节相同，报告除目标输出目录外一致。
角色输入的 build-manifest 已核对 addon manifest digest，匹配当前 Gacha v3.6.4
源锁。这是编译等价性证据，不是本轮新的 Blender/上游视觉资格。

额外用冻结的单材质输入分别检查：正确 ILM 打包、缺失通道、未知 pack 类型、未来
schema、sidecar/extras 冲突、重复 pack。六种情况的报告和产物均与原版一致，五种
错误均退出 1 且不创建输出目录。它们是差分验收用例，不新增渲染特性。

## 验证与限制

[机器摘要](../quality/refactor-r23-validation-2026-09-15.json)记录源码身份和结果。

- CPU lane：Rust 166 passed / 33 ignored，Python 28 passed；包括文档、图依赖、
  静态独立性、schema 与默认 CLI 测试。
- strict profile CLI：14 passed，包括输出目录拒绝覆盖、完整材料覆盖与失败不发布。
- 独立 consumers：五配置通过。
- workspace/all-targets/all-features clippy 和 cargo fmt check 通过。
- 当前文档链接、历史清单字节保留、嵌套 compiler 缓存覆盖检查通过。
- 资产锁、fixture、schema、WGSL 哈希与此前基线保持一致。

本轮未运行 GPU、性能、Blender 或完整隔离重建。渲染/facade 只改说明，showcase
只移动测试模块；编译运行时产物逐字节等价，因此未重复渲染测量。已有日期化
资格保留原范围，R6 完整闭环仍待执行。没有用本批 CPU/编译通过宣称上游 parity。
