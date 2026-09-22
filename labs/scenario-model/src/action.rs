//! 动作图（docs/showcase/01 §场景、配方与验证身份）。
//!
//! 动作包含施加参数、变形/移动、等待就绪、固定步进、观测与检查点；按依赖
//! 关系和稳定次序执行。本模块只承载纯数据与验证：
//!
//! - 时间轴使用整数 tick（[`Tick`]，规范十进制字符串编码）与有理步长
//!   （[`Step`]，分子/分母均为正整数）；
//! - 节点按声明顺序保存（有序列表，不排序）；执行次序由
//!   [`ActionGraph::stable_execution_order`] 以拓扑序 + 声明序决胜给出；
//! - 验证拒绝：重复节点 ID、循环依赖、悬空引用、未声明的同 tick 写冲突
//!   （两个动作在同一 tick 写同一目标且相互间没有声明的依赖路径）；
//! - 运行时 Entity/Handle 不进入本模型；目标一律使用稳定逻辑 ID
//!   （[`TargetRef`]）。配方不携带任意代码脚本。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::canon::{CanonicalU64, FiniteF64, ParameterMap};
use crate::ident::{ActionId, TargetId, ViewId};

/// 整数 tick（64 位，规范十进制字符串编码）。
pub type Tick = CanonicalU64;

/// 有理步长：固定步进每 tick 推进 `numerator/denominator` 秒。
///
/// 分子、分母均为正整数（64 位，规范十进制字符串编码）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// 分子（正）。
    pub numerator: CanonicalU64,
    /// 分母（正）。
    pub denominator: CanonicalU64,
}

impl Step {
    /// 构造（运行期入口；反序列化经 [`ActionGraph::validate`] 校验）。
    #[must_use]
    pub fn new(numerator: u64, denominator: u64) -> Self {
        Self {
            numerator: CanonicalU64(numerator),
            denominator: CanonicalU64(denominator),
        }
    }
}

/// 写入/引用目标的作用域。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TargetScope {
    /// 组合实体。
    Entity,
    /// 材质实例。
    Material,
    /// 灯光。
    Light,
    /// 视图（相机/渲染目标配置）。
    View,
}

/// 写入/引用目标：稳定逻辑 ID + 作用域。
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct TargetRef {
    /// 作用域。
    pub scope: TargetScope,
    /// 稳定逻辑 ID。
    pub id: TargetId,
}

/// 等待就绪条件。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReadyCondition {
    /// 声明的资产全部装载完成。
    AssetsLoaded,
    /// 渲染管线全部就绪。
    PipelinesReady,
    /// 连续稳定渲染窗口（帧数）。
    StableWindow {
        /// 要求的连续稳定帧数（正）。
        frames: u32,
    },
}

/// 动作种类（serde 以 `type` 标签区分；不携带任意代码）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionKind {
    /// 施加参数到目标。
    ApplyParams {
        /// 目标。
        target: TargetRef,
        /// 参数实例数据（map，不排序语义由字节层处理）。
        params: ParameterMap,
    },
    /// 变形（蒙皮/形态通道）。
    Deform {
        /// 目标。
        target: TargetRef,
        /// 变形通道名。
        channel: String,
        /// 通道值（声明 f64 精度）。
        value: FiniteF64,
    },
    /// 移动（平移分量，声明 f64 精度）。
    Move {
        /// 目标。
        target: TargetRef,
        /// 平移向量。
        translation: [FiniteF64; 3],
    },
    /// 等待就绪；不推进实验状态。
    WaitReady {
        /// 就绪条件。
        condition: ReadyCondition,
    },
    /// 固定步进：推进给定 tick 数，每 tick 有理步长。
    FixedStep {
        /// 推进 tick 数（正）。
        ticks: CanonicalU64,
        /// 每 tick 步长。
        step: Step,
    },
    /// 观测（采集一份命名观测；可选绑定视图）。
    Observe {
        /// 观测名（观测合同的键）。
        observation: String,
        /// 绑定视图。
        view: Option<ViewId>,
    },
    /// 检查点：记录已应用命令水位。
    Checkpoint {
        /// 检查点标签。
        label: String,
    },
}

/// 动作图节点。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ActionNode {
    /// 节点稳定逻辑 ID。
    pub id: ActionId,
    /// 计划执行 tick。
    pub tick: Tick,
    /// 依赖的节点 ID（全部完成后本节点才可执行）。
    #[serde(default)]
    pub depends_on: Vec<ActionId>,
    /// 本节点声明的写入目标（同 tick 冲突判定的输入）。
    #[serde(default)]
    pub writes: Vec<TargetRef>,
    /// 动作内容。
    pub kind: ActionKind,
}

/// 动作图：节点按声明顺序保存（有序列表）。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ActionGraph {
    /// 节点列表（声明序）。
    pub nodes: Vec<ActionNode>,
}

/// 动作图验证失败。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ActionError {
    /// 重复节点 ID。
    #[error("duplicate action id: {0}")]
    DuplicateId(String),
    /// 悬空依赖引用。
    #[error("action {node} depends on unknown action {reference}")]
    DanglingReference {
        /// 发起依赖的节点。
        node: String,
        /// 未知引用。
        reference: String,
    },
    /// 循环依赖。
    #[error("action graph has a dependency cycle involving: {}", nodes.join(", "))]
    Cycle {
        /// 环上节点（稳定序）。
        nodes: Vec<String>,
    },
    /// 未声明的同 tick 写冲突。
    #[error(
        "undeclared write conflict at tick {tick}: {first} and {second} both write {target_scope:?}/{target_id} without a declared ordering edge"
    )]
    WriteConflict {
        /// 冲突 tick。
        tick: String,
        /// 目标作用域。
        target_scope: TargetScope,
        /// 目标 ID。
        target_id: String,
        /// 先声明的节点。
        first: String,
        /// 后声明的节点。
        second: String,
    },
    /// 非法有理步长（分子/分母为 0）或步进 tick 数为 0。
    #[error("invalid step or tick count in action {0}")]
    InvalidStep(String),
}

impl ActionGraph {
    /// 校验：重复 ID、悬空引用、循环、非法步长、未声明同 tick 写冲突。
    pub fn validate(&self) -> Result<(), ActionError> {
        let mut ids = BTreeSet::new();
        for node in &self.nodes {
            if !ids.insert(node.id.as_str()) {
                return Err(ActionError::DuplicateId(node.id.to_string()));
            }
        }
        for node in &self.nodes {
            for dependency in &node.depends_on {
                if !ids.contains(dependency.as_str()) {
                    return Err(ActionError::DanglingReference {
                        node: node.id.to_string(),
                        reference: dependency.to_string(),
                    });
                }
            }
            match &node.kind {
                ActionKind::FixedStep { ticks, step } => {
                    if ticks.0 == 0 || step.numerator.0 == 0 || step.denominator.0 == 0 {
                        return Err(ActionError::InvalidStep(node.id.to_string()));
                    }
                }
                ActionKind::WaitReady {
                    condition: ReadyCondition::StableWindow { frames },
                } if *frames == 0 => {
                    return Err(ActionError::InvalidStep(node.id.to_string()));
                }
                _ => {}
            }
        }
        let order = self.stable_execution_order()?;
        // 未声明的同 tick 写冲突：同 tick 写同一目标且两条节点之间没有
        // 任一方向的（传递）依赖路径。
        let index_of: BTreeMap<&str, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect();
        let mut reachable: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); self.nodes.len()];
        // 按拓扑序累积传递闭包：reach(n) = {n} ∪ ⋃ reach(dep)。
        for &index in &order {
            reachable[index].insert(index);
            let deps: Vec<usize> = self.nodes[index]
                .depends_on
                .iter()
                .map(|dep| index_of[dep.as_str()])
                .collect();
            for dep in deps {
                let dep_reach: Vec<usize> = reachable[dep].iter().copied().collect();
                reachable[index].extend(dep_reach);
            }
        }
        for (tick, group) in group_by_tick(self) {
            for (position, &a) in group.iter().enumerate() {
                for &b in &group[position + 1..] {
                    if a == b {
                        continue;
                    }
                    if reachable[a].contains(&b) || reachable[b].contains(&a) {
                        continue;
                    }
                    let writes_a: BTreeSet<&TargetRef> = self.nodes[a].writes.iter().collect();
                    if let Some(target) = self.nodes[b]
                        .writes
                        .iter()
                        .find(|target| writes_a.contains(target))
                    {
                        return Err(ActionError::WriteConflict {
                            tick: tick.to_string(),
                            target_scope: target.scope,
                            target_id: target.id.to_string(),
                            first: self.nodes[a].id.to_string(),
                            second: self.nodes[b].id.to_string(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// 稳定执行次序：拓扑序，同层按声明序决胜；返回节点下标序列。
    ///
    /// 仅做拓扑排序与循环检测，不检查写冲突（完整校验见 [`Self::validate`]）。
    pub fn stable_execution_order(&self) -> Result<Vec<usize>, ActionError> {
        let index_of: BTreeMap<&str, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect();
        let mut indegree = vec![0usize; self.nodes.len()];
        let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); self.nodes.len()];
        for (index, node) in self.nodes.iter().enumerate() {
            for dependency in &node.depends_on {
                let Some(&dep) = index_of.get(dependency.as_str()) else {
                    return Err(ActionError::DanglingReference {
                        node: node.id.to_string(),
                        reference: dependency.to_string(),
                    });
                };
                indegree[index] += 1;
                dependents[dep].push(index);
            }
        }
        // Kahn：就绪集始终取声明序最小者，保证次序稳定。
        let mut ready: BTreeSet<usize> = (0..self.nodes.len())
            .filter(|index| indegree[*index] == 0)
            .collect();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(&index) = ready.iter().next() {
            ready.remove(&index);
            order.push(index);
            for &next in &dependents[index] {
                indegree[next] -= 1;
                if indegree[next] == 0 {
                    ready.insert(next);
                }
            }
        }
        if order.len() != self.nodes.len() {
            let mut nodes: Vec<String> = (0..self.nodes.len())
                .filter(|index| indegree[*index] > 0)
                .map(|index| self.nodes[index].id.to_string())
                .collect();
            nodes.sort();
            return Err(ActionError::Cycle { nodes });
        }
        Ok(order)
    }
}

fn group_by_tick(graph: &ActionGraph) -> BTreeMap<Tick, Vec<usize>> {
    let mut groups: BTreeMap<Tick, Vec<usize>> = BTreeMap::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        groups.entry(node.tick).or_default().push(index);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::TargetId;

    fn target(id: &str) -> TargetRef {
        TargetRef {
            scope: TargetScope::Material,
            id: TargetId::new(id).expect("target id"),
        }
    }

    fn node(id: &str, tick: u64, depends_on: &[&str], writes: &[&str]) -> ActionNode {
        ActionNode {
            id: ActionId::new(id).expect("action id"),
            tick: CanonicalU64(tick),
            depends_on: depends_on
                .iter()
                .map(|dep| ActionId::new(*dep).expect("dep id"))
                .collect(),
            writes: writes.iter().map(|w| target(w)).collect(),
            kind: ActionKind::Checkpoint {
                label: id.to_string(),
            },
        }
    }

    fn graph_of(nodes: Vec<ActionNode>) -> ActionGraph {
        ActionGraph { nodes }
    }

    #[test]
    fn rejects_duplicate_ids() {
        let graph = graph_of(vec![node("a", 0, &[], &[]), node("a", 1, &[], &[])]);
        assert_eq!(
            graph.validate(),
            Err(ActionError::DuplicateId("a".to_string()))
        );
    }

    #[test]
    fn rejects_dangling_reference() {
        let graph = graph_of(vec![node("a", 0, &["missing"], &[])]);
        assert_eq!(
            graph.validate(),
            Err(ActionError::DanglingReference {
                node: "a".to_string(),
                reference: "missing".to_string(),
            })
        );
    }

    #[test]
    fn rejects_cycles() {
        let graph = graph_of(vec![
            node("a", 0, &["c"], &[]),
            node("b", 1, &["a"], &[]),
            node("c", 2, &["b"], &[]),
        ]);
        assert!(matches!(graph.validate(), Err(ActionError::Cycle { .. })));
        // 自依赖同样是环。
        let self_cycle = graph_of(vec![node("a", 0, &["a"], &[])]);
        assert!(matches!(
            self_cycle.validate(),
            Err(ActionError::Cycle { .. })
        ));
    }

    #[test]
    fn rejects_undeclared_same_tick_write_conflict() {
        let graph = graph_of(vec![
            node("a", 5, &[], &["hero_face"]),
            node("b", 5, &[], &["hero_face"]),
        ]);
        assert!(matches!(
            graph.validate(),
            Err(ActionError::WriteConflict { tick, .. }) if tick == "5"
        ));
    }

    #[test]
    fn declared_ordering_edge_resolves_conflict() {
        let graph = graph_of(vec![
            node("a", 5, &[], &["hero_face"]),
            node("b", 5, &["a"], &["hero_face"]),
        ]);
        assert_eq!(graph.validate(), Ok(()));
        // 不同 tick 写同一目标不冲突。
        let spread = graph_of(vec![
            node("a", 5, &[], &["hero_face"]),
            node("b", 6, &[], &["hero_face"]),
        ]);
        assert_eq!(spread.validate(), Ok(()));
        // 同 tick 写不同目标不冲突。
        let disjoint = graph_of(vec![
            node("a", 5, &[], &["hero_face"]),
            node("b", 5, &[], &["hero_hair"]),
        ]);
        assert_eq!(disjoint.validate(), Ok(()));
    }

    #[test]
    fn stable_execution_order_is_topological_then_declared() {
        let graph = graph_of(vec![
            node("c", 2, &["a", "b"], &[]),
            node("b", 1, &[], &[]),
            node("a", 0, &[], &[]),
        ]);
        let order = graph.stable_execution_order().expect("order");
        let ids: Vec<&str> = order
            .iter()
            .map(|&index| graph.nodes[index].id.as_str())
            .collect();
        assert_eq!(ids, ["b", "a", "c"]);
    }

    #[test]
    fn rejects_zero_step_or_ticks() {
        let mut fixed = node("step", 0, &[], &[]);
        fixed.kind = ActionKind::FixedStep {
            ticks: CanonicalU64(0),
            step: Step::new(1, 60),
        };
        assert_eq!(
            graph_of(vec![fixed]).validate(),
            Err(ActionError::InvalidStep("step".to_string()))
        );
        let mut zero_denominator = node("step", 0, &[], &[]);
        zero_denominator.kind = ActionKind::FixedStep {
            ticks: CanonicalU64(10),
            step: Step::new(1, 0),
        };
        assert!(matches!(
            graph_of(vec![zero_denominator]).validate(),
            Err(ActionError::InvalidStep(_))
        ));
    }

    #[test]
    fn tick_serializes_as_decimal_string() {
        let json = serde_json::to_string(&node("a", 42, &[], &[])).expect("serialize");
        assert!(json.contains("\"tick\":\"42\""), "{json}");
    }
}
