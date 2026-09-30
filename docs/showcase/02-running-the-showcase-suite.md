# 运行 Showcase 套件

> **Status:** 当前实现操作手册。跨平台统一方案见[场景设计](01-showcase-examples-fixtures.md)，
> 其中 Proposed 部分尚未实现，不能当作当前命令或平台覆盖。
> 2026-09-28（docs/showcase/05 §12 阶段 4）：18 个旧展示场景已退役，
> `apps/showcase` 注册唯一的庭院展台（courtyard），golden 判定按
> [庭院设计](05-courtyard-scene-design.md) §9 的 10 条捕获规格组织。

`apps/showcase` 当前注册唯一场景 courtyard（庭院展台），内容与相机配方由
`labs/scenes` 提供，经 `labs/scene-runtime` 的 CaseDriver 驱动。Android 另有
程序化与静态角色两种 smoke 模式，尚未共享完整目录；构建步骤见
[Contributing](../../CONTRIBUTING.md#android-vulkan-smoke)。
Android 宿主经 `driver_host` 逐帧推进同一 driver 并在设备端重放共享目录配方，
logcat 验收标记为 `ANDROID_REPLAY compiled/captured/refused/report_ready`。

## 用法

```text
CARGO_INCREMENTAL=0 cargo run -p showcase --features interactive -- courtyard # 开窗交互（参数面/机位键位表）
CARGO_INCREMENTAL=0 cargo run -p showcase -- courtyard --capture # headless 捕获
CARGO_INCREMENTAL=0 cargo run -p showcase -- courtyard --capture-spec <id|all> # 按捕获规格捕获+判定
CARGO_INCREMENTAL=0 cargo run -p showcase -- workloads --out <new-dir> # headless 九个固定 workload
```

交互模式（v3 展亭模式 + 06 U1–U3 交互层）：←/→ 循环**内容档**（trio → p1 →
p2 → p3 → p4 → p5 → lod → stage → lamps；内容在中央平台轮换装卸，机位随
内容联动，§7 一一对应），
↑/↓ 切时刻，O/P 切描边/后处理，1/2/3 切灯阵
密度，Space 切旗帜姿态，N 切角色台展品（fixture ↔ odette 真 Genshin
角色——经 `odette://` 资产源加载 `assets/local/genshin-odette/compiled/static-v3`，
目录缺席时启动日志明示且该档加载失败可见；golden 判定内容恒为 fixture），
R 复位机位，C 交互捕获（快照写 `target/stylized/showcase/interactive/`
（gitignored），文件名带观测名/代次/实验 tick/时刻戳；浏览快照不参与
golden 判定），G 导览模式（脚本化巡场——9 内容档 ×4s，收尾回 trio，
任意导航输入退出）；`,`/`.` 切用例（单场景下回绕）。（v2 的 Tab overview
开关与场景内点击聚焦/minimap 随 v3 退役。）**内容按需动态装卸**
（W1–W3，05 §11.2）：切内容档只卸旧档展品、装新档展品（先装后卸，机位补间
落定才退役旧档），画框（地面/墙/平台）与观测相机常驻；post/描边
只换相机附件，flags/face_threshold 直接改在场实体——均不重建场景。鼠标：左键拖拽 orbit、
右键拖拽 pan、滚轮缩放、双击复位（手势与 Inspector 同手感；
点击/拖拽按位移阈值判定，指针在 egui 面板上时手势不触发）。H 切演示模式
（隐藏全部 egui UI，Esc 或 H 退出）。常驻 UI：左下状态条、底部**内容
dock**（芯片点击 = 内容档命令，tooltip 带内容说明）、右侧**参数抽屉**
（catalog schema 驱动控件——Enum 分段/下拉、U32 步进滑杆 + 0/100/1000 预设
芯片、Bool 开关、OptionalF32 滑杆+自动勾选；tier 不足 Standard 的选项置灰
并带结构化原因 tooltip）。命令结果在右下 toast 可见（确认 2s 消退；
拒绝琥珀色驻留，× 或下次命令关闭；快照路径 6s）。全部命令（键位/鼠标/
UI/BRP 四驱动源）经会话级 journal 录制。切内容档机位补间 0.4s（纯视觉插值，
不进 journal/不产生 revision；冒烟路径用 `--no-camera-transition` 钉关）。
tier 需求（outline_mode=hybrid 需 Standard）超过当前
已解析档时先经产品重配置 API（ADR-0030）尝试会话内升档，设备级缺口才结构化
拒绝并要求新 device 会话；资产根变更仍以结构化信息拒绝（R2）。`workloads`
子命令 headless 运行，使用默认 feature，不依赖 egui/Inspector；数值合同见
[性能合同](../performance/targets.md)。静态角色交互检查使用 `inspect` 子命令
（`--features inspector`），见下文证据纪律。

**BRP 实时检视（bevy-copilot，可选）**：交互宿主（`--features interactive`）
在 `127.0.0.1:15702` 暴露 BRP（bevy/bevy_remote 只挂该 feature；headless/lane
路径不含）。`CourtyardSettings` 及其枚举已反射注册（get-resources 可读当前
zone/时刻/描边等钉值）；自定义方法 `showcase/session_command` 接受与键位
同一 `SessionCommand` 通道的载荷，例：
`bevy_copilot brp rpc execute "showcase/session_command" --params '{"type":"set_parameter","data":{"key":"zone","value":"p2"}}'`。
bevy_copilot_extras（send-keys/screenshot）未集成——该 crate 未发布，
路径依赖 sibling checkout 会破坏工程独立性（AGENTS.md）。

捕获规格（17 条，`--capture-spec` 与 golden lane 共用身份）：`courtyard-trio` /
`courtyard-ramp` / `courtyard-outline-hull` / `courtyard-outline-hybrid`
（Standard tier）/ `courtyard-lighting` / `courtyard-flags` / `courtyard-stage` /
`courtyard-lod` / `courtyard-dusk` / `courtyard-night` / `courtyard-post` /
`courtyard-stress` / `courtyard-rim` / `courtyard-shadow` / `courtyard-palette` /
`courtyard-metal` / `courtyard-cutout`（后 5 条 = V4 cel 特性档）。

- 捕获输出：`target/stylized/showcase/captures/<capture_id>.png`（gitignored，
  local-only）；
- 角色台加载第一方 `fixtures/synthetic/reference-scene/reference_scene.glb`
  （资产门控收窄（v2）：仅 `courtyard-stage` 捕获在缺席时 BLOCKED，不做空台
  降级；其余捕获不依赖 GLB）；
- golden 基线：17 条捕获的 canonical 基线由参考宿主（Linux/Vulkan）产出；
  其他平台经 overlay 通道候选（`fixtures/golden/showcase/<platform>/`）。
  判定 = 整图精确为主（v2 一帧一主体；夜景 lamps 等暗部捕获保留整图宽松
  non-black + luma/chroma 包络 + ROI 区域双层门限，AABB 投影自动计算）；
  验收记录 `fixtures/golden/showcase.courtyard.acceptance.md`。

## 证据纪律

showcase 是 manual QA / debug 入口，不是 release gate；release 判定走
golden（`tests/integration/tests/showcase_golden.rs`，12 个
`showcase_golden_courtyard_*` 用例）与 benchmark 基线。

静态角色交互检查使用独立的 `inspect` 子命令，参见[角色检查器](03-character-inspector.md)。
它不计入固定用例列表，也不替代 Blender 参考验收。
