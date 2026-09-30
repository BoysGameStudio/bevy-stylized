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
//! - 用例声明的 tier 需求高于会话已解析上限 → 先经产品公共重配置 API
//!   （ADR-0030）尝试会话内升档；设备已具备能力则直接切换，设备级缺口
//!   （特性/limits 不在 Stage B 快照中）才拒绝为
//!   [`RefusalReason::TierRequired`]（`reconfiguration_attempted` 区分
//!   "已尝试被设备拒绝——需要新 device 会话"与"无能力快照——无法尝试"）；
//! - 用例声明了资产依赖而会话资产根在启动时固定 →
//!   [`RefusalReason::AssetRootFixed`]；
//! - 切换时重置视图为会话基线（上一用例的视图不泄漏），再应用目标用例
//!   的内容级 preset；持久化（保存/重载/录制/回放）属于后续阶段。

use bevy::math::Vec3;
use bevy::prelude::World;
use serde::{Deserialize, Serialize};

use bevy_stylized::prelude::StylizedExecutionTier;
use bevy_stylized::prelude::{StylizedViewDescriptor, StylizedViewSettings};
use bevy_stylized::reconfiguration::StylizedReconfigurationExt;
use scenario_model::ident::{Generation, SceneId};
use scenes::metadata::COURTYARD_CAMERA_PRESETS;
use scenes::registry::{SceneBinding, ViewPresetFn, bindings};
use scenes::{
    CameraAttachment, CameraRecipe, CourtyardFlagsPose, CourtyardOutlineMode, CourtyardSettings,
    CourtyardTimeOfDay, CourtyardZone, views,
};

use crate::driver::{CaseDriver, DriverStatus};
use crate::error::RuntimeError;

/// 会话命令（数据；输入设备适配归宿主翻译）。
///
/// V1 相机/参数命令（docs/showcase/05 §11.1/§11.6）：机位 preset、自由
/// 相机偏移与参数面。f32 载荷使本类型不再满足 `Eq`。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SessionCommand {
    /// 下一个用例（循环）。
    NextCase,
    /// 上一个用例（循环）。
    PrevCase,
    /// 按 SceneId 跳转。
    SelectCase(SceneId),
    /// 切到机位 preset（§7；当前仅 courtyard 声明机位表）。
    SetCameraPreset(String),
    /// 自由相机：在当前机位上 orbit（yaw/pitch 增量，弧度）。
    CameraOrbit {
        /// 偏航增量。
        yaw: f32,
        /// 俯仰增量。
        pitch: f32,
    },
    /// 自由相机：平移目标点（相机相对系，米）。
    CameraPan {
        /// 右向位移。
        dx: f32,
        /// 上向位移。
        dy: f32,
    },
    /// 自由相机：缩放（factor > 1 拉近，即距离除以 factor）。
    CameraZoom {
        /// 缩放因子。
        factor: f32,
    },
    /// 回到当前 preset 的配方机位（清除 orbit/pan/zoom 偏移）。
    ResetView,
    /// 改场景参数（§8；当前仅 courtyard 声明参数面）。
    SetParameter {
        /// 参数键（catalog 声明）。
        key: String,
        /// 参数值（JSON）。
        value: serde_json::Value,
    },
    /// 交互捕获（docs/showcase/06 §9，U3）：对当前观测相机内容发起一次
    /// ad-hoc 读回（driver [`CaseDriver::observe_adhoc`] 入口，与动作图
    /// `Observe` 同一读回通道）。读回跨帧完成——本命令只发起（结果
    /// Applied），像素回执经 `driver().observations()` 到达，落盘归宿主。
    /// 浏览快照，不参与 golden 判定；构建在途时结构化拒绝（DriverBusy）。
    Capture,
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
        /// 是否已尝试会话内重配置（ADR-0030）：`true` = 设备级缺口，需要
        /// 新 device 会话；`false` = 无能力快照，无法尝试。
        #[serde(default)]
        reconfiguration_attempted: bool,
    },
    /// 用例需要固定/不同的资产根。
    AssetRootFixed(SceneId),
    /// 未知场景 ID。
    UnknownScene(SceneId),
    /// 未知机位 preset（含无机位表的当前用例）。
    UnknownCameraPreset(String),
    /// 未知参数键（当前用例无参数面或未声明该键）。
    UnknownParameter(String),
    /// 参数值非法（类型/范围/枚举取值）。
    InvalidParameter(String),
    /// 相机操作非法（相机未构建、非有限输入、越界）。
    InvalidCamera(String),
    /// 当前状态不允许切换（driver 未到可切换终态）。
    DriverBusy(String),
}

/// 命令的纯评估结果（不接触世界；CPU 可测）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandPlan {
    /// 切换到目标下标。
    Switch(usize),
    /// 作用于当前用例（机位/自由相机/参数；不切换、不改变代次——参数
    /// 触发的内容重建在执行层发生）。
    Local,
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
    /// 已应用（机位/自由相机/视图参数；未切换用例）。
    Applied,
    /// 被拒绝（原因结构化；当前场景保留）。
    Refused(RefusalReason),
}

/// 由 eye/look 推导 orbit 姿态（inspector OrbitCamera 的同一模型：
/// eye = target + (sin yaw · cos pitch, sin pitch, cos yaw · cos pitch) · d）。
fn orbit_pose(eye: Vec3, look: Vec3) -> (f32, f32, f32) {
    let offset = eye - look;
    let distance = offset.length().max(0.001);
    let pitch = (offset.y / distance).clamp(-1.0, 1.0).asin();
    let yaw = offset.x.atan2(offset.z);
    (yaw, pitch, distance)
}

/// 补间缓动（ease-in-out 三次方）。
fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

/// 角度插值（最短路径，弧度）。
fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let mut delta = (to - from) % std::f32::consts::TAU;
    if delta > std::f32::consts::PI {
        delta -= std::f32::consts::TAU;
    } else if delta < -std::f32::consts::PI {
        delta += std::f32::consts::TAU;
    }
    from + delta * t
}

/// 机位补间状态（06 §8；确定性口径：补间是纯视觉插值，不进
/// CommandJournal、不产生 revision；仅由显式 `advance` 推进，默认关闭）。
///
/// 起点以「目标点 + orbit 姿态」记录：插值在目标点线性、姿态球面
/// （yaw 最短路径），终点锚定 CameraState 的当前配方机位。
#[derive(Clone, Debug, PartialEq)]
struct CameraTransition {
    /// 起点目标点。
    from_target: Vec3,
    /// 起点 yaw（弧度）。
    from_yaw: f32,
    /// 起点 pitch（弧度）。
    from_pitch: f32,
    /// 起点距离（米）。
    from_distance: f32,
    /// 已推进秒数。
    elapsed: f32,
    /// 总时长（秒，> 0）。
    duration: f32,
}

impl CameraTransition {
    /// 从当前视图姿态起跳（from 由 eye/look 反解 orbit 姿态）。
    fn new(eye: Vec3, look: Vec3, duration: f32) -> Self {
        let (yaw, pitch, distance) = orbit_pose(eye, look);
        Self {
            from_target: look,
            from_yaw: yaw,
            from_pitch: pitch,
            from_distance: distance,
            elapsed: 0.0,
            duration,
        }
    }

    /// 当前进度（0..=1，缓动后）。
    fn progress(&self) -> f32 {
        ease_in_out((self.elapsed / self.duration).clamp(0.0, 1.0))
    }
}

/// 相机状态（controller 持有；preset 机位 + 自由偏移，§11.3）。
///
/// yaw/pitch/distance 围绕 `target`（look）旋转；`base_*` 是当前 preset
/// 的配方机位（ResetView 的锚点）。场景重建（代次变化）后从构建期配方
/// 重新派生。`transition` 活跃时上述字段是补间终点锚，视图姿态经
/// [`CameraState::view_pose`] 插值呈现。
#[derive(Clone, Debug, PartialEq)]
struct CameraState {
    /// 状态所属代次。
    generation: Generation,
    /// 当前机位 preset（None = 场景默认机位）。
    preset: Option<String>,
    /// reset 锚点。
    base_target: Vec3,
    /// reset 锚点 yaw。
    base_yaw: f32,
    /// reset 锚点 pitch。
    base_pitch: f32,
    /// reset 锚点距离。
    base_distance: f32,
    /// 当前目标点（pan 移动它）。
    target: Vec3,
    /// 当前 yaw（弧度）。
    yaw: f32,
    /// 当前 pitch（弧度，±1.5 内）。
    pitch: f32,
    /// 当前距离（米，> 0.001）。
    distance: f32,
    /// 进行中的机位补间（06 §8；None = 无补间）。
    transition: Option<CameraTransition>,
}

impl CameraState {
    fn from_pose(eye: Vec3, look: Vec3, generation: Generation, preset: Option<String>) -> Self {
        let (yaw, pitch, distance) = orbit_pose(eye, look);
        Self {
            generation,
            preset,
            base_target: look,
            base_yaw: yaw,
            base_pitch: pitch,
            base_distance: distance,
            target: look,
            yaw,
            pitch,
            distance,
            transition: None,
        }
    }

    /// 当前 eye（target + orbit 偏移；终点锚，补间中请用 view_pose）。
    fn eye(&self) -> Vec3 {
        self.target
            + Vec3::new(
                self.yaw.sin() * self.pitch.cos(),
                self.pitch.sin(),
                self.yaw.cos() * self.pitch.cos(),
            ) * self.distance
    }

    /// 呈现用视图姿态（eye, look）：补间中返回插值，否则返回当前姿态。
    fn view_pose(&self) -> (Vec3, Vec3) {
        let Some(transition) = &self.transition else {
            return (self.eye(), self.target);
        };
        let k = transition.progress();
        let target = transition.from_target.lerp(self.target, k);
        let yaw = lerp_angle(transition.from_yaw, self.yaw, k);
        let pitch = transition.from_pitch + (self.pitch - transition.from_pitch) * k;
        let distance =
            transition.from_distance + (self.distance - transition.from_distance) * k;
        let eye = target
            + Vec3::new(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos()) * distance;
        (eye, target)
    }

    /// 把当前呈现姿态烙进锚点字段并清除补间（导航命令抢占补间：从
    /// 补间中间态继续，06 §8）。
    fn bake_view_pose(&mut self) {
        if self.transition.is_none() {
            return;
        }
        let (eye, target) = self.view_pose();
        let (yaw, pitch, distance) = orbit_pose(eye, target);
        self.target = target;
        self.yaw = yaw;
        self.pitch = pitch;
        self.distance = distance;
        self.transition = None;
    }
}

/// 当前相机状态（只读视图；供宿主窗口相机统一等消费，§11.3 第一条）。
#[derive(Clone, Debug, PartialEq)]
pub struct CameraViewInfo {
    /// 当前机位 preset（None = 场景默认机位）。
    pub preset: Option<String>,
    /// 当前 eye。
    pub eye: Vec3,
    /// 当前 look（目标点）。
    pub look: Vec3,
    /// 当前 yaw（弧度）。
    pub yaw: f32,
    /// 当前 pitch（弧度）。
    pub pitch: f32,
    /// 当前距离（米）。
    pub distance: f32,
}

/// 平台无关的会话控制器。
pub struct SessionController {
    driver: CaseDriver,
    cases: Vec<CaseEntry>,
    current: usize,
    base_view: StylizedViewDescriptor,
    tier_ceiling: Option<StylizedExecutionTier>,
    camera: Option<CameraState>,
    /// 机位补间时长（秒；None = 关闭，默认关闭——捕获/冒烟路径零行为
    /// 变化；交互窗口由宿主显式开启并逐帧 `advance_camera_transition`
    /// 推进，06 §8 确定性口径）。
    camera_transition: Option<f32>,
    /// 延迟退役的旧区展品（W3：先装后卸——补间落定后才 despawn，
    /// 05 §12 W1–W3 时序规则；补间关闭时命令路径立即退役，本列表恒空）。
    pending_retire: Vec<bevy::prelude::Entity>,
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
            camera: None,
            camera_transition: None,
            pending_retire: Vec::new(),
        })
    }

    /// 设置机位补间时长（秒；None = 关闭）。默认关闭：捕获/冒烟路径
    /// 保持瞬跳与确定性；交互宿主开启后须在每帧命令执行前调用
    /// [`SessionController::advance_camera_transition`] 推进（06 §8）。
    pub fn set_camera_transition(&mut self, duration: Option<f32>) {
        self.camera_transition = duration.filter(|duration| *duration > 0.0);
        if self.camera_transition.is_none()
            && let Some(state) = &mut self.camera
        {
            state.transition = None;
        }
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
        match command {
            SessionCommand::SetCameraPreset(preset) => {
                // 当前仅 courtyard 声明机位表（§7）。
                if self.current().id.as_str() == "courtyard"
                    && COURTYARD_CAMERA_PRESETS
                        .iter()
                        .any(|entry| entry.id == preset)
                {
                    return CommandPlan::Local;
                }
                return CommandPlan::Refused(RefusalReason::UnknownCameraPreset(preset.clone()));
            }
            SessionCommand::CameraOrbit { yaw, pitch } => {
                if !yaw.is_finite() || !pitch.is_finite() {
                    return CommandPlan::Refused(RefusalReason::InvalidCamera(
                        "orbit delta must be finite".into(),
                    ));
                }
                // pitch 越界校验在已同步的相机状态上才可行；状态缺失/过期时
                // 由执行层在同步后校验。
                if let Some(state) = &self.camera
                    && state.generation == self.driver.generation()
                    && (state.pitch + pitch).abs() > 1.5
                {
                    return CommandPlan::Refused(RefusalReason::InvalidCamera(
                        "pitch beyond ±1.5 rad".into(),
                    ));
                }
                return CommandPlan::Local;
            }
            SessionCommand::CameraPan { dx, dy } => {
                if !dx.is_finite() || !dy.is_finite() {
                    return CommandPlan::Refused(RefusalReason::InvalidCamera(
                        "pan delta must be finite".into(),
                    ));
                }
                return CommandPlan::Local;
            }
            SessionCommand::CameraZoom { factor } => {
                if !factor.is_finite() || *factor <= 0.0 {
                    return CommandPlan::Refused(RefusalReason::InvalidCamera(
                        "zoom factor must be finite and positive".into(),
                    ));
                }
                if let Some(state) = &self.camera
                    && state.generation == self.driver.generation()
                    && state.distance / factor <= 0.001
                {
                    return CommandPlan::Refused(RefusalReason::InvalidCamera(
                        "zoomed distance must stay above 0.001 m".into(),
                    ));
                }
                return CommandPlan::Local;
            }
            SessionCommand::ResetView => return CommandPlan::Local,
            SessionCommand::Capture => {
                // ad-hoc 读回要求 driver 空闲（Done；观测相机已构建）。
                // 构建在途/失败 = DriverBusy（结构化拒绝，当前场景保留）。
                return if matches!(self.driver.status(), DriverStatus::Done) {
                    CommandPlan::Local
                } else {
                    CommandPlan::Refused(RefusalReason::DriverBusy(
                        "capture requires an idle driver (build finished)".into(),
                    ))
                };
            }
            SessionCommand::SetParameter { key, value } => {
                return self.evaluate_parameter(key, value);
            }
            SessionCommand::NextCase | SessionCommand::PrevCase | SessionCommand::SelectCase(_) => {
            }
        }
        let target = match command {
            SessionCommand::NextCase => (self.current + 1) % self.cases.len(),
            SessionCommand::PrevCase => (self.current + self.cases.len() - 1) % self.cases.len(),
            SessionCommand::SelectCase(id) => {
                let Some(index) = self.cases.iter().position(|entry| entry.id == *id) else {
                    return CommandPlan::Refused(RefusalReason::UnknownScene(id.clone()));
                };
                index
            }
            _ => unreachable!("local commands returned above"),
        };
        let entry = &self.cases[target];
        let ceiling = self.tier_ceiling.unwrap_or_default();
        if let Some(required) = entry.tier
            && required > ceiling
        {
            return CommandPlan::Refused(RefusalReason::TierRequired {
                required,
                resolved: ceiling,
                reconfiguration_attempted: false,
            });
        }
        if entry.has_assets {
            return CommandPlan::Refused(RefusalReason::AssetRootFixed(entry.id.clone()));
        }
        CommandPlan::Switch(target)
    }

    /// 执行命令：合法性评估 →（tier 拒绝时先尝试会话内重配置，R2/ADR-0030）
    /// → 视图基线重置 → 账本切换。
    pub fn command(&mut self, world: &mut World, command: &SessionCommand) -> CommandOutcome {
        let mut reconfiguration_attempted = false;
        let mut plan = self.evaluate(command);
        // R2/ADR-0030：tier 拒绝先经产品公共 API 尝试会话内重配置；设备
        // 已具备能力时无需新会话。请求不低于用例需求档，保留会话其余配置
        // （class/fallback policy/Stage A 特性断言不变）。
        if let CommandPlan::Refused(RefusalReason::TierRequired { required, .. }) = plan
            && let Some(mut config) = world
                .get_resource::<bevy_stylized::StylizedCapabilityConfig>()
                .cloned()
        {
            reconfiguration_attempted = true;
            if config.requested_tier < required {
                config.requested_tier = required;
            }
            // 设备级缺口（Err）：保留原拒绝，由下方标记 attempted。
            if let Ok(applied) = world.apply_stylized_reconfiguration(config) {
                self.tier_ceiling = Some(applied.current.tier);
                plan = self.evaluate(command);
            }
        }
        let target = match plan {
            CommandPlan::Switch(target) => target,
            CommandPlan::Local => return self.execute_local(world, command),
            CommandPlan::Refused(mut reason) => {
                if let RefusalReason::TierRequired {
                    reconfiguration_attempted: attempted,
                    ..
                } = &mut reason
                {
                    *attempted = reconfiguration_attempted;
                }
                return CommandOutcome::Refused(reason);
            }
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

    /// 参数命令的纯评估（catalog schema 校验 + tier 门控）。
    fn evaluate_parameter(&self, key: &str, value: &serde_json::Value) -> CommandPlan {
        if self.current().id.as_str() != "courtyard" {
            // 当前仅 courtyard 声明参数面（§8）。
            return CommandPlan::Refused(RefusalReason::UnknownParameter(key.to_string()));
        }
        let entry = scenario_model::catalog_entry("courtyard").expect("catalog entry");
        let Some(spec) = entry.parameters.iter().find(|spec| spec.name == key) else {
            return CommandPlan::Refused(RefusalReason::UnknownParameter(key.to_string()));
        };
        if let Err(error) = spec.validate_value(key, value) {
            return CommandPlan::Refused(RefusalReason::InvalidParameter(error.to_string()));
        }
        // outline_mode=hybrid 需 Standard（§8；与用例切换同一 tier 拒绝路径，
        // 会话内重配置尝试在 command() 的既有块内发生）。
        let ceiling = self.tier_ceiling.unwrap_or_default();
        if key == "outline_mode"
            && value.as_str() == Some("hybrid")
            && ceiling < StylizedExecutionTier::Standard
        {
            return CommandPlan::Refused(RefusalReason::TierRequired {
                required: StylizedExecutionTier::Standard,
                resolved: ceiling,
                reconfiguration_attempted: false,
            });
        }
        CommandPlan::Local
    }

    /// 同步相机状态：代次变化（场景构建/重建）后从构建期配方重派生，
    /// preset 复位为场景默认机位。补间开启且有旧姿态时，从旧视图姿态
    /// 起跳到新机位（06 §8；重建跨帧推进，起跳点 = 构建完成一刻的旧
    /// 呈现姿态）。
    fn sync_camera_state(&mut self) -> Result<(), RefusalReason> {
        let generation = self.driver.generation();
        let stale = match &self.camera {
            Some(state) => state.generation != generation,
            None => true,
        };
        if stale {
            let Some(recipe) = self.driver.built_recipe().cloned() else {
                return Err(RefusalReason::InvalidCamera(
                    "observation camera not built yet".into(),
                ));
            };
            let transition = match (&self.camera, self.camera_transition) {
                (Some(old), Some(duration)) => {
                    let (eye, look) = old.view_pose();
                    Some(CameraTransition::new(eye, look, duration))
                }
                _ => None,
            };
            let mut state =
                CameraState::from_pose(recipe.eye, recipe.look, generation, None);
            state.transition = transition;
            self.camera = Some(state);
        }
        Ok(())
    }

    /// 推进机位补间（宿主每帧驱动；补间关闭/无补间时为空操作）。
    /// 推进后对观测相机应用当前呈现姿态（窗口/捕获相机同源，
    /// §11.3 第一条）；到达终点清除补间并落定锚点姿态。
    ///
    /// W3 先装后卸：延迟退役的旧区展品在「无活跃补间」时冲刷（补间
    /// 落定、从未起跳——如相机状态尚未惰性同步、或补间被关闭）。
    pub fn advance_camera_transition(&mut self, world: &mut World, dt: f32) {
        let mut synced = false;
        if self.camera_transition.is_some() && dt > 0.0 && self.sync_camera_state().is_ok() {
            synced = true;
            let finished = {
                let state = self.camera.as_mut().expect("camera state synced");
                match &mut state.transition {
                    Some(transition) => {
                        transition.elapsed += dt;
                        transition.elapsed >= transition.duration
                    }
                    // 无补间（如惰性同步前没有旧姿态可起跳）——落到下方
                    // 共享的退役冲刷判定。
                    None => false,
                }
            };
            if finished {
                let state = self.camera.as_mut().expect("camera state synced");
                state.transition = None;
            }
        }
        // 退役冲刷：无活跃补间（落定/从未起跳/补间关闭）即退役旧区展品
        // （05 §12 W1–W3 时序规则：补间起跳视野含旧区，提前退役 = 展品凭空消失）。
        let no_active_transition = self
            .camera
            .as_ref()
            .is_none_or(|state| state.transition.is_none());
        if no_active_transition && !self.pending_retire.is_empty() {
            let pending = std::mem::take(&mut self.pending_retire);
            scenes::retire_entities(world, &pending);
        }
        if synced {
            self.apply_camera_state(world);
        }
    }

    /// 把当前相机状态应用到观测相机（附件保持：附件由参数驱动，与机位
    /// 正交，§11.1）。补间中应用呈现姿态（窗口/捕获相机同源）。
    fn apply_camera_state(&mut self, world: &mut World) -> CommandOutcome {
        let state = self.camera.clone().expect("camera state synced");
        let attachment = self
            .driver
            .applied_recipe()
            .map(|recipe| recipe.attachment.clone())
            .unwrap_or(CameraAttachment::None);
        let (eye, look) = state.view_pose();
        let recipe = CameraRecipe {
            eye,
            look,
            projection: None,
            attachment,
        };
        match self.driver.apply_camera_recipe(world, &recipe) {
            Ok(()) => CommandOutcome::Applied,
            Err(error) => CommandOutcome::Refused(RefusalReason::InvalidCamera(error.to_string())),
        }
    }

    /// 执行 Local 命令（机位/自由相机/参数；evaluate 已确认合法性）。
    fn execute_local(&mut self, world: &mut World, command: &SessionCommand) -> CommandOutcome {
        match command {
            SessionCommand::SetCameraPreset(preset) => {
                if let Err(reason) = self.sync_camera_state() {
                    return CommandOutcome::Refused(reason);
                }
                let entry = COURTYARD_CAMERA_PRESETS
                    .iter()
                    .find(|entry| entry.id == preset)
                    .expect("evaluate checked the preset");
                let state = self.camera.as_mut().expect("camera state synced");
                // 补间开启时从当前呈现姿态起跳到目标 preset（06 §8）。
                let transition = self.camera_transition.map(|duration| {
                    let (eye, look) = state.view_pose();
                    CameraTransition::new(eye, look, duration)
                });
                state.preset = Some(preset.clone());
                let (yaw, pitch, distance) = orbit_pose(entry.eye, entry.look);
                state.base_target = entry.look;
                state.base_yaw = yaw;
                state.base_pitch = pitch;
                state.base_distance = distance;
                state.target = entry.look;
                state.yaw = yaw;
                state.pitch = pitch;
                state.distance = distance;
                state.transition = transition;
                self.apply_camera_state(world)
            }
            SessionCommand::CameraOrbit { yaw, pitch } => {
                if let Err(reason) = self.sync_camera_state() {
                    return CommandOutcome::Refused(reason);
                }
                let state = self.camera.as_mut().expect("camera state synced");
                // 自由相机命令抢占进行中的补间（从补间中间态继续，06 §8）。
                state.bake_view_pose();
                let new_pitch = state.pitch + pitch;
                if new_pitch.abs() > 1.5 {
                    return CommandOutcome::Refused(RefusalReason::InvalidCamera(
                        "pitch beyond ±1.5 rad".into(),
                    ));
                }
                state.yaw += yaw;
                state.pitch = new_pitch;
                self.apply_camera_state(world)
            }
            SessionCommand::CameraPan { dx, dy } => {
                if let Err(reason) = self.sync_camera_state() {
                    return CommandOutcome::Refused(reason);
                }
                let state = self.camera.as_mut().expect("camera state synced");
                state.bake_view_pose();
                let forward = (state.target - state.eye()).normalize_or_zero();
                let right = forward.cross(Vec3::Y).normalize_or_zero();
                let up = right.cross(forward);
                state.target += right * dx + up * dy;
                self.apply_camera_state(world)
            }
            SessionCommand::CameraZoom { factor } => {
                if let Err(reason) = self.sync_camera_state() {
                    return CommandOutcome::Refused(reason);
                }
                let state = self.camera.as_mut().expect("camera state synced");
                state.bake_view_pose();
                let new_distance = state.distance / factor;
                if new_distance <= 0.001 {
                    return CommandOutcome::Refused(RefusalReason::InvalidCamera(
                        "zoomed distance must stay above 0.001 m".into(),
                    ));
                }
                state.distance = new_distance;
                self.apply_camera_state(world)
            }
            SessionCommand::ResetView => {
                if let Err(reason) = self.sync_camera_state() {
                    return CommandOutcome::Refused(reason);
                }
                let state = self.camera.as_mut().expect("camera state synced");
                state.bake_view_pose();
                state.target = state.base_target;
                state.yaw = state.base_yaw;
                state.pitch = state.base_pitch;
                state.distance = state.base_distance;
                self.apply_camera_state(world)
            }
            SessionCommand::SetParameter { key, value } => self.apply_parameter(world, key, value),
            SessionCommand::Capture => {
                // 读回跨帧完成：发起 = Applied；像素回执经 observations() 到达。
                match self.driver.observe_adhoc(world, "interactive-capture") {
                    Ok(()) => CommandOutcome::Applied,
                    Err(error) => {
                        CommandOutcome::Refused(RefusalReason::DriverBusy(error.to_string()))
                    }
                }
            }
            _ => unreachable!("evaluate dispatches only local commands here"),
        }
    }

    /// 应用 courtyard 参数（§8/§11.2）。
    ///
    /// 运行时语义分层（05 §11.2 目标态，W1/W2 落地）：time_of_day live
    /// （视图 preset）；post/outline_mode live（相机附件换，见
    /// [`Self::apply_live_attachment`]）；flags live（Deform 绝对设定）与
    /// face_threshold live（ApplyParams）——展品不在场时只写设置，下次
    /// 激活生效；zone/crowd/stage_character 走同场景重建（代次 +1）。
    /// time_of_day 从会话视图基线起步应用内容级 preset（与场景切换的
    /// 视图重置同一语义），同时写回 CourtyardSettings 使后续重建保持
    /// 当前时刻。
    fn apply_parameter(
        &mut self,
        world: &mut World,
        key: &str,
        value: &serde_json::Value,
    ) -> CommandOutcome {
        let mut settings = world
            .get_resource::<CourtyardSettings>()
            .copied()
            .unwrap_or_default();
        // W3 增量装卸的 diff 基准（prev 快照；settings 是可变的工作副本）。
        let prev_settings = settings;
        match key {
            "zone" => {
                // 分区激活（v2）：增量装卸路径（W3——卸旧区装新区，画框/
                // 观测相机常驻）；机位随 builder 配方联动（§7 一一对应）。
                settings.zone = CourtyardZone::from_id(value.as_str().expect("validated"))
                    .expect("catalog validated zone id");
            }
            "time_of_day" => {
                settings.time_of_day = match value.as_str().expect("validated") {
                    "day" => CourtyardTimeOfDay::Day,
                    "dusk" => CourtyardTimeOfDay::Dusk,
                    _ => CourtyardTimeOfDay::Night,
                };
                world.insert_resource(settings);
                let preset: ViewPresetFn = match settings.time_of_day {
                    CourtyardTimeOfDay::Day => views::courtyard_day,
                    CourtyardTimeOfDay::Dusk => views::courtyard_dusk,
                    CourtyardTimeOfDay::Night => views::courtyard_night,
                };
                let mut view = world.resource_mut::<StylizedViewSettings>();
                *view = StylizedViewSettings(self.base_view.clone());
                preset(&mut view.0);
                return CommandOutcome::Applied;
            }
            "outline_mode" => {
                settings.outline_mode = match value.as_str().expect("validated") {
                    "off" => CourtyardOutlineMode::Off,
                    "hybrid" => CourtyardOutlineMode::Hybrid,
                    _ => CourtyardOutlineMode::Hull,
                };
                world.insert_resource(settings);
                // 05 §11.2（W1）：outline_mode 只动相机附件——live 换附件，
                // 不动内容、不换代次。
                return self.apply_live_attachment(world, &settings);
            }
            "crowd" => {
                settings.crowd = value.as_u64().expect("validated") as u32;
            }
            "post" => {
                settings.post = value.as_bool().expect("validated");
                world.insert_resource(settings);
                // 05 §11.2（W1）：post = 相机附件换。
                return self.apply_live_attachment(world, &settings);
            }
            "flags" => {
                settings.flags = match value.as_str().expect("validated") {
                    "posed" => CourtyardFlagsPose::Posed,
                    _ => CourtyardFlagsPose::Rest,
                };
                world.insert_resource(settings);
                // 05 §11.2（W2）：flags = live Deform（绝对设定 rotation_z，
                // 确定性）；展品不在场（非 p5）时只写设置，下次
                // 激活生效。
                let angle = match settings.flags {
                    CourtyardFlagsPose::Posed => 0.5,
                    CourtyardFlagsPose::Rest => 0.0,
                };
                for target in ["flags.0", "flags.1"] {
                    if scene_target_present(world, target) {
                        let target_ref = scenario_model::action::TargetRef {
                            scope: scenario_model::action::TargetScope::Entity,
                            id: scenario_model::ident::TargetId::new(target)
                                .expect("target id"),
                        };
                        crate::driver::apply_deform(
                            world,
                            &target_ref,
                            "rotation_z",
                            scenario_model::canon::FiniteF64::try_new(angle).expect("finite"),
                        )
                        .expect("flags root has Transform");
                    }
                }
                return CommandOutcome::Applied;
            }
            "stage_character" => {
                // 角色台展品（fixture/odette）：内容重建路径（同 zone/crowd）。
                settings.stage_character =
                    scenes::CourtyardStageCharacter::from_id(value.as_str().expect("validated"))
                        .expect("catalog validated stage_character id");
            }
            _ if key.starts_with("trio.face.") => {
                settings.face_threshold = if value.is_null() {
                    None
                } else {
                    Some(value.as_f64().expect("validated") as f32)
                };
                world.insert_resource(settings);
                // 05 §11.2（W2）：face_threshold = live ApplyParams——三个
                // face 落点同写（重建语义是全部落点读同一覆盖值，live 必须
                // 同义）；None = 各落点回梯度默认（0.2/默认/0.8）。展品不
                // 在场（非 trio）时只写设置。
                const GRADIENT: [Option<f32>; 3] = [Some(0.2), None, Some(0.8)];
                for (index, target) in ["trio.face.0", "trio.face.1", "trio.face.2"]
                    .into_iter()
                    .enumerate()
                {
                    if !scene_target_present(world, target) {
                        continue;
                    }
                    let applied = settings.face_threshold.or(GRADIENT[index]);
                    let mut params = scenario_model::canon::ParameterMap::default();
                    params.insert(
                        "face_threshold".to_string(),
                        applied.map_or(serde_json::Value::Null, |v| serde_json::json!(v)),
                    );
                    let target_ref = scenario_model::action::TargetRef {
                        scope: scenario_model::action::TargetScope::Entity,
                        id: scenario_model::ident::TargetId::new(target).expect("target id"),
                    };
                    crate::driver::apply_instance_params(world, &target_ref, &params)
                        .expect("face target is a stylized instance");
                }
                return CommandOutcome::Applied;
            }
            _ => unreachable!("evaluate validated the parameter key"),
        }
        // 05 §11.2（W3）：分区展品动态装卸——画框与观测相机常驻，只装卸
        // 变动的展品桶；代次 +1 与 Switched 语义不变（toast/journal/补间
        // 触发点与全量重建路径一致）。
        world.insert_resource(settings);
        let update = scenes::courtyard_update(world, prev_settings, settings);
        if let Err(error) = self.driver.note_content_update(world, update.recipe) {
            return CommandOutcome::Refused(RefusalReason::DriverBusy(error.to_string()));
        }
        // 退役时序（05 §12 W1–W3 时序规则）：zone 变化且补间开 → 延迟到补间落定
        // （先装后卸，补间起跳视野仍含旧区展品）；其余情况立即退役。
        let defer = prev_settings.zone != settings.zone && self.camera_transition.is_some();
        if defer {
            self.pending_retire.extend(update.retire);
        } else {
            scenes::retire_entities(world, &update.retire);
        }
        CommandOutcome::Switched {
            case: self.cases[self.current].id.clone(),
            generation: self.driver.generation(),
        }
    }

    /// W1 相机附件 live 切换（05 §11.2 post/outline_mode 行）：附件由参数
    /// 驱动、与机位正交——只替换观测相机附件组件，不动内容、不换代次。
    /// eye/look/projection 取 driver 的应用配方（精确配方值，不经
    /// orbit 姿态往返重建——重建有 ulp 级误差，会让观测相机偏离配方机位
    /// 末位字节）；补间中到达时先落定目标姿态，宿主同帧 advance 即回到
    /// 插值姿态（无可见瞬跳）。
    fn apply_live_attachment(
        &mut self,
        world: &mut World,
        settings: &CourtyardSettings,
    ) -> CommandOutcome {
        let Some(applied) = self.driver.applied_recipe().cloned() else {
            return CommandOutcome::Refused(RefusalReason::InvalidCamera(
                "observation camera not built yet".into(),
            ));
        };
        let recipe = CameraRecipe {
            eye: applied.eye,
            look: applied.look,
            projection: applied.projection.clone(),
            attachment: scenes::courtyard_attachment(settings),
        };
        match self.driver.apply_camera_recipe(world, &recipe) {
            Ok(()) => CommandOutcome::Applied,
            Err(error) => {
                CommandOutcome::Refused(RefusalReason::InvalidCamera(error.to_string()))
            }
        }
    }

    /// 当前相机状态（只读视图；供 PC-4 的窗口相机统一消费）。
    ///
    /// 惰性同步：首次访问或代次变化后从构建期配方派生；相机尚未构建
    /// （driver 未完成 Build）时为 None。补间中返回插值后的呈现姿态
    /// （06 §8；锚点字段保持终点，呈现与观测相机同源）。
    pub fn camera_view(&mut self) -> Option<CameraViewInfo> {
        self.sync_camera_state().ok()?;
        let state = self.camera.as_ref().expect("camera state synced");
        let (eye, look) = state.view_pose();
        Some(CameraViewInfo {
            preset: state.preset.clone(),
            eye,
            look,
            yaw: state.yaw,
            pitch: state.pitch,
            distance: state.distance,
        })
    }
}

/// SceneTarget 在场性查询（W2 live 路由：展品不在场 = 只写设置）。
fn scene_target_present(world: &mut World, id: &str) -> bool {
    let mut query = world.query::<&scenes::SceneTarget>();
    query.iter(world).any(|target| target.0 == id)
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
        // 05 §10：18 个旧场景退役，courtyard 是唯一条目。
        assert_eq!(controller.cases().len(), 1);
        assert_eq!(controller.cases()[0].id.as_str(), "courtyard");
    }

    #[test]
    fn evaluate_capture_requires_idle_driver() {
        // 06 §9：Capture 是 SessionCommand 变体；driver 未落定（新鲜控制器
        // 处于 Build）时结构化拒绝（DriverBusy），当前场景保留。
        let controller = controller();
        assert_eq!(
            controller.evaluate(&SessionCommand::Capture),
            CommandPlan::Refused(RefusalReason::DriverBusy(
                "capture requires an idle driver (build finished)".into()
            ))
        );
    }

    #[test]
    fn evaluate_goto_next_prev_and_unknown() {
        let controller = controller();
        // 单场景目录：循环切回 courtyard 自身；它声明了资产依赖，资产根
        // 固定的会话结构化拒绝（R2）。
        for command in [SessionCommand::NextCase, SessionCommand::PrevCase] {
            assert_eq!(
                controller.evaluate(&command),
                CommandPlan::Refused(RefusalReason::AssetRootFixed(
                    SceneId::new("courtyard").expect("id")
                ))
            );
        }
        assert_eq!(
            controller.evaluate(&SessionCommand::SelectCase(
                SceneId::new("courtyard").expect("id")
            )),
            CommandPlan::Refused(RefusalReason::AssetRootFixed(
                SceneId::new("courtyard").expect("id")
            ))
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
        // outline_mode=hybrid 需 Standard（courtyard 参数面）：Portable 会话拒绝。
        let hybrid = SessionCommand::SetParameter {
            key: "outline_mode".into(),
            value: serde_json::json!("hybrid"),
        };
        assert_eq!(
            controller.evaluate(&hybrid),
            CommandPlan::Refused(RefusalReason::TierRequired {
                required: StylizedExecutionTier::Standard,
                resolved: StylizedExecutionTier::Portable,
                reconfiguration_attempted: false,
            })
        );
        // Standard 会话允许。
        controller.set_tier_ceiling(StylizedExecutionTier::Standard);
        assert_eq!(controller.evaluate(&hybrid), CommandPlan::Local);
        // courtyard 声明了资产依赖：会话资产根固定 → 切换拒绝。
        assert_eq!(
            controller.evaluate(&SessionCommand::SelectCase(
                SceneId::new("courtyard").expect("id")
            )),
            CommandPlan::Refused(RefusalReason::AssetRootFixed(
                SceneId::new("courtyard").expect("id")
            ))
        );
    }

    /// R2/ADR-0030：设备级缺口（快照无 storage buffer）→ 尝试会话内重配置
    /// 后仍拒绝，`reconfiguration_attempted = true`，会话状态不变。
    #[test]
    fn command_attempts_reconfiguration_and_reports_device_gap() {
        use bevy_stylized::prelude::{StylizedCapabilityConfig, StylizedResolvedCapabilities};
        use bevy_stylized::reconfiguration::StylizedReconfigGeneration;
        use bevy_stylized::stylized_core::{
            StylizedCapabilitySet, StylizedGpuLimits, resolve_variant,
        };

        let mut app = bevy::prelude::App::new();
        let mut controller = controller();
        controller.set_tier_ceiling(StylizedExecutionTier::Portable);
        // Portable-only 快照（无 storage buffer）：Standard 不可解析。
        let actual = StylizedCapabilitySet {
            limits: StylizedGpuLimits {
                max_bind_groups: 4,
                max_storage_buffers_per_shader_stage: 0,
                max_uniform_buffers_per_shader_stage: 12,
            },
            ..StylizedCapabilitySet::default()
        };
        let incumbent = resolve_variant(
            bevy_stylized::stylized_core::StylizedSurfaceClass::Cel,
            StylizedExecutionTier::Portable,
            &actual,
            bevy_stylized::stylized_core::StylizedFallbackPolicy::ReduceTier,
        )
        .expect("portable resolves");
        app.insert_resource(StylizedCapabilityConfig::default());
        app.insert_resource(StylizedResolvedCapabilities {
            actual,
            resolved: Ok(incumbent.clone()),
            report: None,
        });

        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "outline_mode".into(),
                value: serde_json::json!("hybrid"),
            },
        );
        assert_eq!(
            outcome,
            CommandOutcome::Refused(RefusalReason::TierRequired {
                required: StylizedExecutionTier::Standard,
                resolved: StylizedExecutionTier::Portable,
                reconfiguration_attempted: true,
            })
        );
        // 会话仍停在原用例；已解析 tier 未被伪装上调。
        assert_eq!(controller.current().id.as_str(), "courtyard");
        assert_eq!(
            app.world()
                .get_resource::<StylizedReconfigGeneration>()
                .map(|g| g.0),
            // 重解析落回 Portable（tier_reduced），请求配置确实变化 → 代次 +1，
            // 但会话上限保持 Portable（不冒充重新初始化）。
            Some(1)
        );
    }

    /// 无能力快照（无 StylizedCapabilityConfig/Stage B 资源）→ 无法尝试，
    /// `reconfiguration_attempted = false`。
    #[test]
    fn command_without_snapshot_refuses_without_attempt() {
        let mut app = bevy::prelude::App::new();
        let mut controller = controller();
        controller.set_tier_ceiling(StylizedExecutionTier::Portable);
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "outline_mode".into(),
                value: serde_json::json!("hybrid"),
            },
        );
        assert_eq!(
            outcome,
            CommandOutcome::Refused(RefusalReason::TierRequired {
                required: StylizedExecutionTier::Standard,
                resolved: StylizedExecutionTier::Portable,
                reconfiguration_attempted: false,
            })
        );
        assert_eq!(controller.current().id.as_str(), "courtyard");
    }
}

#[cfg(test)]
mod courtyard_command_tests {
    //! V1 相机/参数命令的纯 CPU 测试（docs/showcase/05 §11.1/§11.2）：
    //! courtyard 以无渲染器的最小 App 真实构建（builder 是 CPU 内容装配；
    //! 观测相机与附件是组件数据）。
    use super::*;
    use crate::driver::DriverLimits;
    use bevy::asset::{AssetApp, Assets};
    use bevy::prelude::{App, Image, Mesh};
    use scenario_model::action::ActionGraph;
    use scenario_model::ident::SessionId;

    /// 构建并跑完 courtyard（代次 0、Done 终态）的 controller + App。
    fn courtyard_controller() -> (App, SessionController) {
        courtyard_controller_with(|_| {})
    }

    /// 同上，但在 driver 创建后、首次构建前执行 `pre_build`（宿主侧
    /// 后置装配的模拟点——惰性基线的回归测试在此插入宿主实体）。
    fn courtyard_controller_with(pre_build: impl FnOnce(&mut App)) -> (App, SessionController) {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<bevy::world_serialization::WorldAsset>();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<bevy_stylized::prelude::StylizedCelMaterial>>()
            .init_resource::<Assets<bevy_stylized::prelude::StylizedCharacterMaterial>>()
            .init_resource::<Assets<bevy_stylized::prelude::StylizedHeroMaterial>>()
            .init_resource::<Assets<bevy_stylized::prelude::StylizedOutlineMaterial>>()
            .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
            .insert_resource(StylizedViewSettings(StylizedViewDescriptor::default()));
        let binding = bindings()
            .into_iter()
            .find(|binding| binding.id == "courtyard")
            .expect("courtyard binding");
        let driver = CaseDriver::new(
            app.world(),
            SessionId::new("courtyard-command-test").expect("session id"),
            binding,
            ActionGraph::default(),
            DriverLimits {
                implicit_readiness: false,
                ..DriverLimits::default()
            },
        )
        .expect("plan");
        let mut controller =
            SessionController::from_catalog(driver, 0, StylizedViewDescriptor::default())
                .expect("controller");
        pre_build(&mut app);
        crate::headless::run_to_completion(&mut app, controller.driver_mut(), 100)
            .expect("courtyard build");
        (app, controller)
    }

    fn camera_translation(app: &App, controller: &SessionController) -> Vec3 {
        let camera = controller
            .driver()
            .observation_entity()
            .expect("observation camera");
        app.world()
            .entity(camera)
            .get::<bevy::prelude::Transform>()
            .expect("transform")
            .translation
    }

    #[test]
    fn set_camera_preset_moves_camera_and_unknown_preset_is_refused() {
        let (mut app, mut controller) = courtyard_controller();
        // 默认机位 = center-trio（默认内容 trio 的联动机位，§7）。
        let view = controller.camera_view().expect("camera view");
        assert_eq!(view.preset, None);
        assert!(
            (view.eye - Vec3::new(3.2, 3.4, 5.2)).length() < 1e-4,
            "default eye must be the center-trio preset (orbit roundtrip tolerance)"
        );

        assert_eq!(
            controller.evaluate(&SessionCommand::SetCameraPreset("flags".into())),
            CommandPlan::Local
        );
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetCameraPreset("flags".into()),
        );
        assert_eq!(outcome, CommandOutcome::Applied);
        let preset_eye = COURTYARD_CAMERA_PRESETS
            .iter()
            .find(|preset| preset.id == "flags")
            .expect("preset")
            .eye;
        assert!(
            (camera_translation(&app, &controller) - preset_eye).length() < 1e-4,
            "camera must be at the preset eye (orbit roundtrip tolerance)"
        );
        let view = controller.camera_view().expect("camera view");
        assert_eq!(view.preset.as_deref(), Some("flags"));
        assert_eq!(
            view.look,
            COURTYARD_CAMERA_PRESETS
                .iter()
                .find(|preset| preset.id == "flags")
                .expect("preset")
                .look
        );

        // 未知 preset 与非 courtyard 用例：结构化拒绝。
        assert_eq!(
            controller.evaluate(&SessionCommand::SetCameraPreset("nope".into())),
            CommandPlan::Refused(RefusalReason::UnknownCameraPreset("nope".into()))
        );
    }

    #[test]
    fn orbit_pan_zoom_and_reset_view() {
        let (mut app, mut controller) = courtyard_controller();
        let base = controller.camera_view().expect("camera view");
        // orbit：yaw +0.5（pitch 不变）。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::CameraOrbit {
                yaw: 0.5,
                pitch: 0.0,
            },
        );
        assert_eq!(outcome, CommandOutcome::Applied);
        let orbited = controller.camera_view().expect("camera view");
        assert!((orbited.yaw - (base.yaw + 0.5)).abs() < 1e-6);
        assert_eq!(camera_translation(&app, &controller), orbited.eye);
        assert_ne!(orbited.eye, base.eye);

        // pitch 越 ±1.5 rad → 拒绝且状态不变。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::CameraOrbit {
                yaw: 0.0,
                pitch: 2.0,
            },
        );
        assert!(matches!(
            outcome,
            CommandOutcome::Refused(RefusalReason::InvalidCamera(_))
        ));
        assert_eq!(
            controller.camera_view().expect("camera view").pitch,
            orbited.pitch
        );

        // zoom：factor=2 距离减半；过近拒绝。
        let outcome =
            controller.command(app.world_mut(), &SessionCommand::CameraZoom { factor: 2.0 });
        assert_eq!(outcome, CommandOutcome::Applied);
        let zoomed = controller.camera_view().expect("camera view");
        assert!((zoomed.distance - orbited.distance / 2.0).abs() < 1e-4);
        let outcome =
            controller.command(app.world_mut(), &SessionCommand::CameraZoom { factor: 1e9 });
        assert!(matches!(
            outcome,
            CommandOutcome::Refused(RefusalReason::InvalidCamera(_))
        ));
        assert_eq!(
            controller.evaluate(&SessionCommand::CameraZoom { factor: 0.0 }),
            CommandPlan::Refused(RefusalReason::InvalidCamera(
                "zoom factor must be finite and positive".into()
            ))
        );

        // pan：dx=1 使目标点沿相机右向（yaw=0.5 后的旋转系）移动 1 米。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::CameraPan { dx: 1.0, dy: 0.0 },
        );
        assert_eq!(outcome, CommandOutcome::Applied);
        let panned = controller.camera_view().expect("camera view");
        assert!(
            ((panned.look - zoomed.look).length() - 1.0).abs() < 1e-4,
            "pan right by 1 m: {panned:?}"
        );

        // ResetView 回到当前 preset 的配方机位（无 preset = 场景默认）。
        let outcome = controller.command(app.world_mut(), &SessionCommand::ResetView);
        assert_eq!(outcome, CommandOutcome::Applied);
        let reset = controller.camera_view().expect("camera view");
        assert_eq!(reset.look, base.look);
        assert!((reset.distance - base.distance).abs() < 1e-4);

        // preset 切换后 ResetView 回到该 preset 机位。
        controller.command(
            app.world_mut(),
            &SessionCommand::SetCameraPreset("flags".into()),
        );
        controller.command(
            app.world_mut(),
            &SessionCommand::CameraOrbit {
                yaw: 0.3,
                pitch: 0.0,
            },
        );
        controller.command(app.world_mut(), &SessionCommand::ResetView);
        let view = controller.camera_view().expect("camera view");
        let flags = COURTYARD_CAMERA_PRESETS
            .iter()
            .find(|entry| entry.id == "flags")
            .expect("preset");
        assert!(
            (view.eye - flags.eye).length() < 1e-4,
            "reset returns to the preset pose: {:?}",
            view.eye
        );
        assert!((view.look - flags.look).length() < 1e-4);
    }

    #[test]
    fn camera_transition_tweens_to_preset_and_completes() {
        let (mut app, mut controller) = courtyard_controller();
        controller.set_camera_transition(Some(0.4));
        let from = controller.camera_view().expect("camera view");
        let target_preset = COURTYARD_CAMERA_PRESETS
            .iter()
            .find(|entry| entry.id == "flags")
            .expect("preset");

        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetCameraPreset("flags".into()),
        );
        assert_eq!(outcome, CommandOutcome::Applied);
        // 起跳即应用 from 姿态：无瞬跳（k=0）。
        let start = controller.camera_view().expect("camera view");
        assert!(
            (start.eye - from.eye).length() < 1e-4,
            "tween starts at the current pose: {:?} vs {:?}",
            start.eye,
            from.eye
        );
        // 半途（0.2/0.4，缓动后 k=0.5）：介于两端之间；观测相机与视图同源。
        controller.advance_camera_transition(app.world_mut(), 0.2);
        let mid = controller.camera_view().expect("camera view");
        assert!((mid.eye - from.eye).length() > 1e-3);
        assert!((mid.eye - target_preset.eye).length() > 1e-3);
        assert!(
            (camera_translation(&app, &controller) - mid.eye).length() < 1e-4,
            "observation camera follows the interpolated pose"
        );
        // 推进到终点：落定 preset 机位，补间清除（再推进不再移动）。
        controller.advance_camera_transition(app.world_mut(), 0.2);
        let end = controller.camera_view().expect("camera view");
        assert!((end.eye - target_preset.eye).length() < 1e-4);
        assert!((end.look - target_preset.look).length() < 1e-4);
        controller.advance_camera_transition(app.world_mut(), 1.0);
        let settled = controller.camera_view().expect("camera view");
        assert_eq!(settled.eye, end.eye);
    }

    #[test]
    fn camera_transition_is_off_by_default_and_snaps() {
        let (mut app, mut controller) = courtyard_controller();
        let target_preset = COURTYARD_CAMERA_PRESETS
            .iter()
            .find(|entry| entry.id == "flags")
            .expect("preset");
        controller.command(
            app.world_mut(),
            &SessionCommand::SetCameraPreset("flags".into()),
        );
        // 默认关闭：瞬时到位（捕获/冒烟路径零行为变化）；advance 空操作。
        let view = controller.camera_view().expect("camera view");
        assert!((view.eye - target_preset.eye).length() < 1e-4);
        controller.advance_camera_transition(app.world_mut(), 0.2);
        let settled = controller.camera_view().expect("camera view");
        assert_eq!(settled.eye, view.eye);
    }

    #[test]
    fn navigation_command_preempts_transition_from_intermediate_pose() {
        let (mut app, mut controller) = courtyard_controller();
        controller.set_camera_transition(Some(0.4));
        controller.command(
            app.world_mut(),
            &SessionCommand::SetCameraPreset("flags".into()),
        );
        controller.advance_camera_transition(app.world_mut(), 0.2);
        let mid = controller.camera_view().expect("camera view");
        // orbit 抢占：补间清除，从中间态继续（yaw 在中间姿态上加 0.5）。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::CameraOrbit {
                yaw: 0.5,
                pitch: 0.0,
            },
        );
        assert_eq!(outcome, CommandOutcome::Applied);
        let orbited = controller.camera_view().expect("camera view");
        let (mid_yaw, mid_pitch, mid_distance) = orbit_pose(mid.eye, mid.look);
        assert!((orbited.yaw - (mid_yaw + 0.5)).abs() < 1e-4);
        assert!((orbited.pitch - mid_pitch).abs() < 1e-4);
        assert!((orbited.distance - mid_distance).abs() < 1e-4);
        // 补间已清除：再推进不改变机位。
        controller.advance_camera_transition(app.world_mut(), 1.0);
        let settled = controller.camera_view().expect("camera view");
        assert_eq!(settled.eye, orbited.eye);
    }

    #[test]
    fn rebuild_tweens_from_old_pose_to_new_recipe() {
        let (mut app, mut controller) = courtyard_controller();
        controller.set_camera_transition(Some(0.4));
        let from = controller.camera_view().expect("camera view");
        // 切分区（内容重建，代次 +1）：构建完成后从旧姿态起跳到新机位。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "zone".into(),
                value: serde_json::json!("p2"),
            },
        );
        assert!(matches!(outcome, CommandOutcome::Switched { .. }));
        crate::headless::run_to_completion(&mut app, controller.driver_mut(), 100)
            .expect("rebuild completes");
        // 惰性同步在首次访问时发生：起跳点 = 旧姿态。
        let start = controller.camera_view().expect("camera view");
        assert!(
            (start.eye - from.eye).length() < 1e-4,
            "rebuild tween starts at the pre-switch pose"
        );
        controller.advance_camera_transition(app.world_mut(), 10.0);
        let end = controller.camera_view().expect("camera view");
        let built = controller.driver().built_recipe().expect("built recipe");
        assert!((end.eye - built.eye).length() < 1e-4);
    }

    /// W3 先装后卸（05 §12 W1–W3 时序规则）：补间关闭时切分区立即退役旧区；
    /// 补间开启时旧区展品延迟到补间落定才退役（起跳视野不空场）。
    #[test]
    fn zone_switch_defers_old_zone_retire_until_transition_settles() {
        let (mut app, mut controller) = courtyard_controller();
        // 补间关（默认）：trio → p2 立即退役。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "zone".into(),
                value: serde_json::json!("p2"),
            },
        );
        assert!(matches!(outcome, CommandOutcome::Switched { .. }));
        let buckets = app
            .world()
            .resource::<scenes::CourtyardBuckets>();
        assert!(buckets.exhibits.contains_key("p2"));
        assert!(!buckets.exhibits.contains_key("trio"));
        // 补间开：p2 → p1，旧区（p2）展品延迟退役。
        controller.set_camera_transition(Some(0.4));
        let p2_entities = app.world().resource::<scenes::CourtyardBuckets>().exhibits["p2"]
            .clone();
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "zone".into(),
                value: serde_json::json!("p1"),
            },
        );
        assert!(matches!(outcome, CommandOutcome::Switched { .. }));
        assert!(
            p2_entities
                .iter()
                .all(|entity| app.world().get_entity(*entity).is_ok()),
            "old zone exhibits stay during the transition (load-first)"
        );
        assert!(
            app.world()
                .resource::<scenes::CourtyardBuckets>()
                .exhibits
                .contains_key("p1"),
            "new zone loaded immediately"
        );
        // 补间落定 → 旧区退役。
        controller.advance_camera_transition(app.world_mut(), 10.0);
        assert!(
            p2_entities
                .iter()
                .all(|entity| app.world().get_entity(*entity).is_err()),
            "old zone exhibits retire when the transition settles"
        );
    }

    /// W2：flags live Deform——展品在场时直接改 root 旋转（绝对设定，
    /// 不换代次）；不在场时只写设置（下次激活生效）。
    #[test]
    fn flags_live_deform_when_present_and_settings_only_when_absent() {
        let (mut app, mut controller) = courtyard_controller();
        let flags_command = |value: &str| SessionCommand::SetParameter {
            key: "flags".into(),
            value: serde_json::json!(value),
        };
        // 缺席（默认 trio）：Applied、代次不变、设置已写。
        let generation = controller.driver().generation();
        let outcome = controller.command(app.world_mut(), &flags_command("posed"));
        assert_eq!(outcome, CommandOutcome::Applied);
        assert_eq!(controller.driver().generation(), generation);
        assert_eq!(
            app.world().resource::<CourtyardSettings>().flags,
            CourtyardFlagsPose::Posed
        );
        // 在场（p5）：live Deform 落到 flags.0/1 root（绝对 0.5 rad）。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "zone".into(),
                value: serde_json::json!("p5"),
            },
        );
        assert!(matches!(outcome, CommandOutcome::Switched { .. }));
        let generation = controller.driver().generation();
        let outcome = controller.command(app.world_mut(), &flags_command("posed"));
        assert_eq!(outcome, CommandOutcome::Applied);
        assert_eq!(controller.driver().generation(), generation);
        let mut query = app
            .world_mut()
            .query::<(&scenes::SceneTarget, &bevy::prelude::Transform)>();
        for (target, transform) in query
            .iter(app.world())
            .filter(|(target, _)| target.0 == "flags.0" || target.0 == "flags.1")
        {
            let expected = bevy::math::Quat::from_rotation_z(0.5);
            assert!(
                transform.rotation.abs_diff_eq(expected, 1e-6),
                "{} rotation must be the posed absolute set",
                target.0
            );
        }
        // 回 rest：绝对 0（非累积）。
        let outcome = controller.command(app.world_mut(), &flags_command("rest"));
        assert_eq!(outcome, CommandOutcome::Applied);
        let mut query = app
            .world_mut()
            .query::<(&scenes::SceneTarget, &bevy::prelude::Transform)>();
        for (target, transform) in query
            .iter(app.world())
            .filter(|(target, _)| target.0 == "flags.0")
        {
            assert!(
                transform
                    .rotation
                    .abs_diff_eq(bevy::math::Quat::IDENTITY, 1e-6),
                "{} rotation must reset to identity",
                target.0
            );
        }
    }

    #[test]
    fn set_stage_character_updates_settings_and_rebuilds() {
        let (mut app, mut controller) = courtyard_controller();
        assert_eq!(
            app.world()
                .get_resource::<CourtyardSettings>()
                .copied()
                .unwrap_or_default()
                .stage_character,
            scenes::CourtyardStageCharacter::Fixture,
            "default must stay fixture (golden/evidence content)"
        );
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "stage_character".into(),
                value: serde_json::json!("odette"),
            },
        );
        assert!(matches!(
            outcome,
            CommandOutcome::Switched { ref case, generation }
                if case.as_str() == "courtyard" && generation.get() == 1
        ));
        assert_eq!(
            app.world().resource::<CourtyardSettings>().stage_character,
            scenes::CourtyardStageCharacter::Odette
        );
    }

    #[test]
    fn set_parameter_rebuilds_content_and_switches_view_live() {
        let (mut app, mut controller) = courtyard_controller();
        // time_of_day：live 视图切换，不重建（代次不变）。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "time_of_day".into(),
                value: serde_json::json!("night"),
            },
        );
        assert_eq!(outcome, CommandOutcome::Applied);
        assert_eq!(controller.driver().generation().get(), 0);
        let view = app.world().resource::<StylizedViewSettings>();
        assert_eq!(view.0.day_night_factor, 0.0);
        assert_eq!(view.0.environment.fog.end, 48.0);
        assert_eq!(
            app.world().resource::<CourtyardSettings>().time_of_day,
            CourtyardTimeOfDay::Night
        );

        // crowd：同场景重建（generation +1），灯阵移除；时刻保持 night。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "crowd".into(),
                value: serde_json::json!(0),
            },
        );
        assert!(matches!(
            outcome,
            CommandOutcome::Switched { ref case, generation }
                if case.as_str() == "courtyard" && generation.get() == 1
        ));
        crate::headless::run_to_completion(&mut app, controller.driver_mut(), 100)
            .expect("rebuild");
        let mut query = app.world_mut().query::<&scenes::SceneTarget>();
        let targets: Vec<&str> = query.iter(app.world()).map(|target| target.0).collect();
        assert!(!targets.contains(&"lamps"));
        assert!(targets.contains(&"trio.face.0"));
        assert_eq!(
            app.world()
                .resource::<StylizedViewSettings>()
                .0
                .day_night_factor,
            0.0,
            "rebuild must keep the current time_of_day"
        );

        // face_threshold（05 §11.2，W2）：live ApplyParams——不换代次，
        // 阈值覆盖三实例（与重建语义同义：全部落点读同一覆盖值）。
        let generation_before = controller.driver().generation();
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "trio.face.0.face_threshold".into(),
                value: serde_json::json!(0.5),
            },
        );
        assert_eq!(outcome, CommandOutcome::Applied);
        assert_eq!(controller.driver().generation(), generation_before);
        let mut query = app.world_mut().query::<(
            &scenes::SceneTarget,
            &bevy_stylized::prelude::StylizedInstanceData,
        )>();
        for (target, data) in query
            .iter(app.world())
            .filter(|(target, _)| target.0.starts_with("trio.face."))
        {
            assert_eq!(data.face_threshold, Some(0.5), "{}", target.0);
        }
        // null 回梯度默认（0.2/默认/0.8，与 builder 同表）。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "trio.face.1.face_threshold".into(),
                value: serde_json::Value::Null,
            },
        );
        assert_eq!(outcome, CommandOutcome::Applied);
        let mut query = app.world_mut().query::<(
            &scenes::SceneTarget,
            &bevy_stylized::prelude::StylizedInstanceData,
        )>();
        for (target, data) in query
            .iter(app.world())
            .filter(|(target, _)| target.0.starts_with("trio.face."))
        {
            let expected = match target.0 {
                "trio.face.0" => Some(0.2),
                "trio.face.1" => None,
                _ => Some(0.8),
            };
            assert_eq!(data.face_threshold, expected, "{}", target.0);
        }

        // 未知键 / 越界值 / 非 courtyard 用例：结构化拒绝。
        assert_eq!(
            controller.evaluate(&SessionCommand::SetParameter {
                key: "nope".into(),
                value: serde_json::json!(1),
            }),
            CommandPlan::Refused(RefusalReason::UnknownParameter("nope".into()))
        );
        assert!(matches!(
            controller.evaluate(&SessionCommand::SetParameter {
                key: "crowd".into(),
                value: serde_json::json!(1001),
            }),
            CommandPlan::Refused(RefusalReason::InvalidParameter(_))
        ));
    }

    #[test]
    fn outline_mode_hybrid_requires_standard_tier() {
        let (mut app, mut controller) = courtyard_controller();
        controller.set_tier_ceiling(StylizedExecutionTier::Portable);
        assert_eq!(
            controller.evaluate(&SessionCommand::SetParameter {
                key: "outline_mode".into(),
                value: serde_json::json!("hybrid"),
            }),
            CommandPlan::Refused(RefusalReason::TierRequired {
                required: StylizedExecutionTier::Standard,
                resolved: StylizedExecutionTier::Portable,
                reconfiguration_attempted: false,
            })
        );
        // 无能力快照：command() 无法尝试会话内重配置，保持拒绝。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "outline_mode".into(),
                value: serde_json::json!("hybrid"),
            },
        );
        assert!(matches!(
            outcome,
            CommandOutcome::Refused(RefusalReason::TierRequired {
                reconfiguration_attempted: false,
                ..
            })
        ));

        // Standard 会话（05 §11.2，W1）：live 换附件——不换代次、观测相机
        // 实体不变，原地插入 ScreenOutline。
        controller.set_tier_ceiling(StylizedExecutionTier::Standard);
        let camera_before = controller.driver().observation_entity();
        let generation_before = controller.driver().generation();
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "outline_mode".into(),
                value: serde_json::json!("hybrid"),
            },
        );
        assert_eq!(outcome, CommandOutcome::Applied);
        assert_eq!(controller.driver().generation(), generation_before);
        assert_eq!(
            controller.driver().observation_entity(),
            camera_before,
            "attachment swap must reuse the observation camera entity"
        );
        let camera = camera_before.expect("observation camera");
        assert!(
            app.world()
                .entity(camera)
                .contains::<bevy_stylized::stylized_render::screen_outline::StylizedScreenOutline>(
                )
        );

        // 机位切换保留附件（附件由参数驱动，与机位正交）。
        controller.command(
            app.world_mut(),
            &SessionCommand::SetCameraPreset("outline".into()),
        );
        assert!(
            app.world()
                .entity(camera)
                .contains::<bevy_stylized::stylized_render::screen_outline::StylizedScreenOutline>(
                ),
            "camera preset switch must preserve the attachment"
        );
    }

    /// 惰性基线回归（2026-09-29 交互宿主事故）：driver 创建后、首次构建
    /// 前出生的宿主实体（交互宿主的窗口/监视器/BRP 系统实体同理）必须
    /// 视为宿主所有，场景重建不得退役。旧实现把基线钉在 CaseDriver::new，
    /// 这类实体被误记进 gen0 账本并在首次切换整批退役。
    #[test]
    fn host_entities_spawned_after_driver_creation_survive_rebuild() {
        #[derive(bevy::prelude::Component)]
        struct HostInfra;
        let mut host = std::cell::Cell::new(bevy::prelude::Entity::PLACEHOLDER);
        let (mut app, mut controller) = courtyard_controller_with(|app| {
            host.set(app.world_mut().spawn(HostInfra).id());
        });
        let host_entity = host.get();
        assert!(app.world().get_entity(host_entity).is_ok());
        // 参数驱动的同场景重建（zone 切换同路径）。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "zone".to_string(),
                value: serde_json::json!("p2"),
            },
        );
        assert!(matches!(outcome, CommandOutcome::Switched { .. }));
        crate::headless::run_to_completion(&mut app, controller.driver_mut(), 100)
            .expect("rebuild");
        assert!(
            app.world().get_entity(host_entity).is_ok(),
            "host infrastructure must not be retired by a scene rebuild"
        );
    }

    /// 代次配方同步（2026-09-29 交互宿主事故的回归测试；W3 增量装卸后
    /// 语义更新）：增量路径没有跨帧构建窗口——命令同步完成装卸并替换
    /// 配方，camera_view 立即按新代配方派生（不存在「用上一代配方派生
    /// 新代状态」的窗口，旧实现的锁死缺陷在两条路径下都不得复现）。
    #[test]
    fn camera_view_never_derives_from_the_previous_generations_recipe() {
        let (mut app, mut controller) = courtyard_controller();
        // gen0 = trio 配方（center-trio）。
        let view = controller.camera_view().expect("view");
        assert!((view.eye - Vec3::new(3.2, 3.4, 5.2)).length() < 1e-3);
        // zone 切换（gen1）：增量更新同步完成——camera_view 立即给出新代
        // 配方（p3 → outline-west），无 None 窗口、无旧配方残留。
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetParameter {
                key: "zone".to_string(),
                value: serde_json::json!("p3"),
            },
        );
        assert!(matches!(outcome, CommandOutcome::Switched { .. }));
        let view = controller
            .camera_view()
            .expect("incremental update applies the new recipe synchronously");
        assert!((view.eye - Vec3::new(0.0, 1.3, 3.4)).length() < 1e-3);
        assert_eq!(view.preset.as_deref(), None, "preset resets on generation");
        // 再次确认无在途构建（状态机不回 Build）。
        assert_eq!(
            controller.driver().status(),
            DriverStatus::Done,
            "incremental update keeps the driver in Done"
        );
    }

    #[test]
    fn camera_commands_require_a_built_camera() {
        // 未构建（driver 未泵送）：相机命令结构化拒绝。
        let binding = bindings()
            .into_iter()
            .find(|binding| binding.id == "courtyard")
            .expect("courtyard binding");
        let mut app = App::new();
        let driver = CaseDriver::new(
            app.world(),
            SessionId::new("camera-not-built").expect("session id"),
            binding,
            ActionGraph::default(),
            DriverLimits::default(),
        )
        .expect("plan");
        let mut controller =
            SessionController::from_catalog(driver, 0, StylizedViewDescriptor::default())
                .expect("controller");
        let outcome = controller.command(
            app.world_mut(),
            &SessionCommand::SetCameraPreset("flags".into()),
        );
        assert!(matches!(
            outcome,
            CommandOutcome::Refused(RefusalReason::InvalidCamera(_))
        ));
    }

    #[test]
    fn driver_camera_replacement_swaps_attachments() {
        let (mut app, mut controller) = courtyard_controller();
        let camera = controller
            .driver()
            .observation_entity()
            .expect("observation camera");
        // 默认（hull/post off）：无附件。
        assert!(
            !app.world()
                .entity(camera)
                .contains::<bevy_stylized::stylized_render::post::StylizedPostProcess>()
        );
        // 直接经 driver API 替换配方：PostProcess 附件插入。
        let mut recipe = controller
            .driver()
            .applied_recipe()
            .expect("recipe")
            .clone();
        recipe.attachment = CameraAttachment::PostProcess(
            bevy_stylized::stylized_render::post::StylizedPostProcess {
                vignette_strength: 0.5,
                chromatic_aberration: 0.004,
                ..Default::default()
            },
        );
        controller
            .driver_mut()
            .apply_camera_recipe(app.world_mut(), &recipe)
            .expect("apply");
        assert!(
            app.world()
                .entity(camera)
                .contains::<bevy_stylized::stylized_render::post::StylizedPostProcess>()
        );
        // 换回 None：附件移除（remove 路径）。
        recipe.attachment = CameraAttachment::None;
        controller
            .driver_mut()
            .apply_camera_recipe(app.world_mut(), &recipe)
            .expect("apply");
        assert!(
            !app.world()
                .entity(camera)
                .contains::<bevy_stylized::stylized_render::post::StylizedPostProcess>()
        );
    }

    #[test]
    fn camera_and_parameter_commands_replay_exactly() {
        // 录制（courtyard 初始用例，代次 0 已构建）。
        let (mut app, mut controller) = courtyard_controller();
        let mut journal = crate::journal::CommandJournal::new(
            SessionId::new("courtyard-replay").expect("session"),
            18,
        );
        for command in [
            SessionCommand::SetCameraPreset("center-trio".into()),
            SessionCommand::CameraOrbit {
                yaw: 0.4,
                pitch: -0.1,
            },
            SessionCommand::SetParameter {
                key: "time_of_day".into(),
                value: serde_json::json!("dusk"),
            },
            SessionCommand::ResetView,
            SessionCommand::SetParameter {
                key: "crowd".into(),
                value: serde_json::json!(0),
            },
        ] {
            controller.command_recorded(app.world_mut(), &command, &mut journal);
            crate::headless::run_to_completion(&mut app, controller.driver_mut(), 100)
                .expect("pump");
        }

        // 重放：新鲜会话 + 同一日志 → 逐条结果一致（逻辑身份确定性）。
        let (mut replay_app, mut replay_controller) = courtyard_controller();
        let report = replay_controller.replay(&mut replay_app, &journal);
        assert!(
            report.is_exact(&journal),
            "replay mismatches: {:?}",
            report.mismatches
        );
        assert_eq!(report.applied, 5);
        assert!(report.indeterminate.is_empty());
    }
}
