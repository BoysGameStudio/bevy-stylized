//! 平台无关的会话控制器（docs/showcase/01：共享应用层通过命令/观测接口
//! 调用 runtime；布局和输入适配不另建业务逻辑）。
//!
//! [`SessionController`] 拥有：目录浏览（`scenario-model` catalog × scenes
//! 注册表）、用例切换命令（prev/next/goto）、会话限制报告（tier/资产根
//! 拒绝，R2）与观测访问。命令是**数据**（[`SessionCommand`]），不携带
//! winit/egui/触摸等输入语义——桌面宿主把 KeyCode 翻译成命令，未来
//! Android 宿主把触摸翻译成命令，业务逻辑只有这一份。
//!
//! 会话限制规则（与 G1 交互模式一致）：
//!
//! - 用例声明的 tier 需求高于会话已解析上限 → [`RefusalReason::TierRequired`]
//!   （已解析能力不能在同一会话内上调，需要新会话）；
//! - 用例声明了资产依赖而会话资产根在启动时固定 →
//!   [`RefusalReason::AssetRootFixed`]；
//! - 切换时重置视图为会话基线（上一用例的视图不泄漏），再应用目标用例
//!   的内容级 preset；持久化（保存/重载/录制/回放）属于后续阶段。

use bevy::prelude::World;
use serde::{Deserialize, Serialize};

use bevy_stylized::prelude::StylizedExecutionTier;
use bevy_stylized::prelude::{StylizedViewDescriptor, StylizedViewSettings};
use scenario_model::ident::{Generation, SceneId};
use scenes::registry::{SceneBinding, ViewPresetFn, bindings};

use crate::driver::{CaseDriver, DriverStatus};
use crate::error::RuntimeError;

/// 会话命令（数据；输入设备适配归宿主翻译）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SessionCommand {
    /// 下一个用例（循环）。
    NextCase,
    /// 上一个用例（循环）。
    PrevCase,
    /// 按 SceneId 跳转。
    SelectCase(SceneId),
}

/// 目录条目（浏览视图；场景目录元数据 + 绑定）。
#[derive(Clone, Debug)]
pub struct CaseEntry {
    /// 场景 ID。
    pub id: SceneId,
    /// 摘要（catalog）。
    pub summary: String,
    /// 最低 tier 需求（catalog 声明）。
    pub tier: Option<StylizedExecutionTier>,
    /// 是否声明了资产依赖（会话资产根固定时不可切换）。
    pub has_assets: bool,
    /// 内容级视图 preset。
    pub view: Option<ViewPresetFn>,
    /// 构建绑定。
    pub binding: SceneBinding,
}

/// 会话限制/命令拒绝原因（结构化报告，不静默忽略）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum RefusalReason {
    /// tier 需求超过会话已解析上限。
    TierRequired {
        /// 用例要求。
        required: StylizedExecutionTier,
        /// 会话已解析上限。
        resolved: StylizedExecutionTier,
    },
    /// 用例需要固定/不同的资产根。
    AssetRootFixed(SceneId),
    /// 未知场景 ID。
    UnknownScene(SceneId),
    /// 当前状态不允许切换（driver 未到可切换终态）。
    DriverBusy(String),
}

/// 命令的纯评估结果（不接触世界；CPU 可测）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandPlan {
    /// 切换到目标下标。
    Switch(usize),
    /// 结构化拒绝。
    Refused(RefusalReason),
}

/// 命令执行结果。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum CommandOutcome {
    /// 已切换（目标用例与新代次）。
    Switched {
        /// 目标用例。
        case: SceneId,
        /// 新代次。
        generation: Generation,
    },
    /// 被拒绝（原因结构化；当前场景保留）。
    Refused(RefusalReason),
}

/// 平台无关的会话控制器。
pub struct SessionController {
    driver: CaseDriver,
    cases: Vec<CaseEntry>,
    current: usize,
    base_view: StylizedViewDescriptor,
    tier_ceiling: Option<StylizedExecutionTier>,
}

impl SessionController {
    /// 以 catalog × 注册表构建目录（顺序与 catalog 一致），接管一个已
    /// 构造的 driver（初始用例由其场景绑定决定）。`base_view` 是切换时
    /// 恢复的会话视图基线（宿主配置）。
    pub fn from_catalog(
        driver: CaseDriver,
        current: usize,
        base_view: StylizedViewDescriptor,
    ) -> Result<Self, RuntimeError> {
        let catalog = scenario_model::catalog();
        let registry = bindings();
        if catalog.len() != registry.len() {
            return Err(RuntimeError::Scene(
                "catalog and binding table disagree in length".into(),
            ));
        }
        let cases = catalog
            .into_iter()
            .zip(registry)
            .map(|(entry, binding)| CaseEntry {
                id: entry.id.clone(),
                summary: entry.summary.clone(),
                tier: entry.tier,
                has_assets: !entry.assets.is_empty(),
                view: binding.view.map(|(_, preset)| preset),
                binding,
            })
            .collect();
        Ok(Self {
            driver,
            cases,
            current,
            base_view,
            tier_ceiling: None,
        })
    }

    /// 目录（浏览；顺序 = catalog 顺序）。
    #[must_use]
    pub fn cases(&self) -> &[CaseEntry] {
        &self.cases
    }

    /// 当前用例。
    #[must_use]
    pub fn current(&self) -> &CaseEntry {
        &self.cases[self.current]
    }

    /// 设置会话已解析 tier 上限（宿主在能力解析后提供；R2）。
    pub fn set_tier_ceiling(&mut self, ceiling: StylizedExecutionTier) {
        self.tier_ceiling = Some(ceiling);
    }

    /// 底层 driver（观测回执/实验时钟/账本只读访问）。
    #[must_use]
    pub fn driver(&self) -> &CaseDriver {
        &self.driver
    }

    /// 底层 driver（泵送/动作序列执行等宿主驱动操作）。
    pub fn driver_mut(&mut self) -> &mut CaseDriver {
        &mut self.driver
    }

    /// 纯命令评估：目标与合法性（不接触世界）。
    #[must_use]
    pub fn evaluate(&self, command: &SessionCommand) -> CommandPlan {
        let target = match command {
            SessionCommand::NextCase => (self.current + 1) % self.cases.len(),
            SessionCommand::PrevCase => (self.current + self.cases.len() - 1) % self.cases.len(),
            SessionCommand::SelectCase(id) => {
                let Some(index) = self.cases.iter().position(|entry| entry.id == *id) else {
                    return CommandPlan::Refused(RefusalReason::UnknownScene(id.clone()));
                };
                index
            }
        };
        let entry = &self.cases[target];
        let ceiling = self.tier_ceiling.unwrap_or_default();
        if let Some(required) = entry.tier
            && required > ceiling
        {
            return CommandPlan::Refused(RefusalReason::TierRequired {
                required,
                resolved: ceiling,
            });
        }
        if entry.has_assets {
            return CommandPlan::Refused(RefusalReason::AssetRootFixed(entry.id.clone()));
        }
        CommandPlan::Switch(target)
    }

    /// 执行命令：合法性评估 → 视图基线重置 → 账本切换。
    pub fn command(&mut self, world: &mut World, command: &SessionCommand) -> CommandOutcome {
        let plan = self.evaluate(command);
        let CommandPlan::Switch(target) = plan else {
            let CommandPlan::Refused(reason) = plan else {
                unreachable!("CommandPlan is Switch or Refused")
            };
            return CommandOutcome::Refused(reason);
        };
        // 重置视图为会话基线（上一用例的视图不泄漏）→ 应用目标用例 preset。
        *world.resource_mut::<StylizedViewSettings>() =
            StylizedViewSettings(self.base_view.clone());
        self.driver.set_view(self.cases[target].view);
        match self.driver.switch_scene(world, self.cases[target].binding) {
            DriverStatus::Running => {
                self.current = target;
                CommandOutcome::Switched {
                    case: self.cases[target].id.clone(),
                    generation: self.driver.generation(),
                }
            }
            DriverStatus::Failed(error) => {
                CommandOutcome::Refused(RefusalReason::DriverBusy(error.to_string()))
            }
            DriverStatus::Done => unreachable!("switch_scene never returns Done"),
        }
    }

    /// 非阻塞推进一步（宿主每帧调用一次并自行推进渲染）。
    pub fn update(&mut self, world: &mut World) -> DriverStatus {
        self.driver.update(world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::DriverLimits;
    use scenario_model::action::ActionGraph;
    use scenario_model::ident::SessionId;

    fn controller() -> SessionController {
        let app = bevy::prelude::App::new();
        let driver = CaseDriver::new(
            app.world(),
            SessionId::new("controller-test").expect("session id"),
            bindings().into_iter().next().expect("binding"),
            ActionGraph::default(),
            DriverLimits::default(),
        )
        .expect("plan");
        SessionController::from_catalog(driver, 0, StylizedViewDescriptor::default())
            .expect("controller")
    }

    #[test]
    fn catalog_order_matches_registry_and_scenario_catalog() {
        let controller = controller();
        assert_eq!(controller.cases().len(), 17);
        let ids: Vec<&str> = controller
            .cases()
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "overview",
                "ramp",
                "lighting",
                "face",
                "hair",
                "metal",
                "outline",
                "animation",
                "post",
                "stress",
                "reference",
                "performance",
                "outline-advanced",
                "lod",
                "environment-day",
                "environment-dusk",
                "environment-night",
            ]
        );
    }

    #[test]
    fn evaluate_goto_next_prev_and_unknown() {
        let mut controller = controller();
        assert_eq!(
            controller.evaluate(&SessionCommand::NextCase),
            CommandPlan::Switch(1)
        );
        controller.current = 16;
        assert_eq!(
            controller.evaluate(&SessionCommand::NextCase),
            CommandPlan::Switch(0)
        );
        controller.current = 0;
        assert_eq!(
            controller.evaluate(&SessionCommand::PrevCase),
            CommandPlan::Switch(16)
        );
        assert_eq!(
            controller.evaluate(&SessionCommand::SelectCase(
                SceneId::new("outline").expect("id")
            )),
            CommandPlan::Switch(6)
        );
        assert_eq!(
            controller.evaluate(&SessionCommand::SelectCase(
                SceneId::new("no-such-case").expect("id")
            )),
            CommandPlan::Refused(RefusalReason::UnknownScene(
                SceneId::new("no-such-case").expect("id")
            ))
        );
    }

    #[test]
    fn evaluate_reports_tier_and_asset_root_refusals() {
        let mut controller = controller();
        controller.set_tier_ceiling(StylizedExecutionTier::Portable);
        // outline-advanced 声明 Standard（catalog）；Portable 会话拒绝。
        assert_eq!(
            controller.evaluate(&SessionCommand::SelectCase(
                SceneId::new("outline-advanced").expect("id")
            )),
            CommandPlan::Refused(RefusalReason::TierRequired {
                required: StylizedExecutionTier::Standard,
                resolved: StylizedExecutionTier::Portable,
            })
        );
        // Standard 会话允许。
        controller.set_tier_ceiling(StylizedExecutionTier::Standard);
        assert_eq!(
            controller.evaluate(&SessionCommand::SelectCase(
                SceneId::new("outline-advanced").expect("id")
            )),
            CommandPlan::Switch(12)
        );
        // reference 声明了资产依赖：会话资产根固定 → 拒绝。
        assert_eq!(
            controller.evaluate(&SessionCommand::SelectCase(
                SceneId::new("reference").expect("id")
            )),
            CommandPlan::Refused(RefusalReason::AssetRootFixed(
                SceneId::new("reference").expect("id")
            ))
        );
    }
}
