//! Small, explicit boss-AI interchange format, not an Unreal bytecode interpreter.
use crate::{Catalog, CrtRng, Result, SimError, content::TargetKind, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BossAi {
    pub schema_version: u32,
    pub initial_state: String,
    pub states: Vec<AiState>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiState {
    pub id: String,
    pub rules: Vec<AiRule>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiRule {
    pub condition: Condition,
    pub selection: Selection,
    #[serde(default)]
    pub next_state: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    Always,
    HpAtMost { percent: u8 },
    RoundModulo { divisor: u32, remainder: u32 },
    ActionsAtLeast { count: u64 },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selection {
    Sequence { actions: Vec<AiAction> },
    Weighted { choices: Vec<WeightedAction> },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightedAction {
    pub weight: u32,
    pub action: AiAction,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiAction {
    pub skill: String,
    pub target: AiTarget,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiTarget {
    SelfOnly,
    FirstOpponent,
    RandomOpponent,
    LowestHpAlly,
    AllOpponents,
    AllAllies,
}

impl BossAi {
    pub fn from_json(json: &str) -> Result<Self> {
        let value: Self = serde_json::from_str(json)?;
        value.validate_structure()?;
        Ok(value)
    }
    pub fn validate_structure(&self) -> Result<()> {
        ensure(
            self.schema_version == 1,
            "unsupported boss AI schema_version",
        )?;
        ensure(
            !self.states.is_empty() && self.states.len() <= 128,
            "AI needs 1..128 states",
        )?;
        let mut ids = BTreeMap::new();
        for (index, state) in self.states.iter().enumerate() {
            ensure(
                !state.id.is_empty() && ids.insert(&state.id, index).is_none(),
                "empty or duplicate AI state id",
            )?;
        }
        ensure(
            ids.contains_key(&self.initial_state),
            "AI initial_state does not exist",
        )?;
        for state in &self.states {
            ensure(
                !state.rules.is_empty() && state.rules.len() <= 64,
                "AI state needs 1..64 rules",
            )?;
            ensure(
                matches!(
                    state.rules.last().map(|r| &r.condition),
                    Some(Condition::Always)
                ),
                "AI state must end with an always fallback",
            )?;
            for (i, rule) in state.rules.iter().enumerate() {
                if let Some(next) = &rule.next_state {
                    ensure(
                        ids.contains_key(next),
                        format!("unknown AI next_state: {next}"),
                    )?;
                }
                match rule.condition {
                    Condition::HpAtMost { percent } => {
                        ensure(percent <= 100, "invalid AI HP percent")?
                    }
                    Condition::RoundModulo { divisor, remainder } => ensure(
                        divisor > 0 && remainder < divisor,
                        "invalid AI round modulo",
                    )?,
                    Condition::Always => {
                        ensure(i + 1 == state.rules.len(), "always rule must be last")?
                    }
                    Condition::ActionsAtLeast { .. } => (),
                }
                match &rule.selection {
                    Selection::Sequence { actions } => ensure(
                        !actions.is_empty() && actions.len() <= 256,
                        "AI sequence needs 1..256 actions",
                    )?,
                    Selection::Weighted { choices } => {
                        ensure(
                            !choices.is_empty() && choices.len() <= 256,
                            "AI weighted table needs 1..256 entries",
                        )?;
                        let mut sum = 0_u64;
                        for choice in choices {
                            ensure(choice.weight > 0, "AI weights must be positive")?;
                            sum += u64::from(choice.weight);
                        }
                        ensure(
                            sum <= i32::MAX as u64,
                            "AI weight sum exceeds supported range",
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
    pub(crate) fn compile(&self, catalog: &Catalog, learned: &[usize]) -> Result<CompiledAi> {
        self.validate_structure()?;
        let indices: BTreeMap<_, _> = self
            .states
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.as_str(), i))
            .collect();
        let action = |a: &AiAction| -> Result<CompiledAction> {
            let skill = catalog.skill_id(&a.skill)?;
            ensure(
                learned.contains(&skill),
                format!("AI uses unlearned skill: {}", a.skill),
            )?;
            let compatible = match catalog.skills[skill].target {
                TargetKind::SelfOnly => a.target == AiTarget::SelfOnly,
                TargetKind::Ally => matches!(a.target, AiTarget::SelfOnly | AiTarget::LowestHpAlly),
                TargetKind::Opponent => {
                    matches!(a.target, AiTarget::FirstOpponent | AiTarget::RandomOpponent)
                }
                TargetKind::AllAllies => a.target == AiTarget::AllAllies,
                TargetKind::AllOpponents => a.target == AiTarget::AllOpponents,
            };
            ensure(
                compatible,
                format!("AI target is incompatible with skill {}", a.skill),
            )?;
            Ok(CompiledAction {
                skill,
                target: a.target,
            })
        };
        let mut states = Vec::new();
        for state in &self.states {
            let mut rules = Vec::new();
            for rule in &state.rules {
                let selection = match &rule.selection {
                    Selection::Sequence { actions } => CompiledSelection::Sequence(
                        actions.iter().map(&action).collect::<Result<_>>()?,
                    ),
                    Selection::Weighted { choices } => CompiledSelection::Weighted(
                        choices
                            .iter()
                            .map(|c| Ok((c.weight, action(&c.action)?)))
                            .collect::<Result<_>>()?,
                    ),
                };
                rules.push(CompiledRule {
                    condition: rule.condition.clone(),
                    selection,
                    next_state: rule.next_state.as_ref().map(|s| indices[s.as_str()]),
                });
            }
            states.push(rules);
        }
        Ok(CompiledAi {
            initial: indices[self.initial_state.as_str()],
            states,
        })
    }
}

#[derive(Debug)]
pub(crate) struct CompiledAi {
    initial: usize,
    states: Vec<Vec<CompiledRule>>,
}
#[derive(Debug)]
struct CompiledRule {
    condition: Condition,
    selection: CompiledSelection,
    next_state: Option<usize>,
}
#[derive(Debug)]
enum CompiledSelection {
    Sequence(Vec<CompiledAction>),
    Weighted(Vec<(u32, CompiledAction)>),
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct CompiledAction {
    pub skill: usize,
    pub target: AiTarget,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiMemory {
    pub state: usize,
    pub actions_taken: u64,
    cursors: Vec<Vec<usize>>,
}
impl CompiledAi {
    pub fn memory(&self) -> AiMemory {
        AiMemory {
            state: self.initial,
            actions_taken: 0,
            cursors: self.states.iter().map(|s| vec![0; s.len()]).collect(),
        }
    }
    pub fn choose(
        &self,
        memory: &mut AiMemory,
        hp: u32,
        max_hp: u32,
        round: u32,
        rng: &mut CrtRng,
    ) -> Result<CompiledAction> {
        let state = memory.state;
        for (index, rule) in self.states[state].iter().enumerate() {
            let matches = match rule.condition {
                Condition::Always => true,
                Condition::HpAtMost { percent } => {
                    u64::from(hp) * 100 <= u64::from(max_hp) * u64::from(percent)
                }
                Condition::RoundModulo { divisor, remainder } => round % divisor == remainder,
                Condition::ActionsAtLeast { count } => memory.actions_taken >= count,
            };
            if !matches {
                continue;
            }
            let selected = match &rule.selection {
                CompiledSelection::Sequence(actions) => {
                    let cursor = &mut memory.cursors[state][index];
                    let action = actions[*cursor];
                    *cursor = (*cursor + 1) % actions.len();
                    action
                }
                CompiledSelection::Weighted(choices) => {
                    let total = choices.iter().map(|c| c.0).sum();
                    let mut roll = rng.index(total); // Deliberately draws even for one entry.
                    let mut action = choices[0].1;
                    for (weight, candidate) in choices {
                        if roll < *weight {
                            action = *candidate;
                            break;
                        }
                        roll -= weight;
                    }
                    action
                }
            };
            memory.actions_taken = memory.actions_taken.saturating_add(1);
            if let Some(next) = rule.next_state {
                memory.state = next;
            }
            return Ok(selected);
        }
        Err(SimError("AI has no matching rule".into()))
    }
}
