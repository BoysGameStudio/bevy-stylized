//! 命令日志：持久化确认、恢复水位与确定性重放
//! （line 834 保存/重载/录制/撤销/回放保持逻辑身份；R17：状态不明的
//! 请求不自动重跑）。
//!
//! 语义：
//!
//! - 每条被接受的命令先记录为 [`JournalEntryState::Tentative`]（接受未
//!   确认），结果落笔后转为 [`JournalEntryState::Confirmed`]（拒绝也是
//!   一种已完成决定，随原因记录）；
//! - 载入时尾部的 Tentative 条目是**不确定**：只报告，绝不自动重跑
//!   （R17）；
//! - 重放 = 在新鲜会话上按序重发 Confirmed 命令，逐条核对结果（用例
//!   顺序、代次、拒绝）——逻辑身份确定性来自命令序列本身；
//! - 撤销 = 按日志前缀重放（回到某个已确认检查点，经确定性重建），
//!   不是状态逆操作；交互层的完整 undo 语义（部分提交回滚）不在本层。
//!
//! 持久化格式：版本化 JSON（schema 版本门控、重复键拒绝、未知版本拒绝，
//! 沿用 scenario-model 的摄入约定）；日志身份经 Execution 域摘要（记录
//! 型执行数据）。

use serde::{Deserialize, Serialize};

use bevy::prelude::{App, World};
use bevy_stylized::prelude::StylizedSchemaVersion;
use scenario_model::canon::{self, CanonError};
use scenario_model::ident::SessionId;
use scenario_model::identity::{self, ContractKind};

use crate::controller::{CommandOutcome, SessionCommand, SessionController};

/// 日志 schema 版本（major 变更 = 独立合同；minor = additive）。
pub const COMMAND_JOURNAL_SCHEMA: StylizedSchemaVersion = StylizedSchemaVersion::new(1, 0);

/// 条目状态（恢复水位）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum JournalEntryState {
    /// 接受未确认（载入时不确定：报告，不重跑）。
    Tentative,
    /// 结果已确认（含拒绝——拒绝是已完成的决定）。
    Confirmed(CommandOutcome),
}

/// 单条日志条目。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalEntry {
    /// 序号（从 0 单调）。
    pub sequence: u64,
    /// 命令。
    pub command: SessionCommand,
    /// 命令前代次。
    pub generation_before: u64,
    /// 状态（恢复水位）。
    pub state: JournalEntryState,
}

/// 命令日志（版本化持久化 + 身份摘要）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandJournal {
    /// schema 版本。
    pub schema: StylizedSchemaVersion,
    /// 记录时会话身份。
    pub session: SessionId,
    /// 初始用例下标（重放起点）。
    pub initial_case: usize,
    /// 条目（序号顺序）。
    pub entries: Vec<JournalEntry>,
}

/// 日志摄入失败。
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    /// 严格解析失败（语法/重复键）。
    #[error(transparent)]
    Strict(#[from] CanonError),
    /// 缺少 schema 版本字段。
    #[error("journal carries no `schema` version field")]
    MissingSchemaVersion,
    /// 未知/过新 schema 版本。
    #[error("unsupported journal schema version {found} (reader supports {supported})")]
    UnsupportedVersion {
        /// 输入声明的版本。
        found: StylizedSchemaVersion,
        /// 本实现支持的版本。
        supported: StylizedSchemaVersion,
    },
    /// 模型解码失败（含未知字段）。
    #[error("journal decode failed: {0}")]
    Model(String),
    /// 序号不连续。
    #[error("journal entry sequence is not contiguous from 0")]
    NonContiguousSequence,
}

impl CommandJournal {
    /// 空日志。
    #[must_use]
    pub fn new(session: SessionId, initial_case: usize) -> Self {
        Self {
            schema: COMMAND_JOURNAL_SCHEMA,
            session,
            initial_case,
            entries: Vec::new(),
        }
    }

    /// Execution 域身份摘要（条目与状态全部入载荷）。
    pub fn digest(&self) -> Result<String, CanonError> {
        let payload = canon::canonical_bytes(self)?;
        Ok(identity::digest_hex(&identity::contract_digest(
            ContractKind::Execution,
            canon::CANON_COMPILER_VERSION,
            &payload,
        )))
    }

    /// 版本化 JSON 输出（候选；由调用方决定原子发布）。
    pub fn to_json(&self) -> Result<Vec<u8>, JournalError> {
        serde_json::to_vec_pretty(self).map_err(|error| JournalError::Model(error.to_string()))
    }

    /// 摄入：严格解析（拒绝重复键）→ 版本门控 → 模型解码 → 序号连续性。
    pub fn from_json(bytes: &[u8]) -> Result<Self, JournalError> {
        let value = canon::parse_strict(bytes)?;
        let version = value
            .get("schema")
            .ok_or(JournalError::MissingSchemaVersion)?;
        let found: StylizedSchemaVersion = serde_json::from_value(version.clone())
            .map_err(|error| JournalError::Model(format!("schema version: {error}")))?;
        if !COMMAND_JOURNAL_SCHEMA.can_read(found) {
            return Err(JournalError::UnsupportedVersion {
                found,
                supported: COMMAND_JOURNAL_SCHEMA,
            });
        }
        let journal: CommandJournal = serde_json::from_value(value)
            .map_err(|error| JournalError::Model(error.to_string()))?;
        for (index, entry) in journal.entries.iter().enumerate() {
            if entry.sequence != index as u64 {
                return Err(JournalError::NonContiguousSequence);
            }
        }
        Ok(journal)
    }
}

/// 重放报告。
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayReport {
    /// 已重放条数。
    pub applied: usize,
    /// 尾部不确定条目序号（只报告，未重跑）。
    pub indeterminate: Vec<u64>,
    /// 结果不一致的条目（序号 + 记录结果 + 重放结果描述）。
    pub mismatches: Vec<String>,
}

impl ReplayReport {
    /// 全部已确认条目重放一致且无遗漏。
    #[must_use]
    pub fn is_exact(&self, journal: &CommandJournal) -> bool {
        self.mismatches.is_empty()
            && self.applied + self.indeterminate.len() == journal.entries.len()
    }
}

impl SessionController {
    /// 记录一条命令到日志（接受先记 Tentative，结果落笔转 Confirmed；
    /// 崩溃于两点之间的尾部 Tentative 在载入时按不确定报告，见 R17）。
    pub fn command_recorded(
        &mut self,
        world: &mut World,
        command: &SessionCommand,
        journal: &mut CommandJournal,
    ) -> CommandOutcome {
        let sequence = journal.entries.len() as u64;
        let generation_before = self.driver().generation().get();
        journal.entries.push(JournalEntry {
            sequence,
            command: command.clone(),
            generation_before,
            state: JournalEntryState::Tentative,
        });
        let outcome = self.command(world, command);
        journal.entries.last_mut().expect("entry just pushed").state =
            JournalEntryState::Confirmed(outcome.clone());
        outcome
    }

    /// 重放日志（桌面 headless 形式）：Confirmed 条目按序重发，每条被
    /// 接受的切换都推进到终态（与录制节奏一致），并核对结果；Tentative
    /// 尾部只报告。交互宿主应按同一节奏逐帧推进（replay 是宿主驱动操作，
    /// 泵送归宿主——本形式即桌面 headless 包装）。
    ///
    /// 调用方必须保证控制器处于与日志相同的初始用例（`initial_case`）；
    /// 重放用 `journal.initial_case` 对齐控制器当前位置——不一致时第一
    /// 条结果就会失配并被记录。
    pub fn replay(&mut self, app: &mut App, journal: &CommandJournal) -> ReplayReport {
        let mut report = ReplayReport {
            applied: 0,
            indeterminate: Vec::new(),
            mismatches: Vec::new(),
        };
        for entry in &journal.entries {
            let JournalEntryState::Confirmed(expected) = &entry.state else {
                report.indeterminate.push(entry.sequence);
                continue;
            };
            let generation_before = self.driver().generation().get();
            if generation_before != entry.generation_before {
                report.mismatches.push(format!(
                    "entry {}: generation before replay {} != recorded {}",
                    entry.sequence, generation_before, entry.generation_before
                ));
                continue;
            }
            let outcome = self.command(app.world_mut(), &entry.command);
            if &outcome != expected {
                report.mismatches.push(format!(
                    "entry {}: outcome {outcome:?} != recorded {expected:?}",
                    entry.sequence
                ));
                continue;
            }
            // 与录制节奏一致：接受的切换推进到终态再处理下一条。
            if matches!(outcome, CommandOutcome::Switched { .. })
                && let Err(error) = crate::headless::run_to_completion(app, self.driver_mut(), 700)
            {
                report.mismatches.push(format!(
                    "entry {}: switch build did not complete: {error}",
                    entry.sequence
                ));
                continue;
            }
            report.applied += 1;
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scenario_model::ident::SceneId;

    fn journal_with(states: Vec<JournalEntryState>) -> CommandJournal {
        CommandJournal {
            schema: COMMAND_JOURNAL_SCHEMA,
            session: SessionId::new("journal-test").expect("session"),
            initial_case: 6,
            entries: states
                .into_iter()
                .enumerate()
                .map(|(sequence, state)| JournalEntry {
                    sequence: sequence as u64,
                    command: SessionCommand::SelectCase(SceneId::new("animation").expect("id")),
                    generation_before: 0,
                    state,
                })
                .collect(),
        }
    }

    #[test]
    fn save_load_roundtrip_and_digest_stability() {
        let journal = journal_with(vec![
            JournalEntryState::Confirmed(CommandOutcome::Switched {
                case: SceneId::new("animation").expect("id"),
                generation: scenario_model::ident::Generation::new(1),
            }),
            JournalEntryState::Confirmed(CommandOutcome::Refused(
                crate::controller::RefusalReason::UnknownScene(SceneId::new("nope").expect("id")),
            )),
        ]);
        let bytes = journal.to_json().expect("encode");
        let back = CommandJournal::from_json(&bytes).expect("decode");
        assert_eq!(journal, back);
        assert_eq!(
            journal.digest().expect("digest"),
            back.digest().expect("digest")
        );
        assert_eq!(journal.digest().expect("digest").len(), 64);
    }

    #[test]
    fn rejects_unknown_versions_and_duplicate_keys() {
        let journal = journal_with(vec![]);
        let bytes = journal.to_json().expect("encode");
        let newer_major = String::from_utf8(bytes.clone())
            .expect("utf8")
            .replace("\"major\": 1", "\"major\": 2");
        assert!(matches!(
            CommandJournal::from_json(newer_major.as_bytes()),
            Err(JournalError::UnsupportedVersion { .. })
        ));
        let newer_minor = String::from_utf8(bytes.clone())
            .expect("utf8")
            .replace("\"minor\": 0", "\"minor\": 1");
        assert!(matches!(
            CommandJournal::from_json(newer_minor.as_bytes()),
            Err(JournalError::UnsupportedVersion { .. })
        ));
        let duplicate = String::from_utf8(bytes)
            .expect("utf8")
            .replace("\"schema\": {", "\"schema\": {\"major\": 1,");
        assert!(matches!(
            CommandJournal::from_json(duplicate.as_bytes()),
            Err(JournalError::Strict(CanonError::DuplicateProperty(_)))
        ));
        let missing = br#"{"session":"s1","initial_case":0,"entries":[]}"#;
        assert!(matches!(
            CommandJournal::from_json(missing),
            Err(JournalError::MissingSchemaVersion)
        ));
    }

    #[test]
    fn rejects_non_contiguous_sequences() {
        let mut journal = journal_with(vec![
            JournalEntryState::Tentative,
            JournalEntryState::Tentative,
        ]);
        journal.entries[1].sequence = 7;
        let bytes = journal.to_json().expect("encode");
        assert!(matches!(
            CommandJournal::from_json(&bytes),
            Err(JournalError::NonContiguousSequence)
        ));
    }
}
