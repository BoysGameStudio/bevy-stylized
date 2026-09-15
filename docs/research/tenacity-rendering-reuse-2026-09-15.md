# Tenacity 渲染工具复用调研 — 2026-09-15

结论：有值得迁移的内容，优先是交互式角色检查工具和截图生命周期处理。
迁移单位应是经过抽离的功能与回归用例，不能整包引入 `view-ink`、`bevy_cel`
或游戏客户端。本项目仍只负责风格化渲染，Gacha Setup 只验收当前最新稳定版。

本次只读检查相邻仓库源码、依赖、测试和相关设计文档，未运行 Tenacity、未执行
其测试、未复制角色资产或修改任何运行时代码。以下“存在”指源码中有实现，
不等于已经在 bevy-stylized 验证可用。源码身份见
[调研清单](../quality/tenacity-reuse-inventory-2026-09-15.json)。

## 候选与迁移判断

表中源路径相对 Tenacity 根目录；它们是调研来源，不是构建输入。

| 候选 | 源码与实际行为 | 本项目落点／判断 |
|---|---|---|
| 轨道相机与取景 | `crates/view-ink/src/showcase/viewport.rs`：`camera_pose`、`input`、`sync_camera_pose`，全身／上身／脸部取景、旋转、缩放、平移 | **P0 改造迁入** `apps/showcase`；删除 `pose_plan`、移动轨迹、演员朝向和 58 单位默认身高，使用米制资产边界与显式相机状态 |
| 双视口与联动相机 | 同文件：`StageImages`、`StageCamera`、`route_instances`，离屏纹理与 RenderLayers 隔离 | **P0 借鉴实现**；A/B 相机可联动，显示当前方案与保存的检查配置；不得把任意 B 视图称为 Blender 真值或已接受基线 |
| 光照工作台 | `showcase/environment.rs`：共享光方向、Gacha 调试模式；`showcase/panel.rs`：工作台操作界面 | **P0 抽离**；UI 写入本项目 View／Material／Instance 契约，移除 `InkCamera`、`OdetteMaterialOwner` 等游戏绑定 |
| 截图重载竞态 | `showcase/capture.rs`：`invalidate_scene`、`captured`；只接受当前请求实体回调，重载时取消消费者而不删除渲染器尚在使用的截图实体 | **P0 迁移行为与测试**；特别参考 `replacing_scene_retires_capture_but_preserves_the_renderers_in_flight_entity`，适合新交互检查器 |
| 可保存检查配置 | `showcase/state.rs::Recipe` 与 `capture.rs`：相机、外观、身份、构建记录、PNG／APNG 输出 | **P0 缩减后迁入**；保存资产、相机、光照、渲染设置、调试模式和 SHA-256，删除动画 clip/frame/speed、布料调试字段；首阶段不做录像时间线 |
| 加载与管线就绪 | `showcase/readiness.rs`：RenderApp 统计待编译管线、错误和实际渲染帧 | **复用本项目现有实现，参考其 UI 状态反馈**；`labs/reference/src/character.rs` 已有资源、材质采用、瞬时／致命管线错误、连续就绪与超时检查，不再复制一套弱化门禁 |
| 更丰富材质诊断 | `crates/bevy-cel/src/genshin_lighting.rs`：SourceLit、RampCoordinate、RampBand、FaceFactor、PaletteMultiplier、SourceNormal | **P1 按缺口补充**当前 Hero 调试视图；先接已有 Lit/Sharp、BaseColor、PrePalette、NdotH，再添加确有定位用途的通道。算法继续以当前上游证据为准 |
| 描边同步契约 | `bevy-cel/src/outline_runtime.rs` 与 `tests/outline_contract.rs::outline_tracks_replaced_source_mesh_and_dynamic_bounds`：源 mesh、AABB、morph 同步 | **P1 对照测试覆盖**；本项目 `outline.rs` 已有副本、蒙皮和 morph 同步，不整体替换。只补证实缺少的边界：源网格替换、动态包围盒、移除和清理 |
| 面部朝向输入 | `bevy-cel/src/face_runtime.rs::FaceSdfRig`、`FaceSdfMaterialInstance`：从外部头部变换获得面部世界基准 | **P1 参考契约**；源实现按角色克隆材质，与本项目共享材质／实例数据方向不同，不能照搬。只消费外部姿态，不接管动画或表情 |
| 表现层 BRP | `crates/bevy-brp-kit/src/lib.rs::add_presentation_brp_remote`：RemotePlugin、HTTP、Transform 注册 | **P1 可选开发入口**；直接用 Bevy Remote 实现小型 app 适配，不能依赖该 crate，因为其 Cargo 和模块仍依赖 `shared::GameSim` |
| 全屏后处理 | `bevy-cel/src/post_process.rs`、`post_process_runtime.rs`：Bloom → 风格化屏幕效果 → 显示变换 | **P2 单独评估**；有渲染参考价值，但不能将其 bloom、暗角、畸变默认值带入刚通过的颜色对照。新增效果需独立开关、证据和 GPU 成本检查 |

## 明确不能直接复制的部分

- `ShowcaseAsset` 固定 `models/odette.glb`、`odette-npr` 和九个贴图文件名；迁移后必须
  使用显式输入包和 descriptor 纹理清单，不能恢复相邻项目资产依赖。
- `measure_bounds` 只查询带 `SkinnedMesh` 的网格，且依赖名为 `Face` 的实体；当前
  独立静态资产也必须正确取景。应同时支持静态与蒙皮网格、显式语义焦点，并区分
  初始取景边界与动态剔除边界。参考捕获仍使用声明的固定相机，不能被自动取景覆盖。
- `camera_pose` 直接调用播放计划。只能抽离轨道数学和交互，不能带入动作轨迹。
- `genshin_lighting.rs` 同时包含纯调色参数和 Lightyear 时间线、关卡潮汐／波次逻辑；
  可参考参数分层与预设交互，不能复制整个插件或未经当前 Gacha 验证的数值。
- `bevy_cel` 是另一套材质／着色器体系，虽然双方 Cargo 都固定 Bevy 0.19.1，接口、
  属性、实例传输和上游映射仍不同。不会用它替换刚通过验证的产品渲染路径。
- `bevy-autotest` 主要驱动键盘／ActionState、固定 tick 和游戏就绪人数，依赖
  leafwing-input-manager；不作为渲染测试框架迁入。
- 动画沙盒、角色控制、重定向、Kawaii／布料求解、碰撞修补和游戏 FBX 导出约定均不迁入。
  外部 skin/morph 的渲染兼容仍可以用本项目的小型自制 fixture 验证。
- `gltf_viewer.rs` 写死 fighter/arena/dummy 与第 120 帧截图、150 帧退出；不如当前
  readiness 机制，不适合作为新检查器基础。

## 对已有材料的审慎判断

`bevy-view-kit` 主要是游戏 UI、血条、飘字、粒子和震屏，并非现成轨道相机库。
本次检查中未找到比本项目现有 benchmark／GPU harness 更明确的独立渲染性能入口，
因此没有建议迁移整套 Tenacity 自动化。此结论限于已检查路径，不声称全仓库不存在。

BSN 光照工作台设计文档可参考交互目标，但本次源码检索未找到文档中同名
`GachaLightingLabRoot`／`GachaPresetAnchor`；不能把文档的“完成”直接视为当前可复制实现。
`bevy_cel` README 标明源于 MIT 的 `bevy_cel_game`；正式复制代码前应按选中文件保留
原作者和来源记录，第三方角色／纹理仍走现有原始来源构建，不复制 Tenacity 衍生资产。

## 建议实施顺序与验收

**第一批：现有 Showcase 增加静态角色检查模式。**

建议新增 `apps/showcase/src/inspector/`，按 asset、camera、lighting、panel、capture
划分内部模块；不新增第二个独立 viewer 应用，也不增加新的产品 crate。
CLI 显式接收编译资产／检查配置，使用现有 facade 的材质接入。
可选 UI 依赖限于开发应用；产品 crate 不依赖 egui、Tenacity 或游戏协议。

完成条件：加载当前独立构建的 Odette、全身／脸部取景、轨道／缩放／平移、可重置
光照、已有材质诊断、联动 A/B、带身份的截图与配置导入导出。重载期间不得输出过期
截图；缺 GPU、贴图缺失、采用失败和管线错误必须明确反馈。
固定参考相机及现有六组图像门禁保持不变。

**第二批：实时稳定性与针对性渲染契约。**

用常规分辨率／AA运行相机、光方向与根变换扫描，检查高光、脸部、透明与描边的时间
稳定性。按缺口补充诊断视图和描边／外部面部基准测试。性能数据沿用本项目的 harness，
记录 GPU、分辨率、AA、预热、采样帧、GPU 时间、draw calls 和显存；不能用离线
超采样对照通过来替代实时性能验收。

**第三批：第二角色与可选后处理。**

先用第二个角色验证材质语义与焦点配置不依赖 Odette 名称，再独立评估新的渲染效果。
全部上游验收面向届时最新稳定版；历史版本不增加兼容义务。

实际迁移时，每批记录源文件与摘要、抽离项、目标文件和回归证据。去掉 Tenacity
目录、断网并从原始资产重建后仍须运行。本次交付仅为调研与迁移建议，未执行上述实施。
