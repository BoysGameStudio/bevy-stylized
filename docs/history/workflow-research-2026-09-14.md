# Blender、Gacha Setup 与 bevy_stylized 工作流调研

本次调研确认了从角色资源获取到 Blender 导入，再到运行时验证的实际边界。
HoyoToon 当前提供的 Odette 原始包与 Tenacity 本地原始资源内容对应；Gacha Setup
v3.6.4 可以执行完整导入并保存候选，但存在材质与骨架问题。现有
bevy_stylized 导出契约没有直接接受这个新候选，因此不能把工作流描述为已经
实现任意 Gacha 角色的一键无损转换。

据此交付两个个人技能：`blender-gacha` 负责资源、插件、Blender 创作与参考图；
`bevy-stylized` 负责导出契约、语义映射、运行时接入与验证。它们以可检查的文件和
证据交接，而不是重复实现同一套流程。

## 1. 调研范围与证据等级

调研日期为 2026-09-14。项目代码基于 `a47af6c`，本机 Blender 为 5.2.1 LTS，
已安装 Gacha Setup v3.6.4；项目 Reference lock 仍记录 v3.6.2。实际执行在独立
`target/tmp/skill-validation/` 中产生候选，不改变既有角色源文件、签名和 Golden。

证据分为：上游文档陈述、指定版本源码行为、实际资源内容比对、本机执行结果。
“未执行”“跳过”“仅生成报告”“指标未通过”保留各自含义。技能文件格式校验不能
替代角色导入或视觉验证；本次分别执行了相应检查。

主要来源是 [Gacha Setup 仓库](https://github.com/PaoloESAN/gacha-setup)、
[文档站](https://gacha-setup.pages.dev/) 和
[HoyoToon 角色资源目录](https://assets.hoyotoon.com/?game=gi&type=characters)。
运行时证据来自本仓库实际代码；本地角色加工历史来自 Tenacity 的源资源 README、
导出脚本与 Git 历史。

## 2. 资源来源：Odette 已有可验证的对应关系

Gacha Setup 的资源指引将 HoyoToon 和 hiragara 列为角色资产获取渠道。它们是资源
来源；Gacha Setup 是导入与着色工具，本身不附带游戏角色模型。
[资源指引](https://gacha-setup.pages.dev/character-models/)

HoyoToon 页面由前端加载目录。对页面公开脚本及其实际数据接口的检查得到：

| 字段 | 当前返回值 |
|---|---|
| 角色 ID | `10000150` |
| 英文名 / 中文名 | Odette / 奥黛塔 |
| 内部模型名 | `Avatar_Girl_Sword_Odette` |
| 变体路径 | `Characters/Odette/Snow Swan's Dance/` |
| 文件 channel | `live` |
| 文件组成 | 1 FBX、11 材质 JSON、41 贴图 PNG，共 53 文件 |

目录元数据中的外部 Nanoka 版本为 7.0；不能据此独立断言资源的游戏发布状态或
提取工具版本。文件接口本次未返回 activeRelease，因此保存了每项返回路径、URL、
etag、channel 与下载后 SHA-256。etag 作为缓存标识保存，没有将其当作 SHA-256。
[目录接口](https://assets.hoyotoon.com/api/catalog?game=gi&type=characters)、
[Odette 文件接口](https://assets.hoyotoon.com/api/remote-assets?game=gi&type=characters&ids=10000150)

实际下载全部 53 个文件，与
`Tenacity/assets-src/characters/odette/` 比较：

- FBX 和 41 张贴图全部逐字节一致。
- 11 份 JSON 字节不同，但解析后的结构和值全部相同。
- FBX SHA-256 为
  `278df9bd75c5fa9b82f90c1123926bdff10483dd85285e8c5becc16f2e5eb02a`。

这证明两份资源内容对应，不能反推原始下载者当时使用的网站。
Tenacity 在 `0155196c9f659b1fe06a14a7148675d663728398` 将原始包及
Gacha Setup v3.3.2 存档纳入源资源。来源记录现在可以提供一个经过验证的重新获取
渠道，而不必伪造最初下载历史。

接口是网站实现的观测结果，不是稳定 SDK 承诺。未来重新获取时应重新发现目录、
选定角色和变体，并跟随返回的下载链接；不能从角色名称推测文件 URL，也不能混合
不同 channel 或变体的文件。

## 3. Blender 导入：完整流程与实际问题

上游要求 Blender 5.2+，通过 Character Setup Wizard 选择游戏和角色目录。
独立的 Basic Setup 子步骤用于局部操作，不能把仅导入 FBX 当作完整材质与骨架配置。
[Quickstart](https://gacha-setup.pages.dev/quickstart/)、
[Genshin 导入说明](https://gacha-setup.pages.dev/genshin/setup-character/)

v3.6.4 的 `config_ui.json` 展示完整顺序：导入模型、清理、载入与替换材质、贴图、
描边、材质数据、变换、骨架、头部驱动、色彩管理、整理与后处理。目录导入采用递归
首个 FBX 匹配，因此应该建立仅包含角色 FBX、Materials、Textures 的输入目录，
或者明确选择角色 FBX。Tenacity 源目录内另有 Mixamo 动画，不能忽略这种歧义。
[版本化配置](https://github.com/PaoloESAN/gacha-setup/blob/v3.6.4/setup_wizard/config_ui.json)、
[模型导入实现](https://github.com/PaoloESAN/gacha-setup/blob/v3.6.4/setup_wizard/genshin_import_character_model.py)

实际试验使用正常用户配置，在后台独立进程中显式选择原始 FBX，开启完整 rigging，
关闭 hair/clothes physics，通过模型步骤的 UI continuation 完成全部流程。
最终保存 `Odette_v3.6.4.blend`，包含 221 个对象、215 个 mesh 对象、38 个材质和
一个 armature。大部分 mesh 对象是控制器，不是角色运行时网格。

重新打开候选后，只读检查得到：没有缺失图像文件、6 个无效约束、1 个无效驱动。
导入日志出现 V3 材质数据 applier 调用
`_V2_MaterialDataApplier__handle_material_value_not_found` 的异常，随后尝试旧版
applier 又遇到缺失节点。已下载 tag v3.6.4 对应源码，SHA-256 与安装文件一致，
确认不是本地私改产生的源码差异。
[材质数据实现](https://github.com/PaoloESAN/gacha-setup/blob/v3.6.4/setup_wizard/material_data_import_setup/material_data_applier.py)

这些证据说明“流程返回 FINISHED”不足以证明参数和骨架正确，尚不能断言所有警告
都影响最终画面。技能要求按角色、参数和活动约束进一步分类，而非自动忽略警告，
也不把关闭骨架或删除原始效果网格当作默认修复。

## 4. 参考渲染与灯光语义

上游灯光控制包含 Light Direction 对象、preset/custom palette、day/night、
shadow position 和 Fresnel。材质节点输入可能由 rig 驱动，读取未求值的 socket
默认值不能代表实际渲染颜色。参考清单必须记录这些控制，同时固定相机、姿态、
世界光、曝光、色彩变换、AA 和后处理。
[灯光文档](https://gacha-setup.pages.dev/genshin/lights-coloramp/)

本次使用统一脚本生成 512×512 的全身和脸部诊断图：Standard、exposure 0、
固定正面正交相机、Sun energy 3、关闭 compositor。它们只验证可见结构，不是
完整上游后处理复现或新的 Golden。新版候选脸部、头发和身体贴图可见，原始效果裙
出现明显不透明的花纹条带。旧 `Odette.blend` 使用同一诊断配置也有该现象，不能
将其直接归因为 v3.6.4 新回归。

Tenacity 游戏视觉版本刻意移除了 `Eff_001` 和 `Eff_002`，所以当前游戏模型与
原始角色外形不同。这样的历史修改要保留在案例记录中，不能变成通用导入规则。
骨架控制面板和毛发布料控制也不等同于运行时物理系统。
[骨架文档](https://gacha-setup.pages.dev/genshin/rigging/)

## 5. 导出边界：哪些数据还没有自动贯通

现有桥接器锁定 Blender 版本、节点签名和导出选项，并产生 GLB、manifest、
reference sidecar 与 signature。实际运行在新版候选上时，它因
`Compositing Nodetree` 缺少预期的 Image 输入而 fail-closed。这个结果证明当前
契约没有接受新导入候选；没有修改签名去绕过失败。
[桥接器](../../tools/blender/export_reference.py)

另外，当前选项是 `export_apply=true`、`use_selection=false`，且不限制
visible/renderable/active scene。单纯隐藏或选中对象不能构成运行时导出范围。
应在单独导出副本中剔除控制器和上游描边 modifier，检查 skin/morph 与修改器，
避免几何描边被烘焙后又由引擎描一次。
[导出设置](../../schemas/authoring/export-settings.v1.json)

四类格式必须分别解释：

| 格式 | 责任 |
|---|---|
| export-manifest-v1 | 版本、输入与输出、哈希、导出设置 |
| export-sidecar-v1.1 | 上游 reference 元数据和 Genshin ground truth |
| bevy-stylized.sidecar/v1 | `<asset>.npr.json` 中按材质名索引的运行时 descriptor |
| bevy_stylized.material/v1 | 单个材质的 typed extras/descriptor |

compiler 的 `bevy_stylized.compile/v1` 则用于纹理打包指令。这些不是可相互改名
替代的格式。需要显式决定 static material、instance state 和 view lighting 的
映射位置，并核实参数、纹理通道和颜色空间。
[运行时元数据](../../crates/bevy-stylized-gltf/src/metadata.rs)、
[编译器](../../crates/bevy-stylized-tools/src/compile.rs)

## 6. Bevy 接入：结构成功不等于视觉一致

对当前 Tenacity 游戏 GLB 的检查确认 5 meshes、8 materials、1 skin、19 animations、
8 embedded images。它没有 bevy_stylized typed material metadata，但项目 CLI
validate/compile 均可成功。compiler 生成默认 Cel descriptor 并提取颜色贴图，
不能因此宣称已转换完整 Genshin shader。

接入需要在 DefaultPlugins 的 GltfPlugin 创建之前注册 `_NPR_*`，然后等待场景
与资产就绪，明确把对应实体替换为运行时 NPR 材质。项目中的 `golden_odette`
和 `reference-comparison` 有具体 adoption 代码；这些是案例，而不是所有新 GLB
都会自动执行的转换。
[插件注册](../../crates/bevy-stylized-gltf/src/setup.rs)、
[Odette 测试](../../crates/bevy-stylized-render/tests/golden_odette.rs)

对 Genshin，ILM 的 R/G/B/A 分别承载 metallic、AO、specular suppression、material
ID；还需分别处理 ramp 行与日夜寻址、face SDF、packed normal、顶点控制色和
palette。MetalMap 不能直接按名字绑定为 MatCap；Blender 后处理、驱动和物理也
不能从 glTF 的存在性推断为已支持。
[行为契约](07-gacha-setup-behavior-contract-v1.md)、
[对比实现](../../labs/reference/src/lib.rs)

文档存在版本漂移：旧集成指南将 prepass 列为关闭，当前 primary material 代码
已经开启类型级 prepass，成本由相机 opt-in 控制；shadow 仍关闭。技能明确使用
当前源码和测试判断能力，避免把旧文档结论永久写死。
[Character material](../../crates/bevy-stylized-render/src/character.rs)

## 7. 已执行验证

| 检查 | 结果与实际含义 |
|---|---|
| HoyoToon 53 文件比对 | 42 个二进制文件相同；11 JSON 语义相同 |
| v3.6.4 完整导入 | 完成并保存候选；材质和骨架问题仍需分类 |
| 新进程 Blender audit | 图像路径无缺失；6 无效约束、1 无效驱动 |
| 新旧原始角色诊断渲染 | 已查看；效果裙问题两边均可见，不是视觉通过证明 |
| 新候选进入现有 export bridge | fail-closed，Compositing Nodetree 输入签名不符 |
| 旧游戏 GLB validate/compile | 通过；不构成完整 Genshin 语义转换 |
| Odette Cel GPU Golden | 全身、脸部 max delta 均为 0，匹配既有基线 |
| 三层 GPU 对比 | 报告生成通过；三对视觉判据全部不通过 |

三层对比中的 production/upstream 为 PSNR 20.96 dB、mean 0.0379、shadow IoU
0.8957，`passed=false`。比较使用 synthetic reference_scene，并非完整 Odette。
因此“测试命令通过”和“视觉指标通过”必须分开报告。
[三层测试](../../labs/reference/tests/three_layer.rs)、
[判据](08-golden-tolerance-and-cross-layer-criteria.md)

没有为技能文档运行无关的性能 sweep，也没有更新 Golden。性能工作流已说明相同
设备、分辨率、实例数、resolved tier 与视觉变化的比较要求，以及 skipped/missing
cells 不能冒充覆盖。

## 8. 技能结构与维护

`blender-gacha` 提供 source_manifest 和 inspect_blend 两个只读工具，并按任务
读取 Sources、Authoring、Handoff、Odette 资料。`bevy-stylized` 提供 inspect_asset，
并按需读取 Export、Runtime、Verification、Evidence。技能不捆绑第三方模型和
贴图，不硬编码个人项目位置，不把案例的服装裁剪、缩放和动画名称推广到其他角色。

交接内容是：来源清单、实际版本与选项、候选 Blender 工程和依赖、日志与检查结果、
参考图与捕获设置、目标运行时行为和已知缺口。它不是新设的运行时 schema。

已保留的适配任务包括：材质数据 applier 与活动骨架问题调查、真实角色 export
signature 的资格验证、reference ground truth 到 typed runtime metadata 的转换、
效果网格及 Metallic 等缺失语义。技能能准确定位并组织这些任务；它的完成不意味着
这些渲染器能力已经实现。

## 来源索引

所有在线来源在 2026-09-14 查阅。运行时文件引用对应本次工作区代码。

1. PaoloESAN，Gacha Setup [仓库](https://github.com/PaoloESAN/gacha-setup) 与
   [v3.6.4 发布](https://github.com/PaoloESAN/gacha-setup/releases/tag/v3.6.4)。
2. Gacha Setup，[Quickstart](https://gacha-setup.pages.dev/quickstart/)、
   [Character Assets](https://gacha-setup.pages.dev/character-models/)、
   [Genshin Setup](https://gacha-setup.pages.dev/genshin/setup-character/)。
3. Gacha Setup，[Lights & Coloramp](https://gacha-setup.pages.dev/genshin/lights-coloramp/)、
   [Rigging](https://gacha-setup.pages.dev/genshin/rigging/)、
   [Bugs & Solutions](https://gacha-setup.pages.dev/genshin/bugs-solutions/)。后者当时为 Coming soon。
4. Gacha Setup，[Changelog](https://gacha-setup.pages.dev/changelog/)。页面当时仍称
   v3.4.0 为 Latest；实际版本判断使用 releases 和已安装包。
5. HoyoToon，[Genshin Characters](https://assets.hoyotoon.com/?game=gi&type=characters)、
   [公开 catalogue](https://assets.hoyotoon.com/api/catalog?game=gi&type=characters)、
   [Odette files](https://assets.hoyotoon.com/api/remote-assets?game=gi&type=characters&ids=10000150)。
6. Tenacity，本地 `assets-src/characters/odette/README.md`、原始包、
   `assets-src/blender/{build_odette_game,export_odette}.py` 及上述入库提交。
