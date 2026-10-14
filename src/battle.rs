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
mod grace;
mod order;

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
    #[serde(default)]
    pub enemy_knowledge: crate::knowledge::EnemyHistory,
}
impl BattleConfig {
    pub fn throne_pirro(seed: u32) -> Result<Self> {
        let mut config = Self::from_json(include_str!("../data/throne_pirro.json"))?;
        config.seed = seed;
        Ok(config)
    }
    /// Bundled opening encounter for external callers; no runtime file access.
    pub fn throne_opening(seed: u32) -> Result<Self> {
        let mut config = Self::from_json(include_str!("../data/throne_opening.json"))?;
        config.seed = seed;
        Ok(config)
    }
    /// Second Chapter 1 sewer fight: two grunts and their leader.
    pub fn throne_second(seed: u32) -> Result<Self> {
        Self::throne_second_at_level(seed, 2)
    }
    pub fn throne_second_at_level(seed: u32, level: u8) -> Result<Self> {
        let mut config = Self::from_json(match level {
            1 => include_str!("../data/throne_second_level1.json"),
            2 => include_str!("../data/throne_second.json"),
            _ => {
                return Err(SimError(
                    "second-fight loadouts support levels 1 and 2".into(),
                ));
            }
        })?;
        config.seed = seed;
        Ok(config)
    }
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
    Escaped,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BattleState {
    #[serde(skip_serializing_if="Option::is_none")]
    pub grace: Option<crate::bg_effects::BattleModifiers>,
    pub enemy_knowledge: crate::knowledge::EnemyHistory,
    pub pirro: Option<crate::pirro::PirroState>,
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
impl BattleState {
    /// Scheduled order for this round, including actors who have already acted.
    /// Actors broken or defeated during the round are skipped on execution.
    pub fn turn_order(&self) -> &[ActorId] {
        &self.queue
    }
}
#[derive(Debug, Clone)]
struct ActorDefinition {
    id: String,
    name: String,
    side: Side,
    stats: Stats,
    skills: Vec<usize>,
    ai: Option<CompiledAi>,
    combat: Option<CombatProfile>,
}
#[derive(Debug, Clone)]
struct Scenario {
    catalog: Catalog,
    actors: Vec<ActorDefinition>,
    rules: RulesProfile,
    night_buff_turns: u8,
}
/// Cloning copies only mutable state and shares immutable scenario/content.
#[derive(Debug, Clone)]
pub struct Battle {
    search_lottery_trace: bool,
    scenario: Arc<Scenario>,
    state: BattleState,
    initial: Arc<BattleState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Latent {
        command: Box<Command>,
    },
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
    LatentActivated {
        actor: ActorId,
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
    /// Scripted normal start, before player/enemy initialization. This profile
    /// is exposed only through the bounded two-command recruitment adapter.
    pub(crate) fn new_hikari_recruitment(config: BattleConfig) -> Result<Self> {
        ensure(config.players.iter().map(|p|p.id.as_str()).eq(["throne","osvald","agnea","partitio"])
            && config.enemies.iter().map(|e|e.id.as_str()).eq(["ENE_EVE_WAR_DST_090","ENE_EVE_WAR_DST_080","ENE_EVE_WAR_DST_080"]),
            "Hikari recruitment requires the carried four-player party and imported soldiers")?;
        let mut crt = OpeningCrtState::new(true);
        crt.hikari_recruitment = true;
        crt.used_voices.resize(4, 0);
        crt.settled_damage.resize(4, false);
        Self::new_internal(config, ItemCatalog::builtin()?, Some(crt))
    }
    fn is_hikari_recruitment(&self) -> bool {
        self.state.opening_crt.as_ref().is_some_and(|c|c.hikari_recruitment)
    }
    pub(crate) fn new_cape_cold_mug(config: BattleConfig, trace: bool) -> Result<Self> {
        ensure(
            config.players.len() == 1
                && config.players[0].id == "osvald"
                && config.enemies.len() == 1
                && config.enemies[0].id == "cape_cold_man",
            "unsupported Mug formation",
        )?;
        let mut crt = OpeningCrtState::new(trace);
        crt.cape_cold_mug = true;
        Self::new_internal(config, ItemCatalog::builtin()?, Some(crt))
    }
    fn is_mug(&self) -> bool {
        self.state
            .opening_crt
            .as_ref()
            .is_some_and(|c| c.cape_cold_mug)
    }
    pub fn new_pirro(config: BattleConfig, items: ItemCatalog, trace: bool) -> Result<Self> {
        ensure(
            config.rules == RulesProfile::Ot2OpeningV1
                && config.players.iter().map(|p| p.id.as_str()).eq(["throne"])
                && config
                    .enemies
                    .iter()
                    .map(|e| e.id.as_str())
                    .eq(["pirro_boss"]),
            "Pirro requires solo Throné and the imported boss",
        )?;
        let mut crt = OpeningCrtState::new(trace);
        crt.pirro_battle = true;
        Self::new_internal(config, items, Some(crt))
    }
    fn is_pirro(&self) -> bool {
        self.state
            .opening_crt
            .as_ref()
            .is_some_and(|c| c.pirro_battle)
    }
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
    /// English/2x source callback projection for the second Chapter 1 fight.
    /// Ends after combat and drops; later field/story events belong to callers.
    pub fn new_throne_second(
        config: BattleConfig,
        items: ItemCatalog,
        trace: bool,
    ) -> Result<Self> {
        ensure(
            config.rules == RulesProfile::Ot2OpeningV1
                && config
                    .players
                    .iter()
                    .map(|p| p.id.as_str())
                    .eq(["throne", "pirro", "scaracci"])
                && config.enemies.iter().map(|e| e.id.as_str()).eq([
                    "grunt_right",
                    "leader",
                    "grunt_left",
                ]),
            "second-fight CRT projection requires the sewer formation",
        )?;
        let mut crt = OpeningCrtState::new(trace);
        crt.second_battle = true;
        Self::new_internal(config, items, Some(crt))
    }
    fn is_second_battle(&self) -> bool {
        self.state
            .opening_crt
            .as_ref()
            .is_some_and(|c| c.second_battle)
    }
    pub(crate) fn new_mansion_escape(
        config: BattleConfig,
        situation: crate::escape::Situation,
        trace: bool,
    ) -> Result<Self> {
        let mut crt = OpeningCrtState::new(trace);
        crt.mansion_situation = Some(situation);
        Self::new_internal(config, ItemCatalog::builtin()?, Some(crt))
    }
    fn crt_data(&self) -> &'static opening_crt::OpeningData {
        let crt = self.state.opening_crt.as_ref();
        if crt.is_some_and(|c|c.lord_sands) {
            opening_crt::lord_sands_data()
        } else if crt.is_some_and(|c|c.temenos_recruitment) {
            opening_crt::temenos_data()
        } else if crt.is_some_and(|c|c.step_ahead) {
            opening_crt::gravell_data()
        } else if crt.is_some_and(|c|c.carried_voices.is_some()) {
            opening_crt::carried_field_item_data()
        } else if self.is_hikari_recruitment() {
            opening_crt::hikari_recruitment_data()
        } else if crt.is_some_and(|c|c.field_item) {
            opening_crt::field_rare_data()
        } else if self.is_mug() {
            opening_crt::mug_data()
        } else if crt.is_some_and(|c| c.field_break) {
            if self.scenario.actors.iter().filter(|a|a.side==Side::Player).count()==2 {
                opening_crt::field_party_data()
            } else { opening_crt::field_break_data() }
        } else if crt.is_some_and(|c| c.mansion_situation.is_some()) {
            opening_crt::mansion_data()
        } else if self.is_pirro() {
            opening_crt::pirro_data()
        } else if self.is_second_battle() {
            opening_crt::second_data()
        } else {
            opening_crt::data()
        }
    }
    pub(crate) fn new_field_break(
        config: BattleConfig,
        situation: crate::escape::Situation,
    ) -> Result<Self> {
        let mut crt = OpeningCrtState::new(true);
        crt.mansion_situation = Some(situation);
        crt.field_break = true;
        Self::new_internal(config, ItemCatalog::builtin()?, Some(crt))
    }
    pub(crate) fn escape_check(&mut self, probability: Option<f32>) -> bool {
        let before = self.state.rng;
        let success = probability.is_none_or(|p| self.state.rng.weighted_bool(p));
        self.trace_rng("escape_check", self.current_actor(), before);
        if success {
            self.state.outcome = Outcome::Escaped;
        }
        success
    }

    pub(crate) fn new_field_item(config:BattleConfig,situation:crate::escape::Situation)->Result<Self> {
        let mut crt=OpeningCrtState::new(true);
        crt.mansion_situation=Some(situation); crt.field_break=true; crt.field_item=true;
        Self::new_internal(config,ItemCatalog::builtin()?,Some(crt))
    }
    pub(crate) fn new_carried_field_item(config:BattleConfig,situation:crate::escape::Situation,voices:Vec<usize>)->Result<Self> {
        let mut crt=OpeningCrtState::new(true);
        crt.mansion_situation=Some(situation);crt.field_break=true;crt.field_item=true;
        crt.used_voices=vec![0;voices.iter().copied().max().unwrap_or(0)+1];crt.carried_voices=Some(voices);crt.settled_damage=vec![false;config.players.len()];
        Self::new_internal(config,ItemCatalog::builtin()?,Some(crt))
    }
    pub(crate) fn new_carried_step_ahead(config:BattleConfig,situation:crate::escape::Situation,voices:Vec<usize>)->Result<Self> {
        Self::new_carried_step_ahead_members(config,situation,voices,None)
    }
    pub(crate) fn new_carried_step_ahead_members(config:BattleConfig,situation:crate::escape::Situation,voices:Vec<usize>,members:Option<Vec<usize>>)->Result<Self> {
        let mut crt=OpeningCrtState::new(true);
        crt.mansion_situation=Some(situation);crt.field_break=true;crt.field_item=true;crt.step_ahead=true;crt.step_ahead_members=members;
        crt.used_voices=vec![0;voices.iter().copied().max().unwrap_or(0)+1];crt.carried_voices=Some(voices);crt.settled_damage=vec![false;config.players.len()];
        Self::new_internal(config,ItemCatalog::builtin()?,Some(crt))
    }
    pub(crate) fn new_lord_sands(config:BattleConfig)->Result<Self> {
        Self::new_route_boss(config,Some(vec![0,1,3]))
    }
    pub(crate) fn new_route_boss(config:BattleConfig,members:Option<Vec<usize>>)->Result<Self> {
        let mut crt=OpeningCrtState::new(true);
        crt.mansion_situation=Some(crate::escape::Situation::Normal);crt.field_break=true;crt.field_item=true;crt.step_ahead=true;crt.lord_sands=true;
        crt.step_ahead_members=members;crt.carried_voices=Some(vec![0,1,5,3]);
        crt.used_voices=vec![0;6];crt.settled_damage=vec![false;4];
        Self::new_internal(config,ItemCatalog::builtin()?,Some(crt))
    }
    pub(crate) fn new_temenos_recruitment(config:BattleConfig,situation:crate::escape::Situation,voices:Vec<usize>)->Result<Self> {
        let mut crt=OpeningCrtState::new(true);
        crt.mansion_situation=Some(situation);crt.field_break=true;crt.field_item=true;crt.step_ahead=true;crt.temenos_recruitment=true;
        crt.used_voices=vec![0;voices.iter().copied().max().unwrap_or(0)+1];crt.carried_voices=Some(voices);crt.settled_damage=vec![false;config.players.len()];
        Self::new_internal(config,ItemCatalog::builtin()?,Some(crt))
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
                !id.is_empty() && (ids.insert(id.clone())
                    || (side == Side::Enemy && opening_crt.as_ref().is_some_and(|c|c.hikari_recruitment||c.field_break||c.carried_voices.is_some()))),
                "empty or duplicate combatant id",
            )?;
            stats.validate()?;
            ensure(
                config.rules != RulesProfile::Ot2OpeningV1 || stats.speed <= 999,
                "OT2 speed exceeds supported 999 cap",
            )?;
            let resources = resources.unwrap_or_else(|| Resources::full(&stats));
            if side==Side::Player && opening_crt.as_ref().is_some_and(|c|c.carried_voices.is_some()) {
                let index=opening_crt.as_ref().unwrap().carried_voices.as_ref().unwrap()[actors.len()];
                resources.validate_with_latent_max(&stats,opening_crt::carried_field_item_data().player_latent_maxima[index])?;
            } else if side==Side::Player && opening_crt.as_ref().is_some_and(|c|c.hikari_recruitment) {
                resources.validate_with_latent_max(&stats, opening_crt::hikari_recruitment_data().player_latent_maxima[actors.len()])?;
            } else { resources.validate(&stats)?; }
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
            ensure(
                config.rules != RulesProfile::Ot2OpeningV1 || side != Side::Enemy
                    || combat.as_ref().is_some_and(|c| c.enemy_initialization.is_some()),
                "OT2 enemies require imported initialization metadata",
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
                revealed_weapon_weaknesses: 0,
                revealed_element_weaknesses: 0,
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
        for (i,d) in definitions.iter().enumerate() {
            if d.side==Side::Enemy {
                if let Some(k)=config.enemy_knowledge.get(&d.id) {
                    actors[i].revealed_weapon_weaknesses=k.weapons;
                    actors[i].revealed_element_weaknesses=k.elements;
                }
            }
        }
        let state = BattleState {
            grace: None,
            enemy_knowledge: config.enemy_knowledge,
            pirro: None,
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
            search_lottery_trace: false,
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
            search_lottery_trace: false,
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
                if definition.side == Side::Player
                    && self
                        .state
                        .opening_crt
                        .as_ref()
                        .is_some_and(|c| c.mansion_situation.is_some())
                {
                    continue; // Field selector has already initialized party timing.
                }
                let before = self.state.rng;
                if definition.side == Side::Enemy {
                    let profile = definition.combat.as_ref().expect("combat");
                    self.state.actors[i].enemy_charge_max = opening_crt::initialize_enemy_rng(
                        &mut self.state.rng,
                        profile.enemy_bp_base,
                        profile.enemy_initialization.expect("validated enemy initialization"),
                    )
                    .0;
                } else {
                    self.state.rng.next_u15();
                }
                self.trace_rng("actor_initialization", Some(ActorId(i)), before);
            }
        }
        if self.state.outcome == Outcome::Ongoing {
            if self.is_pirro() {
                let before = self.state.rng;
                self.state.pirro = Some(crate::pirro::PirroState::new(&mut self.state.rng));
                self.trace_rng("pirro_ai_initialization", Some(ActorId(1)), before);
            }
            self.start_round(&mut Vec::new());
            self.order_voice(None);
        }
    }
    pub fn state(&self) -> &BattleState {
        &self.state
    }
    pub fn enemy_knowledge(&self) -> crate::knowledge::EnemyHistory {
        let mut result=self.state.enemy_knowledge.clone();
        for (i,d) in self.scenario.actors.iter().enumerate() {
            if d.side==Side::Enemy {
                let k=result.entry(d.id.clone()).or_default();
                k.weapons |= self.state.actors[i].revealed_weapon_weaknesses;
                k.elements |= self.state.actors[i].revealed_element_weaknesses;
            }
        }
        result
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
            Some(Weapon::Lance) => Ok("attack_lance"),
            Some(Weapon::Dagger) => Ok("attack_dagger"),
            Some(Weapon::Axe) => Ok("attack_axe"),
            Some(Weapon::Staff) => Ok("attack_staff"),
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
    /// Whether latent can be activated at the current menu in the supported
    /// solo Throné profile. Other profiles do not implement latent actions.
    pub fn can_use_latent(&self) -> bool {
        self.current_actor() == Some(ActorId(0))
            && self.state.pirro.as_ref().is_some_and(|p| {
                p.latent_unlocked
                    && p.double_action_remaining == 0
                    && self.state.actors[0].resources.latent
                        == self.crt_data().latent.as_ref().unwrap().maximum
            })
    }
    pub fn item_count(&self, id: &str) -> Result<u32> {
        Ok(self.state.inventory[self.catalog().item_id(id)?])
    }
    /// At an opening player command menu, select 1 BP then reduce it to 0.
    /// OnBoostChange calls the boost voice only on an increase. Held BP is
    /// released by the decrease; no action, turn, or BP regeneration is changed.
    pub fn boost_cancel_cycle(&mut self) -> Result<()> {
        self.boost_sequence(&[1, 0])
    }
    /// Search probes need lottery checkpoints, not a copy of the whole route.
    /// The accepted routine is replayed with the original full trace settings.
    pub(crate) fn lottery_search_trace(&mut self) {
        self.search_lottery_trace=true;
        if let Some(crt)=self.state.opening_crt.as_mut() {crt.trace=Some(Vec::new());}
    }
    pub(crate) fn omit_search_trace(&mut self) {
        if let Some(crt)=self.state.opening_crt.as_mut() {crt.trace=None;}
    }
    /// Selected BP after each up/down input, starting and ending at zero.
    /// Validate the entire sequence before mutating CRT or voice-candidate state.
    pub fn boost_sequence(&mut self, levels: &[u8]) -> Result<()> {
        self.validate_boost_inputs(levels, 0)?;
        self.apply_boost_inputs(levels);
        Ok(())
    }
    /// Preview inputs starting at 0 selected BP without committing an action.
    /// The path may end at any available BP; use it to inspect conditional
    /// voice draws when searching a complete command-menu input path.
    pub fn preview_boost_inputs(&self, levels: &[u8]) -> Result<BattleState> {
        self.validate_boost_inputs(levels, levels.last().copied().unwrap_or(0))?;
        let mut preview = self.clone();
        preview.apply_boost_inputs(levels);
        Ok(preview.state)
    }
    fn validate_boost_inputs(&self, levels: &[u8], final_bp: u8) -> Result<()> {
        ensure(
            self.scenario.rules == RulesProfile::Ot2OpeningV1 && self.state.opening_crt.is_some(),
            "boost cycling requires the opening CRT profile",
        )?;
        let actor = self
            .current_actor()
            .ok_or_else(|| SimError("battle is over".into()))?;
        ensure(
            self.side(actor) == Some(Side::Player),
            "it is an enemy turn",
        )?;
        ensure(
            self.state.actors[actor.0].resources.bp > 0,
            "boost cycling requires at least 1 BP",
        )?;
        let maximum = self.state.actors[actor.0].resources.bp.min(3);
        let mut previous = 0u8;
        for &level in levels {
            ensure(
                level <= maximum && previous.abs_diff(level) == 1,
                "boost inputs must change one level within available BP",
            )?;
            previous = level;
        }
        ensure(
            previous == final_bp,
            "boost inputs must end at the action's BP level",
        )
    }
    fn apply_boost_inputs(&mut self, levels: &[u8]) {
        let mut previous = 0;
        for &level in levels {
            self.apply_boost_input(previous,level);
            previous = level;
        }
    }
    fn apply_boost_input(&mut self,previous:u8,level:u8) {
        let actor=self.current_actor().expect("validated player menu");
        if level>previous {
            self.voice(actor,8,VoiceCondition {border:level.into(),..Default::default()});
        } else {
            self.trace_rng("boost_reduce",Some(actor),self.state.rng);
        }
    }
    /// Continue a validated headless command-menu cursor without resetting the
    /// selected BP to zero. Selected BP belongs to that cursor, not spent BP.
    pub fn advance_boost_input(&mut self, previous: u8, level: u8) -> Result<()> {
        ensure(self.scenario.rules == RulesProfile::Ot2OpeningV1 && self.state.opening_crt.is_some(),
            "boost cycling requires the opening CRT profile")?;
        let actor=self.current_actor().ok_or_else(|| SimError("battle is over".into()))?;
        ensure(self.side(actor)==Some(Side::Player),"it is an enemy turn")?;
        let maximum=self.state.actors[actor.0].resources.bp.min(3);
        ensure(previous<=maximum && level<=maximum && previous.abs_diff(level)==1,
            "boost inputs must change one level within available BP")?;
        self.apply_boost_input(previous,level);
        Ok(())
    }
    /// Begin a new draw-count epoch at an observed Pirro R3 menu. This changes
    /// bookkeeping only: it retains every combat, AI, order and voice field.
    /// The high state bit is unobservable to CRT rand and can be canonicalized.
    pub fn rebase_pirro_identification(&mut self) -> Result<()> {
        ensure(self.is_pirro() && self.state.round==3 && self.current_actor()==Some(ActorId(0)),
            "identification checkpoint requires Pirro R3 player menu")?;
        self.state.rng = CrtRng::new(self.state.rng.state & 0x7fff_ffff);
        if let Some(crt)=self.state.opening_crt.as_mut() {crt.trace=Some(Vec::new());}
        self.initial=std::sync::Arc::new(self.state.clone());
        Ok(())
    }
    /// Identification leaves 3 BP selected. Execute its already-boosted R3
    /// Darkest Night without drawing the 0->3 boost voices a second time.
    pub fn pirro_identified_darkest_night(&mut self) -> Result<StepReport> {
        ensure(self.is_pirro() && self.state.round==3 && self.current_actor()==Some(ActorId(0)),
            "identified action requires Pirro R3 player menu")?;
        let actor=ActorId(0);
        let prepared=self.prepare(actor,Action::Skill(self.catalog().skill_id("ABI_SET_THI_020")?,3),Some(ActorId(1)))?;
        Ok(self.execute(actor,prepared,self.state.rng,false))
    }
    pub fn actor_name_id(&self,actor:ActorId)->Option<&str> {self.scenario.actors.get(actor.0).map(|a|a.id.as_str())}
    pub fn step_player(&mut self, command: Command) -> Result<StepReport> {
        self.step_player_inputs(command, None)
    }
    /// Preview boost inputs after the normal latent activation voice, without
    /// committing either. R8 early-latent searches must start from this boundary.
    pub fn preview_boost_inputs_after_latent(&self, levels: &[u8]) -> Result<BattleState> {
        let mut next = self.clone();
        next.activate_latent_for_command()?;
        next.preview_boost_inputs(levels)
    }
    fn activate_latent_for_command(&mut self) -> Result<()> {
        ensure(
            self.is_pirro() && self.current_actor() == Some(ActorId(0)),
            "latent double action requires Throné's player menu",
        )?;
        ensure(self.can_use_latent(), "Throné's latent power is not ready")?;
        self.state.actors[0].resources.latent = 0;
        self.state.pirro.as_mut().unwrap().double_action_remaining = 2;
        self.state.queue.insert(self.state.cursor + 1, ActorId(0));
        self.voice(ActorId(0), 19, VoiceCondition::default());
        Ok(())
    }
    /// Complete boost-button path from zero to the command's final selected BP.
    /// Replaces automatic boost inputs; does not spend BP until the action runs.
    pub fn step_player_with_boost_inputs(
        &mut self,
        command: Command,
        levels: &[u8],
    ) -> Result<StepReport> {
        self.step_player_inputs(command, Some(levels))
    }
    fn step_player_inputs(
        &mut self,
        command: Command,
        levels: Option<&[u8]>,
    ) -> Result<StepReport> {
        if let Command::Latent { command } = command {
            ensure(
                self.is_pirro() && self.current_actor() == Some(ActorId(0)),
                "latent double action requires Throné's player menu",
            )?;
            ensure(self.can_use_latent(), "Throné's latent power is not ready")?;
            ensure(
                !matches!(*command, Command::Latent { .. }),
                "nested latent activation",
            )?;
            // Validate/execute on a cheap state clone so invalid inner commands are atomic.
            let mut next = self.clone();
            let before = next.state.rng;
            next.activate_latent_for_command()?;
            let mut report = next.step_player_inputs(*command, levels)?;
            report
                .events
                .insert(0, Event::LatentActivated { actor: ActorId(0) });
            report.events.insert(
                1,
                Event::ResourceChanged {
                    actor: ActorId(0),
                    resource: Resource::Latent,
                    before: u32::from(self.crt_data().latent.as_ref().unwrap().maximum),
                    after: 0,
                },
            );
            report.rng_before = before;
            *self = next;
            return Ok(report);
        }
        let actor = self
            .current_actor()
            .ok_or_else(|| SimError("battle is over".into()))?;
        ensure(
            self.side(actor) == Some(Side::Player),
            "it is an enemy turn",
        )?;
        if let Command::Skill {id,..}|Command::Boost {id,..}=&command {
            if matches!(id.as_str(),"bewildering_grace"|"hired_help_guards"|"hired_help_beastlings"|"aggressive_slash"|"critical_scope"|"elemental_barrage"|"holy_light") {
                let mut next=self.clone();
                let report=next.grace_step(command,levels)?;
                *self=next;return Ok(report);
            }
        }
        let prepared = match command {
            Command::Latent { .. } => unreachable!(),
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
                ensure(!self.state.grace.as_ref().is_some_and(|g|g.items_disabled),"Items are scattered: item commands are disabled")?;
                self.prepare(actor, Action::Item(self.catalog().item_id(&id)?), target)?
            }
            Command::Wait => self.prepare(actor, Action::Wait, None)?,
        };
        let before = self.state.rng;
        if let Some(levels) = levels {
            let final_bp = match prepared.action {
                Action::Skill(_, bp) => bp,
                _ => 0,
            };
            self.validate_boost_inputs(levels, final_bp)?;
            self.apply_boost_inputs(levels);
        }
        Ok(self.execute(actor, prepared, before, levels.is_none()))
    }
    pub fn step_enemy(&mut self) -> Result<StepReport> {
        ensure(
            !self
                .state
                .opening_crt
                .as_ref()
                .is_some_and(|c| c.field_break),
            "Enemy actions are not exposed by the first-action field-break profile",
        )?;
        let actor = self
            .current_actor()
            .ok_or_else(|| SimError("battle is over".into()))?;
        ensure(self.side(actor) == Some(Side::Enemy), "it is a player turn")?;
        ensure(!self.is_hikari_recruitment() || actor == ActorId(4),
            "Recruitment grunt actions are outside the two-command route")?;
        if self.is_pirro() {
            return self.step_pirro_enemy();
        }
        let before = self.state.rng;
        let memory_before = self.state.ai[actor.0].clone();
        let actor_before = self.state.actors[actor.0].clone();
        let scenario = Arc::clone(&self.scenario);
        let definition = &scenario.actors[actor.0];
        let prepared = (|| {
            let charged = !self.is_mug()
                && self.state.actors[actor.0].enemy_charge_max > 0
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
                Ok(self.execute(actor, action, before, true))
            }
            Err(error) => {
                self.state.rng = before;
                self.state.ai[actor.0] = memory_before;
                self.state.actors[actor.0] = actor_before;
                Err(error)
            }
        }
    }
    fn step_pirro_enemy(&mut self) -> Result<StepReport> {
        let mut next = self.clone();
        let before = next.state.rng;
        let boss = ActorId(1);
        let hp = next.state.actors[1].resources.hp;
        let boss_ratio = hp as f32 / next.scenario.actors[1].stats.max_hp as f32;
        let player_ratio =
            next.state.actors[0].resources.hp as f32 / next.scenario.actors[0].stats.max_hp as f32;
        if boss_ratio < 0.75 && !next.state.pirro.as_ref().unwrap().latent_unlocked {
            next.state.pirro.as_mut().unwrap().latent_unlocked = true;
            // The sequence director releases the player directly, using
            // ActionPointInit. It does not call the manager's full-gauge helper.
            next.state.actors[0].resources.latent =
                next.crt_data().latent.as_ref().unwrap().initial;
            next.trace_rng("pirro_latent_tutorial", Some(ActorId(0)), next.state.rng);
        }
        // BoostActionCheck (including its BP reset/threshold reroll) belongs
        // to BattleAI_NormalEnemy.UpdateThink. Pirro's boss state machine
        // does not call or inherit it; reaching full BP alone consumes no RNG.
        // His scripted charged orders are handled by PirroState instead.
        let slot = next.state.pirro.as_mut().unwrap().choose(
            boss_ratio,
            player_ratio,
            hp,
            &mut next.state.rng,
        )?;
        let skill = next.scenario.actors[1].skills[slot as usize];
        let target = match next.catalog().skills[skill].target {
            TargetKind::SelfOnly => Some(boss),
            TargetKind::AllOpponents => None,
            TargetKind::Opponent => next.ai_target(boss, AiTarget::RandomOpponent)?,
            _ => return Err(SimError("unsupported Pirro target".into())),
        };
        let action = next.prepare(boss, Action::Skill(skill, 0), target)?;
        let last = !next.state.queue[next.state.cursor + 1..].contains(&boss);
        next.state
            .pirro
            .as_mut()
            .unwrap()
            .post_think(last, &mut next.state.rng);
        ensure(
            !next.state.pirro.as_ref().unwrap().omen_pending,
            "Pirro entered the charged multi-order phase, outside the supported fixed routine; refusing an approximate CRT prediction",
        )?;
        next.trace_rng("pirro_ai_and_target", Some(boss), before);
        let report = next.execute(boss, action, before, true);
        *self = next;
        Ok(report)
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
                    if id == "attack_staff" {
                        ensure(
                            ((self.is_mug() || self.is_hikari_recruitment() || self.state.opening_crt.as_ref().is_some_and(|c|c.field_break)) && matches!(action, Action::Skill(_, 0))) || (self.state.opening_crt.as_ref().is_some_and(|c|c.temenos_recruitment) && matches!(action,Action::Skill(_,1))),
                            "staff CRT callbacks are restricted to the unboosted Cape Cold Mug action",
                        )?;
                    }
                    if self.side(actor) == Some(Side::Player) {
                        (self.state.opening_crt.as_ref().is_some_and(|c|c.step_ahead)
                            && matches!(id,"stimulate"|"springy_boots"|"donate_bp")) || (self
                            .state
                            .opening_crt
                            .as_ref()
                            .is_some_and(|c| c.field_break)
                            && matches!(id, "attack_sword" | "attack_dagger" | "attack_staff" | "ABI_SET_THI_020" | "ABI_SET_SCH_010" | "ABI_SET_SCH_020"))
                            || (self.is_pirro()
                                && matches!(
                                    id,
                                    "attack_sword" | "attack_dagger" | "ABI_SET_THI_020"
                                ))
                            || id == self.default_attack(actor)?
                            || (self.is_mug()
                                && matches!(id, "ABI_SET_SCH_010" | "ABI_SET_SCH_020"))
                            || (self.is_second_battle()
                                && matches!(
                                    id,
                                    "ABI_SET_THI_020" | "ABI_SET_GST_0170" | "ABI_SET_GST_0190"
                                ))
                    } else {
                        (self.is_pirro() && id.starts_with("ABI_BOS_THI_C01_010_"))
                            || (self.is_hikari_recruitment() && matches!(id,"ABI_ENE_0010"|"ABI_ENE_0180"|"ABI_ENE_0190"))
                            || (self.is_mug() && id == "ABI_NPB_2570")
                            || matches!(id, "ABI_ENE_0041" | "ABI_ENE_0500")
                            || (id == "ABI_ENE_0010"
                                && self
                                    .state
                                    .opening_crt
                                    .as_ref()
                                    .is_some_and(|c| c.mansion_situation.is_some()))
                            || (self.is_second_battle()
                                && matches!(id, "ABI_ENE_0200" | "ABI_ENE_0010"))
                    }
                }
                Action::Defend => true,
                Action::Item(i) => {
                    (self.is_pirro() || self.is_hikari_recruitment() || self.state.opening_crt.as_ref().is_some_and(|c|c.field_item))
                        && self
                            .crt_data()
                            .skills
                            .contains_key(&self.catalog().items[i].id)
                }
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
                ensure(!self.state.actors[actor.0].has_status(StatusKind::Silence)||skill.id.starts_with("attack_"),"Silence prevents this skill")?;
                ensure(
                    boost <= 3 && (boost == 0 || skill.boosted.len() == 3),
                    "unsupported boost level",
                )?;
                ensure(
                    self.state.actors[actor.0].resources.bp >= boost,
                    "not enough BP",
                )?;
                for effect in skill.tier(boost).0 {
                    if let Effect::HpRatioDamage { weapon, .. } = effect {
                        ensure(
                            self.scenario.actors[actor.0]
                                .combat
                                .as_ref()
                                .is_some_and(|c| {
                                    self.side(actor) == Some(Side::Enemy)
                                        || c.weapons.iter().any(|w| w.weapon == *weapon)
                                }),
                            "HP-ratio damage needs an equipped weapon and combat profile",
                        )?;
                    }
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
        let effects = match action {
            Action::Skill(i, bp) => self.catalog().skills[i].tier(bp).0,
            Action::Item(i) => self.catalog().items[i].effects.as_slice(),
            _ => &[],
        };
        if effects.iter().any(|e| {
            matches!(
                e,
                Effect::ElementalFixedDamage { .. } | Effect::UnarmedHpRatioDamage { .. }
            )
        }) {
            ensure(
                self.scenario.actors[actor.0].combat.is_some()
                    && targets
                        .iter()
                        .all(|id| self.scenario.actors[id.0].combat.is_some()),
                "elemental/ratio damage requires combat profiles",
            )?;
        }
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
    fn execute(
        &mut self,
        actor: ActorId,
        action: Prepared,
        rng_before: CrtRng,
        automatic_boost: bool,
    ) -> StepReport {
        let scenario = Arc::clone(&self.scenario);
        let mut events = Vec::new();
        if let Some(crt) = &mut self.state.opening_crt {
            crt.settled_damage.fill(false);
        }
        self.state.actors[actor.0].guard = false;
        let mut normal_boost = 0;
        let (name, effects, hit, hits) = match action.action {
            Action::Skill(i, boost) => {
                let skill = &scenario.catalog.skills[i];
                // BattleCommandMenuActorBP invokes SetBestMagicAttackWeapon
                // when selecting elemental magic, including after a staff hit.
                if matches!(skill.id.as_str(),"ABI_SET_SCH_010"|"ABI_SET_SCH_020") {
                    self.state.actors[actor.0].main_weapon=scenario.actors[actor.0].combat.as_ref().unwrap().weapons.iter().max_by_key(|w|w.magic).map(|w|w.weapon);
                }
                normal_boost = boost;
                for level in 1..=if automatic_boost { boost } else { 0 } {
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
                self.promote_next_order(actor);
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
        } else if self.state.opening_crt.is_some() && self.crt_data().skills.contains_key(name) {
            let aggregate_reaction=action.targets.len()>1 || self.state.opening_crt.as_ref().is_some_and(|c|c.field_item)
                || (self.is_hikari_recruitment() && matches!(action.action, Action::Item(_)));
            let mut reaction=None;
            let mut shared_break_timer=None;
            let defer_break=aggregate_reaction && self.crt_data().skills[name].iter().filter(|s|matches!(s,AttackStep::Damage {..})).count()==1;
            for (i,&target) in action.targets.iter().enumerate() {
                let (pending,timer)=self.execute_scripted_skill(actor, target, name, effects, i==0, aggregate_reaction, defer_break, &mut events);
                if timer.is_some() {shared_break_timer=timer;}
                if pending.is_some_and(|kind|reaction.is_none_or(|old|kind>old)) {reaction=pending;}
            }
            // LibAilment.Battle_ExecDamageHP accumulates Kill/Break/Weakness
            // flags across its target loop, then chooses one reaction voice.
            // All target damage-notification timers precede this choice.
            if let Some(kind)=reaction {self.voice(actor,kind,VoiceCondition::default());}
            // A simultaneous multi-target hit schedules one command-level break timer,
            // after the aggregate reaction voice (retail Fireball trace).
            if let Some(target)=shared_break_timer {self.timer_rng("break_slow_timer",Some(target));}
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
        self.complete_player_action(actor,events,rng_before)
    }
    fn complete_player_action(&mut self,actor:ActorId,mut events:Vec<Event>,rng_before:CrtRng)->StepReport {
        if let Some(p) = self.state.pirro.as_mut() {
            p.force_miss = false;
        }
        self.refresh_outcome();
        if self.state.outcome==Outcome::Ongoing && self.state.opening_crt.as_ref().is_some_and(|c|c.step_ahead) {
            self.grace_post_action(actor,&mut events);self.refresh_outcome();
        }
        if self.state.outcome != Outcome::Ongoing {
            events.push(Event::BattleEnded {
                outcome: self.state.outcome,
            });
            if self.state.outcome == Outcome::Victory && self.state.opening_crt.is_some() {
                if self.state.opening_crt.as_ref().is_some_and(|c|c.field_item) {
                    self.voice(actor,2,VoiceCondition::default());
                } else if self.is_second_battle() || self.is_pirro() || self.is_mug() || self.is_hikari_recruitment() {
                    self.finish_second(actor);
                } else {
                    self.finish_opening(actor);
                }
            }
        } else {
            if self.is_pirro() && actor == ActorId(0) {
                let p = self.state.pirro.as_mut().unwrap();
                p.double_action_remaining = p.double_action_remaining.saturating_sub(1);
            }
            self.state.cursor += 1;
            while self.state.cursor < self.state.queue.len()
                && (!self.state.actors[self.state.queue[self.state.cursor].0].alive()
                    || self.state.actors[self.state.queue[self.state.cursor].0].broken())
            {
                self.state.cursor += 1;
            }
            if self.state.cursor == self.state.queue.len() {self.grace_last_act();}
            if self.state.cursor == self.state.queue.len() {
                self.start_round(&mut events);
                events.push(Event::RoundStarted {
                    round: self.state.round,
                });
            }
            // Guards are the attack enforcers; the hiring merchant did not break.
            let hired=events.iter().any(|e|matches!(e,Event::Action {name,..} if matches!(name.as_str(),"hired_help_guards"|"hired_help_beastlings")));
            self.order_voice((!hired&&events.iter().any(|e|matches!(e,Event::Broken {..}))).then_some(actor));
            if self
                .state
                .pirro
                .as_ref()
                .is_some_and(|p| p.phase_frame_pending)
            {
                // ChangeActionMode's fade completes after the action/round
                // callbacks, including order voice, before command input.
                // ApplyChangeActionMode reinitializes the enemy's idle frame
                // using main CRT. The winning capture orders voice at5796,
                // then frame at5797; the loss has no next-enemy voice draw.
                self.state.pirro.as_mut().unwrap().phase_frame_pending = false;
                let before = self.state.rng;
                self.state.rng.next_u15();
                self.trace_rng("pirro_phase_frame", Some(ActorId(1)), before);
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
        let data = opening_crt::data();
        let mut pending_break_timer = None;
        let mut break_busy_until = 0;
        let mut voice_busy_until = 0;
        let normal_id = self.default_attack(actor).ok();
        let plan = if self.state.opening_crt.as_ref().is_some_and(|c|c.temenos_recruitment||c.lord_sands) {
            &self.crt_data().normal_attacks[&format!("{}:{boost}",normal_id.unwrap())][..]
        } else if self.is_hikari_recruitment() {
            &self.crt_data().normal_attacks[normal_id.expect("validated recruitment weapon")][..]
        } else if attack.weapon == Some(combat::Weapon::Staff) {
            &self.crt_data().staff_attack[..]
        } else {
            opening_crt::attack_plan(attack.weapon.expect("normal weapon"), boost)
        };
        for step in plan {
            if pending_break_timer.is_some_and(|tick| tick <= step.tick()) {
                self.timer_rng("break_slow_timer", Some(target));
                pending_break_timer = None;
            }
            match *step {
                AttackStep::Check { .. } => hit = self.normal_accuracy(actor, target, attack),
                AttackStep::Voice { tick, r#type } => {
                    let duration_key = if self.is_hikari_recruitment() {
                        format!("{}:{}",normal_id.unwrap(),actor.0)
                    } else { "attack_staff".into() };
                    voice_busy_until = self.crt_data().skill_voice_duration_ticks
                        .get(&duration_key).filter(|_| self.is_hikari_recruitment() || attack.weapon == Some(combat::Weapon::Staff))
                        .map_or(u32::MAX, |duration| tick.saturating_add(*duration));
                    self.voice(actor, r#type, VoiceCondition::default())
                }
                AttackStep::Damage { tick, last, .. } => {
                    // DefaultTarget stays in the source command until its final
                    // key. Overkill hits still calculate damage and consume CRT.
                    let broken_before = self.state.actors[target.0].broken();
                    let revealed_before = (self.state.actors[target.0].revealed_weapon_weaknesses,
                        self.state.actors[target.0].revealed_element_weaknesses);
                    self.apply_attack(actor, target, attack, Some(hit), events);
                    let just_break = !broken_before && self.state.actors[target.0].broken();
                    if boost == 0 && hit && tick >= voice_busy_until
                        && self.side(actor) == Some(Side::Player) && self.side(target) == Some(Side::Enemy) {
                        let reaction = if last && !self.state.actors[target.0].alive() { Some(12) }
                            else if just_break { Some(10) }
                            else if revealed_before != (self.state.actors[target.0].revealed_weapon_weaknesses,
                                self.state.actors[target.0].revealed_element_weaknesses) { Some(9) }
                            else { None };
                        if let Some(kind) = reaction { self.voice(actor, kind, VoiceCondition::default()); }
                    }
                    // SetDamageForEnemy finalizes death on the final authored
                    // hit, clearing Break before the slow-timer condition.
                    if last && !self.state.actors[target.0].alive() {
                        self.state.actors[target.0].break_until_round = 0;
                    }
                    if hit && (just_break || self.state.actors[target.0].broken()) {
                        if just_break {
                            pending_break_timer = Some(tick + data.break_timer_delay_ticks);
                            break_busy_until = tick + data.break_busy_ticks;
                        } else if tick >= break_busy_until {
                            // Battle_ExecDamageHP -> PlayFinishBlow. Its zero
                            // delay schedules K2_SetTimerDelegate, which draws
                            // CRT even with zero initial-start variance.
                            self.timer_rng("broken_hit_slow_timer", Some(target));
                        }
                    }
                    // JustBreak is latched by the damage call and survives the
                    // death-state cleanup for the subsequent break-camera callback.
                }
            }
        }
        if pending_break_timer.is_some() {
            self.timer_rng("break_slow_timer", Some(target));
        }
    }
    fn execute_scripted_skill(
        &mut self,
        actor: ActorId,
        target: ActorId,
        id: &str,
        effects: &[Effect],
        emit_voice: bool,
        defer_reaction: bool,
        defer_break: bool,
        events: &mut Vec<Event>,
    ) -> (Option<u8>,Option<ActorId>) {
        let mut deferred=None;
        let data = self.crt_data();
        // StartAbilityCamera sets Playing until the command's post-action
        // callback. PlayFinishBlow returns before its timer while Playing.
        // A newly inflicted break still schedules its separate timer.
        let ability_camera_active = data.skill_uses_ability_camera[id];
        let mut pending_break_timer = None;
        let mut break_busy_until = 0;
        let mut voice_busy_until = 0;
        for step in &data.skills[id] {
            if pending_break_timer.is_some_and(|tick| tick <= step.tick()) {
                self.timer_rng("break_slow_timer", Some(target));
                pending_break_timer = None;
            }
            match *step {
                AttackStep::Voice { tick, r#type } => {
                    let duration_key = if let Some(indices)=self.state.opening_crt.as_ref().and_then(|c|c.carried_voices.as_ref()) {format!("{id}:{}",indices[actor.0])}
                        else if self.is_hikari_recruitment() { format!("{id}:{}",actor.0) }
                        else if self.state.opening_crt.as_ref().is_some_and(|c|c.field_item) && actor.0==1 && id=="shadow_soulstone_m" { "shadow_soulstone_m_osvald".into() }
                        else { id.into() };
                    voice_busy_until = tick
                        + data
                        .skill_voice_duration_ticks
                            .get(&duration_key)
                            .copied()
                            .unwrap_or(0);
                    if emit_voice { self.voice(
                        actor,
                        r#type,
                        VoiceCondition {
                            ability: match id {
                                "ABI_SET_THI_020" => Some("ABI_THI_020_01"),
                                "ABI_SET_GST_0170" => Some("ABI_GST_0170_01"),
                                "ABI_SET_GST_0190" => Some("ABI_GST_0190_01"),
                                _ => data.ability_voice_ids.get(id).map(String::as_str),
                            },
                            target_self: actor == target,
                            ..Default::default()
                        },
                    ); }
                }
                AttackStep::Damage {
                    tick,
                    last,
                    effect: index,
                } => {
                    let broken_before = self.state.actors[target.0].broken();
                    let revealed_before = (
                        self.state.actors[target.0].revealed_weapon_weaknesses,
                        self.state.actors[target.0].revealed_element_weaknesses,
                    );
                    for effect in effects
                        .iter()
                        .enumerate()
                        .filter_map(|(i, e)| index.is_none_or(|v| v == i).then_some(e))
                    {
                        self.apply_action_effect(actor, target, effect, events);
                    }
                    // LibAilment prioritizes last-hit kills, then JustBreak,
                    // then a newly discovered weakness. Discovery persists
                    // even when a higher-priority reaction or voice gate wins.
                    // Unlike ordinary boosted attacks, CommonCommand permits
                    // these reactions. Its scalar voice gate uses game time.
                    if self.side(actor) == Some(Side::Player)
                        && self.side(target) == Some(Side::Enemy)
                        && self.state.actors[actor.0].alive()
                        && tick >= voice_busy_until
                    {
                        let reaction = if last && !self.state.actors[target.0].alive() {
                            Some(12)
                        } else if !broken_before && self.state.actors[target.0].broken() {
                            Some(10)
                        } else if revealed_before
                            != (
                                self.state.actors[target.0].revealed_weapon_weaknesses,
                                self.state.actors[target.0].revealed_element_weaknesses,
                            )
                        {
                            Some(9)
                        } else {
                            None
                        };
                        if let Some(voice_type) = reaction {
                            if defer_reaction {deferred=Some(voice_type);} else {
                                self.voice(actor, voice_type, VoiceCondition::default());
                            }
                        }
                    }
                    if last && !self.state.actors[target.0].alive() {
                        self.state.actors[target.0].break_until_round = 0;
                    }
                    if self.side(target) == Some(Side::Enemy)
                        && self.state.actors[target.0].broken()
                    {
                        if !broken_before {
                            pending_break_timer = Some(tick + data.break_timer_delay_ticks);
                            break_busy_until = tick + data.break_busy_ticks;
                        } else if !ability_camera_active && tick >= break_busy_until {
                            self.timer_rng("broken_hit_slow_timer", Some(target));
                        }
                    }
                }
                AttackStep::Check { .. } => {
                    unreachable!("imported CommonCommand has no cached hit check")
                }
            }
        }
        let shared_timer=if defer_break && pending_break_timer.is_some() {Some(target)} else {None};
        if pending_break_timer.is_some() && !defer_break {
            self.timer_rng("break_slow_timer", Some(target));
        }
        (deferred,shared_timer)
    }
    fn apply_effect(&mut self, target: ActorId, effect: &Effect, events: &mut Vec<Event>) {
        let stats = &self.scenario.actors[target.0].stats;
        let resources = &mut self.state.actors[target.0].resources;
        let (resource, before, after) = match effect {
            Effect::Damage { .. }
            | Effect::OrderShift { .. }
            | Effect::NoEffect
            | Effect::ElementalFixedDamage { .. }
            | Effect::UnarmedHpRatioDamage { .. }
            | Effect::SpRatioDamage { .. }
            | Effect::ForceMiss
            | Effect::SignOfBoost { .. }
            | Effect::HpRatioDamage { .. }
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
            Effect::NoEffect => (),
            Effect::Restore {resource,recovery} if self.state.opening_crt.as_ref().is_some_and(|c|c.step_ahead)=>{
                let amount=match recovery {
                    Recovery::Fixed {amount}=>*amount,
                    Recovery::Percent {percent}=>{
                        let cap=match resource {
                            Resource::Hp=>self.scenario.actors[target.0].stats.max_hp,
                            Resource::Sp=>self.scenario.actors[target.0].stats.max_sp,
                            Resource::Bp=>5,
                            Resource::Latent=>u32::from(crate::field_escape::player_latent_max(&self.scenario.actors[target.0].id).expect("validated player")),
                        };
                        (cap as f32*(*percent as f32*0.01_f32)) as u32
                    },
                };
                self.grace_recover(actor,target,*resource,amount,(actor!=target).then_some(target),events);
            },
            Effect::OrderShift { positions } => {
                self.shift_action_orders(actor, &[target], i32::from(*positions));
            }
            Effect::ElementalFixedDamage { amount, element } => {
                let broken = self.state.actors[target.0].broken();
                let weak = self.scenario.actors[target.0]
                    .combat
                    .as_ref()
                    .unwrap()
                    .element_weaknesses
                    .contains(element);
                let value = combat::elemental_fixed_damage(*amount,weak,broken,self.scenario.actors[target.0].combat.as_ref().unwrap().use_cat_damage_type);
                if weak {
                    self.state.actors[target.0].revealed_element_weaknesses |= 1 << *element as u8;
                }
                self.resolve_damage(actor, target, (value as u32).max(1), false, weak, events);
            }
            Effect::UnarmedHpRatioDamage { percent, hit_bonus } => {
                // JudgeSpecialHitRate applies ATTACK_HIT_FAILED only to CalcType 0.
                // HP_RATIO_DAMAGE is CalcType 2, including Pirro's Pierce Through.
                let a = self.scenario.actors[actor.0].combat.as_ref().unwrap();
                let d = self.scenario.actors[target.0].combat.as_ref().unwrap();
                let before = self.state.rng;
                let hit = self
                    .state
                    .rng
                    .weighted_bool(combat::hit_probability(a.accuracy, d.evasion, *hit_bonus));
                self.trace_rng("hp_ratio_accuracy", Some(actor), before);
                if hit {
                    let amount =
                        (self.scenario.actors[target.0].stats.max_hp * u32::from(*percent) / 100)
                            .min(self.state.actors[target.0].resources.hp)
                            .max(1);
                    self.resolve_damage(actor, target, amount, false, false, events);
                } else {
                    events.push(Event::Miss { actor, target });
                    self.damage_notification(target, None);
                }
            }
            Effect::SpRatioDamage { percent } => {
                let before = self.state.actors[target.0].resources.sp;
                let amount =
                    self.scenario.actors[target.0].stats.max_sp * u32::from(*percent) / 100;
                self.state.actors[target.0].resources.sp = before.saturating_sub(amount);
                events.push(Event::ResourceChanged {
                    actor: target,
                    resource: Resource::Sp,
                    before,
                    after: self.state.actors[target.0].resources.sp,
                });
                self.timer_rng("sp_damage_notification_timer", Some(target));
            }
            Effect::ForceMiss => {
                let before = self.state.rng;
                self.roll_status_application();
                self.trace_rng("scripted_forced_miss_status", Some(actor), before);
                if let Some(p) = self.state.pirro.as_mut() {
                    p.force_miss = true;
                }
            }
            Effect::SignOfBoost { enabled } => {
                if *enabled {
                    let before = self.state.rng;
                    self.roll_status_application();
                    self.trace_rng("sign_of_boost_status", Some(actor), before);
                }
                if let Some(p) = self.state.pirro.as_mut() {
                    p.sign_of_boost = *enabled;
                }
            }
            Effect::Damage { attack } => self.apply_attack(actor, target, attack, None, events),
            Effect::HpRatioDamage {
                percent,
                weapon,
                hit_bonus,
            } => self.apply_hp_ratio_damage(actor, target, *percent, *weapon, *hit_bonus, events),
            Effect::Status { status } => {
                let before = self.state.rng;
                self.roll_status_application();
                if self.is_pirro() {
                    self.trace_rng("pirro_status_application", Some(actor), before);
                } else if self.is_mug() {
                    self.trace_rng("npc_status_application", Some(actor), before);
                }
                let statuses = &mut self.state.actors[target.0].statuses;
                if let Some(old) = statuses.iter_mut().find(|s| s.kind == status.kind) {
                    old.turns = if old.turns == 255 || status.turns == 255 {
                        255
                    } else {
                        old.turns.saturating_add(status.turns).min(9)
                    };
                } else {
                    statuses.push(*status);
                }
                events.push(Event::StatusApplied {
                    actor: target,
                    status: *status,
                });
                if status.kind==StatusKind::ActFirst {
                    self.promote_next_order(target);
                }
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
            _ => {
                self.apply_effect(target, effect, events);
                if self.is_pirro()
                    && matches!(
                        effect,
                        Effect::Restore {
                            resource: Resource::Hp,
                            ..
                        }
                    )
                {
                    self.timer_rng("recovery_notification_timer", Some(target));
                }
            }
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
        target: ActorId,
        attack: &combat::Attack,
        prechecked_hit: Option<bool>,
        events: &mut Vec<Event>,
    ) {
        self.apply_attack_profile(actor,target,attack,prechecked_hit,None,events);
    }
    fn apply_attack_profile(&mut self,actor:ActorId,mut target:ActorId,attack:&combat::Attack,prechecked_hit:Option<bool>,summon:Option<&CombatProfile>,events:&mut Vec<Event>) {
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
        if let Some(weapon) = attack.weapon.filter(|_|summon.is_none()) {
            self.state.actors[actor.0].main_weapon = Some(weapon);
        }
        let scenario = Arc::clone(&self.scenario);
        let attacker = summon.unwrap_or_else(||scenario.actors[actor.0]
            .combat
            .as_ref()
            .expect("validated combat profile"));
        let defender = scenario.actors[target.0]
            .combat
            .as_ref()
            .expect("validated combat profile");
        let broken = self.state.actors[target.0].broken();
        let magical = attack.kind == DamageKind::Magical;
        let accuracy =
            combat::hit_probability(attacker.accuracy, defender.evasion, attack.hit_bonus);
        let before = self.state.rng;
        let crit_chance = combat::critical_probability_with_statuses(
            attacker.critical,
            scenario.actors[target.0].stats.speed,
            self.state.actors[target.0].has_status(StatusKind::SpeedUp),
            self.state.actors[target.0].has_status(StatusKind::SpeedDown),
            attack.critical_bonus,
        );
        let forced_miss =
            actor == ActorId(1) && self.state.pirro.as_ref().is_some_and(|p| p.force_miss);
        let critical = !forced_miss && (self.state.actors[target.0].has_status(StatusKind::ForceCritical) || (!magical && self.state.rng.weighted_bool(crit_chance)));
        let hit = if forced_miss {
            false
        } else if let Some(hit) = prechecked_hit {
            hit
        } else {
            magical || broken || !self.state.actors[target.0].alive() || critical || self.state.rng.weighted_bool(accuracy)
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
        if weapon_weak {
            self.state.actors[target.0].revealed_weapon_weaknesses |=
                1 << attack.weapon.unwrap() as u8;
        }
        if element_weak {
            self.state.actors[target.0].revealed_element_weaknesses |=
                1 << attack.element.unwrap() as u8;
        }
        let equipped = (if summon.is_some() {attacker.main_weapon} else {self.state.actors[actor.0].main_weapon})
            .and_then(|w| attacker.weapons.iter().find(|s| s.weapon == w));
        let atk = if magical {
            attacker.magic + equipped.map_or(0, |w| w.magic)
        } else {
            attacker.attack + equipped.map_or(0, |w| w.attack)
        };
        let amount = combat::damage_with_equipment(
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
                attack_up: summon.is_none()&&self.state.actors[actor.0].has_status(if magical {
                    StatusKind::MagicUp
                } else {
                    StatusKind::AttackUp
                }),
                defense_down: self.state.actors[target.0].has_status(if magical {
                    StatusKind::MagicDefenseDown
                } else { StatusKind::DefenseDown }),
                defense_up: self.state.actors[target.0].has_status(if magical {
                    StatusKind::MagicDefenseUp
                } else {
                    StatusKind::DefenseUp
                }),
            },
            attack,
            combat::variance(&mut self.state.rng),
            defender.use_cat_damage_type,
            summon.is_none()&&self.state.actors[actor.0].has_status(if magical { StatusKind::MagicDown } else { StatusKind::AttackDown }),
            if summon.is_none()&&attack.element==Some(combat::Element::Fire) {attacker.fire_damage_bonus}else{0},
        );
        self.trace_rng("critical_hit_and_damage", Some(actor), before);
        self.resolve_damage(
            actor,
            target,
            amount,
            critical,
            weapon_weak || element_weak,
            events,
        );
    }
    fn apply_hp_ratio_damage(
        &mut self,
        actor: ActorId,
        target: ActorId,
        percent: u8,
        weapon: combat::Weapon,
        hit_bonus: i32,
        events: &mut Vec<Event>,
    ) {
        self.state.actors[actor.0].main_weapon = Some(weapon);
        let attacker = self.scenario.actors[actor.0]
            .combat
            .as_ref()
            .expect("combat profile");
        let defender = self.scenario.actors[target.0]
            .combat
            .as_ref()
            .expect("combat profile");
        let weak = defender.weapon_weaknesses.contains(&weapon);
        let before = self.state.rng;
        let hit = self.state.actors[target.0].broken()
            || self.state.rng.weighted_bool(combat::hit_probability(
                attacker.accuracy,
                defender.evasion,
                hit_bonus,
            ));
        self.trace_rng("hp_ratio_accuracy", Some(actor), before);
        if !hit {
            events.push(Event::Miss { actor, target });
            self.damage_notification(target, None);
            return;
        }
        let amount = ((u64::from(self.scenario.actors[target.0].stats.max_hp) * u64::from(percent)
            / 100) as u32)
            .min(self.state.actors[target.0].resources.hp)
            .max(1);
        if weak {
            self.state.actors[target.0].revealed_weapon_weaknesses |= 1 << weapon as u8;
        }
        self.resolve_damage(actor, target, amount, false, weak, events);
    }
    fn resolve_damage(
        &mut self,
        actor: ActorId,
        target: ActorId,
        amount: u32,
        critical: bool,
        weak: bool,
        events: &mut Vec<Event>,
    ) {
        // HOLD_OUT restores HP to one in SetDamageForEnemy; displayed damage
        // and OnTakeDamage receive the original value.
        let hp_loss =
            if target == ActorId(1) && self.state.pirro.as_ref().is_some_and(|p| p.hold_out) {
                amount.min(self.state.actors[target.0].resources.hp.saturating_sub(1))
            } else {
                amount
            };
        let broken = self.state.actors[target.0].broken();
        events.push(Event::Damage {
            actor,
            target,
            amount,
            critical,
            weak,
            broken_before: broken,
        });
        if self.side(target) == Some(Side::Enemy) && !broken && weak {
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
                self.break_action_order(target);
                if self.state.opening_crt.as_ref().is_some_and(|c|c.step_ahead)&&self.side(actor)==Some(Side::Player) {
                    let maximum=crate::field_escape::player_latent_max(&self.scenario.actors[actor.0].id).expect("carried player");
                    let resource=&mut self.state.actors[actor.0].resources.latent;
                    *resource=resource.saturating_add(25).min(maximum);
                }
                if let Some(p) = self.state.pirro.as_mut() {
                    p.on_break();
                    if p.latent_unlocked {
                        self.state.actors[0].resources.latent =
                            (self.state.actors[0].resources.latent + 25)
                                .min(self.crt_data().latent.as_ref().unwrap().maximum);
                    }
                }
            }
        }
        let was_alive=self.state.actors[target.0].alive();
        self.apply_effect(target, &Effect::FixedDamage { amount: hp_loss }, events);
        if was_alive&&!self.state.actors[target.0].alive()
            && self.state.opening_crt.as_ref().is_some_and(|c|c.step_ahead) {
            self.grace_player_death(target);
        }
        if self.state.pirro.as_ref().is_some_and(|p| p.latent_unlocked)
            && target == ActorId(0)
            && self.state.actors[0].alive()
        {
            let gain =
                ((amount as f32 / self.scenario.actors[0].stats.max_hp as f32) * 1.3_f32 * 100.0)
                    .floor() as u16;
            self.state.actors[0].resources.latent = self.state.actors[0]
                .resources
                .latent
                .saturating_add(gain)
                .min(self.crt_data().latent.as_ref().unwrap().maximum);
        }
        if (self.is_hikari_recruitment()||self.state.opening_crt.as_ref().is_some_and(|c|c.step_ahead)) && self.side(target)==Some(Side::Player)
            && self.state.actors[target.0].alive() {
            // SetDamageForPlayer -> CalcPotentialityActionPoint. The bounded
            // route requires all four Chapter 1 release flags and no gain support.
            // This scalar bookkeeping has no CRT call, including reaching max.
            let maximum=if self.is_hikari_recruitment() {self.crt_data().player_latent_maxima[target.0]}
                else {crate::field_escape::player_latent_max(&self.scenario.actors[target.0].id).expect("carried player")};
            let gain=((amount as f32 / self.scenario.actors[target.0].stats.max_hp as f32)
                * 1.3_f32 * 100.0).floor() as u16;
            let resource=&mut self.state.actors[target.0].resources.latent;
            let before=*resource;
            *resource=resource.saturating_add(gain).min(maximum);
            if before!=*resource {events.push(Event::ResourceChanged {actor:target,resource:Resource::Latent,
                before:u32::from(before),after:u32::from(*resource)});}
        }
        self.damage_notification(target, Some(amount));
    }
    fn trace_rng(&mut self, reason: &'static str, actor: Option<ActorId>, before: CrtRng) {
        if self.search_lottery_trace && reason!="bewildering_grace_lottery" {return;}
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
        if self.side(actor) != Some(Side::Player) && !self.is_pirro() {
            return;
        }
        let data = self.crt_data();
        let voice_index =
            if let Some(indices)=self.state.opening_crt.as_ref().and_then(|c|c.carried_voices.as_ref()) {indices[actor.0]}
            else if actor == ActorId(1) && self.state.pirro.as_ref().is_some_and(|p| p.second_phase) {
                2
            } else {
                actor.0
            };
        let before = self.state.rng;
        let Some(crt) = self.state.opening_crt.as_mut() else {
            return;
        };
        if actor.0 >= crt.settled_damage.len() || voice_index >= crt.used_voices.len() {
            return;
        }
        if matches!(voice_type, 4 | 11) && crt.settled_damage[actor.0] {
            return;
        }
        let chosen = opening_crt::select_voice_from(
            data,
            voice_index,
            voice_type,
            condition,
            &mut crt.used_voices[voice_index],
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
                9 => "weakness_voice",
                10 => "break_voice",
                11 => "avoid_voice",
                12 => "kill_voice",
                17 => "guard_voice",
                18 => "ability_voice",
                19 => "latent_voice",
                15 | 16 => "item_voice",
                2 => "victory_voice",
                _ => "voice",
            },
            Some(actor),
            before,
        );
    }
    fn order_voice(&mut self, breaker: Option<ActorId>) {
        if let Some(actor) = self.current_actor() {
            let ally_breaker=breaker.filter(|b|*b!=actor).and_then(|b|match self.scenario.actors[b.0].id.as_str() {
                "osvald"=>Some("ePROFESSOR"),"throne"=>Some("eTHIEF"),"hikari"=>Some("eFENCER"),"agnea"=>Some("eDANCER"),"partitio"=>Some("eMERCHANT"),_=>None});
            let state = &self.state.actors[actor.0];
            self.voice(
                actor,
                3,
                VoiceCondition {
                    ally_breaker,
                    pinch: u64::from(state.resources.hp) * 4
                        <= u64::from(self.scenario.actors[actor.0].stats.max_hp),
                    decondition: false,
                    border: 0,
                    ..Default::default()
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
    fn timer_rng(&mut self, reason: &'static str, actor: Option<ActorId>) {
        if self.state.opening_crt.is_some() {
            let before = self.state.rng;
            self.state.rng.next_u15();
            self.trace_rng(reason, actor, before);
        }
    }
    fn finish_second(&mut self, actor: ActorId) {
        let data = self.crt_data();
        if data.enemy_death_slow {
            // This profile has one boss. SetDamageForEnemy settles its death
            // on the last damage key; the delayed ENEMY_DEAD_SLOW timer fires
            // after damage UI, before victory voice/drop. No rendering runs.
            self.timer_rng("enemy_death_slow_timer", Some(ActorId(1)));
        }
        if data.victory_voice {
            self.voice(actor, 2, VoiceCondition::default());
        }
        let before = self.state.rng;
        let mut items = BTreeMap::new();
        for (i, drop) in data.drops.iter().enumerate() {
            let roll_before = self.state.rng;
            // The 100% leader drop still traverses RandomBoolWithWeight.
            if self.state.rng.weighted_bool(drop.rate as f32 * 0.01_f32) {
                *items.entry(drop.item.clone()).or_insert(0) += 1;
                if let Ok(id) = self.catalog().item_id(&drop.item) {
                    self.state.inventory[id] = self.state.inventory[id].saturating_add(1);
                } else {
                    assert!(self.is_mug() || self.is_hikari_recruitment(), "unimported usable drop item");
                }
            }
            let players = self
                .scenario
                .actors
                .iter()
                .filter(|a| a.side == Side::Player)
                .count();
            self.trace_rng("enemy_drop", Some(ActorId(players + i)), roll_before);
        }
        self.state.opening_crt.as_mut().unwrap().rewards = Some(opening_crt::BattleRewards {
            before,
            after: self.state.rng,
            items,
        });
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
        let encounter_steps_after_field_reset = data.encounter.reset(&mut self.state.rng, true);
        self.trace_rng("field_encounter_reset_b_then_a", None, rng_after_rewards);
        let rng_after_field_reset = self.state.rng;
        // MS_TOU_10_0300 has EncountReset=true. EventManager.EndEvent resets
        // the counter again when the post-battle story event completes.
        let encounter_steps = if data.post_event_encounter_reset {
            let steps = data.encounter.reset(&mut self.state.rng, true);
            self.trace_rng(
                "post_battle_event_encounter_reset_b_then_a",
                None,
                rng_after_field_reset,
            );
            steps
        } else {
            encounter_steps_after_field_reset
        };
        self.state
            .opening_crt
            .as_mut()
            .expect("opening CRT")
            .after_battle = Some(AfterBattle {
            rng_before_rewards,
            rng_after_rewards,
            rng_after_field_reset,
            rng_after_post_battle_event: self.state.rng,
            healing_grapes,
            encounter_steps_after_field_reset,
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
        if round == 1 {
            match self
                .state
                .opening_crt
                .as_ref()
                .and_then(|c| c.mansion_situation)
            {
                Some(crate::escape::Situation::Preemptive) => {
                    rows.retain(|id| self.side(*id) == Some(Side::Player))
                }
                Some(crate::escape::Situation::Surprise) => {
                    rows.retain(|id| self.side(*id) == Some(Side::Enemy))
                }
                _ => (),
            }
        }
        combat::sort_order(&mut rows, |id| {
            let s = &self.state.actors[id.0];
            // The first preview uses the timing values cached at initialization.
            // Adding the night talent marks order rows but does not call
            // SetupWaitTime; EndOrder/BeginBattleTurn refresh that cache later.
            let speed_status = if self.state.round == 1 {
                &self.initial.actors[id.0]
            } else {
                s
            };
            let speed = (self.scenario.actors[id.0].stats.speed as f32
                * combat::parameter_disease_ratio(speed_status.has_status(StatusKind::SpeedUp), speed_status.has_status(StatusKind::SpeedDown))) as u32;
            if s.broken() {
                if s.break_until_round <= round {
                    speed + 1_000_000
                } else if round > self.state.round {
                    speed
                } else {
                    1
                }
            } else if round > self.state.round && s.has_status(StatusKind::ActFirst)
                && self.side(id) == Some(Side::Player) {
                // BattlePlayerBP.CalcNextTurnWaitTime, entry 23419: disease15.
                speed + 20_000
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
        combat::shuffle_order(&mut rows, |id| !states[id.0].broken()
            && !states[id.0].has_status(StatusKind::ActFirst), &mut self.state.rng);
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
                    if status.turns != 255 && status.kind != StatusKind::MagicReflection {
                        status.turns = status.turns.saturating_sub(1);
                    }
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
            // ChangeBattleTurn -> BeginAddBoostPoint schedules its delegate
            // after actor revisions, before UpdateNextOrder's shuffle.
            self.timer_rng("round_bp_timer", None);
        }
        self.state.queue = if first {
            let mut rows = self.source_order(self.state.round);
            // InitializeActionOrder -> SetupFirstTurnPlayer: normal encounters
            // promote the first shuffled player before applying start talents.
            if let Some(index) = rows
                .iter()
                .position(|id| self.side(*id) == Some(Side::Player))
            {
                rows[..=index].rotate_right(1);
            }
            rows
        } else {
            std::mem::take(&mut self.state.next_queue)
        };
        self.state
            .queue
            .retain(|id| self.state.actors[id.0].alive() && !self.state.actors[id.0].broken());
        if first
            && self.state.opening_crt.as_ref().is_some_and(|crt| {
                let data = if crt.hikari_recruitment {
                    self.crt_data()
                } else if crt.cape_cold_mug {
                    opening_crt::mug_data()
                } else if crt.field_break {
                    self.crt_data()
                } else if crt.mansion_situation.is_some() {
                    opening_crt::mansion_data()
                } else if crt.pirro_battle {
                    opening_crt::pirro_data()
                } else if crt.second_battle {
                    opening_crt::second_data()
                } else {
                    opening_crt::data()
                };
                data.battle_start_voice
            })
        {
            let before = self.state.rng;
            // PlayPlayerSideVoice_BattleStart first searches for the living
            // EnemyGroup.TalkPlayerID. Only the fallback calls RandomInteger,
            // even if that fallback contains just one player.
            let fixed = self
                .crt_data()
                .battle_start_speaker
                .as_deref()
                .and_then(|id| {
                    self.scenario.actors.iter().enumerate().find_map(|(i, a)| {
                        (a.id == id && a.side == Side::Player && self.state.actors[i].alive())
                            .then_some(ActorId(i))
                    })
                });
            let speaker = fixed.unwrap_or_else(|| {
                let players: Vec<_> = self
                    .scenario
                    .actors
                    .iter()
                    .enumerate()
                    .filter(|(i, a)| a.side == Side::Player && self.state.actors[*i].alive())
                    .map(|(i, _)| ActorId(i))
                    .collect();
                players[self.state.rng.index(players.len() as u32) as usize]
            });
            self.trace_rng("battle_start_speaker", Some(speaker), before);
            self.voice(speaker, 1, VoiceCondition::default());
        }
        if first && (self.is_mug() || self.is_hikari_recruitment() || self.state.opening_crt.as_ref().is_some_and(|c|c.field_break))
            && self.scenario.actors.iter().any(|a|a.id=="osvald" && a.side==Side::Player) {
            let mut seen=std::collections::BTreeSet::new();
            for i in 0..self.scenario.actors.len() {
                let d=&self.scenario.actors[i];
                if d.side!=Side::Enemy || !seen.insert(d.id.clone()) {continue;}
                let id=d.id.clone();
                let spans=crate::knowledge::study(d.combat.as_ref().unwrap(),
                    self.state.enemy_knowledge.entry(id.clone()).or_default(), &mut self.state.rng);
                let known=self.state.enemy_knowledge[&id];
                for (j,other) in self.scenario.actors.iter().enumerate() {
                    if other.side==Side::Enemy && other.id==id {
                        self.state.actors[j].revealed_weapon_weaknesses=known.weapons;
                        self.state.actors[j].revealed_element_weaknesses=known.elements;
                    }
                }
                for (before,after) in spans {
                    if let Some(trace)=self.state.opening_crt.as_mut().and_then(|c|c.trace.as_mut()) {
                        trace.push(RngSpan {reason:"study_foe_weakness",actor:Some(ActorId(i)),before,after});
                    }
                }
            }
        }
        if first && (self.state.opening_crt.as_ref().is_some_and(|c|c.lord_sands)
            || self.scenario.night_buff_turns>0 && self.actor_id("temenos").is_some()) {
            // ExecSpecialAction_DebuffInNight calls AddDisease directly, no lottery.
            for (i,state) in self.state.actors.iter_mut().enumerate() {
                if self.scenario.actors[i].side==Side::Enemy {
                    state.statuses.extend([Status {kind:StatusKind::DefenseDown,turns:3},Status {kind:StatusKind::MagicDefenseDown,turns:3},Status {kind:StatusKind::Blind,turns:1}]);
                }
            }
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
        if first && self.state.opening_crt.as_ref().is_some_and(|c|
            c.step_ahead && c.mansion_situation == Some(crate::escape::Situation::Normal)) {
            // BattleManagerBP.AddBattleFirstActionCharacter accepts only
            // BattleStartType 0 (normal). Preemptive starts keep their initial
            // player-only queue and preview an ordinary all-actor next turn.
            // SetBattleFirstActionTurn rebuilds, sorts and shuffles current
            // player orders before UpdateNextOrder. Never filter the old queue.
            let mut bonus:Vec<_>=self.state.actors.iter().enumerate()
                .filter(|(i,a)|a.alive()&&self.scenario.actors[*i].side==Side::Player&&self.state.opening_crt.as_ref().unwrap().step_ahead_members.as_ref().is_none_or(|ids|ids.contains(i)))
                .map(|(i,_)|ActorId(i)).collect();
            combat::sort_order(&mut bonus,|id|self.scenario.actors[id.0].stats.speed);
            let before=self.state.rng;
            combat::shuffle_order(&mut bonus, |_|true, &mut self.state.rng);
            self.trace_rng("step_ahead_current_order_shuffle",None,before);
            self.state.queue=bonus;
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

