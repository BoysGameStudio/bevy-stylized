# Showcase 宿主 UI 框架设计（提案）

- **Status: Proposed**（2026-09-30 起案；接受与否是 owner 决策）
- 范围：**展示/验证宿主的 UI 框架**——showcase 交互窗口、Inspector、未来
  browser（V2 冻结）/Android（V3 冻结）壳共用的 UI 架构。不是游戏内 UI
  （游戏 HUD/菜单不在本框架范围）。
- 前置文档：01（共享应用层合同 §426-443）、05 §11（交互设计）、
  06（courtyard UI 重设计，U1–U3 已落地）。

## 0. 现状盘点（2026-09-30 代码核对）

已落地（06 U1–U3 + W1–W3，全部有 lane/冒烟证据）：

- **命令单通道**：键位（keymap）/鼠标手势（mousemap）/UI 队列（dock/抽屉）
  /BRP 四驱动源同进 `command_recorded`（会话级 journal）；
- **控件元数据单一源**：catalog `ParameterSpec.kind` + `ui` 元数据
  （分组/presets），`param_panel::widget_kind` 是纯函数选型（有单测）；
- **反馈**：toast 堆叠（确认/拒绝/快照路径）、building 指示、tier 置灰
  （控制器 `evaluate` 同一判定）；
- **演示能力**：H 演示模式、G 导览（脚本数据）、C 捕获（driver observe）。

现状的结构性弱点：

1. **组装是命令式的**：`overlay_ui`（lib.rs）手写 egui 窗口/锚定/遍历——
   控件选型可测，但「什么面板、什么顺序、什么锚定」不可测，只在 egui
   壳里存在；
2. **壳逻辑不可移植**：browser（HTML）/Android 壳若动工，布局/门控/反馈
   语义要在另一种 UI 技术里重新发明——01 合同要求「相同控件元数据、
   控制器和命令校验用于所有布局」，现状只有控件选型这层是共享的；
3. **视图状态靠散件**：状态条/toast/抽屉各自从 `DesktopSession` 字段和
  资源里零散取数，没有一份「这一帧 UI 该显示什么」的显式快照——
  写壳的人必须反向阅读 desktop_tick 才能凑齐数据源。

## 1. 目标与约束

目标：把「显示什么」与「怎么画」分开——**视图模型是平台无关的纯数据
（可单测），壳只是渲染器 + 输入采集器**。三壳布局不同，语义同源
（01 §438：相同控件元数据、控制器和命令校验用于所有布局）。

约束（继承，不重新讨论）：

- 命令层单通道：UI/键位/触摸只是 SessionCommand 驱动源（01 §426）；
- UI 参数从 catalog 派生（01 §428；05 §11.4）；
- 无新重依赖（PC 壳 = bevy_egui 已引入；browser 壳 = HTML 页面桥）；
- 严格捕获与交互画面解耦（01 §431）；
- 既有功能零回归（U1–U3/W1–W3 的行为与证据全部保留）。

## 2. 架构（三层）

```
┌─────────────────────────────────────────────────┐
│ 壳适配层（每平台一份，薄）                        │
│  PC: egui 渲染 widget tree + 采集键鼠             │
│  Browser: HTML/DOM（V2 冻结） Android: 折叠面板（V3）│
├─────────────────────────────────────────────────┤
│ 视图模型层（平台无关，纯数据，可单测）             │
│  SessionViewState → WidgetTree（面板/控件/反馈）   │
├─────────────────────────────────────────────────┤
│ 命令通道层（已存在，不动）                         │
│  SessionCommand ← command_recorded（journal）    │
└─────────────────────────────────────────────────┘
```

### 2.1 视图模型层（新，核心）

**`SessionViewState`**：一帧 UI 的只读输入快照（数据，不含 egui/bevy
类型）：

| 字段 | 内容 | 来源（现状映射） |
|---|---|---|
| `case` / `generation` | 当前用例与代次 | controller |
| `zone` / `preset` / `time_of_day` / `outline` / `post` / `crowd` / `flags` / `character` / `face_threshold` | 当前参数值 | CourtyardSettings + last_camera_view |
| `tier` | resolved tier（含门控判定所需的拒绝原因） | StylizedResolvedCapabilities |
| `driver_running` | building 指示 | desktop_tick 快照 |
| `toasts` | toast 列表（文本/驻留/剩余时间） | DesktopSession |
| `tour_active` / `presentation` | 演示/导览状态 | DesktopSession |

**`WidgetTree`**：纯数据 UI 描述（面板列表，每个面板 = 锚定 + 控件列表；
控件 = WidgetKind + 键 + 当前值 + 门控状态 + 文案）。构建函数
`build_widget_tree(&SessionViewState, &CatalogEntry) -> WidgetTree` 是纯
函数——**布局、分组、门控、文案全部可单测**（「hybrid 在 Portable 下
置灰且带原因」这类断言不再需要 egui 实例）。

锚定用平台无关枚举（`Anchor::LeftBottom / CenterBottom / RightTop / …`），
壳各自映射到 egui anchor / CSS。06 §3 的布局图（状态条左下、dock 底部、
抽屉右侧、toast 右下）是 anchor 数据，不是 egui 调用。

### 2.2 命令通道层（不动）

视图模型的控件事件只产 `SessionCommand`（01 合同）；宿主本地状态
（演示模式/导览/面板折叠）不进命令（06 §8 已确立的边界）。journal
录制位置不变（desktop_tick 命令循环 + BRP 入口）。

### 2.3 壳适配层

壳职责只有三个：**渲染** widget tree、**采集**输入翻译成
KeyPresses/MouseGestures、**呈现**反馈（toast/状态条按视图模型数据画）。
PC egui 壳是参照实现（现有 overlay_ui 改为 tree 渲染器）；browser/
Android 壳在 V2/V3 解冻时按同一 tree 映射（06 §11 表）。

## 3. 与既有代码的关系（改造映射）

| 现状 | 框架化后 |
|---|---|
| `param_panel::widget_kind` + `courtyard_value` | 保留为视图模型层的控件选型/取值构件（已被单测覆盖） |
| `overlay_ui` 手写 egui 组装 | 壳：遍历 WidgetTree 画 egui（行为不变，冒烟对比截图） |
| DesktopSession 散件字段 | 每帧采集为 SessionViewState（desktop_tick 单点采集） |
| ZONE_DESCRIPTIONS / Keys 文案 | 进视图模型层的静态文案表（壳不持有文案） |
| 06 §11 三壳映射表 | 不变——本框架是它的实现形态 |

## 4. 对抗性评估（2026-09-30 起案轮）

| # | 攻击 | 处置 |
|---|---|---|
| G1 | 只有一个壳在用，视图模型是过度抽象 | **部分成立**：价值 = 可单测的布局/门控（现状只能靠截图亲验）+ V2/V3 壳解冻时的直接载体。缓解：F1 只加纯数据层不动壳，F2 才换壳渲染且行为不变（截图对比）；若 F2 后无收益证据，框架就此止步不追加 |
| G2 | WidgetTree 重建每帧分配，性能 | egui immediate 模式本就每帧重建；tree 是几百字节级数据，可忽略 |
| G3 | egui 概念（anchor/折叠/抽屉）泄漏进「平台无关」模型 | anchor 是枚举数据不是 egui 类型；折叠状态是壳本地状态（不进 tree，同 presentation 边界） |
| G4 | toast/状态条不是「控件」，硬塞进 widget 模型别扭 | 成立——树分两类节点：`Panel`（控件组）与 `Status`/`Toasts`（只读呈现），不强行统一为控件 |
| G5 | 与 06 已落地工作的关系含糊 | §3 映射表逐项对应；本框架不改任何已验证行为 |

## 5. 分期

| 阶段 | 内容 | 验收 | 状态 |
|---|---|---|---|
| F1 | 视图模型层：`SessionViewState` + `WidgetTree` + `build_widget_tree` 纯函数 + 单测（布局/门控/文案断言） | cpu lane + 单测 | 待批准 |
| F2 | PC 壳换血：overlay_ui 改为 tree 渲染器；行为不变（冒烟截图逐项对比 U1–U3 既有证据） | cpu lane + 窗口冒烟 | 待批准 |
| F3 | browser/Android 壳实现 | 各自 lane | **冻结**（V2/V3 分期不变，本框架只定义接口） |

降级序：F2 → F1 保留为纯测试辅助层（视图模型仍可直接喂断言）。
