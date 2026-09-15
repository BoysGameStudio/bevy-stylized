# Bevy Stylized 独立化调研

调研日期：2026-09-14。代码基线：`a47af6c`。本轮范围为调研、计划和计划审查，
没有执行目录搬迁、删除 legacy、改动运行时代码或重建接受基线。

## 1. 独立性的现状

项目已经有独立的 Cargo workspace 和六个产品 crate，直接 manifest 路径都在
仓库内部；尚未具备独立的资产制作、真实角色验证、参考体系和完整新环境验收链。
不能用“产品代码没有外部 path dependency”代替“整个项目完全独立”。

`cargo metadata --locked --offline --format-version 1 --no-deps` 显示 15 个
workspace members，默认选择也是全部 15 个。这个检查仅证明本次读取的 workspace
manifest 依赖情况；不宣称完成了所有第三方传递依赖和发布包审计。

| 范围 | 实际情况 | 独立化影响 |
|---|---|---|
| 产品 crate | facade/core/render/gltf/profile-genshin/tools 共 6 个，内部 path 依赖 | 可保留产品边界，无须重写整个 renderer |
| 默认工作区 | 6 产品 + 7 dev + 2 legacy，共 15 包 | 普通 workspace 验证仍编译迁移期参考实现 |
| Legacy | 95 个 Rust/WGSL 文件，约 17,240 行 | 应明确退出活动架构，而非改目录名后继续依赖 |
| 真实角色输入 | Odette GLB、9 项 NPR 贴图读取 Tenacity 的产物 | 改环境变量名称不能消除制作依赖 |
| Reference | comparison 同时依赖 production 和 bevy_cel | 参考体系必须调整后才能移除 legacy |
| 工具 | Blender capture 默认使用个人绝对路径及 target 下的 shader 缓存 | 换机器或 clean cache 后不可重现 |
| 测试 | 缺 GPU/资产常返回成功；首次 Golden 可自动创建 | 绿色测试可能没有执行目标验证 |
| 发布 | 6 产品均 publish=false，path-only 内部依赖，许可证未定 | 独立开发与可发布需分开验收 |

主要依据：[根 Cargo 配置](../../Cargo.toml)、
[架构规范](../architecture/03-repository-and-crate-architecture.md)、
[legacy 来源](../history/legacy-provenance.md)。

## 2. 具体耦合清单

| 编号 | 位置 | 当前行为 | 需要替换的责任 |
|---|---|---|---|
| C01 | render 的 `tests/golden_odette.rs` | TENACITY_ROOT、固定客户端目录、DEF-spine.006、31.5 scale/24.5 offset | 按独立 asset/case manifest 驱动的真实角色测试 |
| C02 | render 的 `tests/golden_character.rs` | 合成用例与 Tenacity 角色用例混放 | 公共合成测试和独立外部角色 qualification 分开 |
| C03 | dev/odette-reference-lab 的 lib/tests | 读取 Tenacity 资产并使用 bevy_cel | 独立 reference lab 与显式资产包 |
| C04 | dev/reference-comparison 的 lib | 缺 Tenacity 贴图时所有槽返回 None、退化为参数对比 | 测试用例身份和所需输入不可静默改变 |
| C05 | Blender render_golden.py | 默认 Tenacity odette-npr 目录、固定版本 provenance | 声明式 source/toolchain/case 输入与实测 provenance |
| C06 | Cargo、CI feature matrix | legacy 在 members 和默认矩阵中 | 删除活动 legacy 及消费者后精确重建矩阵 |
| C07 | 文档、fixtures README、两个个人技能 | 仍用 Tenacity 制作流程作为当前可执行路径 | 当前操作以本项目自有工具为准；历史另存 |
| C08 | 新旧加工产物 | packed face maps、pupil composite 等并非原始 FBX/贴图重命名 | 从原始包或锁定上游 shader 独立生成并验证 |

进一步发现：

- `no_reference_deps.rs` 使用 `--no-deps`，逐个检查列出的包的直接 normal
  dependencies，名字虽称“closure”，没有遍历完整 resolve 图；遗漏 tools、build
  edges 和部分 dev 包黑名单。需要真正的角色白名单和传递图检查。
- render 的 dev-dependency 回指 facade；facade normal dependency 又指 render。
  Cargo 允许的测试关系不等于清晰的产品层边界，适宜迁移跨 crate 测试。
- tools 通过 gltf crate 引入 Bevy；纯文件校验和编译不应因工程组织而要求 GPU
  引擎链，须先拆分纯契约描述与 Bevy 注册适配，不能只切一个 feature 开关。
- `gpu-driven-spike/tests/spike.rs` 的 `../../target/tmp` 从其 manifest 目录解析
  到 crates/dev/target，非 workspace target；现有相对目录计数已经脆弱。
- `tools/check_docs.py` 递归扫描整个仓库，包括 gitignored target 中下载的第三方
  Markdown；此前已经因此发生无关文档报错。文档治理范围也必须独立、确定。

## 3. 资产和参考的实际可行路径

前一轮已从 HoyoToon 取回 Odette 的 53 个原始文件。FBX + 41 PNG 与本地二进制
相同，11 JSON 解析后相同，因此可用直接来源重建独立输入，不必读取另一个项目。
但是原始资产不含 Tenacity 的 19 动画、游戏服装删减、物理节点、运行时 pupil
合成结果及所有 packed face 产物；把旧 GLB 拷入本仓库不是完整解耦。

建议使用三类资产：

1. 自制合成 fixtures：代码和小型 golden 可随仓库分发，作为必需 CI。
2. 项目自制 skinned character fixture：覆盖真实 skin/morph/material/LOD 导入，
   不要求另一个项目的动画或模型。
3. 直接来源的 Genshin qualification pack：锁 ID、variant、文件哈希、插件/shader
   来源和独立制作 recipe；原始/派生第三方艺术资源保持 local-only。

当前 v3.6.4 新导入已知有材质 applier 警告、6 无效约束和 1 无效驱动；现有 bridge
因 Compositing Nodetree Image socket 不符拒绝导出。不能将新版本直接替换到
规范锁定文件，也不能承诺不加适配就重现角色。
详见[既有工作流调研](../history/workflow-research-2026-09-14.md)。

## 4. 验证体系的问题

[GPU workflow](../../.github/workflows/gpu.yml) 被 `if: false` 关闭。
[CPU workflow](../../.github/workflows/rust.yml) 执行 workspace 测试，但 GPU 用例
可 graceful skip；这不是 GPU qualification。合成 Golden 多数也放在被忽略的
fixtures/golden-local 下，并可能在基线不存在时写入新基线。

三层测试只断言产生三份报告；此前已实测所有视觉 pair 的 passed=false，但测试
本身通过。Production/upstream 的 PSNR 约 20.96 dB 是已知未达标结果，不应因
退出 legacy 就删除这些指标或放宽门槛。
[三层测试](../../labs/reference/tests/three_layer.rs)、
[Cel Golden](../../tests/integration/tests/golden_cel.rs)。

新的工程独立性门禁必须验证：真正拒绝访问 Tenacity；从空的项目缓存准备所有
必需自制资产；在要求 GPU 时缺 GPU 必须失败；基线缺失不能自动通过；可选外部包
没有准备时明确报告 NOT_REQUESTED/BLOCKED，不能占用已通过的用例数量。

## 5. 目录和架构选择的依据

保留一个 root workspace 和六个产品包，显式 default-members。Cargo 的默认包
选择与 --workspace 不同，不能通过 default-members 把 legacy 从默认选择中隐藏
后就宣称已经移除。[Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)

运行时 WGSL 留在所属 crate 的 src/shaders 下，有利于嵌入与 package 自包含；
没有理由为了匹配旧文档的理想树再复制一个根 shaders 目录。Rust examples 放在
facade/examples，由 Cargo 实际发现；多 crate 集成测试需要自己的 manifest。

发布包会移除 path 定位并按 version 解析内部依赖，`cargo package` 还会在打包后
构建验证。只看源码 workspace 编译不足以证明发行自包含；但现阶段 publish=false
和许可决策仍保留，不把独立化计划视为对外发布授权。
[Cargo Package](https://doc.rust-lang.org/cargo/commands/cargo-package.html)

feature 是 additive 的能力配置；需要独立 consumer 测试确认 minimal features，
不能让 dev-dependency feature 统一掩盖产品缺失。
[Cargo Features](https://doc.rust-lang.org/cargo/reference/features.html)

## 6. 结论

真正的独立化应同时完成资产生产链、参考来源、代码依赖图、工具路径解析、验证
状态语义、文档权威和分发边界的迁移。实施顺序必须保证独立替代与覆盖先到位，
再移除 legacy；历史来源和署名保留，不再承担活动构建或测试责任。
