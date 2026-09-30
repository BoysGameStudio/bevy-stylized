# Showcase 庭院 UI/交互重设计（提案）

- **Status: Accepted**（2026-09-29 起案 Proposed；2026-09-30 owner 拍板接受，
  U1 动工。回填 docs/showcase/05-courtyard-scene-design.md §11 的设计锁定注记
  同步落笔；本文档不改变 docs/adr/0031-single-courtyard-scene.md 的 Status）
- **v3 注记（2026-09-30，owner 展亭模式决策）**：minimap、场景内点击聚焦、
  overview/Tab 随 v3 退役（同一位置无空间导航——展位模式下同位必穿模）；
  本文档的 §4.2/§5 点击行与 U2 相关行是 dated 落地记录，不改写。dock、
  参数抽屉、toast、补间、演示/导览/捕获在 v3 下原样成立。
- 范围：courtyard 交互窗口（`apps/showcase` interactive feature）的 UI 与交互。
  浏览器交互 V2 / Android 交互 V3 仍处于冻结分期，本文档只做映射设计，不启动实现。

## 0. 现状盘点（2026-09-29 代码核对；2026-09-30 二轮复核更新行号与键位）

> 本表是 U1–U3 落地**前**的现状基线（dated，不改写）；落地状态见 §13
> 与 05 §12 U1–U3 行。

| 维度 | 现状 | 出处 |
|---|---|---|
| UI 壳 | 三个 egui 浮动窗：左下 Status（等宽单行状态 + outline 分段文本）、右上 Legend（9 区静态说明列表）、左上 Keys（默认折叠的键位表） | `apps/showcase/src/lib.rs:546/583/610` |
| 输入 | **纯键盘**：←/→ 分区循环、Tab overview、↑/↓ 时刻、O/P、1/2/3 crowd、Space flags、N stage_character 翻转（V2-5）、R reset、`,`/`.` case 导航；**庭院窗口无任何鼠标处理**（无 orbit/pan/zoom/点击，全 crate 无 MouseButton 路径） | `apps/showcase/src/keymap.rs`、`apps/showcase/src/lib.rs` |
| 反馈 | 命令结果只进 stderr（`[interactive] applied/refused: …`），UI 内不可见 | `apps/showcase/src/lib.rs:501-509` |
| 相机 | 切分区 = 机位瞬跳；窗口相机每帧从控制器相机状态拷贝 Transform（相机统一已成立） | `apps/showcase/src/lib.rs:519-522` |
| overview | Tab 从 overview 返回时固定回 trio，无「上一分区」记忆（V1 简化，已在 keymap 注明） | `apps/showcase/src/keymap.rs:103-117` |
| Capture | C 键暂缓：driver 无 ad-hoc observe 通道 | `apps/showcase/src/keymap.rs:10-11`；05 §11.1 |
| 鼠标手势参照 | Inspector 已有完整 orbit/pan/zoom/双击 reset（拖拽在渲染图上直接生效）；05 §11.3 第二条「上移共享层」是已记录的前置缺口 | `apps/showcase/src/inspector/panel.rs:167-201` |
| 点击聚焦 | 未接。05 §11.3 已记前置缺口 A4：共享层无 mesh 射线检测；分区展品都是球/盒，手写 ray-primitive 命中成本可控，不需要引入 bevy_picking | 05 §11.3 |
| 参数面板 | §11.4 最小切片已落地（静态声明 + tier 置灰）；schema 驱动控件生成未落地 | 05 §11.4 |
| 其他驱动源 | BRP 自定义方法 `showcase/session_command` 与键位同一 SessionCommand 通道（bevy-copilot 可驱动） | `apps/showcase/src/lib.rs:449-458` |

## 1. 问题诊断（「单调」的具体构成）

1. **UI 是纯文本堆叠**：三个窗口全是文字列表，分区之间无空间关系呈现——
   庭院是有明确方位的物理场景（05 §4 布局），UI 却把它拍平成九行文字。
2. **交互通道只有键盘**：展示场景的主角是画面本身，但用户无法用鼠标直接
   探索（orbit/zoom 是检视类工具的基本手势，Inspector 已有而庭院没有）。
3. **命令反馈不可见**：拒绝原因（tier 不足、未知参数）只打 stderr；
   窗口用户看不到任何成功/失败确认。
4. **分区切换无过渡**：机位瞬跳 + 展品瞬时生灭，用户无法感知「从哪到哪」，
   分区在庭院里的方位关系完全靠脑补。
5. **展示场景没有展示模式**：截图/演示时 UI 无法隐藏，也没有自动巡场。

## 2. 设计目标与约束

目标：庭院交互窗口从「验证驱动的键位面板」升级为「可演示的 NPR 展台」——
验证语义（命令层、参数校验、tier 门控）一项不减，体验层补齐空间导航、
直接操纵、可见反馈、演示能力。

约束（继承现行合同，不重新讨论）：

- **命令层单通道**：UI/键位/BRP 都只是 SessionCommand 的驱动源
  （docs/showcase/01-showcase-examples-fixtures.md:426-443；05 §11.7）。
  本设计不为 UI 开任何旁路。
- **UI 参数从目录派生**：控件元数据单一源 = catalog 参数 schema（05 §8/§11.4）。
- **严格捕获与交互画面解耦**（01:443）：交互层的相机动画/分辨率/UI 显隐
  不影响 golden 捕获路径（固定像素 target）。
- **无新重依赖**：UI 仍用 bevy_egui（已引入）；minimap 用 egui Painter 自绘；
  点击聚焦手写 ray-primitive（05 §11.3 A4 结论）；不引入 bevy_picking 或其他
  UI 框架。
- **相机统一不破坏**（05 §11.3 第一条）：窗口相机永远从控制器相机状态派生，
  相机过渡动画在控制器相机状态内实现，不另起第二相机。

## 3. 信息架构与布局

画布优先：UI 全部退到边缘，中央无遮挡；所有面板可隐藏（§8 演示模式）。

```
┌────────────────────────────────────────────────────────┐
│ Bevy Stylized · courtyard · gen 12 · tier: Standard     │ 顶栏（单行）
│┌─────────┐                                  ┌────────┐ │
││ 键位帮助 │                                  │minimap │ │ 左上折叠 / 右上常驻
││ (折叠)   │                                  │ §4.2   │ │
│└─────────┘                                  └────────┘ │
│                                            ┌──────────┐│
│              场 景 画 布                    │ 参数面板  ││ 右侧抽屉（§6，
│                                            │ （抽屉）  ││  可收起）
│                                            └──────────┘│
│                                          ┌─ toast 堆叠 ││ 右下（§7）
│ zone p2 · outline · dusk · crowd 100 …   └────────────┘│ 左下状态条（沿用）
├────────────────────────────────────────────────────────┤
│ [trio] [P1] [P2] [P3] [P4] [P5] [LOD] [stage] [lamps] │ 底部分区 dock（§4.1）
└────────────────────────────────────────────────────────┘
```

与现状的对应关系：Status 窗口保留（左下，内容不变）；Legend 窗口退役——
分区说明由 dock 芯片的 hover 提示与 minimap 承担；Keys 窗口保留（折叠默认不变）。

## 4. 分区导航

### 4.1 分区 dock（底部常驻）

9 个分区芯片按 05 §11.6 循环序横排（trio → p1 … → lamps）+ 右端 overview 开关。
每个芯片：分区名 + 一句话说明（现 Legend 文案下移为 tooltip）+ 激活态高亮。
空间方位表达归 minimap（§4.2），芯片不标注罗盘方位——lamps 是半环、LOD 是
轴、stage 是点，九区没有统一的「方位」可标，硬标只会误导。

- 点击芯片 = `SetParameter(zone, …)`，与 ←/→ 完全同命令；
- Tab overview 行为修正：会话记忆上一分区，从 overview 返回时回到
  **上一分区**而非固定 trio（消除 keymap.rs:103-117 的 V1 简化注记）；
- 键盘循环序不变（既有键位表与单测不动）。

### 4.2 minimap（右上常驻，egui Painter 自绘）

庭院俯视示意图：北朝上（−z 向上），分区点位由既有单一源派生
（`labs/scenes/src/metadata.rs` 的 `COURTYARD_REGION_AABBS`——trio/p1–p5/lamps/
stage 八区 AABB——加 `COURTYARD_LOD_BALLS` 四球；UI 侧只做
缩放投影，不复制坐标数据）：

- 点状分区（trio/P1–P5/stage）取 AABB 中心投影为圆点；**lamps 不取 AABB
  中心**——其 AABB（min(−11.12,0,−11.12) ~ max(11.12,1.84,0)）覆盖整个
  北半场，中心 (0,·,−5.56) 落在环内空地而非灯阵上，取中心是误导。lamps
  画为北半环弧线（半径取 AABB 外接尺度，弧线本身即命中区）；LOD 轴画
  四球连线（near (−1.2,·,9.4) → very_far (−8,·,−44)，远点超出图幅时轴以
  折线箭头示意出界方向）；
- 激活分区高亮 + 当前机位视锥朝向楔形（数据源：DesktopSession.last_camera_view，
  与状态条同源）；点击分区图形 = 聚焦该分区（同 dock 命令），命中判定按
  图形几何（点=半径阈值，弧=带阈值），不做像素级精确；
- overview 模式下全部点亮，明示「同屏浏览，无判定职责」。

## 5. 场景内直接交互（鼠标）

手势集与 Inspector 完全一致（用户跨窗口零学习成本），经 05 §11.3 第二条
的共享 orbit 相机上移落地：

| 手势 | 命令 | 说明 |
|---|---|---|
| 左键拖拽（空白处） | `CameraOrbit { yaw, pitch }` | 在当前机位上偏移（01:429 自由操作产生新 revision） |
| 右键拖拽 | `CameraPan { dx, dy }` | 同上 |
| 滚轮 | `CameraZoom { factor }` | 距离指数缩放，沿用 Inspector 限幅 |
| 左键点击展品 | `SetParameter(zone, …)` | 射线命中分区展品 → 聚焦该分区（05 §11.3 第三条）；未命中不动作 |
| 双击 | `ResetView` | 回当前分区配方机位 |

点击聚焦实现：手写 ray-primitive（05 §11.3 A4 结论），但**命中图元不是
分区 AABB**——lamps 的分区 AABB 覆盖整个北半场（含 stage/p5/trio 北半），
AABB 命中会大面积误触发。命中图元按展品本体定义（builder 生成展品时
已知道每个球/展台/灯柱的几何，随展品注册命中 primitive：球/圆柱/盒）；
lamps 命中到最近灯柱实例，stage 命中角色台基座。未命中任何展品不动作。
egui 输入门控沿用 `EguiWantsInput` 既有模式——指针在面板上时场景手势
不触发。手势翻译层（winit 鼠标事件 → 命令）与键位翻译层同构，用合成
输入事件单测，不依赖物理鼠标。点击与拖拽以按下→抬起的指针位移阈值
区分（翻译层内判定，合成事件可测）；从展品上起拖同样按 orbit 处理，
聚焦只认阈值内的点击。

悬停反馈（U2 后段，可选）：指针悬停展品时分区轮廓高亮。依赖同一射线
通道；若视觉噪声过大则降级为仅 minimap/dock 联动高亮，设计评审时定。

## 6. 参数面板（schema 驱动，落地 05 §11.4 完整态）

右侧抽屉，控件从 catalog 参数 schema 自动生成（05 §8 的 ParameterKind 单一源）：

| ParameterKind | 控件 |
|---|---|
| Enum | 分段按钮（值 ≤4）/ 下拉（>4） |
| F32 | 滑杆（范围取自 schema） |
| U32 | 步进滑杆 + 快捷预设芯片（§16 D5：catalog schema 增可选 `presets` 元数据；crowd 渲染 0/100/1000 锚点档） |
| OptionalF32 | 滑杆 + 「自动」勾选（null） |
| Bool | 开关 |

- tier 门控：hybrid 不足 Standard 时控件置灰 + 结构化原因（沿用现有
  RefusalReason 文案生成）；门禁判定与状态条同一 caps 数据源；
- 控件操作只发 `SetParameter`；校验失败走 §7 toast，控件回弹到当前值；
- 布局分组：Scene（zone、time_of_day）/ Rendering（outline_mode、post）/
  Content（crowd、flags、face_threshold、stage_character）——分组声明进
  catalog 控件元数据，三平台共用（§11 映射）。

## 7. 反馈系统（命令结果可见化）

现状：`CommandOutcome` 只进 stderr（lib.rs:501-509）。设计：

- **toast 堆叠**（右下，自动消退）：Applied → 一行确认（2s 消退）；
  Switched → 「zone p2 · outline」（含新机位名，2s——`Switched` 只携带
  case+generation，机位名从 `DesktopSession.last_camera_view.preset`
  读，与状态条同源缓存，lib.rs:521/550）；Refused → 琥珀色、
  结构化原因全文、驻留至下次命令或点击关闭。tier 拒绝附「如何解锁」
  提示（需 Standard tier 会话）；
- **构建指示**：内容类参数（zone/crowd 等）经 `apply_parameter` 同步触发
  driver 的 retire→build（controller.rs:705-716），构建本身由 driver 动作图
  跨帧推进——指示器不由「命令发出→就绪」计时（该窗口可能为零帧，那样
  指示器就是死 UI），而是直接反映 driver 状态：`DriverStatus::Running`
  期间状态条显示「building · gen N」，`Done`/回到稳态即消失。耗时如实
  呈现，不做假流畅；
- **顶栏 tier 徽章**：resolved tier 常驻显示（Standard/Portable），
  与参数面板置灰同源。

## 8. 相机过渡与演示能力

- **机位过渡**：切分区时机位从当前 view 补间到目标 preset（~0.4s ease
  in-out，eye/look 球面插值）。实现在控制器相机状态内（窗口相机照常每帧
  读控制器状态，相机统一不破坏）；过渡期间到达的导航命令抢占当前过渡。
  补间参数进配方（可关：`camera_transition = off`，捕获/冒烟路径默认关，
  交互窗口默认开）。**确定性口径**：补间是纯视觉插值，不进 CommandJournal、
  不产生 revision（revision 仍只由命令产生）；BRP 冒烟/截图断言必须显式
  钉 `camera_transition = off`，否则断言可能捕到补间中间态而抖动；
- **演示模式（H）**：一键隐藏全部 UI（dock/minimap/面板/状态条/toast），
  纯净画布供截图与展示；Esc 或 H 恢复。隐藏状态不进配方、不影响捕获；
- **导览模式（G）**：脚本化巡场——按循环序逐分区驻留（默认每区 4s，
  时刻/描边等参数不变），每条命令照常走 SessionCommand 通道；录制经
  既有 `command_recorded` 入口进 CommandJournal（journal.rs:172，01:429
  录制语义不变）——交互宿主当前命令循环不持有 journal（lib.rs:501 直调
  `controller.command`），U3 含宿主接线（命令循环换 `command_recorded`
  + 会话级 journal）；任意导航输入退出导览。
  overview 收尾停留。导览脚本是数据（分区序 + 驻留时长），不写死进代码。

## 9. 捕获工作流（C 键落地的前置设计）

现状暂缓原因：driver 无 ad-hoc observe 通道（keymap.rs:10-11）。**（U3 已于
2026-09-30 落地：`SessionCommand::Capture` + `CaseDriver::observe_adhoc`，
见 §13 与 05 §12 U3 行。）** 设计：

- 命令形态沿用 05 §11.1 既有定义：`Capture` 是 SessionCommand 变体
  （UI/键位/BRP 同通道），不是 UI 旁路；其内容语义 = driver 新增 observe
  入口（交互会话内对当前窗口内容触发一次捕获，与严格捕获的固定像素
  target 路径分离；落盘到会话 run 输出目录，文件名带捕获规格与时刻戳）；
- UI：C 键 + 参数面板「Capture」按钮；成功后 toast 显示输出路径；
- 交互捕获**不参与 golden 判定**（判定只走 lane 固定路径），toast 文案
  明示「浏览快照，非判定证据」。

## 10. 视觉语言

- 沿用 egui dark 基底（现状 `Visuals::dark()`），定制范围限于：强调色
  （激活分区/激活控件）、toast 语义色（确认绿/拒绝琥珀）、minimap 线条色——
  取自 05 §4 配色表的庭院主色族，UI 不引入画面外的色相；
- 状态条保持等宽单行（现状形态被验证有效，不动）；
- 面板统一半透明深色底 + 1px 分隔线，圆角关闭（与渲染画面的硬边 NPR
  语言一致）；字号两档（正文/小注），不引入新字体资产。

## 11. 跨平台映射（仅设计；V2/V3 实现仍冻结）

控件元数据（分组 + ParameterKind + tier 门控 + 文案）单一源，三平台壳不同：

| 平台 | dock/minimap | 参数面板 | 输入适配器 |
|---|---|---|---|
| PC | 底部 dock + 右上 minimap（本文） | 右侧抽屉 | 键盘 + 鼠标（§5） |
| Browser（V2，冻结） | 页面顶部条 + SVG minimap，同一元数据 | HTML 侧栏，同一控件声明 | 鼠标/键盘 + 页面控件（05 §11.5） |
| Android（V3，冻结） | 底部横滑 chip 条，minimap 并入折叠页 | 折叠面板，同一控件声明 | 单指 orbit/双指 pinch·pan/双击 reset/点按聚焦（05 §11.5） |

一致性合同不变（05 §11.7）：同一命令序列在任意平台产生同一画面语义。

## 12. 与现行合同的关系

逐条对照 docs/showcase/01-showcase-examples-fixtures.md:426-443：

| 01 合同 | 本设计落实 |
|---|---|
| 输入映射成共同动作，鼠标/触摸只是适配器 | §5 手势全部翻译为 SessionCommand 变体；唯一新增变体是 05 §11.1 已定义的 `Capture`（内容语义为 driver observe 入口，§9），无合同外命令 |
| UI 参数来自目录 | §6 schema 驱动；分组声明进 catalog |
| 三平台布局不同、语义同源 | §11 映射表 |
| 横竖屏/DPI 只影响显示布局 | minimap/dock 为相对锚定布局；窄窗时 dock 折行、minimap 让位参数面板 |
| 自由操作产生新 revision | orbit/pan/zoom 经控制器相机状态、照常产生 revision；机位补间是纯视觉插值，不产生 revision（§8 确定性口径） |
| 自动重放不读取实时输入 | 导览模式走 CommandJournal 录制通道（交互宿主 journal 接线列 U3，§8），回放等价 |

对 05 §11 的细化提议（owner 接受后回填 05）：§11.3 第二/三条（共享 orbit、
点击聚焦）由「前置缺口」转设计锁定；§11.4 schema 驱动控件由「未落地」转
设计锁定；§11.6 Tab 返回上一分区；C=Capture 前置（driver observe 通道）定义。
ADR-0031 不涉及（场景架构决策，UI 层不改其前提）。

## 13. 分期建议（owner 2026-09-30 接受方案；U1–U3 全部已落地，证据见 05 §12 对应行）

| 阶段 | 内容 | 验收门槛 | 状态 |
|---|---|---|---|
| U1 反馈与直接操纵 | toast 堆叠；Tab 上一分区记忆；庭院鼠标手势（mousemap 翻译层与 keymap 同构）；机位补间（控制器相机状态内，可关）；H 演示模式 | cpu lane + 手势/翻译单测（合成输入事件）+ 窗口冒烟（BRP 驱动 + 亲眼看图）+ **owner 物理鼠标确认**（合成事件覆盖翻译层，winit 物理链路同现行物理按键缺口，如实记录） | **已落地（2026-09-30）**——证据见 05 §12 U1 行。「共享 orbit 上移」按 06 §16 结论落实为手势翻译层 + 控制器既有相机命令（Inspector 的 OrbitCamera 数据结构留在 Inspector 不动，§14）；物理鼠标确认待 owner |
| U2 空间导航与参数面板 | 分区 dock；minimap（Painter 自绘 + 图形命中）；场景内点击聚焦射线；schema 驱动参数面板（控件生成 + tier 置灰） | cpu lane + schema 控件单测 + 射线命中断言 + 窗口冒烟 | **已落地（2026-09-30）**——catalog `ui` 元数据（分组 + crowd presets）、builder 命中图元注册（`CourtyardHitMap`）、dock/minimap/参数抽屉宿主接线；单测与 lane、冒烟证据见 05 §12 U2 行。物理指针链路待 owner 确认（同 U1 既有口径） |
| U3 演示与捕获 | 导览模式（脚本数据 + 交互宿主 journal 接线 + journal 语义）；`Capture` 命令 + driver observe 入口 + C 键/按钮 + toast 落盘提示 | cpu lane + observe 通道测试 + 冒烟 | **已落地（2026-09-30）**——`tour.rs` 脚本数据 + 宿主接线（四驱动源同 `command_recorded` 通道）、`SessionCommand::Capture` + `CaseDriver::observe_adhoc` + 回执落盘 toast；单测与 lane、冒烟证据见 05 §12 U3 行。物理 G/C 键链路待 owner 确认（同上） |

browser/Android 壳的实现不在 U1–U3 内（沿用 05 §12 的 V2/V3 分期与冻结状态）。

降级序（工作量或评审压力超出预期时的裁剪顺序，先砍尾部）：① 导览模式
（U3 前半，独立可摘）；② 场景内悬停高亮（§5 末段本就是可选）；③ minimap
视锥楔形（装饰性，数据同源但不影响功能）。dock/minimap/参数面板/toast/
鼠标手势是「单调」诊断的直接对应物，不进入裁剪序。

## 14. 明确不做

- 不引入新 UI 框架或运行时依赖（egui + 自绘；射线手写）；
- 不动 golden 捕获矩阵、判定口径与基线（交互层与严格捕获解耦，01:443）；
- 不动 Inspector（05 §13 既有承诺）；
- 不做游戏化元素（成就、进度、音效等——这是验证/展示工具）；
- 不改变命令层语义与键位表既有映射（只增不改；V1 键位单测继续有效）；
- browser 交互 V2 / Android 交互 V3 维持冻结，本文档只做 §11 映射设计；
- 本提案在 owner 接受前不改动 05 与 ADR-0031 的任何字样。

## 15. 对抗性评估记录（2026-09-29）

初稿完成后按「假设自己是评审，逐条攻击」执行一轮，攻击点与处置如下
（事实类攻击均先对代码核实再改文档）：

| # | 攻击 | 核实证据 | 处置 |
|---|---|---|---|
| A1 | minimap 取 AABB 中心作点位：lamps AABB 覆盖整个北半场，中心 (0,·,−5.56) 落在环内空地，点会画在错误位置 | metadata.rs:219-222（min(−11.12,0,−11.12) ~ max(11.12,1.84,0)） | **成立，已改**：§4.2 改为点状分区取中心、lamps 画弧线、LOD 画四球连线 |
| A2 | 点击聚焦以分区 AABB 做命中：lamps AABB 与 stage/p5/trio 北半大面积重叠，「最近交点」会误触发 | 同上 AABB 表 + §4 布局坐标 | **成立，已改**：§5 命中图元改为展品本体 primitive（builder 生成时注册），不用分区 AABB |
| A3 | 「rebuilding 指示」假设命令发出到就绪有可见窗口；若重建同步完成，指示器零帧可见 = 死 UI | controller.rs:699-710：`apply_parameter` 同步触发 retire→build，构建由 driver 动作图跨帧推进 | **部分成立，已改**：§7 指示器改由 `DriverStatus::Running` 驱动，不赌命令窗口 |
| A4 | 机位补间引入时间依赖：BRP 冒烟的截图/机位断言可能捕到补间中间态；revision 语义含糊 | lib.rs:497-504 窗口相机每帧读控制器状态 | **成立，已改**：§8 补确定性口径——补间不进 journal、不产生 revision、冒烟路径钉 `off` |
| A5 | §9 把 Capture 设计成 driver API 旁路，与 05 §11.1 把 `Capture` 定义为 SessionCommand 变体矛盾 | 05 §11.1 命令表 | **成立，已改**：§9 改为命令变体 + observe 入口作内容语义；§12 对照表同步修正 |
| A6 | dock 芯片「方位角刻度」：lamps 是半环、LOD 是轴、stage/trio 无单一方位，九区标不齐 | 05 §4 布局几何 | **成立，已删**：方位表达全部归 minimap |
| A7 | U1/U2 验收的「窗口冒烟」由 BRP 驱动，但 BRP 发命令不发 winit 事件——物理鼠标链路与现行物理按键缺口同构，不能声称已验证 | 现行 V2-1「物理按键链路待用户确认」记录 | **成立，已改**：§5 手势翻译层用合成输入事件单测；§13 U1 门槛补「owner 物理鼠标确认」 |
| A8 | Legend 退役后分区说明只剩 tooltip，可发现性下降 | 现状 Legend 默认展开 | **接受为权衡**：dock 激活态 + tooltip + minimap 联动足够覆盖；评审可要求保留折叠 Legend |
| A9 | 无工作量估计、无裁剪方案，范围有蔓延风险（验证工具做出展示器全套） | — | **已改**：§13 补降级序（导览 → 悬停高亮 → 视锥楔形） |
| A10 | UI 强调色取庭院配色表，未做对比度/可访问性核查 | 05 §4 配色表是场景材质色，非 UI 色 | **残余风险，记录在案**：实现期对 toast 语义色做对比度检查，不达标回退 egui 默认语义色 |

未成立/不处理的攻击：「egui Painter 自绘 minimap 是过度工程」——dock 已
承担导航，minimap 是空间关系呈现的唯一载体，且 Painter 无新依赖；「导览
模式会被用来跑判定」——§14 已排除判定职责，命令照常进 journal。

### 第二轮（2026-09-30，owner 要求复评）

复评方法同上（事实类攻击先对代码核实再改文档）。首轮表为 dated 记录，
不改写；其漂移引用在第二轮表中修正。

| # | 攻击 | 核实证据 | 处置 |
|---|---|---|---|
| B1 | §0「反馈」引 lib.rs:478-490——V2-5 落地编辑后行号漂移，命令结果 eprintln 实际在 501-509 | lib.rs:503/506/509 | **成立，已改**（§0 与 §7 正文同步） |
| B2 | §0「其他驱动源」引 lib.rs:434-438——BRP 注册实际在 449-458（434-438 是叠加相机配置段） | lib.rs:449-458 | **成立，已改** |
| B3 | §0「相机」引 lib.rs:497-504——窗口相机同步实际在 519-522（497-504 是键位翻译段）；§15 首轮 A4 同一漂移引用 | lib.rs:519-522 | **成立，已改**（A4 行属首轮 dated 记录，不改写，在此修正） |
| B4 | §0「输入」行漏 N 键（stage_character，V2-5 当日落地）与 `,`/`.` case 导航——自称「代码核对」却不全 | keymap.rs:11-13、25-28、43-45 | **成立，已改** |
| B5 | §6 分组声明 Content 漏 stage_character——schema 驱动会生成控件，但文档声明的分组单一源缺它，落到 catalog 元数据时无家可归 | catalog.rs stage_character 参数（V2-5） | **成立，已改**（Content 组补入） |
| B6 | §7 toast「Switched 含新机位名」——`Switched` 只携带 {case, generation}，机位名不在命令结果里，数据源未交代 | controller.rs CommandOutcome 定义 | **成立，已改**：注明机位名读 `DesktopSession.last_camera_view.preset`（lib.rs:521/550 既有缓存，与状态条同源） |
| B7 | §5「左键点击展品聚焦 / 左键拖拽空白 orbit」未定义 click vs drag 判定与「从展品上起拖」的归属——实现期必歧义 | — | **成立，已改**：补位移阈值判定与起拖归属句 |
| B8 | §16 称 OrbitCamera「纯数据结构 ~30 行」——state.rs:44-73 含 transform/validate 两个方法 | inspector/state.rs:44-73 | **成立（措辞），已改** |
| B9 | §7 构建指示引 controller.rs:699-710——retire→build 段实际在 705-716 | controller.rs:705-716 | **成立，已改**（§15 首轮 A3 同源引用同 B3 处理） |
| B10 | 「自由操作产生新 revision」引 01:429，实际文字在 01:433 | 01:433 | **保留不改**：05 §11.1 同引 01:429，两文档一致优先于行号精度；01:426-443 区间引用本身准确 |

未成立的攻击：「H/G 与现行键位冲突」——`KeyPresses`（keymap.rs:19-48）
无 H/G，宿主采集表（lib.rs:287-307）同无，不冲突；「§7 的 driver 状态
读取缺公开通道」——`SessionController::driver()` 只读访问器现成
（controller.rs:313），非缺口；「庭院窗口已有鼠标处理」——全 crate 无
MouseButton 路径（Inspector 走 egui 指针事件，不属庭院窗口），§0 描述
成立；「D4 的『交互路径三缺陷』计数存疑」——05 §12 V2 段（438 行）明确
记录三缺陷（窗口相机 Tonemapping、egui 叠加相机清屏配置、
PrimaryEguiContext 挂载，2026-09-29），计数与出处一致，D4 引用成立。

### 第三轮（2026-09-30，修改后复评）

| # | 攻击 | 核实证据 | 处置 |
|---|---|---|---|
| C1 | §8 称导览命令「照常……进 CommandJournal」——交互宿主当前不录制：lib.rs:501 命令循环直调 `controller.command`，无 journal；「照常」言过其实 | journal.rs:172（`command_recorded` 扩展方法，录制 API 现成）；apps/showcase 全 crate 无 Journal 引用 | **成立，已改**：§8 改述录制语义 + U3 显式列「宿主 journal 接线」；§12 对照表行、§13 U3 行同步 |
| C2 | 二轮表「未成立」注记称 driver 只读访问器需新增——实际 `SessionController::driver()` 已存在 | controller.rs:313 | **成立（自纠），已改** |

第三轮未成立的攻击：「revision 语义与实现不符」——§8 已自定口径
「revision 仍只由命令产生」（相机命令为 CommandPlan::Local，不 bump
代次但照常入 journal，controller.rs:337-379/ journal.rs:172-190），
文档内部一致；「zone 枚举控件形态不明」——catalog zone Enum 10 值
（catalog.rs:315-321），§6 规则（>4 下拉）自动覆盖。

### 第四轮（2026-09-30，修改后全文一致性通读）

对改动后的全文（§0–§16）做内部一致性通读，核交叉引用互相咬合：

| # | 攻击 | 核实证据 | 处置 |
|---|---|---|---|
| D1 | §9 引 keymap.rs:7-11，与二轮修正后的 §0（10-11）对同一注记口径不一 | keymap.rs:10-11（C=Capture 暂缓注记实际所在行） | **成立，已改**（§9 同步为 10-11） |

其余通读项全部咬合：§4.1 引 keymap.rs:103-117 与 §0 一致；§5 表与正文
（click/drag 阈值、起拖归属）一致；§6 分组含 stage_character 与 §0 输入行、
05 §11.6 键位表一致；§8 录制表述与 §12/§13 同步；§16 措辞与
inspector/state.rs:44-73 一致。无新增发现，收敛。

### 第五轮（2026-09-30，U2/U3 落地后实现-文档一致性检视）

U2/U3 落地后逐条核对文档 claims 与代码/记录（dated 追加，不改写历史轮次）：

| # | 攻击 | 核实证据 | 处置 |
|---|---|---|---|
| E1 | §13 U2/U3 转「已落地」是否言过其实——验收门槛逐项 | U2 门槛：cpu lane ✓（退出码 0）、schema 控件单测 ✓（param_panel 2 + catalog 分组/presets 1）、射线命中断言 ✓（pick 4 + builder 注册 2）、窗口冒烟 ✓（截图亲验 dock/minimap/抽屉/五处联动）。U3 门槛：cpu lane ✓、observe 通道测试 ✓（controller Capture 门控 + journal 往返）、冒烟 ✓（BRP capture → Applied + PNG 落盘 + toast 路径亲验） | 成立区间如实保留：物理指针/G/C 键链路仍挂既有「待 owner 物理确认」口径；Capture 忙时拒绝实况未捕到（亚秒级重建），门控有单测——两条都写进 05 §12 状态列 |
| E2 | 「四驱动源同 command_recorded 通道」是否成立 | lib.rs desktop_tick 命令循环（键位/鼠标/UI 队列）与 brp_session_command 均走 `command_recorded`；会话级 CommandJournal 驻 DesktopSession | 成立 |
| E3 | §6 「控件操作只发 SetParameter」与 Capture 按钮的关系 | Capture 按钮发 `SessionCommand::Capture`——Capture 是 05 §11.1 已定义的 SessionCommand 变体（§9 明确「UI/键位/BRP 同通道，不是 UI 旁路」），不违反「控件无旁路」语义 | 不成立（§9 已前置定义） |
| E4 | golden 零变动声明 | gpu lane 12/12 PASS 对 canonical 基线（未重捕）；命中表是纯资源注册（非实体、不进像素路径）；Capture 回执与动作图 Observe 同通道但只在交互会话发起 | 成立 |
| E5 | minimap 命中与绘制是否同一几何源 | 绘制与命中同走 `minimap::world_to_map`/`zone_dots`/`lod_axis_points`/`LAMPS_ARC_RADIUS_M`（showcase 宿主不复制坐标，数据源 = scenes::metadata） | 成立 |
| E6 | 05 §12 U2 行「默认 13 条/overview 236 条」计数 | builders 测试断言：13 = 平台 1 + 展台 5 + trio 球 7；236 = 13 + P1 4 + P2 3 + P3 3 + P4 3 + P5 4 + LOD 4 + stage 2 + lamps 200 | 成立 |

## 16. 方案比选与最终选型（2026-09-29 调研）

对设计中的每个技术决策点做候选比选；事实依据全部经代码/依赖核实。
**结论先行：§3–§13 的设计骨架不变，六个决策点的最终选型如下。**

### D1 UI 框架：维持 bevy_egui

| 候选 | 评估 |
|---|---|
| **bevy_egui 0.41.1（选定）** | 已 pin 并集成（`apps/showcase/Cargo.toml:32`，interactive feature）；Inspector 装配模式现成；面板/抽屉/折叠/自绘（Painter）能力齐备；无新依赖 |
| bevy_ui（retained） | 无新依赖但面板交互（折叠/抽屉/滑杆/tooltip）全靠手写 ECS 布局，Inspector 与庭院现有 egui 资产作废，工作量最大 |
| 第三方 UI crate | 新运行时依赖，直接违反 §14 约束 |

### D2 点击聚焦命中：世界空间 ray-primitive（builder 注册图元）

| 候选 | 评估 |
|---|---|
| **ray-primitive（选定）** | 3D 语义精确（点展品 = 点展品）；图元注册在数据层，与渲染管线无关——实例化展品同样可命中；纯射线数学可无头单测；射线来源 `Camera::viewport_to_world`（bevy 0.19 现成 API） |
| bevy in-tree mesh_picking backend | 0.19.1 在树内（feature flag，非新 crate），但**致命伤**：灯阵/crowd 展品走自定义实例化渲染（`build_portable_instances`），不是逐实体网格，backend 看不到实例——命中覆盖有洞，展台可点、灯阵不可点，语义不一致 |
| 屏幕空间投影命中（次选记录） | 最便宜（AABB 角点投影 2D rect + 深度排序）、可无头测；但「点展品边缘」语义差（锚点+阈值近似），且要把投影管线接进输入翻译层。LOD 球因 05 §6 恒定角直径 ≈11.8° 反而恒易命中。若 ray-primitive 实现期暴露遮挡/精度问题，降级到此方案 |

### D3 机位过渡：控制器相机状态内补间

| 候选 | 评估 |
|---|---|
| **控制器状态内补间（选定）** | 相机统一不破坏（窗口相机照常每帧读控制器）；确定性口径见 §8（不进 journal、不产生 revision、冒烟钉 off） |
| UI 层覆盖窗口 Transform | 直接违反 05 §11.3 第一条相机统一，否决 |
| 不做（保持瞬跳） | 零成本，但「分区在哪、从哪到哪」的空间感知正是 §1 诊断的核心缺失，瞬跳不解决问题 |

### D4 minimap 渲染：egui Painter 自绘

| 候选 | 评估 |
|---|---|
| **egui Painter（选定）** | 留在 UI 层，无新相机；数据从 metadata.rs 派生（单一源）；激活态/视锥楔形每帧重绘零成本 |
| 第二台 2D 相机 + sprite | 多一台相机就要再处理一次渲染序/清屏配置——交互路径三缺陷（2026-09-29 冒烟记录）的前科都在这个坑里，不重复踩 |
| 预渲染静态图资产 | 布局变更即过期，违反数据单一源，否决 |

### D5 crowd 控件：U32 步进滑杆 + presets 快捷芯片

调研发现 §6 初稿控件表漏了 `U32` 变体——crowd 声明为 `U32 { max: 1000 }`
（catalog.rs:340-343）。纯滑杆允许 437 这类合法但无展示锚点的值；场景语义
锚点是 0/100/1000（含 stress 档）。选型：U32 → 步进滑杆，catalog schema 增
**可选** `presets` 元数据渲染快捷芯片（crowd = 0/100/1000）；不收窄校验
（U32 全域仍合法，presets 只是 UI 引导）。该项是 §13 U2 内的小扩展。

### D6 toast 实现：egui Area + 会话状态自管理

egui 无内建 toast；用 egui Area 右下锚定 + 会话 State 内 VecDeque（时间戳
驱动消退）。无备选必要，工作量忽略。

### 最终方案一句话

**egui 自绘 HUD（dock + Painter minimap + schema 参数抽屉 + toast）+ 手写
ray-primitive 点击聚焦 + 控制器内机位补间 + 共享 OrbitCamera 鼠标手势**，
全部由 SessionCommand 单通道驱动，按 §13 U1→U3 分期、降级序兜底。
OrbitCamera 上移成本极低（Inspector 侧 ~30 行：数据结构 + transform/
validate 两方法，`apps/showcase/src/inspector/state.rs:44-73`），U1 的
主要工作量在 toast/手势翻译层与窗口冒烟，不在相机。

