use crate::{
    ActorId, Catalog, Combatant, CrtRng, Enemy, PlayerCharacter, Resources, Result, Side, SimError,
    Skill, Stats,
    ai::{AiMemory, AiTarget, CompiledAi},
    combat::{self, CombatProfile, DamageKind, Status, StatusKind},
    content::{Effect, HitCheck, ItemCatalog, Recovery, Resource, TargetKind},
    ensure,
    opening_crt::{self, AfterBattle, AttackStep, OpeningCrtState, RngSpan, VoiceCondition},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RulesProfile {
    ScaffoldV1,
    Ot2OpeningV1,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleConfig {
    pub schema_version: u32,
    pub rules: RulesProfile,
    pub seed: u32,
    /// Throné's battle-start talent, applied after the first order shuffle.
    #[serde(default)]
    pub night_buff_turns: u8,
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
    pub next_queue: Vec<ActorId>,
    pub opening_crt: Option<OpeningCrtState>,
}
#[derive(Debug)]
struct ActorDefinition {
    id: String,
    name: String,
    side: Side,
    stats: Stats,
    skills: Vec<usize>,
    ai: Option<CompiledAi>,
    combat: Option<CombatProfile>,
}
#[derive(Debug)]
struct Scenario {
    catalog: Catalog,
    actors: Vec<ActorDefinition>,
    rules: RulesProfile,
    night_buff_turns: u8,
}
/// Cloning copies only mutable state and shares immutable scenario/content.
#[derive(Debug, Clone)]
pub struct Battle {
    scenario: Arc<Scenario>,
    state: BattleState,
    initial: Arc<BattleState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Skill {
        id: String,
        target: Option<ActorId>,
    },
    Item {
        id: String,
        target: Option<ActorId>,
    },
    Wait,
    Boost {
        id: String,
        target: Option<ActorId>,
        bp: u8,
    },
    Defend,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    Damage {
        actor: ActorId,
        target: ActorId,
        amount: u32,
        critical: bool,
        weak: bool,
        broken_before: bool,
    },
    ShieldChanged {
        actor: ActorId,
        before: u8,
        after: u8,
    },
    Broken {
        actor: ActorId,
        recover_round: u32,
    },
    BreakRecovered {
        actor: ActorId,
        shields: u8,
    },
    StatusApplied {
        actor: ActorId,
        status: Status,
    },
    Covered {
        protected: ActorId,
        covering: ActorId,
    },
    Steal {
        actor: ActorId,
        target: ActorId,
        success: bool,
    },
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
    Skill(usize, u8),
    Item(usize),
    Wait,
    Defend,
}
struct Prepared {
    action: Action,
    targets: Vec<ActorId>,
}

impl Battle {
    pub fn new(config: BattleConfig, items: ItemCatalog) -> Result<Self> {
        Self::new_internal(config, items, None)
    }
    /// Source callback projection for the supplied opening routine, English,
    /// switched to 2x before the first action. Other actions remain available
    /// through `new`, but are rejected here until their CRT schedules are covered.
    pub fn new_opening(config: BattleConfig, items: ItemCatalog, trace: bool) -> Result<Self> {
        ensure(
            config.rules == RulesProfile::Ot2OpeningV1
                && config
                    .players
                    .iter()
                    .map(|p| p.id.as_str())
                    .eq(["throne", "pirro", "scaracci"])
                && config
                    .enemies
                    .iter()
                    .map(|e| e.id.as_str())
                    .eq(["pursuer_a", "pursuer_b"]),
            "opening CRT projection requires the original formation",
        )?;
        Self::new_internal(config, items, Some(OpeningCrtState::new(trace)))
    }
    fn new_internal(
        config: BattleConfig,
        items: ItemCatalog,
        opening_crt: Option<OpeningCrtState>,
    ) -> Result<Self> {
        ensure(
            config.schema_version == 1,
            "unsupported battle schema_version",
        )?;
        ensure(
            config.night_buff_turns == 0 || config.rules == RulesProfile::Ot2OpeningV1,
            "night talent requires OT2 rules",
        )?;
        ensure(
            config.night_buff_turns <= 9,
            "night buff exceeds nine turns",
        )?;
        ensure(
            !config.players.is_empty() && !config.enemies.is_empty(),
            "a battle needs both players and enemies",
        )?;
        ensure(
            config.players.len() + config.enemies.len() <= 64,
            "scaffold supports at most 64 combatants",
        )?;
        if config.rules == RulesProfile::Ot2OpeningV1 {
            ensure(
                config.players.len() + config.enemies.len() <= 8,
                "OT2 opening rules support at most 8 combatants",
            )?;
        }
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
                       ai: Option<crate::ai::BossAi>,
                       combat: Option<CombatProfile>|
         -> Result<()> {
            ensure(
                !id.is_empty() && ids.insert(id.clone()),
                "empty or duplicate combatant id",
            )?;
            stats.validate()?;
            ensure(
                config.rules != RulesProfile::Ot2OpeningV1 || stats.speed <= 999,
                "OT2 speed exceeds supported 999 cap",
            )?;
            let resources = resources.unwrap_or_else(|| Resources::full(&stats));
            resources.validate(&stats)?;
            if let Some(c) = &combat {
                c.validate()?;
                if let Some(id) = &c.steal_item {
                    catalog.item_id(id)?;
                }
            }
            ensure(
                config.rules != RulesProfile::Ot2OpeningV1 || combat.is_some(),
                "OT2 rules require combat profiles for all actors",
            )?;
            let skills: Vec<_> = skills
                .iter()
                .map(|s| catalog.skill_id(s))
                .collect::<Result<_>>()?;
            ensure(
                skills.iter().collect::<BTreeSet<_>>().len() == skills.len(),
                "duplicate learned skill",
            )?;
            let ai = ai.map(|a| a.compile(&catalog, &skills)).transpose()?;
            actors.push(Combatant {
                resources,
                shields: combat.as_ref().map_or(0, |c| c.shields),
                statuses: combat
                    .as_ref()
                    .map_or_else(Vec::new, |c| c.statuses.clone()),
                main_weapon: combat.as_ref().and_then(|c| c.main_weapon),
                break_until_round: 0,
                boosted_this_round: false,
                guard: false,
                cover_by: None,
                cover_hits: 0,
                item_stolen: false,
                enemy_charge_max: 0,
                order_sample: 0,
            });
            definitions.push(ActorDefinition {
                id,
                name,
                side,
                stats,
                skills,
                ai,
                combat,
            });
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
                p.combat,
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
                e.combat,
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
        let needs_combat = catalog
            .skills
            .iter()
            .flat_map(|s| {
                s.effects
                    .iter()
                    .chain(s.boosted.iter().flat_map(|t| t.effects.iter()))
            })
            .any(|e| matches!(e, Effect::Damage { .. }));
        ensure(
            !needs_combat || definitions.iter().all(|d| d.combat.is_some()),
            "damage formula requires combat profiles on every target",
        )?;
        for definition in &definitions {
            if let Some(c) = &definition.combat {
                ensure(
                    definition.side != Side::Enemy
                        || config.rules != RulesProfile::Ot2OpeningV1
                        || c.shields > 0,
                    "OT2 enemy needs positive starting shields",
                )?;
            }
        }
        let state = BattleState {
            round: 0,
            actors,
            rng: CrtRng::new(config.seed),
            inventory,
            ai,
            queue: Vec::new(),
            cursor: 0,
            outcome: Outcome::Ongoing,
            next_queue: Vec::new(),
            opening_crt,
        };
        let mut battle = Self {
            scenario: Arc::new(Scenario {
                catalog,
                actors: definitions,
                rules: config.rules,
                night_buff_turns: config.night_buff_turns,
            }),
            initial: Arc::new(state.clone()),
            state,
        };
        battle.initialize();
        Ok(battle)
    }
    /// Start another seed without parsing content or compiling AI again.
    pub fn restarted(&self, seed: u32) -> Self {
        let mut next = Self {
            scenario: Arc::clone(&self.scenario),
            state: (*self.initial).clone(),
            initial: Arc::clone(&self.initial),
        };
        next.state.rng = CrtRng::new(seed);
        next.initialize();
        next
    }
    fn initialize(&mut self) {
        self.refresh_outcome();
        if self.scenario.rules == RulesProfile::Ot2OpeningV1 {
            let scenario = Arc::clone(&self.scenario);
            for (i, definition) in scenario.actors.iter().enumerate() {
                let before = self.state.rng;
                if definition.side == Side::Enemy {
                    // Enemy frame initialization uses the main CRT stream.
                    self.state.rng.next_u15();
                    self.state.actors[i].enemy_charge_max =
                        definition.combat.as_ref().expect("combat").enemy_bp_base
                            + self.state.rng.index(3) as u8;
                }
                // Initialize uses PLAYER_WAIT_TIME_REVISION (1..1) for both sides.
                self.state.rng.next_u15();
                self.trace_rng("actor_initialization", Some(ActorId(i)), before);
            }
        }
        if self.state.outcome == Outcome::Ongoing {
            self.start_round(&mut Vec::new());
            self.order_voice();
        }
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
    pub fn default_attack(&self, actor: ActorId) -> Result<&'static str> {
        use crate::combat::Weapon;
        let weapon = self
            .scenario
            .actors
            .get(actor.0)
            .and_then(|d| d.combat.as_ref())
            .and_then(|c| c.main_weapon);
        match weapon {
            Some(Weapon::Sword) => Ok("attack_sword"),
            Some(Weapon::Dagger) => Ok("attack_dagger"),
            Some(Weapon::Axe) => Ok("attack_axe"),
            _ => Err(SimError("no supported default normal attack".into())),
        }
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
            Command::Skill { id, target } => self.prepare(
                actor,
                Action::Skill(self.catalog().skill_id(&id)?, 0),
                target,
            )?,
            Command::Boost { id, target, bp } => self.prepare(
                actor,
                Action::Skill(self.catalog().skill_id(&id)?, bp),
                target,
            )?,
            Command::Defend => {
                ensure(
                    self.scenario.rules == RulesProfile::Ot2OpeningV1,
                    "Defend requires OT2 rules",
                )?;
                Prepared {
                    action: Action::Defend,
                    targets: Vec::new(),
                }
            }
            Command::Item { id, target } => {
                self.prepare(actor, Action::Item(self.catalog().item_id(&id)?), target)?
            }
            Command::Wait => self.prepare(actor, Action::Wait, None)?,
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
        let actor_before = self.state.actors[actor.0].clone();
        let scenario = Arc::clone(&self.scenario);
        let definition = &scenario.actors[actor.0];
        let prepared = (|| {
            let charged = self.state.actors[actor.0].enemy_charge_max > 0
                && self.state.actors[actor.0].resources.bp
                    >= self.state.actors[actor.0].enemy_charge_max;
            if charged {
                self.state.actors[actor.0].enemy_charge_max =
                    definition.combat.as_ref().expect("combat").enemy_bp_base
                        + self.state.rng.index(3) as u8;
                self.state.actors[actor.0].resources.bp = 0;
            }
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
                charged,
                &mut self.state.rng,
            )?;
            let target = self.ai_target(actor, choice.target)?;
            self.prepare(actor, Action::Skill(choice.skill, 0), target)
        })();
        match prepared {
            Ok(action) => {
                self.trace_rng("enemy_ai_and_target", Some(actor), before);
                Ok(self.execute(actor, action, before))
            }
            Err(error) => {
                self.state.rng = before;
                self.state.ai[actor.0] = memory_before;
                self.state.actors[actor.0] = actor_before;
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
        if self.state.opening_crt.is_some() {
            let supported = match action {
                Action::Skill(i, _) => {
                    let id = self.catalog().skills[i].id.as_str();
                    if self.side(actor) == Some(Side::Player) {
                        id == self.default_attack(actor)?
                    } else {
                        matches!(id, "ABI_ENE_0041" | "ABI_ENE_0500")
                    }
                }
                Action::Defend => true,
                _ => false,
            };
            ensure(
                supported,
                "action's full CRT callback schedule is not supported by the opening routine profile",
            )?;
        }
        let kind = match action {
            Action::Skill(index, boost) => {
                ensure(
                    self.scenario.actors[actor.0].skills.contains(&index),
                    "actor has not learned this skill",
                )?;
                let skill = &self.catalog().skills[index];
                ensure(
                    boost <= 3 && (boost == 0 || skill.boosted.len() == 3),
                    "unsupported boost level",
                )?;
                ensure(
                    self.state.actors[actor.0].resources.bp >= boost,
                    "not enough BP",
                )?;
                for effect in skill.tier(boost).0 {
                    if let Effect::Damage { attack } = effect {
                        let c = self.scenario.actors[actor.0]
                            .combat
                            .as_ref()
                            .ok_or_else(|| {
                                SimError("damage formula needs combat profiles".into())
                            })?;
                        if self.side(actor) == Some(Side::Player) {
                            ensure(
                                attack
                                    .weapon
                                    .is_none_or(|w| c.weapons.iter().any(|v| v.weapon == w)),
                                "weapon is not equipped",
                            )?;
                        }
                    }
                }
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
            Action::Wait | Action::Defend => {
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
            TargetKind::Ally | TargetKind::OtherAlly | TargetKind::Opponent => {
                let id = target
                    .ok_or_else(|| SimError("single-target action requires a target".into()))?;
                ensure(
                    self.state.actors.get(id.0).is_some_and(Combatant::alive),
                    "target is missing or knocked out; recovery does not revive",
                )?;
                ensure(
                    allies(id) == (kind != TargetKind::Opponent),
                    "target belongs to the wrong side",
                )?;
                ensure(
                    kind != TargetKind::OtherAlly || id != actor,
                    "action must target another ally",
                )?;
                vec![id]
            }
            TargetKind::AllAllies | TargetKind::AllOpponents | TargetKind::EveryoneExceptSelf => {
                ensure(
                    target.is_none(),
                    "all-target action does not take a single target",
                )?;
                self.state
                    .actors
                    .iter()
                    .enumerate()
                    .filter(|(i, a)| {
                        a.alive()
                            && if kind == TargetKind::EveryoneExceptSelf {
                                *i != actor.0
                            } else {
                                allies(ActorId(*i)) == (kind == TargetKind::AllAllies)
                            }
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
        if let Some(crt) = &mut self.state.opening_crt {
            crt.settled_damage = [false; 3];
        }
        self.state.actors[actor.0].guard = false;
        let mut normal_boost = 0;
        let (name, effects, hit, hits) = match action.action {
            Action::Skill(i, boost) => {
                let skill = &scenario.catalog.skills[i];
                normal_boost = boost;
                for level in 1..=boost {
                    self.voice(
                        actor,
                        8,
                        VoiceCondition {
                            border: level.into(),
                            ..Default::default()
                        },
                    );
                }
                if boost > 0 {
                    let bp = &mut self.state.actors[actor.0].resources.bp;
                    let before = *bp;
                    *bp -= boost;
                    self.state.actors[actor.0].boosted_this_round = true;
                    events.push(Event::ResourceChanged {
                        actor,
                        resource: Resource::Bp,
                        before: before.into(),
                        after: (before - boost).into(),
                    });
                }
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
                let (effects, hits) = skill.tier(boost);
                (skill.id.as_str(), effects, &skill.hit, hits)
            }
            Action::Item(i) => {
                self.state.inventory[i] -= 1;
                let item = &scenario.catalog.items[i];
                (
                    item.id.as_str(),
                    item.effects.as_slice(),
                    &HitCheck::Guaranteed,
                    1,
                )
            }
            Action::Wait => ("wait", &[][..], &HitCheck::Guaranteed, 1),
            Action::Defend => {
                let before = self.state.rng;
                self.roll_status_application();
                self.trace_rng("guard_disease", Some(actor), before);
                self.state.actors[actor.0].guard = true;
                self.voice(actor, 17, VoiceCondition::default());
                // Guard promotes the actor in the already generated next-round queue.
                if let Some(index) = self.state.next_queue.iter().position(|id| *id == actor) {
                    let id = self.state.next_queue.remove(index);
                    let insert = self
                        .state
                        .next_queue
                        .iter()
                        .take_while(|id| self.state.actors[id.0].guard)
                        .count();
                    self.state.next_queue.insert(insert, id);
                }
                ("defend", &[][..], &HitCheck::Guaranteed, 1)
            }
        };
        events.insert(
            0,
            Event::Action {
                actor,
                name: name.to_owned(),
            },
        );
        let normal = effects.first().and_then(|e| match e {
            Effect::Damage { attack } if attack.normal_attack => Some(attack),
            _ => None,
        });
        if let Some(attack) = normal {
            for &target in &action.targets {
                self.execute_normal(actor, target, attack, normal_boost, &mut events);
            }
        } else {
            for _ in 0..hits {
                for &target in &action.targets {
                    if !self.state.actors[target.0].alive() {
                        continue;
                    }
                    let success = match hit {
                        HitCheck::Guaranteed => true,
                        HitCheck::Weighted { probability } => {
                            self.state.rng.weighted_bool(*probability)
                        }
                    };
                    if !success {
                        events.push(Event::Miss { actor, target });
                        continue;
                    }
                    for effect in effects {
                        if !self.state.actors[target.0].alive() {
                            break;
                        }
                        self.apply_action_effect(actor, target, effect, &mut events);
                    }
                }
            }
        }
        self.refresh_outcome();
        if self.state.outcome != Outcome::Ongoing {
            events.push(Event::BattleEnded {
                outcome: self.state.outcome,
            });
            if self.state.outcome == Outcome::Victory && self.state.opening_crt.is_some() {
                self.finish_opening(actor);
            }
        } else {
            self.state.cursor += 1;
            while self.state.cursor < self.state.queue.len()
                && (!self.state.actors[self.state.queue[self.state.cursor].0].alive()
                    || self.state.actors[self.state.queue[self.state.cursor].0].broken())
            {
                self.state.cursor += 1;
            }
            if self.state.cursor == self.state.queue.len() {
                self.start_round(&mut events);
                events.push(Event::RoundStarted {
                    round: self.state.round,
                });
            }
            self.order_voice();
        }
        StepReport {
            actor,
            events,
            rng_before,
            rng_after: self.state.rng,
            outcome: self.state.outcome,
        }
    }
    fn normal_accuracy(
        &mut self,
        actor: ActorId,
        target: ActorId,
        attack: &combat::Attack,
    ) -> bool {
        let before = self.state.rng;
        let hit = if self.state.actors[target.0].broken() || !self.state.actors[target.0].alive() {
            true
        } else {
            let from = self.scenario.actors[actor.0]
                .combat
                .as_ref()
                .expect("combat");
            let to = self.scenario.actors[target.0]
                .combat
                .as_ref()
                .expect("combat");
            self.state.rng.weighted_bool(combat::hit_probability(
                from.accuracy,
                to.evasion,
                attack.hit_bonus,
            ))
        };
        self.trace_rng("normal_accuracy", Some(actor), before);
        hit
    }
    fn execute_normal(
        &mut self,
        actor: ActorId,
        target: ActorId,
        attack: &combat::Attack,
        boost: u8,
        events: &mut Vec<Event>,
    ) {
        // StartMainAction caches accuracy. Only authored ExecStepCounter keys
        // replace it. In particular sword/dagger Lv3 reuse it for all four hits.
        let mut hit = self.normal_accuracy(actor, target, attack);
        for step in opening_crt::attack_plan(attack.weapon.expect("normal weapon"), boost) {
            match *step {
                AttackStep::Check => hit = self.normal_accuracy(actor, target, attack),
                AttackStep::Voice { r#type } => {
                    self.voice(actor, r#type, VoiceCondition::default())
                }
                AttackStep::Damage { .. } => {
                    // DefaultTarget stays in the source command until its final
                    // key. Overkill hits still calculate damage and consume CRT.
                    self.apply_attack(actor, target, attack, Some(hit), events);
                    // Boosted normal attacks explicitly disable hit-reaction voices.
                    // Unboosted English 2x attacks hit while their attack voice is
                    // active; the source early gate suppresses types 9/10/12.
                    // Final-key placement is checked when source data is loaded.
                }
            }
        }
    }
    fn apply_effect(&mut self, target: ActorId, effect: &Effect, events: &mut Vec<Event>) {
        let stats = &self.scenario.actors[target.0].stats;
        let resources = &mut self.state.actors[target.0].resources;
        let (resource, before, after) = match effect {
            Effect::Damage { .. }
            | Effect::Status { .. }
            | Effect::Steal { .. }
            | Effect::Cover { .. } => {
                unreachable!("combat effect is dispatched before scalar recovery")
            }
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
    fn apply_action_effect(
        &mut self,
        actor: ActorId,
        target: ActorId,
        effect: &Effect,
        events: &mut Vec<Event>,
    ) {
        match effect {
            Effect::Damage { attack } => self.apply_attack(actor, target, attack, None, events),
            Effect::Status { status } => {
                self.roll_status_application();
                let statuses = &mut self.state.actors[target.0].statuses;
                if let Some(old) = statuses.iter_mut().find(|s| s.kind == status.kind) {
                    old.turns = old.turns.saturating_add(status.turns).min(9);
                } else {
                    statuses.push(*status);
                }
                events.push(Event::StatusApplied {
                    actor: target,
                    status: *status,
                });
            }
            Effect::Cover { hits } => {
                // Cover Me! makes the selected ally protect the caster, for N hits.
                self.state.actors[actor.0].cover_by = Some(target);
                self.state.actors[actor.0].cover_hits = *hits;
            }
            Effect::Steal { bonus } => {
                let item = self.scenario.actors[target.0]
                    .combat
                    .as_ref()
                    .and_then(|c| c.steal_item.as_ref());
                let success = item.is_some() && !self.state.actors[target.0].item_stolen && {
                    let hp = self.state.actors[target.0].resources.hp as f32;
                    let max = self.scenario.actors[target.0].stats.max_hp as f32;
                    self.state.rng.weighted_bool(
                        (1.0_f32 - hp / max + 0.01_f32) + f32::from(*bonus) * 0.01_f32,
                    )
                };
                if success {
                    let id = self
                        .catalog()
                        .item_id(item.expect("checked steal item"))
                        .expect("validated item");
                    self.state.inventory[id] = self.state.inventory[id].saturating_add(1);
                    self.state.actors[target.0].item_stolen = true;
                }
                events.push(Event::Steal {
                    actor,
                    target,
                    success,
                });
            }
            _ => self.apply_effect(target, effect, events),
        }
    }
    fn roll_status_application(&mut self) {
        // CheckAddDisease, then Battle_ExecAddDisease's non-random branch.
        // ADD_GUARD / ADD_ATK_UP / ADD_DEF_DOWN contain 20 entries, including
        // 19 None slots. Every slot draws RandomIntegerInRange(0, 0).
        self.state.rng.weighted_bool(1.0);
        for _ in 0..20 {
            self.state.rng.index(1);
        }
    }
    fn apply_attack(
        &mut self,
        actor: ActorId,
        mut target: ActorId,
        attack: &combat::Attack,
        prechecked_hit: Option<bool>,
        events: &mut Vec<Event>,
    ) {
        if let Some(cover) = self.state.actors[target.0].cover_by {
            if cover != target
                && self.state.actors[target.0].cover_hits > 0
                && self.state.actors[cover.0].alive()
                && !self.state.actors[cover.0].broken()
            {
                self.state.actors[target.0].cover_hits -= 1;
                events.push(Event::Covered {
                    protected: target,
                    covering: cover,
                });
                target = cover;
            }
        }
        if let Some(weapon) = attack.weapon {
            self.state.actors[actor.0].main_weapon = Some(weapon);
        }
        let scenario = Arc::clone(&self.scenario);
        let attacker = scenario.actors[actor.0]
            .combat
            .as_ref()
            .expect("validated combat profile");
        let defender = scenario.actors[target.0]
            .combat
            .as_ref()
            .expect("validated combat profile");
        let broken = self.state.actors[target.0].broken();
        let magical = attack.kind == DamageKind::Magical;
        let accuracy =
            combat::hit_probability(attacker.accuracy, defender.evasion, attack.hit_bonus);
        let before = self.state.rng;
        let crit_chance = combat::critical_probability(
            attacker.critical,
            scenario.actors[target.0].stats.speed,
            self.state.actors[target.0].has_status(StatusKind::SpeedUp),
            attack.critical_bonus,
        );
        let critical = !magical && self.state.rng.weighted_bool(crit_chance);
        let hit = if let Some(hit) = prechecked_hit {
            hit
        } else {
            magical || broken || critical || self.state.rng.weighted_bool(accuracy)
        };
        if !hit {
            self.trace_rng("critical_and_hit", Some(actor), before);
            events.push(Event::Miss { actor, target });
            self.damage_notification(target, None);
            return;
        }
        let weapon_weak = attack
            .weapon
            .is_some_and(|w| defender.weapon_weaknesses.contains(&w));
        let element_weak = attack
            .element
            .is_some_and(|e| defender.element_weaknesses.contains(&e));
        let equipped = self.state.actors[actor.0]
            .main_weapon
            .and_then(|w| attacker.weapons.iter().find(|s| s.weapon == w));
        let atk = if magical {
            attacker.magic + equipped.map_or(0, |w| w.magic)
        } else {
            attacker.attack + equipped.map_or(0, |w| w.attack)
        };
        let amount = combat::damage(
            combat::DamageInput {
                attack: atk.min(999),
                defense: if magical {
                    defender.magic_defense
                } else {
                    defender.defense
                },
                growth: if magical {
                    attacker.growth_magic
                } else {
                    attacker.growth_attack
                },
                guard: self.state.actors[target.0].guard,
                weapon_weak,
                element_weak,
                broken,
                critical,
                attack_up: self.state.actors[actor.0].has_status(if magical {
                    StatusKind::MagicUp
                } else {
                    StatusKind::AttackUp
                }),
                defense_down: !magical
                    && self.state.actors[target.0].has_status(StatusKind::DefenseDown),
            },
            attack,
            combat::variance(&mut self.state.rng),
        );
        self.trace_rng("critical_hit_and_damage", Some(actor), before);
        events.push(Event::Damage {
            actor,
            target,
            amount,
            critical,
            weak: weapon_weak || element_weak,
            broken_before: broken,
        });
        if self.side(target) == Some(Side::Enemy) && !broken && (weapon_weak || element_weak) {
            let state = &mut self.state.actors[target.0];
            let before = state.shields;
            state.shields = state.shields.saturating_sub(1);
            events.push(Event::ShieldChanged {
                actor: target,
                before,
                after: state.shields,
            });
            if before > 0 && state.shields == 0 {
                state.break_until_round = self.state.round.saturating_add(2);
                events.push(Event::Broken {
                    actor: target,
                    recover_round: state.break_until_round,
                });
            }
        }
        self.apply_effect(target, &Effect::FixedDamage { amount }, events);
        self.damage_notification(target, Some(amount));
    }
    fn trace_rng(&mut self, reason: &'static str, actor: Option<ActorId>, before: CrtRng) {
        if let Some(trace) = self
            .state
            .opening_crt
            .as_mut()
            .and_then(|c| c.trace.as_mut())
        {
            trace.push(RngSpan {
                reason,
                actor,
                before,
                after: self.state.rng,
            });
        }
    }
    fn voice(&mut self, actor: ActorId, voice_type: u8, condition: VoiceCondition) {
        let before = self.state.rng;
        let Some(crt) = self.state.opening_crt.as_mut() else {
            return;
        };
        if actor.0 >= 3 {
            return;
        }
        if matches!(voice_type, 4 | 11) && crt.settled_damage[actor.0] {
            return;
        }
        let chosen = opening_crt::select_voice(
            actor.0,
            voice_type,
            condition,
            &mut crt.used_voices[actor.0],
            &mut self.state.rng,
        );
        if chosen.is_some() && matches!(voice_type, 4 | 11) {
            crt.settled_damage[actor.0] = true;
        }
        self.trace_rng(
            match voice_type {
                1 => "battle_start_voice",
                3 => "order_voice",
                4 => "damage_voice",
                5 => "death_voice",
                6 => "attack_voice",
                7 => "last_attack_voice",
                8 => "boost_voice",
                11 => "avoid_voice",
                17 => "guard_voice",
                _ => "voice",
            },
            Some(actor),
            before,
        );
    }
    fn order_voice(&mut self) {
        if let Some(actor) = self.current_actor() {
            let state = &self.state.actors[actor.0];
            self.voice(
                actor,
                3,
                VoiceCondition {
                    pinch: u64::from(state.resources.hp) * 4
                        <= u64::from(self.scenario.actors[actor.0].stats.max_hp),
                    decondition: false,
                    border: 0,
                },
            );
        }
    }
    fn damage_notification(&mut self, target: ActorId, amount: Option<u32>) {
        if self.state.opening_crt.is_none() {
            return;
        }
        let before = self.state.rng;
        // PlayDamageUI/PlayMissUI -> DisplayTimer -> K2_SetTimer. Even its
        // zero initial variance draws once from the main CRT stream.
        self.state.rng.next_u15();
        self.trace_rng("damage_notification_timer", Some(target), before);
        if let Some(damage) = amount {
            if self.state.actors[target.0].alive() {
                let max = self.scenario.actors[target.0].stats.max_hp;
                self.voice(
                    target,
                    4,
                    VoiceCondition {
                        border: (damage as f32 / max as f32 * 100.0) as u32,
                        ..Default::default()
                    },
                );
            } else {
                self.voice(target, 5, VoiceCondition::default());
            }
        } else {
            self.voice(target, 11, VoiceCondition::default());
        }
    }
    fn finish_opening(&mut self, actor: ActorId) {
        let data = opening_crt::data();
        if data.victory_voice {
            self.voice(actor, 2, VoiceCondition::default());
        }
        let rng_before_rewards = self.state.rng;
        let mut healing_grapes = 0;
        for i in 3..5 {
            let before = self.state.rng;
            healing_grapes += u32::from(
                self.state
                    .rng
                    .weighted_bool(data.drop_rate as f32 * 0.01_f32),
            );
            self.trace_rng("enemy_drop", Some(ActorId(i)), before);
        }
        let id = self
            .catalog()
            .item_id("healing_grape")
            .expect("opening item");
        self.state.inventory[id] = self.state.inventory[id].saturating_add(healing_grapes);
        let rng_after_rewards = self.state.rng;
        // On Shown Field After Battle Impl resets Dng_Cty_3_1's counter.
        // The scripted encounter does not perform random-encounter selection.
        // Scripted encounters do not clear FirstEncount (ordinary lotteries do).
        let encounter_steps = data.encounter.reset(&mut self.state.rng, true);
        self.trace_rng("field_encounter_reset_b_then_a", None, rng_after_rewards);
        self.state
            .opening_crt
            .as_mut()
            .expect("opening CRT")
            .after_battle = Some(AfterBattle {
            rng_before_rewards,
            rng_after_rewards,
            rng_after_field_reset: self.state.rng,
            healing_grapes,
            encounter_steps,
        });
    }
    fn source_order(&mut self, round: u32) -> Vec<ActorId> {
        let mut rows: Vec<_> = self
            .state
            .actors
            .iter()
            .enumerate()
            .filter(|(_, s)| s.alive())
            .map(|(i, _)| ActorId(i))
            .collect();
        combat::sort_order(&mut rows, |id| {
            let s = &self.state.actors[id.0];
            let speed = (self.scenario.actors[id.0].stats.speed as f32
                * if s.has_status(StatusKind::SpeedUp) {
                    1.5_f32
                } else {
                    1.0
                }) as u32;
            if s.broken() {
                if s.break_until_round <= round {
                    speed + 1_000_000
                } else if round > self.state.round {
                    speed
                } else {
                    1
                }
            } else if self.state.round > 1
                && round > self.state.round
                && self.side(id) == Some(Side::Enemy)
            {
                let unit = f32::from(s.order_sample) * 3.051_851e-5_f32;
                (speed as f32 * (0.7_f32 + (1.5_f32 - 0.7_f32) * unit)) as u32
            } else {
                speed
            }
        });
        let states = &self.state.actors;
        let before = self.state.rng;
        combat::shuffle_order(&mut rows, |id| !states[id.0].broken(), &mut self.state.rng);
        self.trace_rng(
            if round == self.state.round {
                "current_order_shuffle"
            } else {
                "next_order_shuffle"
            },
            None,
            before,
        );
        rows
    }
    fn start_ot2_round(&mut self, events: &mut Vec<Event>) {
        let first = self.state.round == 0;
        self.state.round = self.state.round.saturating_add(1);
        if !first {
            for (i, state) in self.state.actors.iter_mut().enumerate() {
                for status in &mut state.statuses {
                    status.turns = status.turns.saturating_sub(1);
                }
                state.statuses.retain(|s| s.turns > 0);
                if !state.alive() {
                    continue;
                }
                if state.broken() && state.break_until_round <= self.state.round {
                    state.break_until_round = 0;
                    state.shields = self.scenario.actors[i]
                        .combat
                        .as_ref()
                        .expect("combat profile")
                        .shields;
                    events.push(Event::BreakRecovered {
                        actor: ActorId(i),
                        shields: state.shields,
                    });
                }
                let cap = if self.scenario.actors[i].side == Side::Enemy {
                    state.enemy_charge_max
                } else {
                    5
                };
                if !state.boosted_this_round && !state.broken() {
                    state.resources.bp = state.resources.bp.saturating_add(1).min(cap);
                }
                state.boosted_this_round = false;
            }
        }
        // The initial two queues use Initialize's 1..1 revision. Subsequent
        // BeginBattleTurn calls draw one revision per living actor; players'
        // 1..1 range still draws, enemies use 0.7..1.5 for the next preview.
        if !first {
            let before = self.state.rng;
            for actor in &mut self.state.actors {
                if actor.alive() {
                    actor.order_sample = self.state.rng.next_u15();
                }
            }
            self.trace_rng("round_timing_revision", None, before);
        }
        self.state.queue = if first {
            self.source_order(self.state.round)
        } else {
            std::mem::take(&mut self.state.next_queue)
        };
        self.state
            .queue
            .retain(|id| self.state.actors[id.0].alive() && !self.state.actors[id.0].broken());
        if first && self.state.opening_crt.is_some() {
            let before = self.state.rng;
            let speaker = ActorId(self.state.rng.index(3) as usize);
            self.trace_rng("battle_start_speaker", Some(speaker), before);
            self.voice(speaker, 1, VoiceCondition::default());
        }
        if first && self.scenario.night_buff_turns > 0 {
            // InitializeActionOrder precedes ExecSpecialAction_BuffInNight.
            // Direct AddDisease does not use the ailment probability loop.
            for (i, actor) in self.state.actors.iter_mut().enumerate() {
                if actor.alive() && self.scenario.actors[i].side == Side::Player {
                    for kind in [
                        StatusKind::AttackUp,
                        StatusKind::MagicUp,
                        StatusKind::SpeedUp,
                    ] {
                        actor.statuses.retain(|s| s.kind != kind);
                        actor.statuses.push(Status {
                            kind,
                            turns: self.scenario.night_buff_turns,
                        });
                    }
                }
            }
            // AGI_UP marks the existing next-order rows. ExecMoveNextOrder(0,-1)
            // moves each marked row forward one place before ChangeBattleTurn
            // promotes that list to the first current queue. It does not re-sort.
            let mut top = 0;
            for i in 0..self.state.queue.len() {
                if self.side(self.state.queue[i]) == Some(Side::Player) {
                    let to = i.saturating_sub(1).max(top);
                    self.state.queue.swap(i, to);
                    if to == top {
                        top += 1;
                    }
                }
            }
        }
        self.state.next_queue = self.source_order(self.state.round.saturating_add(1));
        self.state.cursor = 0;
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
    fn start_round(&mut self, events: &mut Vec<Event>) {
        if self.scenario.rules == RulesProfile::Ot2OpeningV1 {
            self.start_ot2_round(events);
            return;
        }
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
