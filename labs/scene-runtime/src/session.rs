//! 会话重配置协调（R2 + 验收矩阵事件序列）。
//!
//! 事务状态：接受中（Active）→ 排空/作废请求（Draining）→ 保存配方检查点
//! → 重建（Rebuilding）→ 校验（Validating）→ 恢复（Active）。
//! 设备/App 生命周期的实际建立、重建与销毁经 [`SessionHost`] 归宿主——
//! 产品公共重配置 API 是已知缺口（`StylizedPlugin::finish` 当前只初始化
//! 能力结果），本层只定义协调与宿主接口，不能直接写 resolved 资源冒充
//! 重新初始化。
//!
//! 身份规则：
//!
//! - `SessionPlan` 是可重复配置（以 scenario-model 的 session hash 标识），
//!   `SessionId` 是一次实际生命周期；同计划重建设备也分配新 SessionId；
//!   计数（session/request）在协调器生命周期内单调，不回绕、不复用；
//! - 接管失败/被取消：原会话 A 保持原身份恢复，候选 B 有失败记录并只清理 B；
//! - device loss：作废旧回执槽位，新 session 身份不同，晚到回执经
//!   [`SessionCoordinator::route_receipt`] 判定为忽略，不能命中新槽位；
//! - 严格 GPU gate 的每用例进程隔离不变（R4）；本协调器服务共享会话模式
//!   （交互与明确的生命周期测试）。

use std::collections::HashSet;

use scenario_model::ident::{RequestId, SessionId};
use scenario_model::identity::session_identity;
use scenario_model::recipe::SessionPlan;

/// 宿主接口：设备/会话操作的实际执行者。
///
/// 调用按协调器的事务步逐次发出；宿主可以把耗时操作拆到多次 `update`
/// 之间（每次调用必须非阻塞地返回当步结果）。句柄是宿主的诊断标识，
/// 不进入工作台身份。
pub trait SessionHost {
    /// 宿主错误。
    type Error: std::fmt::Display;

    /// 建立全新会话（App/Device 生命周期）。
    fn establish(&mut self, plan: &SessionPlan) -> Result<String, Self::Error>;
    /// 重配置事务的重建步（候选会话）。
    fn rebuild(&mut self, plan: &SessionPlan) -> Result<String, Self::Error>;
    /// 校验候选会话可恢复。
    fn validate(&mut self, handle: &str) -> Result<(), Self::Error>;
    /// 销毁句柄关联的资源（失败接管只清理候选侧句柄）。
    fn teardown(&mut self, handle: String);
}

/// 会话槽位：一次实际生命周期。
#[derive(Clone, Debug, PartialEq)]
struct SessionSlot {
    id: SessionId,
    plan_hash: String,
    handle: String,
}

/// 检查点：事务排空后保存的配方/水位记录。
#[derive(Clone, Debug, PartialEq)]
pub struct SessionCheckpoint {
    /// 被退役会话。
    pub retired_session: SessionId,
    /// 候选计划 hash。
    pub candidate_plan_hash: String,
    /// 排空时作废的在途请求。
    pub drained_requests: Vec<RequestId>,
}

/// 协调器状态。
#[derive(Clone, Debug, PartialEq)]
pub enum CoordState {
    /// 接受中（可发新请求、可开始事务）。
    Active,
    /// 停止接收，排空/作废在途请求。
    Draining,
    /// 检查点已保存，宿主重建中。
    Rebuilding,
    /// 宿主校验中。
    Validating,
}

/// 结构化会话事件（证据日志）。
#[derive(Clone, Debug, PartialEq)]
pub enum SessionEvent {
    /// 新会话建立。
    Established(SessionId),
    /// 事务开始（停止接收）。
    TransactionBegan,
    /// 在途请求被作废（排空/device loss）。
    RequestsVoided(Vec<RequestId>),
    /// 检查点已保存。
    CheckpointSaved(SessionCheckpoint),
    /// 候选接管失败：原会话恢复，只清理候选。
    TakeoverFailed { reason: String },
    /// 事务完成：新会话接管。
    TakeoverCommitted {
        retired: SessionId,
        current: SessionId,
    },
    /// device loss：严格运行失败，新会话身份不同。
    DeviceLoss {
        retired: SessionId,
        current: SessionId,
    },
}

/// 回执路由判定。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptVerdict {
    /// 当前会话的已知在途请求，可接受。
    Accept,
    /// 已作废（排空/取消）的请求。
    IgnoreVoided,
    /// 不属于当前会话（旧生命周期晚到）。
    IgnoreRetired,
    /// 当前会话从未签发该请求。
    IgnoreUnknown,
}

/// 同 SessionPlan 场景切换判定。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneSwitchVerdict {
    /// 同一计划：合法原地切换，会话身份不变。
    InPlace,
    /// 跨计划：必须经重配置事务建立新会话身份。
    RequiresNewSession,
}

struct Transaction {
    candidate_plan: SessionPlan,
    candidate_hash: String,
    attempt: Option<String>,
}

/// 会话协调器：非阻塞事务状态机 + 身份分配 + 回执路由。
pub struct SessionCoordinator {
    current: Option<SessionSlot>,
    transaction: Option<Transaction>,
    state: CoordState,
    next_session: u64,
    next_request: u64,
    in_flight: HashSet<RequestId>,
    voided: HashSet<RequestId>,
    log: Vec<SessionEvent>,
}

impl Default for SessionCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionCoordinator {
    /// 空协调器（无活动会话）。
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: None,
            transaction: None,
            state: CoordState::Active,
            next_session: 0,
            next_request: 0,
            in_flight: HashSet::new(),
            voided: HashSet::new(),
            log: Vec::new(),
        }
    }

    fn alloc_session(&mut self) -> SessionId {
        self.next_session += 1;
        SessionId::new(format!("s{}", self.next_session)).expect("generated session id")
    }

    fn alloc_request(&mut self) -> RequestId {
        self.next_request += 1;
        RequestId::new(format!("r{}", self.next_request)).expect("generated request id")
    }

    /// 事件日志（证据用，只追加）。
    #[must_use]
    pub fn log(&self) -> &[SessionEvent] {
        &self.log
    }

    /// 当前状态。
    #[must_use]
    pub fn state(&self) -> &CoordState {
        &self.state
    }

    /// 当前会话身份。
    #[must_use]
    pub fn current_session(&self) -> Option<&SessionId> {
        self.current.as_ref().map(|slot| &slot.id)
    }

    fn plan_hash(plan: &SessionPlan) -> Result<String, String> {
        session_identity(plan).map_err(|error| error.to_string())
    }

    /// 建立首个会话。
    pub fn establish<H: SessionHost>(
        &mut self,
        host: &mut H,
        plan: &SessionPlan,
    ) -> Result<SessionId, String> {
        if self.current.is_some() {
            return Err("session already active; use the reconfiguration transaction".into());
        }
        let plan_hash = Self::plan_hash(plan)?;
        let handle = host.establish(plan).map_err(|error| error.to_string())?;
        let id = self.alloc_session();
        self.current = Some(SessionSlot {
            id: id.clone(),
            plan_hash,
            handle,
        });
        self.log.push(SessionEvent::Established(id.clone()));
        Ok(id)
    }

    /// 签发一个请求身份（仅接受中；事务/恢复期间拒绝——"停止接收"）。
    pub fn issue_request(&mut self) -> Result<RequestId, String> {
        if self.state != CoordState::Active || self.current.is_none() {
            return Err("coordinator is not accepting requests".into());
        }
        let id = self.alloc_request();
        self.in_flight.insert(id.clone());
        Ok(id)
    }

    /// 完成一个请求（移出在途集合）。
    pub fn complete_request(&mut self, request: &RequestId) {
        self.in_flight.remove(request);
    }

    /// 同 SessionPlan 场景切换判定：同计划原地切换复用会话身份。
    pub fn scene_switch_verdict(&self, plan: &SessionPlan) -> Result<SceneSwitchVerdict, String> {
        let Some(slot) = &self.current else {
            return Err("no active session".into());
        };
        Ok(if slot.plan_hash == Self::plan_hash(plan)? {
            SceneSwitchVerdict::InPlace
        } else {
            SceneSwitchVerdict::RequiresNewSession
        })
    }

    /// 开始重配置事务：停止接收 → 进入排空。
    pub fn begin_reconfigure(&mut self, candidate: SessionPlan) -> Result<(), String> {
        if self.state != CoordState::Active || self.current.is_none() {
            return Err("no active session to reconfigure".into());
        }
        let candidate_hash = Self::plan_hash(&candidate)?;
        self.transaction = Some(Transaction {
            candidate_plan: candidate,
            candidate_hash,
            attempt: None,
        });
        self.state = CoordState::Draining;
        self.log.push(SessionEvent::TransactionBegan);
        Ok(())
    }

    /// 非阻塞事务步进：每个状态一次调用做一步（宿主调用必须非阻塞）。
    pub fn update<H: SessionHost>(&mut self, host: &mut H) -> Result<(), String> {
        let Some(mut transaction) = self.transaction.take() else {
            return Ok(());
        };
        let mut keep_transaction = true;
        match self.state {
            CoordState::Draining => {
                // 排空/作废在途请求并保存检查点。
                let drained: Vec<RequestId> = self.in_flight.drain().collect();
                for request in &drained {
                    self.voided.insert(request.clone());
                }
                self.log.push(SessionEvent::RequestsVoided(drained.clone()));
                let checkpoint = SessionCheckpoint {
                    retired_session: self
                        .current
                        .as_ref()
                        .map(|slot| slot.id.clone())
                        .ok_or("active session lost mid-transaction")?,
                    candidate_plan_hash: transaction.candidate_hash.clone(),
                    drained_requests: drained,
                };
                self.log.push(SessionEvent::CheckpointSaved(checkpoint));
                self.state = CoordState::Rebuilding;
            }
            CoordState::Rebuilding => match host.rebuild(&transaction.candidate_plan) {
                Ok(handle) => {
                    transaction.attempt = Some(handle);
                    self.state = CoordState::Validating;
                }
                Err(error) => {
                    self.fail_takeover(transaction.attempt.take(), error.to_string(), host);
                    keep_transaction = false;
                }
            },
            CoordState::Validating => {
                let attempt = transaction
                    .attempt
                    .clone()
                    .ok_or("rebuild produced no handle")?;
                match host.validate(&attempt) {
                    Ok(()) => {
                        // 接管：新 SessionId，旧句柄退役。
                        let retired = self.current.take().ok_or("active session lost")?;
                        host.teardown(retired.handle.clone());
                        let id = self.alloc_session();
                        self.current = Some(SessionSlot {
                            id: id.clone(),
                            plan_hash: transaction.candidate_hash.clone(),
                            handle: attempt,
                        });
                        self.log.push(SessionEvent::TakeoverCommitted {
                            retired: retired.id.clone(),
                            current: id,
                        });
                        self.state = CoordState::Active;
                        keep_transaction = false;
                    }
                    Err(error) => {
                        self.fail_takeover(transaction.attempt.take(), error.to_string(), host);
                        keep_transaction = false;
                    }
                }
            }
            CoordState::Active => {}
        }
        if keep_transaction {
            self.transaction = Some(transaction);
        }
        Ok(())
    }

    /// 接管失败：原会话 A 保持原身份恢复，只清理候选 B 的句柄。
    fn fail_takeover<H: SessionHost>(
        &mut self,
        attempt: Option<String>,
        reason: String,
        host: &mut H,
    ) {
        if let Some(attempt) = attempt {
            host.teardown(attempt);
        }
        self.log.push(SessionEvent::TakeoverFailed { reason });
        self.state = CoordState::Active;
    }

    /// device loss：作废在途，退役旧会话，同计划建立新会话（新身份）。
    pub fn report_device_loss<H: SessionHost>(
        &mut self,
        host: &mut H,
        plan: &SessionPlan,
    ) -> Result<SessionId, String> {
        let retired = self.current.take().ok_or("no active session")?;
        let drained: Vec<RequestId> = self.in_flight.drain().collect();
        for request in &drained {
            self.voided.insert(request.clone());
        }
        self.log.push(SessionEvent::RequestsVoided(drained));
        host.teardown(retired.handle.clone());
        let plan_hash = Self::plan_hash(plan)?;
        let handle = host.establish(plan).map_err(|error| error.to_string())?;
        let id = self.alloc_session();
        self.current = Some(SessionSlot {
            id: id.clone(),
            plan_hash,
            handle,
        });
        self.state = CoordState::Active;
        self.log.push(SessionEvent::DeviceLoss {
            retired: retired.id,
            current: id.clone(),
        });
        Ok(id)
    }

    /// 回执路由：只有当前会话签发的、未作废的在途请求可接受；
    /// 旧会话晚到回执不能命中新槽位。
    #[must_use]
    pub fn route_receipt(&self, session: &SessionId, request: &RequestId) -> ReceiptVerdict {
        let Some(slot) = &self.current else {
            return ReceiptVerdict::IgnoreRetired;
        };
        if slot.id != *session {
            return ReceiptVerdict::IgnoreRetired;
        }
        if self.voided.contains(request) {
            return ReceiptVerdict::IgnoreVoided;
        }
        if self.in_flight.contains(request) {
            return ReceiptVerdict::Accept;
        }
        ReceiptVerdict::IgnoreUnknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_stylized::prelude::StylizedExecutionTier;
    use scenario_model::canon::CanonicalSet;
    use scenario_model::recipe::{AssetSource, Backend, RunnerKind, SESSION_PLAN_SCHEMA};

    fn plan(limits: &str) -> SessionPlan {
        SessionPlan {
            schema: SESSION_PLAN_SCHEMA,
            backend: Backend::Vulkan,
            device_features: CanonicalSet::new(vec![]),
            limits_profile: limits.to_string(),
            tier_ceiling: StylizedExecutionTier::Standard,
            plugins: CanonicalSet::new(vec![]),
            asset_source: AssetSource::Filesystem,
            instrumentation: CanonicalSet::new(vec![]),
            runner: RunnerKind::Headless,
        }
    }

    struct FakeHost {
        established: Vec<String>,
        torn_down: Vec<String>,
        rebuild_result: Result<(), ()>,
        validate_result: Result<(), ()>,
        attempts: u32,
    }

    impl Default for FakeHost {
        fn default() -> Self {
            Self {
                established: Vec::new(),
                torn_down: Vec::new(),
                rebuild_result: Ok(()),
                validate_result: Ok(()),
                attempts: 0,
            }
        }
    }

    impl SessionHost for FakeHost {
        type Error = String;

        fn establish(&mut self, _plan: &SessionPlan) -> Result<String, Self::Error> {
            let handle = format!("handle-{}", self.established.len());
            self.established.push(handle.clone());
            Ok(handle)
        }

        fn rebuild(&mut self, _plan: &SessionPlan) -> Result<String, Self::Error> {
            self.attempts += 1;
            self.rebuild_result
                .map(|()| format!("attempt-{}", self.attempts))
                .map_err(|()| "rebuild failed".to_string())
        }

        fn validate(&mut self, _handle: &str) -> Result<(), Self::Error> {
            self.validate_result
                .map_err(|()| "validate failed".to_string())
        }

        fn teardown(&mut self, handle: String) {
            self.torn_down.push(handle);
        }
    }

    fn drive_to_active<H: SessionHost>(coordinator: &mut SessionCoordinator, host: &mut H) {
        for _ in 0..8 {
            coordinator.update(host).expect("update");
            if *coordinator.state() == CoordState::Active {
                return;
            }
        }
        panic!("transaction did not reach a terminal state");
    }

    /// 验收行：同 session 的 A 已提交 → 候选 B 接管失败 → A 保持原身份，
    /// B 有失败记录并只清理 B。
    #[test]
    fn committed_a_survives_failed_takeover_b() {
        let mut coordinator = SessionCoordinator::new();
        let mut host = FakeHost {
            rebuild_result: Err(()),
            ..FakeHost::default()
        };
        let a = coordinator
            .establish(&mut host, &plan("a"))
            .expect("establish");
        let request = coordinator.issue_request().expect("issue");
        coordinator
            .begin_reconfigure(plan("b"))
            .expect("begin reconfigure");
        // 事务期间停止接收。
        assert!(coordinator.issue_request().is_err());
        drive_to_active(&mut coordinator, &mut host);

        assert_eq!(
            coordinator.current_session(),
            Some(&a),
            "A keeps its identity"
        );
        assert!(
            !host.torn_down.contains(&"handle-0".to_string()),
            "A is not cleaned"
        );
        // 在途请求已排空作废；晚到回执判定为 IgnoreVoided。
        assert_eq!(
            coordinator.route_receipt(&a, &request),
            ReceiptVerdict::IgnoreVoided
        );
        assert!(
            coordinator
                .log()
                .iter()
                .any(|event| matches!(event, SessionEvent::TakeoverFailed { .. }))
        );
        assert!(
            coordinator
                .log()
                .iter()
                .any(|event| matches!(event, SessionEvent::CheckpointSaved(_)))
        );
        // 会话计数不回绕：接管失败不消耗新会话身份。
        assert_eq!(coordinator.next_session, 1);
    }

    /// 验收行变体：重建成功但校验失败——同样只清理候选。
    #[test]
    fn failed_validation_cleans_only_the_candidate() {
        let mut coordinator = SessionCoordinator::new();
        let mut host = FakeHost {
            rebuild_result: Ok(()),
            validate_result: Err(()),
            ..FakeHost::default()
        };
        let a = coordinator
            .establish(&mut host, &plan("a"))
            .expect("establish");
        coordinator
            .begin_reconfigure(plan("b"))
            .expect("begin reconfigure");
        drive_to_active(&mut coordinator, &mut host);
        assert_eq!(coordinator.current_session(), Some(&a));
        assert_eq!(host.torn_down, vec!["attempt-1".to_string()]);
        assert_eq!(coordinator.next_session, 1);
    }

    /// 验收行：请求在途 → device loss → 同计划新 session → 旧回执不能命中新槽位。
    #[test]
    fn device_loss_voids_in_flight_and_old_receipts_miss_new_slots() {
        let mut coordinator = SessionCoordinator::new();
        let mut host = FakeHost {
            rebuild_result: Ok(()),
            ..FakeHost::default()
        };
        let a = coordinator
            .establish(&mut host, &plan("a"))
            .expect("establish");
        let request = coordinator.issue_request().expect("issue");
        let b = coordinator
            .report_device_loss(&mut host, &plan("a"))
            .expect("new session");

        assert_ne!(
            a, b,
            "same-plan rebuild must still produce a new session id"
        );
        assert_eq!(coordinator.current_session(), Some(&b));
        // 旧回执：旧 session 命中 IgnoreRetired；旧 request 即使冒名新 session
        // 也已作废/未知，不能命中新槽位。
        assert_eq!(
            coordinator.route_receipt(&a, &request),
            ReceiptVerdict::IgnoreRetired
        );
        assert_eq!(
            coordinator.route_receipt(&b, &request),
            ReceiptVerdict::IgnoreVoided
        );
        // 新会话签发新请求：身份不复用。
        let fresh = coordinator.issue_request().expect("issue after loss");
        assert_ne!(fresh, request);
        assert_eq!(
            coordinator.route_receipt(&b, &fresh),
            ReceiptVerdict::Accept
        );
        assert!(
            coordinator
                .log()
                .iter()
                .any(|event| matches!(event, SessionEvent::DeviceLoss { .. }))
        );
    }

    /// 同计划场景切换复用会话身份；跨计划必须新会话。
    #[test]
    fn scene_switch_reuses_session_only_within_the_same_plan() {
        let mut coordinator = SessionCoordinator::new();
        let mut host = FakeHost::default();
        let a = coordinator
            .establish(&mut host, &plan("a"))
            .expect("establish");
        assert_eq!(
            coordinator
                .scene_switch_verdict(&plan("a"))
                .expect("verdict"),
            SceneSwitchVerdict::InPlace
        );
        assert_eq!(
            coordinator
                .scene_switch_verdict(&plan("b"))
                .expect("verdict"),
            SceneSwitchVerdict::RequiresNewSession
        );
        assert_eq!(coordinator.current_session(), Some(&a));
    }

    /// 成功接管：新会话身份接管，旧句柄退役。
    #[test]
    fn committed_takeover_assigns_fresh_session_identity() {
        let mut coordinator = SessionCoordinator::new();
        let mut host = FakeHost {
            rebuild_result: Ok(()),
            ..FakeHost::default()
        };
        let a = coordinator
            .establish(&mut host, &plan("a"))
            .expect("establish");
        coordinator
            .begin_reconfigure(plan("b"))
            .expect("begin reconfigure");
        drive_to_active(&mut coordinator, &mut host);
        let current = coordinator.current_session().expect("active").clone();
        assert_ne!(a, current);
        assert_eq!(coordinator.next_session, 2);
        assert!(host.torn_down.contains(&"handle-0".to_string()));
        assert!(
            coordinator
                .log()
                .iter()
                .any(|event| matches!(event, SessionEvent::TakeoverCommitted { .. }))
        );
    }
}
