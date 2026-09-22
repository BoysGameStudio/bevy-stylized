//! 观测周期 API：为直接持有内容的宿主（集成测试夹具、交互宿主）提供
//! 重复的 settle→observe 周期，统一场景/捕获生命周期（docs/showcase/01
//! line 136：中性捕获探针保留在 harness，统一场景生命周期，断言留在测试）。
//!
//! 内容构建不属于本类型：宿主自行装配/变更世界，[`CaptureCycle`] 只拥有
//! 观测相机、实验时钟与观测回执。每个周期是一组动作图，经
//! [`crate::CaseDriver::run_actions`] 执行；帧语义（与手写循环的逐帧等价
//! 关系）在 [`Settle`] 各变体上注明。

use bevy::camera::Projection;
use bevy::math::Vec3;
use bevy::prelude::{App, Entity, Mut, World};

use golden_harness::{CapturedImage, StampedCapture};
use scenario_model::action::{ActionGraph, ActionKind, ActionNode, Step};
use scenario_model::canon::CanonicalU64;
use scenario_model::ident::{ActionId, SessionId};
use scenes::CameraRecipe;
use scenes::registry::SceneBinding;

use crate::driver::{CaseDriver, DriverLimits, DriverStatus};
use crate::error::RuntimeError;
use crate::headless::run_to_completion;
use crate::observe::ObservationReceipt;

/// 观测前的暖场语义。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settle {
    /// 固定 N 帧暖场（N ≥ 2）：复制落在第 N+1 帧。
    ///
    /// 与手写"N 次 update 后请求、下一帧提取"逐帧一致：首个周期为
    /// `fixed_step{N-2} → observe`（Build 泵 + 调度泵 + N-2 tick = N 帧），
    /// 后续周期为 `fixed_step{N-1} → observe`（相机已存在）。
    Frames(u64),
    /// 管线就绪后再固定 N 帧：复制落在第 N 个连续就绪帧的下一帧。
    ///
    /// 与 `readiness::wait(N)` 后请求逐帧一致：`wait_ready → fixed_step{N-1}
    /// → observe`（隐式就绪等待开启；宿主须已安装 readiness 探针）。
    ReadinessThenFrames(u64),
    /// 立即观测：下一帧提取（与"请求 → 下一帧"手写循环一致）。
    Immediate,
}

fn noop_scene(_: &mut World) -> CameraRecipe {
    CameraRecipe::new(Vec3::ZERO, Vec3::ZERO)
}

const NOOP_BINDING: SceneBinding = SceneBinding {
    id: "host-bound",
    symbol: "cycle::noop_scene",
    build: noop_scene,
    view: None,
};

fn action(id: &str, depends_on: &[&str], kind: ActionKind) -> ActionNode {
    ActionNode {
        id: ActionId::new(id).expect("action id"),
        tick: CanonicalU64(0),
        depends_on: depends_on
            .iter()
            .map(|dep| ActionId::new(*dep).expect("dep id"))
            .collect(),
        writes: Vec::new(),
        kind,
    }
}

fn observe_node(depends_on: &[&str]) -> ActionNode {
    action(
        "observe",
        depends_on,
        ActionKind::Observe {
            observation: "capture".to_string(),
            view: None,
        },
    )
}

fn settle_graph(settle: Settle, first_cycle: bool) -> ActionGraph {
    let nodes = match settle {
        Settle::Frames(frames) => {
            let ticks = if first_cycle {
                frames.saturating_sub(2)
            } else {
                frames.saturating_sub(1)
            };
            if ticks == 0 {
                vec![observe_node(&[])]
            } else {
                vec![
                    action(
                        "settle",
                        &[],
                        ActionKind::FixedStep {
                            ticks: CanonicalU64(ticks),
                            step: Step::new(1, 60),
                        },
                    ),
                    observe_node(&["settle"]),
                ]
            }
        }
        Settle::ReadinessThenFrames(frames) => {
            let mut nodes = vec![action(
                "wait",
                &[],
                ActionKind::WaitReady {
                    condition: scenario_model::action::ReadyCondition::PipelinesReady,
                },
            )];
            if frames > 1 {
                nodes.push(action(
                    "settle",
                    &["wait"],
                    ActionKind::FixedStep {
                        ticks: CanonicalU64(frames - 1),
                        step: Step::new(1, 60),
                    },
                ));
                nodes.push(observe_node(&["settle"]));
            } else {
                nodes.push(observe_node(&["wait"]));
            }
            nodes
        }
        Settle::Immediate => vec![observe_node(&[])],
    };
    ActionGraph { nodes }
}

/// 重复 settle→observe 周期（每 App 一台观测相机）。
pub struct CaptureCycle {
    driver: Option<CaseDriver>,
    session: SessionId,
    recipe: CameraRecipe,
    resolution: (u32, u32),
    limits: DriverLimits,
    budget: usize,
}

impl CaptureCycle {
    /// 以显式相机配方与捕获分辨率创建（内容不在本类型生命周期内；
    /// driver 在首个周期时构建，账本基线即此时点）。
    ///
    /// `session` 是这一次 App/Device 生命周期的身份；`budget` 是单个周期
    /// 的最大泵送数。`implicit_readiness` 对应 [`Settle::ReadinessThenFrames`]
    /// 的就绪等待（宿主须已安装 readiness 探针）。
    #[must_use]
    pub fn new(
        session: SessionId,
        recipe: CameraRecipe,
        resolution: (u32, u32),
        implicit_readiness: bool,
        budget: usize,
    ) -> Self {
        Self {
            driver: None,
            session,
            recipe,
            resolution,
            limits: DriverLimits {
                implicit_readiness,
                ..DriverLimits::default()
            },
            budget,
        }
    }

    /// 开始一个周期（装配/重装动作图；driver 在首个周期时构建，
    /// 账本基线即此时点）。之后由宿主逐帧 [`CaptureCycle::pump`]。
    pub fn begin_capture(&mut self, world: &mut World, settle: Settle) -> Result<(), RuntimeError> {
        match &mut self.driver {
            Some(driver) => driver.run_actions(settle_graph(settle, false)),
            None => {
                self.driver = Some(
                    CaseDriver::new(
                        world,
                        self.session.clone(),
                        NOOP_BINDING,
                        settle_graph(settle, true),
                        self.limits,
                    )?
                    .with_camera_recipe(self.recipe.clone())
                    .with_resolution(self.resolution.0, self.resolution.1),
                );
                Ok(())
            }
        }
    }

    /// 非阻塞推进一步（世界边界；宿主每帧调用一次并自行推进渲染）。
    pub fn pump(&mut self, world: &mut World) -> crate::driver::DriverStatus {
        self.driver
            .as_mut()
            .expect("begin_capture must be called first")
            .update(world)
    }

    /// 取走本周期的观测回执（Done 之后调用）。
    pub fn take_receipt(&mut self) -> Result<ObservationReceipt, RuntimeError> {
        self.driver
            .as_mut()
            .expect("begin_capture must be called first")
            .drain_observations()
            .pop()
            .ok_or(RuntimeError::Scene(
                "capture cycle produced no observation".into(),
            ))
    }

    /// 执行一个周期，返回带回执的捕获。
    pub fn capture(
        &mut self,
        app: &mut App,
        settle: Settle,
    ) -> Result<StampedCapture, RuntimeError> {
        Ok(self.capture_receipt(app, settle)?.into())
    }

    /// 执行一个周期并保留完整回执（身份检查用）。
    pub fn capture_receipt(
        &mut self,
        app: &mut App,
        settle: Settle,
    ) -> Result<ObservationReceipt, RuntimeError> {
        self.begin_capture(app.world_mut(), settle)?;
        let driver = self.driver.as_mut().expect("driver armed");
        run_to_completion(app, driver, self.budget)?;
        self.take_receipt()
    }

    /// 更新观测投影（与"销毁旧相机、以新投影重建"渲染等价：
    /// 组件级替换，其余相机配置不变）。
    pub fn set_projection(&mut self, world: &mut World, projection: Projection) {
        let entity = self
            .observation_entity()
            .expect("observation camera exists after the first cycle");
        world.entity_mut(entity).insert(projection);
    }

    /// 观测相机实体。
    #[must_use]
    pub fn observation_entity(&self) -> Option<Entity> {
        self.driver
            .as_ref()
            .and_then(CaseDriver::observation_entity)
    }

    /// 底层 driver（实验时钟/账本等只读访问；首个周期前为 None）。
    #[must_use]
    pub fn driver(&self) -> Option<&CaseDriver> {
        self.driver.as_ref()
    }
}

impl From<ObservationReceipt> for StampedCapture {
    fn from(receipt: ObservationReceipt) -> Self {
        StampedCapture {
            image: receipt.image,
            stamp: receipt.stamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 帧语义算术（固定 N 帧暖场；见 [`Settle::Frames`] 文档）。
    #[test]
    fn settle_frames_graph_shapes() {
        let first = settle_graph(Settle::Frames(24), true);
        assert_eq!(first.nodes.len(), 2);
        let later = settle_graph(Settle::Frames(24), false);
        assert_eq!(later.nodes.len(), 2);
        match &first.nodes[0].kind {
            ActionKind::FixedStep { ticks, .. } => assert_eq!(ticks.0, 22),
            other => panic!("first cycle must start with fixed_step: {other:?}"),
        }
        match &later.nodes[0].kind {
            ActionKind::FixedStep { ticks, .. } => assert_eq!(ticks.0, 23),
            other => panic!("later cycles must start with fixed_step: {other:?}"),
        }
        let tiny = settle_graph(Settle::Frames(2), true);
        assert_eq!(
            tiny.nodes.len(),
            1,
            "2-frame settle degenerates to observe-only"
        );
    }

    #[test]
    fn readiness_settle_shapes() {
        let graph = settle_graph(Settle::ReadinessThenFrames(24), false);
        assert_eq!(graph.nodes.len(), 3);
        assert!(matches!(graph.nodes[0].kind, ActionKind::WaitReady { .. }));
        match &graph.nodes[1].kind {
            ActionKind::FixedStep { ticks, .. } => assert_eq!(ticks.0, 23),
            other => panic!("readiness settle must step N-1: {other:?}"),
        }
    }
}

/// 每 App 一个捕获周期的资源键（内容自理的宿主/测试夹具用）。
#[derive(bevy::prelude::Resource, bevy::prelude::Deref, bevy::prelude::DerefMut)]
pub struct CycleResource(pub CaptureCycle);

/// 资源化观测周期：确保周期存在（首次以给定配方/分辨率创建）→
/// begin → 泵送（宿主 App 的渲染推进）→ 取回图像。
///
/// 重复调用复用同一台观测相机；投影变化用 [`set_cycle_projection`]
/// （与"销毁旧相机、以新投影重建"渲染等价）。超时/失败返回结构化错误。
pub fn capture_in_app(
    app: &mut App,
    session: &SessionId,
    recipe: &CameraRecipe,
    resolution: (u32, u32),
    settle: Settle,
    implicit_readiness: bool,
    budget: usize,
) -> Result<CapturedImage, RuntimeError> {
    if !app.world().contains_resource::<CycleResource>() {
        app.insert_resource(CycleResource(CaptureCycle::new(
            session.clone(),
            recipe.clone(),
            resolution,
            implicit_readiness,
            budget,
        )));
    }
    app.world_mut()
        .resource_scope(|world, mut cycle: Mut<CycleResource>| {
            cycle.begin_capture(world, settle)
        })?;
    let mut done = false;
    for _ in 0..budget {
        let status = app
            .world_mut()
            .resource_scope(|world, mut cycle: Mut<CycleResource>| cycle.pump(world));
        match status {
            DriverStatus::Running => app.update(),
            DriverStatus::Done => {
                done = true;
                break;
            }
            DriverStatus::Failed(error) => return Err(error),
        }
    }
    if !done {
        return Err(RuntimeError::PumpBudgetExhausted);
    }
    let receipt = app
        .world_mut()
        .resource_scope(|_world, mut cycle: Mut<CycleResource>| cycle.take_receipt())?;
    Ok(receipt.image)
}

/// 更新资源化周期的观测投影（见 [`capture_in_app`]）。
pub fn set_cycle_projection(app: &mut App, projection: Projection) {
    app.world_mut()
        .resource_scope(|world, mut cycle: Mut<CycleResource>| {
            if cycle.observation_entity().is_some() {
                cycle.set_projection(world, projection);
            }
        });
}
