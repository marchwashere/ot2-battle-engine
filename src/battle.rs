use crate::{
    ActorId, Catalog, Combatant, CrtRng, Enemy, PlayerCharacter, Resources, Result, Side, SimError,
    Skill, Stats,
    ai::{AiMemory, AiTarget, CompiledAi},
    content::{Effect, HitCheck, ItemCatalog, Recovery, Resource, TargetKind},
    ensure,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RulesProfile {
    ScaffoldV1,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleConfig {
    pub schema_version: u32,
    pub rules: RulesProfile,
    pub seed: u32,
    pub players: Vec<PlayerCharacter>,
    pub enemies: Vec<Enemy>,
    pub skills: Vec<Skill>,
    #[serde(default)]
    pub inventory: BTreeMap<String, u32>,
}
impl BattleConfig {
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Ongoing,
    Victory,
    Defeat,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BattleState {
    pub round: u32,
    pub actors: Vec<Combatant>,
    pub rng: CrtRng,
    pub outcome: Outcome,
    pub inventory: Vec<u32>,
    pub ai: Vec<Option<AiMemory>>,
    queue: Vec<ActorId>,
    cursor: usize,
}
#[derive(Debug)]
struct ActorDefinition {
    id: String,
    name: String,
    side: Side,
    stats: Stats,
    skills: Vec<usize>,
    ai: Option<CompiledAi>,
}
#[derive(Debug)]
struct Scenario {
    catalog: Catalog,
    actors: Vec<ActorDefinition>,
}
/// Cloning copies only mutable state and shares immutable scenario/content.
#[derive(Debug, Clone)]
pub struct Battle {
    scenario: Arc<Scenario>,
    state: BattleState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Skill { id: String, target: Option<ActorId> },
    Item { id: String, target: Option<ActorId> },
    Wait,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    Action {
        actor: ActorId,
        name: String,
    },
    ResourceChanged {
        actor: ActorId,
        resource: Resource,
        before: u32,
        after: u32,
    },
    Miss {
        actor: ActorId,
        target: ActorId,
    },
    KnockedOut {
        actor: ActorId,
    },
    RoundStarted {
        round: u32,
    },
    BattleEnded {
        outcome: Outcome,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StepReport {
    pub actor: ActorId,
    pub events: Vec<Event>,
    pub rng_before: CrtRng,
    pub rng_after: CrtRng,
    pub outcome: Outcome,
}
enum Action {
    Skill(usize),
    Item(usize),
    Wait,
}
struct Prepared {
    action: Action,
    targets: Vec<ActorId>,
}

impl Battle {
    pub fn new(config: BattleConfig, items: ItemCatalog) -> Result<Self> {
        ensure(
            config.schema_version == 1,
            "unsupported battle schema_version",
        )?;
        ensure(
            !config.players.is_empty() && !config.enemies.is_empty(),
            "a battle needs both players and enemies",
        )?;
        ensure(
            config.players.len() + config.enemies.len() <= 64,
            "scaffold supports at most 64 combatants",
        )?;
        let catalog = Catalog::new(config.skills, items)?;
        let mut definitions = Vec::new();
        let mut actors = Vec::new();
        let mut ids = BTreeSet::new();
        let mut add = |id: String,
                       name: String,
                       side,
                       stats: Stats,
                       resources: Option<Resources>,
                       skills: Vec<String>,
                       ai: Option<crate::ai::BossAi>|
         -> Result<()> {
            ensure(
                !id.is_empty() && ids.insert(id.clone()),
                "empty or duplicate combatant id",
            )?;
            stats.validate()?;
            let resources = resources.unwrap_or_else(|| Resources::full(&stats));
            resources.validate(&stats)?;
            let skills: Vec<_> = skills
                .iter()
                .map(|s| catalog.skill_id(s))
                .collect::<Result<_>>()?;
            ensure(
                skills.iter().collect::<BTreeSet<_>>().len() == skills.len(),
                "duplicate learned skill",
            )?;
            let ai = ai.map(|a| a.compile(&catalog, &skills)).transpose()?;
            definitions.push(ActorDefinition {
                id,
                name,
                side,
                stats,
                skills,
                ai,
            });
            actors.push(Combatant { resources });
            Ok(())
        };
        for p in config.players {
            add(
                p.id,
                p.name,
                Side::Player,
                p.stats,
                p.resources,
                p.skills,
                None,
            )?;
        }
        for e in config.enemies {
            add(
                e.id,
                e.name,
                Side::Enemy,
                e.stats,
                e.resources,
                e.skills,
                Some(e.ai),
            )?;
        }
        let mut inventory = vec![0; catalog.items.len()];
        for (id, count) in config.inventory {
            inventory[catalog.item_id(&id)?] = count;
        }
        let ai = definitions
            .iter()
            .map(|d| d.ai.as_ref().map(CompiledAi::memory))
            .collect();
        let mut battle = Self {
            scenario: Arc::new(Scenario {
                catalog,
                actors: definitions,
            }),
            state: BattleState {
                round: 0,
                actors,
                rng: CrtRng::new(config.seed),
                inventory,
                ai,
                queue: Vec::new(),
                cursor: 0,
                outcome: Outcome::Ongoing,
            },
        };
        battle.refresh_outcome();
        if battle.state.outcome == Outcome::Ongoing {
            battle.start_round();
        }
        Ok(battle)
    }
    pub fn state(&self) -> &BattleState {
        &self.state
    }
    pub fn catalog(&self) -> &Catalog {
        &self.scenario.catalog
    }
    pub fn actor_id(&self, id: &str) -> Option<ActorId> {
        self.scenario
            .actors
            .iter()
            .position(|d| d.id == id)
            .map(ActorId)
    }
    pub fn actor_name(&self, actor: ActorId) -> Option<&str> {
        self.scenario.actors.get(actor.0).map(|d| d.name.as_str())
    }
    pub fn side(&self, actor: ActorId) -> Option<Side> {
        self.scenario.actors.get(actor.0).map(|d| d.side)
    }
    pub fn current_actor(&self) -> Option<ActorId> {
        if self.state.outcome == Outcome::Ongoing {
            self.state.queue.get(self.state.cursor).copied()
        } else {
            None
        }
    }
    pub fn item_count(&self, id: &str) -> Result<u32> {
        Ok(self.state.inventory[self.catalog().item_id(id)?])
    }
    pub fn step_player(&mut self, command: Command) -> Result<StepReport> {
        let actor = self
            .current_actor()
            .ok_or_else(|| SimError("battle is over".into()))?;
        ensure(
            self.side(actor) == Some(Side::Player),
            "it is an enemy turn",
        )?;
        let prepared = match command {
            Command::Skill { id, target } => {
                self.prepare(actor, Action::Skill(self.catalog().skill_id(&id)?), target)?
            }
            Command::Item { id, target } => {
                self.prepare(actor, Action::Item(self.catalog().item_id(&id)?), target)?
            }
            Command::Wait => Prepared {
                action: Action::Wait,
                targets: Vec::new(),
            },
        };
        let before = self.state.rng;
        Ok(self.execute(actor, prepared, before))
    }
    pub fn step_enemy(&mut self) -> Result<StepReport> {
        let actor = self
            .current_actor()
            .ok_or_else(|| SimError("battle is over".into()))?;
        ensure(self.side(actor) == Some(Side::Enemy), "it is a player turn")?;
        let before = self.state.rng;
        let memory_before = self.state.ai[actor.0].clone();
        let scenario = Arc::clone(&self.scenario);
        let definition = &scenario.actors[actor.0];
        let prepared = (|| {
            let ai = definition
                .ai
                .as_ref()
                .ok_or_else(|| SimError("enemy has no AI".into()))?;
            let choice = ai.choose(
                self.state.ai[actor.0]
                    .as_mut()
                    .expect("validated AI memory"),
                self.state.actors[actor.0].resources.hp,
                definition.stats.max_hp,
                self.state.round,
                &mut self.state.rng,
            )?;
            let target = self.ai_target(actor, choice.target)?;
            self.prepare(actor, Action::Skill(choice.skill), target)
        })();
        match prepared {
            Ok(action) => Ok(self.execute(actor, action, before)),
            Err(error) => {
                self.state.rng = before;
                self.state.ai[actor.0] = memory_before;
                Err(error)
            }
        }
    }
    /// Bounded stepping: returns to the caller at the next player decision.
    pub fn advance_enemies(&mut self, limit: usize) -> Result<Vec<StepReport>> {
        let mut reports = Vec::new();
        while let Some(actor) = self.current_actor() {
            if self.side(actor) == Some(Side::Player) {
                break;
            }
            ensure(reports.len() < limit, "enemy action budget exhausted")?;
            reports.push(self.step_enemy()?);
        }
        Ok(reports)
    }
    fn prepare(&self, actor: ActorId, action: Action, target: Option<ActorId>) -> Result<Prepared> {
        let kind = match action {
            Action::Skill(index) => {
                ensure(
                    self.scenario.actors[actor.0].skills.contains(&index),
                    "actor has not learned this skill",
                )?;
                let skill = &self.catalog().skills[index];
                ensure(
                    self.state.actors[actor.0].resources.sp >= skill.sp_cost,
                    "not enough SP",
                )?;
                skill.target
            }
            Action::Item(index) => {
                ensure(
                    self.side(actor) == Some(Side::Player),
                    "enemies cannot use the party inventory",
                )?;
                ensure(self.state.inventory[index] > 0, "item is out of stock")?;
                self.catalog().items[index].target
            }
            Action::Wait => {
                return Ok(Prepared {
                    action,
                    targets: Vec::new(),
                });
            }
        };
        let side = self.scenario.actors[actor.0].side;
        let allies = |id: ActorId| self.side(id) == Some(side);
        let targets = match kind {
            TargetKind::SelfOnly => {
                ensure(
                    target.is_none() || target == Some(actor),
                    "self-only action cannot target someone else",
                )?;
                vec![actor]
            }
            TargetKind::Ally | TargetKind::Opponent => {
                let id = target
                    .ok_or_else(|| SimError("single-target action requires a target".into()))?;
                ensure(
                    self.state.actors.get(id.0).is_some_and(Combatant::alive),
                    "target is missing or knocked out; recovery does not revive",
                )?;
                ensure(
                    allies(id) == (kind == TargetKind::Ally),
                    "target belongs to the wrong side",
                )?;
                vec![id]
            }
            TargetKind::AllAllies | TargetKind::AllOpponents => {
                ensure(
                    target.is_none(),
                    "all-target action does not take a single target",
                )?;
                self.state
                    .actors
                    .iter()
                    .enumerate()
                    .filter(|(i, a)| {
                        a.alive() && allies(ActorId(*i)) == (kind == TargetKind::AllAllies)
                    })
                    .map(|(i, _)| ActorId(i))
                    .collect()
            }
        };
        ensure(!targets.is_empty(), "no living targets")?;
        Ok(Prepared { action, targets })
    }
    fn ai_target(&mut self, actor: ActorId, selector: AiTarget) -> Result<Option<ActorId>> {
        if matches!(selector, AiTarget::AllAllies | AiTarget::AllOpponents) {
            return Ok(None);
        }
        if selector == AiTarget::SelfOnly {
            return Ok(Some(actor));
        }
        let ally = selector == AiTarget::LowestHpAlly;
        let side = self.side(actor);
        let mut candidates: Vec<_> = self
            .state
            .actors
            .iter()
            .enumerate()
            .filter(|(i, a)| a.alive() && (self.side(ActorId(*i)) == side) == ally)
            .map(|(i, _)| ActorId(i))
            .collect();
        ensure(!candidates.is_empty(), "AI has no living targets")?;
        let index = match selector {
            AiTarget::RandomOpponent => self.state.rng.index(candidates.len() as u32) as usize,
            AiTarget::LowestHpAlly => {
                candidates.sort_by_key(|a| (self.state.actors[a.0].resources.hp, a.0));
                0
            }
            _ => 0,
        };
        Ok(Some(candidates[index]))
    }
    fn execute(&mut self, actor: ActorId, action: Prepared, rng_before: CrtRng) -> StepReport {
        let scenario = Arc::clone(&self.scenario);
        let mut events = Vec::new();
        let (name, effects, hit) = match action.action {
            Action::Skill(i) => {
                let skill = &scenario.catalog.skills[i];
                if skill.sp_cost != 0 {
                    let sp = &mut self.state.actors[actor.0].resources.sp;
                    let before = *sp;
                    *sp -= skill.sp_cost;
                    events.push(Event::ResourceChanged {
                        actor,
                        resource: Resource::Sp,
                        before,
                        after: *sp,
                    });
                }
                (skill.id.as_str(), skill.effects.as_slice(), &skill.hit)
            }
            Action::Item(i) => {
                self.state.inventory[i] -= 1;
                let item = &scenario.catalog.items[i];
                (
                    item.id.as_str(),
                    item.effects.as_slice(),
                    &HitCheck::Guaranteed,
                )
            }
            Action::Wait => ("wait", &[][..], &HitCheck::Guaranteed),
        };
        events.insert(
            0,
            Event::Action {
                actor,
                name: name.to_owned(),
            },
        );
        for target in action.targets {
            let success = match hit {
                HitCheck::Guaranteed => true,
                HitCheck::Weighted { probability } => self.state.rng.weighted_bool(*probability),
            };
            if !success {
                events.push(Event::Miss { actor, target });
                continue;
            }
            for effect in effects {
                if !self.state.actors[target.0].alive() {
                    break;
                }
                self.apply_effect(target, effect, &mut events);
            }
        }
        self.refresh_outcome();
        if self.state.outcome != Outcome::Ongoing {
            events.push(Event::BattleEnded {
                outcome: self.state.outcome,
            });
        } else {
            self.state.cursor += 1;
            while self.state.cursor < self.state.queue.len()
                && !self.state.actors[self.state.queue[self.state.cursor].0].alive()
            {
                self.state.cursor += 1;
            }
            if self.state.cursor == self.state.queue.len() {
                self.start_round();
                events.push(Event::RoundStarted {
                    round: self.state.round,
                });
            }
        }
        StepReport {
            actor,
            events,
            rng_before,
            rng_after: self.state.rng,
            outcome: self.state.outcome,
        }
    }
    fn apply_effect(&mut self, target: ActorId, effect: &Effect, events: &mut Vec<Event>) {
        let stats = &self.scenario.actors[target.0].stats;
        let resources = &mut self.state.actors[target.0].resources;
        let (resource, before, after) = match effect {
            Effect::FixedDamage { amount } => (
                Resource::Hp,
                resources.hp,
                resources.hp.saturating_sub(*amount),
            ),
            Effect::Restore { resource, recovery } => {
                let (before, cap) = match resource {
                    Resource::Hp => (resources.hp, stats.max_hp),
                    Resource::Sp => (resources.sp, stats.max_sp),
                    Resource::Bp => (u32::from(resources.bp), 5),
                    Resource::Latent => (u32::from(resources.latent), 100),
                };
                let amount = match recovery {
                    Recovery::Fixed { amount } => *amount,
                    Recovery::Percent { percent } => {
                        (u64::from(cap) * u64::from(*percent) / 100) as u32
                    }
                };
                (*resource, before, before.saturating_add(amount).min(cap))
            }
        };
        match resource {
            Resource::Hp => resources.hp = after,
            Resource::Sp => resources.sp = after,
            Resource::Bp => resources.bp = after as u8,
            Resource::Latent => resources.latent = after as u16,
        }
        events.push(Event::ResourceChanged {
            actor: target,
            resource,
            before,
            after,
        });
        if resource == Resource::Hp && before > 0 && after == 0 {
            events.push(Event::KnockedOut { actor: target });
        }
    }
    fn refresh_outcome(&mut self) {
        let alive = |side| {
            self.state
                .actors
                .iter()
                .enumerate()
                .any(|(i, a)| a.alive() && self.scenario.actors[i].side == side)
        };
        self.state.outcome = if !alive(Side::Player) {
            Outcome::Defeat
        } else if !alive(Side::Enemy) {
            Outcome::Victory
        } else {
            Outcome::Ongoing
        };
    }
    fn start_round(&mut self) {
        self.state.round = self.state.round.saturating_add(1);
        self.state.cursor = 0;
        self.state.queue = self
            .state
            .actors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.alive())
            .map(|(i, _)| ActorId(i))
            .collect();
        // Explicit scaffold rule: descending speed, stable actor ID ties, no RNG.
        // OT2's action-time calculation and turn-order RNG are not implemented.
        self.state.queue.sort_by_key(|a| {
            (
                std::cmp::Reverse(self.scenario.actors[a.0].stats.speed),
                a.0,
            )
        });
    }
}
