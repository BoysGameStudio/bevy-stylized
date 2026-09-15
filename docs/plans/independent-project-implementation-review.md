# Independent project implementation review — 2026-09-14

本轮审查独立化后的渲染输入同步、材质资产生命周期、截图基础设施、构建缓存和
测试退役记录。遵守项目只负责风格化渲染的边界；没有新增动画或表情控制系统，
没有提交 Git，也没有接受或覆盖 Golden 基线。

## Findings and changes

| 严重度 | 发现及影响 | 修复与证据 |
|---|---|---|
| High | Hull 副本只在生成时复制数据；更换 mesh/材质或修改独立 morph 权重后仍使用旧值；移除 mesh 后副本残留 | 持续同步变更的 mesh、材质、skin、morph；删除失效组件与副本；mesh 更换时使副本旧 Aabb 失效。CPU 测试修复前失败、修复后通过；GPU 测试在同一实体上驱动 0→1→0 权重并验证描边位移和画面恢复 |
| Medium | 截图 observer 对延迟到达的重复回读再次销毁请求实体，并可能重新插入已消费的数据 | 跟踪 pending 请求，只接受首个完成事件；重复事件不再修改 inbox。新增首帧保留及消费后重复事件测试；GPU 日志不再出现 Entity despawned 警告 |
| High | 隔离验证与主工作区共用 Cargo target，缓存测试二进制含隔离环境的 CARGO_MANIFEST_DIR，回到宿主后测试找不到 Cargo.toml | 隔离运行改用 target/isolated-cargo；清理宿主的项目 package 构建产物并重建，CPU 检查恢复通过。此次没有重新执行完整空缓存隔离流程，历史隔离资格只适用于历史快照 |
| Medium | 角色构建 key 漏记纯 glTF 编译器源码及 Cargo manifests；reference key 漏记其 add-on manifest | 纳入当前四个本地编译依赖 crate 的 Rust 源码和 manifests，以及 root Cargo.toml；reference 加入 add-on manifest。新增 glTF 合同及依赖配置变更使缓存输入变化的 Python 测试 |

Hull 同步在引擎 AnimationSystems 后、transform propagation/bounds 计算前运行。
它读取外部已经求值的数据，不选择动画或生成权重。mesh 更换后的 Aabb 失效处理
不等价于解决所有动态变形边界、描边扩张 bounds 或 LOD 问题；ADR-0008 仍 Proposed。

## Material lifecycle and coverage review

新增无 GPU 的真实材质接管测试验证：两个根实体共享材质 handle；删除一个根后
另一个继续持有材质；最后一个使用者消失后缓存为空，资产生命周期处理后材质释放。
GPU 导入测试继续覆盖实际 glTF 接管及重新加载。MaterialSet 明确为一次性场景接管
配置；修改已接管根的配置不会自动重建材质，应用应重建场景。动态实例参数另有归属。

194 条历史记录分为：54 preserved、2 preserved-plus-independent-character、
2 replaced-independent-source-case、1 replaced-independent-upstream，以及 135 条
retired-reference-implementation。后者引用相关文件不构成逐项语义等价证据。
`independence-coverage.json` 已增加统一说明，并为本轮发现的 outline 漏项记录具体测试。
不能将 194 条处置记录解释为 194 项现行通过测试，也不能据此宣称完整回归覆盖。

本轮重点核查的剩余边界包括：动态 head basis、face 无效输入与数据纹理约束、
上游五类 outline 语义、跨 pass 与动态 bounds；它们仍需各自的针对性证据。
静态角色配方不包含完整 rig、物理、效果裙或所有上游材质功能。

## Static reference readiness

检查了已验证构建的 authoring scene、导出逻辑和 capture metadata：

- 当前 Blender 相机是正交，Bevy 资产接入截图使用透视自动取景，不能直接作像素验收。
- 导出在 Blender 世界坐标中求值静态网格，再转换为 glTF Y-up；匹配相机应明确
  使用 `(x, y, z) -> (x, z, -y)`，保留相同 frame、范围和比例。
- Blender 记录的 world 是线性颜色，而接入测试的 ClearColor 使用 sRGB 构造，
  相同数值不代表相同背景亮度。
- 参考记录只有新增 Sun 的 rotation/energy，漏记 Gacha 的 Light Direction
  控制对象和最终求值的 lightDir。检查发现场景有独立的 Light Direction empty，
  Light Vectors 节点组通过 Object Info Rotation 传递该输入。
  不能把 Sun 的角度直接当作 NPR key-light direction。
- Ambient、palette 和曝光各有归属。还应补齐实际显示转换/gamma与 shader 光照输入
  的求值证据，不能仅凭参数名字或 socket 默认值认定相等。

因此此次没有把现有角色截图包装成同条件视觉验收。下一项渲染工作应先建立可校验
的共享 capture case：记录作者场景、runtime GLB 和纹理身份，相机、背景、曝光、
Gacha 光照求值与 Bevy view 映射；先验证这些输入一致，再输出 full-body/face 对比。
这项工作不需要引入动画系统。已有上游 parity FAIL 不变。

## Validation

本轮实际执行的机器可读汇总见
[review validation](../quality/implementation-review-validation-2026-09-14.json)。

- CPU：160 Rust 测试通过，25 GPU/环境测试明确 ignored；7 Python 测试通过。
- GPU：12 required tests 通过，自制角色导入 1 项通过。
- fmt、workspace/all-targets/all-features Clippy（-D warnings）、文档和差异空白检查。
- Odette 接管/共享/卸载/重载使用已验证的静态构建独立复测；不声明上游视觉等价。

完整 Git-free/empty-cache 隔离验收未在本轮重新运行；不可用之前的报告替代当前
修改后快照的隔离证据。GPU 退出阶段仍可出现回读通道关闭提示，多应用测试仍有
重复日志初始化提示；本轮修复的是重复事件导致的 ECS 实体销毁错误，不屏蔽日志。
