//! 非阻塞 CaseDriver（R1）：状态机由宿主每帧推进一次。
//!
//! 状态：Build → WaitReady →（按稳定执行次序执行动作：FixedStep 进入
//! Stepping、WaitReady 回到等待、Observe 进入 AwaitObservation、
//! Checkpoint 即时记录）→ Done / Failed。driver 自身不调用
//! `app.update()`、不忙等、不阻塞接收；渲染推进归宿主泵送（桌面 headless
//! 包装见 [`crate::headless`]）。
//!
//! 实验时钟绑定规则（R13 的精确表述）：
//!
//! - 实验 tick 只在 Stepping 状态下、每次 [`CaseDriver::update`] 推进 1
//!   步（`fixed_step{ticks, step}` 展开为 ticks 次单步）；等待、观测在途、
//!   取消与宿主额外泵送都不推进实验 tick；
//! - Stepping 内一次 `update` 对应一次实验 tick；该 tick 的渲染帧由宿主
//!   在本次 `update` 返回后的下一次 `app.update()` 产生——因此 step 序列
//!   结束后的 `observe` 提取包含最后一个 tick 的内容；
//! - `observe` 发出的读回经 harness `CaptureStamp` 返回，回执同时携带
//!   实验 tick（发出时点）与中性 stamp；stamp 的主世界 update 序号随宿主
//!   泵送节奏变化，**实验身份（tick 与 id 序列）不随节奏变化**——这是
//!   "不同等待/update 次数和异步到达顺序不改变实验状态"的绑定层。
//!
//! 动作接缝（骨架期）：`apply_params` / `deform` / `move` 的目标绑定
//! （TargetRef → 运行时实体）属于配方编译器（phase G）；本阶段经
//! [`CaseDriver::set_action_hook`] 暴露同一接缝，未注册时返回结构化
//! [`RuntimeError::UnsupportedAction`] 而不是 panic。

use std::collections::HashSet;

use bevy::asset::Assets;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::math::Vec3;
use bevy::prelude::{Camera3d, Entity, Image, Transform, World};

use bevy::render::view::Msaa;
use bevy_stylized::prelude::{StylizedInstanceData, StylizedViewDescriptor, StylizedViewSettings};
use bevy_stylized::stylized_render::screen_outline::StylizedScreenOutline;
use golden_harness::{CaptureRequest, capture, readiness};
use scenario_model::action::{ActionGraph, ActionKind, ActionNode, Step, TargetRef, TargetScope};
use scenario_model::ident::{Generation, RequestId, SampleId, SessionId, ViewId};
use scenes::SceneTarget;
use scenes::registry::{SceneBinding, ViewPresetFn};
use scenes::{CameraAttachment, CameraRecipe};

use crate::clock::ExperimentClock;
use crate::error::RuntimeError;
use crate::observe::{CheckpointRecord, ObservationReceipt};
use crate::scene::SceneLedger;

/// 有界预算与就绪语义（泵送次数；非时间——期限由宿主编排，R13）。
#[derive(Clone, Copy, Debug)]
pub struct DriverLimits {
    /// 就绪等待的最大泵送次数。
    pub readiness_pumps: usize,
    /// 观测读回的最大泵送次数。
    pub readback_pumps: usize,
    /// 构建后是否隐式进入就绪等待（readiness 探针就绪才执行动作）。
    /// 关闭时构建后直接进入动作执行——捕获 profile 用显式 fixed_step
    /// 复现既有暖场帧数；开启时宿主必须已安装 harness readiness 探针。
    pub implicit_readiness: bool,
}

impl Default for DriverLimits {
    fn default() -> Self {
        Self {
            readiness_pumps: 600,
            readback_pumps: 600,
            implicit_readiness: true,
        }
    }
}

/// driver 状态（结构化终态：`Done` / `Failed`）。
#[derive(Clone, Debug, PartialEq)]
pub enum DriverStatus {
    /// 推进中。
    Running,
    /// 全部动作执行完成。
    Done,
    /// 结构化失败（终态）。
    Failed(RuntimeError),
}

#[derive(PartialEq)]
enum State {
    Build,
    WaitReady { waited: usize },
    Actions,
    Stepping { remaining: u64, step: Step },
    AwaitObservation { waited: usize },
    Done,
    Failed,
}

struct PendingObservation {
    request: CaptureRequest,
    observation: String,
    view: ViewId,
    request_id: RequestId,
    sample: SampleId,
    generation: Generation,
    experiment_tick: u64,
}

struct DriverIds {
    session: SessionId,
    generation: u64,
    next_sample: u64,
    next_request: u64,
}

/// 动作接缝：apply_params/deform/move 的目标应用（骨架期由调用方接线；
/// 参数为动作节点与实验 tick 语义下的世界）。
pub type ActionHook = Box<dyn FnMut(&ActionNode, &mut World) + Send + Sync>;

/// 实验 tick 钩子：Stepping 中每个 tick 调用一次（确定性动作序列的
/// 接缝；等待与宿主泵送不触发）。
pub type TickHook = Box<dyn FnMut(u64, &mut World) + Send + Sync>;

/// 非阻塞 CaseDriver。
pub struct CaseDriver {
    scene: SceneBinding,
    view: Option<ViewPresetFn>,
    actions: Vec<ActionNode>,
    next_action: usize,
    clock: ExperimentClock,
    state: State,
    ids: DriverIds,
    ledger: SceneLedger,
    baseline: HashSet<Entity>,
    observation_target: Option<golden_harness::CaptureTarget>,
    observation_entity: Option<Entity>,
    camera_override: Option<CameraRecipe>,
    msaa: Option<Msaa>,
    pending: Option<PendingObservation>,
    observations: Vec<ObservationReceipt>,
    checkpoints: Vec<CheckpointRecord>,
    limits: DriverLimits,
    resolution: (u32, u32),
    action_hook: Option<ActionHook>,
    tick_hook: Option<TickHook>,
}

impl CaseDriver {
    /// 以场景绑定与动作图构造（构造即校验动作图；非法输入返回结构化
    /// 错误而不是在执行中 panic）。`baseline` 之前的实体视为宿主所有。
    pub fn new(
        world: &World,
        session: SessionId,
        scene: SceneBinding,
        actions: ActionGraph,
        limits: DriverLimits,
    ) -> Result<Self, RuntimeError> {
        let order = actions
            .validate()
            .and_then(|()| actions.stable_execution_order())
            .map_err(|error| RuntimeError::Plan(error.to_string()))?;
        let ordered: Vec<ActionNode> = order
            .into_iter()
            .map(|index| actions.nodes[index].clone())
            .collect();
        Ok(Self {
            scene,
            view: None,
            actions: ordered,
            next_action: 0,
            clock: ExperimentClock::default(),
            state: State::Build,
            ids: DriverIds {
                session,
                generation: 0,
                next_sample: 0,
                next_request: 0,
            },
            ledger: SceneLedger {
                generation: Generation::new(0),
                entities: HashSet::new(),
            },
            baseline: SceneLedger::snapshot(world),
            observation_target: None,
            observation_entity: None,
            camera_override: None,
            msaa: None,
            pending: None,
            observations: Vec::new(),
            checkpoints: Vec::new(),
            limits,
            resolution: (512, 512),
            action_hook: None,
            tick_hook: None,
        })
    }

    /// 视图 preset（构建前应用到 `StylizedViewSettings`）。
    #[must_use]
    pub fn with_view(mut self, view: ViewPresetFn) -> Self {
        self.view = Some(view);
        self
    }

    /// 设置/清除下一次构建应用的视图 preset（场景切换时由宿主更新）。
    pub fn set_view(&mut self, view: Option<ViewPresetFn>) {
        self.view = view;
    }

    /// 宿主绑定的相机配方覆盖：构建器的返回值被忽略，观测相机按给定
    /// 配方生成（测试夹具/宿主持有内容的场景；docs/showcase/01 line 131）。
    #[must_use]
    pub fn with_camera_recipe(mut self, recipe: CameraRecipe) -> Self {
        self.camera_override = Some(recipe);
        self
    }

    /// 观测相机实体（Build 之后可用；宿主用于更新投影等相机组件）。
    #[must_use]
    pub fn observation_entity(&self) -> Option<Entity> {
        self.observation_entity
    }

    /// 从 `Done` 终态再执行一段动作序列（同一观测相机/同一代次内的
    /// 重复 settle→observe 周期）。动作图先校验，非法输入返回结构化错误。
    pub fn run_actions(&mut self, actions: ActionGraph) -> Result<(), RuntimeError> {
        if self.state != State::Done {
            return Err(RuntimeError::Scene(
                "run_actions is only legal from the Done state".into(),
            ));
        }
        let order = actions
            .validate()
            .and_then(|()| actions.stable_execution_order())
            .map_err(|error| RuntimeError::Plan(error.to_string()))?;
        self.actions = order
            .into_iter()
            .map(|index| actions.nodes[index].clone())
            .collect();
        self.next_action = 0;
        self.state = State::Actions;
        Ok(())
    }

    /// 观测分辨率（默认 512²）。
    #[must_use]
    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.resolution = (width, height);
        self
    }

    /// 声明的抗锯齿（编译计划经此插入相机；None = 引擎默认）。
    #[must_use]
    pub fn with_msaa(mut self, msaa: Msaa) -> Self {
        self.msaa = Some(msaa);
        self
    }

    /// 从编译的 [`crate::compiler::ScenePlan`] 构造（内容绑定、相机配方、
    /// 分辨率、AA 与有序动作全部来自计划；隐式就绪等待开启）。
    pub fn from_plan(
        world: &World,
        session: SessionId,
        plan: crate::compiler::ScenePlan,
    ) -> Result<Self, RuntimeError> {
        let driver = Self::new(
            world,
            session,
            plan.binding,
            plan.actions,
            DriverLimits::default(),
        )?
        .with_camera_recipe(plan.camera)
        .with_resolution(plan.resolution.0, plan.resolution.1)
        .with_msaa(plan.msaa);
        Ok(driver)
    }

    /// 注册动作接缝（apply_params/deform/move）。
    pub fn set_action_hook(&mut self, hook: ActionHook) {
        self.action_hook = Some(hook);
    }

    /// 注册实验 tick 钩子。
    pub fn set_tick_hook(&mut self, hook: TickHook) {
        self.tick_hook = Some(hook);
    }

    /// 实验时钟（只读）。
    #[must_use]
    pub fn clock(&self) -> &ExperimentClock {
        &self.clock
    }

    /// 当前代次。
    #[must_use]
    pub fn generation(&self) -> Generation {
        Generation::new(self.ids.generation)
    }

    /// 已归一的观测回执流。
    #[must_use]
    pub fn observations(&self) -> &[ObservationReceipt] {
        &self.observations
    }

    /// 检查点记录流。
    #[must_use]
    pub fn checkpoints(&self) -> &[CheckpointRecord] {
        &self.checkpoints
    }

    /// 取走累积的观测回执（交给 validation 等下游）。
    pub fn drain_observations(&mut self) -> Vec<ObservationReceipt> {
        std::mem::take(&mut self.observations)
    }

    /// 状态名（诊断/测试观测用；身份不经由此通道）。
    #[must_use]
    pub fn state_name(&self) -> &'static str {
        match self.state {
            State::Build => "Build",
            State::WaitReady { .. } => "WaitReady",
            State::Actions => "Actions",
            State::Stepping { .. } => "Stepping",
            State::AwaitObservation { .. } => "AwaitObservation",
            State::Done => "Done",
            State::Failed => "Failed",
        }
    }

    /// 当前状态（结构化）。
    #[must_use]
    pub fn status(&self) -> DriverStatus {
        match self.state {
            State::Done => DriverStatus::Done,
            State::Failed => DriverStatus::Failed(RuntimeError::Scene(
                "driver is in the failed terminal state".into(),
            )),
            _ => DriverStatus::Running,
        }
    }

    fn fail(&mut self, error: RuntimeError) -> DriverStatus {
        self.state = State::Failed;
        DriverStatus::Failed(error)
    }

    /// 宿主每帧调用一次：非阻塞推进一步。
    ///
    /// 世界边界：headless 包装传 `world`；交互宿主在独占系统中
    /// 直接传 `&mut World`（事件循环始终归宿主，R1）。
    pub fn update(&mut self, world: &mut World) -> DriverStatus {
        match self.state {
            State::Build => self.update_build(world),
            State::WaitReady { waited } => self.update_wait_ready(world, waited),
            State::Actions => self.update_actions(world),
            State::Stepping { remaining, step } => self.update_stepping(world, remaining, step),
            State::AwaitObservation { waited } => self.update_await_observation(world, waited),
            State::Done => DriverStatus::Done,
            State::Failed => self.status(),
        }
    }

    fn update_build(&mut self, world: &mut World) -> DriverStatus {
        if self.limits.implicit_readiness && world.get_resource::<readiness::Probe>().is_none() {
            // harness 中性探针由宿主装配（插件/子 App 配置是宿主边界）；
            // 不需要隐式就绪的 profile 用 DriverLimits::implicit_readiness 关闭。
            return self.fail(RuntimeError::Scene(
                "readiness probe missing; the host must install it before driving".into(),
            ));
        }
        if let Some(view) = self.view {
            let descriptor = {
                let mut descriptor = StylizedViewDescriptor::default();
                view(&mut descriptor);
                descriptor
            };
            let Some(mut settings) = world.get_resource_mut::<StylizedViewSettings>() else {
                return self.fail(RuntimeError::Scene(
                    "StylizedViewSettings missing; the caller must provide the Stylized plugin"
                        .into(),
                ));
            };
            *settings = StylizedViewSettings(descriptor);
        }
        // 构建器永远执行（内容装配）；相机配方覆盖只替换配方数据，
        // 不跳过内容构建（CaptureCycle 的 noop 绑定返回默认配方）。
        let built = (self.scene.build)(world);
        let recipe = self.camera_override.clone().unwrap_or(built);
        self.spawn_observation_camera(world, &recipe);
        self.ledger =
            SceneLedger::record(Generation::new(self.ids.generation), &self.baseline, world);
        self.state = if self.limits.implicit_readiness {
            State::WaitReady { waited: 0 }
        } else {
            State::Actions
        };
        DriverStatus::Running
    }

    fn spawn_observation_camera(&mut self, world: &mut World, recipe: &CameraRecipe) {
        let target = {
            let mut images = world.resource_mut::<Assets<Image>>();
            capture::create_capture_target(&mut images, self.resolution.0, self.resolution.1)
        };
        let camera = world
            .spawn((
                Camera3d::default(),
                capture::capture_target_bundle(&target),
                Tonemapping::None,
                Transform::from_translation(recipe.eye).looking_at(recipe.look, Vec3::Y),
            ))
            .id();
        match &recipe.attachment {
            CameraAttachment::None => {}
            CameraAttachment::PostProcess(post) => {
                world.entity_mut(camera).insert(*post);
            }
            CameraAttachment::ScreenOutline => {
                world
                    .entity_mut(camera)
                    .insert(StylizedScreenOutline::default());
            }
            CameraAttachment::Prepass {
                depth,
                normal,
                motion_vector,
            } => {
                let mut entity = world.entity_mut(camera);
                if *depth {
                    entity.insert(bevy::core_pipeline::prepass::DepthPrepass);
                }
                if *normal {
                    entity.insert(bevy::core_pipeline::prepass::NormalPrepass);
                }
                if *motion_vector {
                    entity.insert(bevy::core_pipeline::prepass::MotionVectorPrepass);
                }
            }
        }
        if let Some(projection) = &recipe.projection {
            world.entity_mut(camera).insert(projection.clone());
        }
        if let Some(msaa) = self.msaa {
            world.entity_mut(camera).insert(msaa);
        }
        self.observation_target = Some(target);
        self.observation_entity = Some(camera);
    }

    fn update_wait_ready(&mut self, world: &mut World, waited: usize) -> DriverStatus {
        let snapshot = world.resource::<readiness::Probe>().snapshot();
        if let Some(error) = snapshot.error {
            return self.fail(RuntimeError::Pipeline(error));
        }
        if snapshot.ready {
            self.state = State::Actions;
            return DriverStatus::Running;
        }
        if waited >= self.limits.readiness_pumps {
            return self.fail(RuntimeError::ReadinessTimeout);
        }
        self.state = State::WaitReady { waited: waited + 1 };
        DriverStatus::Running
    }

    fn update_actions(&mut self, world: &mut World) -> DriverStatus {
        loop {
            let Some(action) = self.actions.get(self.next_action).cloned() else {
                self.state = State::Done;
                return DriverStatus::Done;
            };
            match &action.kind {
                ActionKind::FixedStep { ticks, step } => {
                    self.next_action += 1;
                    self.state = State::Stepping {
                        remaining: ticks.0,
                        step: *step,
                    };
                    return DriverStatus::Running;
                }
                ActionKind::WaitReady { .. } => {
                    // 等待不推进实验状态。
                    self.next_action += 1;
                    self.state = State::WaitReady { waited: 0 };
                    return DriverStatus::Running;
                }
                ActionKind::Observe { observation, view } => {
                    let pending = self.issue_observation(world, observation, view);
                    self.next_action += 1;
                    self.pending = Some(pending);
                    self.state = State::AwaitObservation { waited: 0 };
                    return DriverStatus::Running;
                }
                ActionKind::Checkpoint { label } => {
                    self.checkpoints.push(CheckpointRecord {
                        label: label.clone(),
                        experiment_tick: self.clock.tick(),
                        applied_actions: self.next_action + 1,
                        generation: self.generation(),
                    });
                    self.next_action += 1;
                }
                ActionKind::ApplyParams { target, params } => {
                    if let Some(hook) = &mut self.action_hook {
                        hook(&action, world);
                    } else if let Err(error) = apply_instance_params(world, target, params) {
                        return self.fail(error);
                    }
                    self.next_action += 1;
                }
                ActionKind::Deform {
                    target,
                    channel,
                    value,
                } => {
                    if let Some(hook) = &mut self.action_hook {
                        hook(&action, world);
                    } else if let Err(error) = apply_deform(world, target, channel, *value) {
                        return self.fail(error);
                    }
                    self.next_action += 1;
                }
                ActionKind::Move {
                    target,
                    translation,
                } => {
                    if let Some(hook) = &mut self.action_hook {
                        hook(&action, world);
                    } else if let Err(error) = apply_move(world, target, translation) {
                        return self.fail(error);
                    }
                    self.next_action += 1;
                }
            }
        }
    }

    fn issue_observation(
        &mut self,
        world: &mut World,
        observation: &str,
        view: &Option<ViewId>,
    ) -> PendingObservation {
        // 观测读回必须读观测相机的实际渲染 target（Build 时创建并复用），
        // 不能新建未被渲染的 target。
        let target = self
            .observation_target
            .clone()
            .expect("observation camera is built before actions run");
        let request = capture::request_capture(world, target);
        self.ids.next_request += 1;
        self.ids.next_sample += 1;
        PendingObservation {
            request,
            observation: observation.to_string(),
            view: view
                .clone()
                .unwrap_or_else(|| ViewId::new("main").expect("view id")),
            request_id: RequestId::new(format!("r{}", self.ids.next_request)).expect("request id"),
            sample: SampleId::new(format!("sp{}", self.ids.next_sample)).expect("sample id"),
            generation: self.generation(),
            experiment_tick: self.clock.tick(),
        }
    }

    fn update_stepping(&mut self, world: &mut World, remaining: u64, step: Step) -> DriverStatus {
        debug_assert!(remaining > 0);
        let tick = self.clock.advance(step);
        if let Some(hook) = &mut self.tick_hook {
            hook(tick, world);
        }
        let remaining = remaining - 1;
        self.state = if remaining == 0 {
            State::Actions
        } else {
            State::Stepping { remaining, step }
        };
        DriverStatus::Running
    }

    fn update_await_observation(&mut self, world: &mut World, waited: usize) -> DriverStatus {
        let pending = self
            .pending
            .take()
            .expect("pending observation in AwaitObservation");
        let Some(capture) = capture::try_take_stamped(world, pending.request.clone()) else {
            if waited >= self.limits.readback_pumps {
                return self.fail(RuntimeError::ReadbackTimeout);
            }
            self.pending = Some(pending);
            self.state = State::AwaitObservation { waited: waited + 1 };
            return DriverStatus::Running;
        };
        self.observations.push(ObservationReceipt {
            session: self.ids.session.clone(),
            generation: pending.generation,
            view: pending.view,
            request: pending.request_id,
            sample: pending.sample,
            observation: pending.observation,
            experiment_tick: pending.experiment_tick,
            stamp: capture.stamp,
            image: capture.image,
        });
        self.state = State::Actions;
        DriverStatus::Running
    }

    /// 同 SessionPlan 内的合法场景切换：作废旧代在途观测（晚到回执被
    /// harness 接受队列忽略），退役旧代实体账本，代次 +1 并重建。
    ///
    /// 只能在 `Done` 或观测在途（AwaitObservation，切换即作废在途观测）
    /// 时调用；严格隔离用例必须新进程（R4），本方法服务共享会话的交互与
    /// 生命周期测试。
    pub fn switch_scene(&mut self, world: &mut World, scene: SceneBinding) -> DriverStatus {
        if !matches!(self.state, State::Done | State::AwaitObservation { .. }) {
            return self.fail(RuntimeError::Scene(
                "scene switch is only legal from Done or with a pending observation".into(),
            ));
        }
        if let Some(pending) = self.pending.take() {
            capture::cancel_capture(world, pending.request);
        }
        self.ledger.retire(world);
        self.ids.generation += 1;
        self.ledger = SceneLedger {
            generation: Generation::new(self.ids.generation),
            entities: HashSet::new(),
        };
        self.scene = scene;
        self.next_action = 0;
        self.state = State::Build;
        DriverStatus::Running
    }
}

// ---------------------------------------------------------------------------
// 动作目标绑定（docs/showcase/01 line 104-110 + R12：参数写入只落在声明的
// 作用域——实例槽位或 view；未声明/越界目标为结构化错误）
// ---------------------------------------------------------------------------

/// 逻辑 ID → 运行时实体（scenes::SceneTarget 注册合同）。
fn resolve_target(world: &mut World, target: &TargetRef) -> Result<Entity, RuntimeError> {
    if target.scope == TargetScope::View {
        if target.id.as_str() != "main" {
            return Err(RuntimeError::Scene(format!(
                "view scope only binds the declared `main` view, got {target:?}"
            )));
        }
        return Ok(Entity::PLACEHOLDER);
    }
    let mut query = world.query::<(Entity, &SceneTarget)>();
    query
        .iter(world)
        .find(|(_, marker)| marker.0 == target.id.as_str())
        .map(|(entity, _)| entity)
        .ok_or_else(|| {
            RuntimeError::Scene(format!(
                "unknown scene target {target:?} (scenes must register it via SceneTarget)"
            ))
        })
}

fn finite_f32(value: &serde_json::Value, key: &str) -> Result<f32, RuntimeError> {
    let Some(number) = value.as_f64() else {
        return Err(RuntimeError::Scene(format!(
            "parameter {key:?} must be a finite number"
        )));
    };
    let mapped = number as f32;
    if !mapped.is_finite() {
        return Err(RuntimeError::Scene(format!(
            "parameter {key:?} must be finite after f32 mapping"
        )));
    }
    Ok(mapped)
}

fn apply_instance_params(
    world: &mut World,
    target: &TargetRef,
    params: &scenario_model::canon::ParameterMap,
) -> Result<(), RuntimeError> {
    if target.scope == TargetScope::View {
        if target.id.as_str() != "main" {
            return Err(RuntimeError::Scene(format!(
                "view scope only binds the declared `main` view, got {target:?}"
            )));
        }
        let mut settings = world.resource_mut::<StylizedViewSettings>();
        for (key, value) in params {
            match key.as_str() {
                "exposure" => settings.0.exposure = finite_f32(value, key)?,
                "day_night_factor" => {
                    let mapped = finite_f32(value, key)?;
                    if !(0.0..=1.0).contains(&mapped) {
                        return Err(RuntimeError::Scene(format!(
                            "parameter {key:?} out of range 0..=1: {mapped}"
                        )));
                    }
                    settings.0.day_night_factor = mapped;
                }
                _ => {
                    return Err(RuntimeError::Scene(format!(
                        "parameter {key:?} is not declared for the view scope"
                    )));
                }
            }
        }
        return Ok(());
    }
    if target.scope != TargetScope::Entity {
        return Err(RuntimeError::UnsupportedAction(
            "Material/Light target scopes are not bindable in this phase",
        ));
    }
    let entity = resolve_target(world, target)?;
    let mut data = world
        .get_mut::<StylizedInstanceData>(entity)
        .ok_or_else(|| {
            RuntimeError::Scene(format!(
                "scene target {target:?} is not a stylized instance (StylizedInstanceData missing)"
            ))
        })?;
    for (key, value) in params {
        match key.as_str() {
            "profile_index" => {
                let Some(index) = value.as_u64() else {
                    return Err(RuntimeError::Scene(format!(
                        "parameter {key:?} must be a non-negative integer"
                    )));
                };
                if index > u64::from(u8::MAX) {
                    return Err(RuntimeError::Scene(format!(
                        "parameter {key:?} out of range 0..=255: {index}"
                    )));
                }
                data.profile_index = index as u32;
            }
            "face_threshold" => {
                data.face_threshold = if value.is_null() {
                    None
                } else {
                    Some(finite_f32(value, key)?)
                };
            }
            "face_side_flip" => {
                let Some(flip) = value.as_bool() else {
                    return Err(RuntimeError::Scene(format!(
                        "parameter {key:?} must be a boolean"
                    )));
                };
                data.face_side_flip = flip;
            }
            "outline_scale" => {
                let mapped = finite_f32(value, key)?;
                if !(0.0..=2.0).contains(&mapped) {
                    return Err(RuntimeError::Scene(format!(
                        "parameter {key:?} out of range 0..=2: {mapped}"
                    )));
                }
                data.outline_scale = mapped;
            }
            _ => {
                return Err(RuntimeError::Scene(format!(
                    "parameter {key:?} is not declared for the entity scope"
                )));
            }
        }
    }
    Ok(())
}

fn apply_deform(
    world: &mut World,
    target: &TargetRef,
    channel: &str,
    value: scenario_model::canon::FiniteF64,
) -> Result<(), RuntimeError> {
    let entity = resolve_target(world, target)?;
    let mut transform = world
        .get_mut::<bevy::prelude::Transform>(entity)
        .ok_or_else(|| RuntimeError::Scene(format!("scene target {target:?} has no Transform")))?;
    match channel {
        // 绝对设定（确定性；非累积）。
        "rotation_z" => {
            transform.rotation = bevy::math::Quat::from_rotation_z(value.get() as f32);
            Ok(())
        }
        _ => Err(RuntimeError::Scene(format!(
            "deform channel {channel:?} is not declared (supported: rotation_z)"
        ))),
    }
}

fn apply_move(
    world: &mut World,
    target: &TargetRef,
    translation: &[scenario_model::canon::FiniteF64; 3],
) -> Result<(), RuntimeError> {
    let entity = resolve_target(world, target)?;
    let mut transform = world
        .get_mut::<bevy::prelude::Transform>(entity)
        .ok_or_else(|| RuntimeError::Scene(format!("scene target {target:?} has no Transform")))?;
    transform.translation = bevy::math::Vec3::new(
        translation[0].get() as f32,
        translation[1].get() as f32,
        translation[2].get() as f32,
    );
    Ok(())
}
