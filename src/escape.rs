//! First-attempt Escape from the mansion's fixed first encounter. This narrow
//! adapter exposes no damaging player actions or continuation after failure.
//! Field selection and post-battle counter resets belong to the route caller.
use crate::{
    ActorId, Command, Outcome,
    Battle, BattleConfig, BattleState, CrtRng, Resources, Result, Side, StepReport, ensure,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Situation {
    Normal,
    Preemptive,
    Surprise,
}

/// LibBattle.CheckEscapeSuccess divides the SUM by four, even for a solo party.
/// Attempt 0 is the first attempt. Incapacitated-enemy/preemptive bypasses are
/// evaluated by the caller before invoking this probability lottery.
pub fn success_percent(
    levels: &[u32],
    danger: u32,
    failed_attempts: u32,
    support_bonus: u32,
) -> Result<u32> {
    ensure(
        !levels.is_empty() && levels.len() <= 4 && levels.iter().all(|l| (1..=99).contains(l)),
        "Escape needs 1..4 valid party levels",
    )?;
    ensure(
        danger <= 99 && support_bonus <= 100,
        "Unsupported Escape rate inputs",
    )?;
    let average = (levels.iter().sum::<u32>() / 4).max(1) as i32;
    let index = ((average - danger as i32).clamp(-10, 10) + 10) as usize;
    let level_rates = [
        10, 25, 25, 25, 25, 40, 40, 40, 55, 55, 55, 55, 55, 70, 70, 70, 85, 85, 85, 85, 100,
    ];
    Ok(level_rates[index] + [0, 15, 30, 100][failed_attempts.min(3) as usize] + support_bonus)
}

#[derive(Debug, Clone)]
pub struct FirstEscapeBattle {
    battle: Battle,
    situation: Situation,
    pub enemy_actions_before_menu: Vec<StepReport>,
}
#[derive(Debug, Serialize)]
pub struct Attempt {
    pub success: bool,
    pub guaranteed: bool,
    pub chance_percent: u32,
    pub menu: CrtRng,
    pub before_check: CrtRng,
    pub after: CrtRng,
    pub state: BattleState,
}
impl FirstEscapeBattle {
    /// Replay a 1 BP dagger break from the actual mansion command menu, then
    /// guaranteed Escape on the next menu. Reject missed/incomplete breaks.
    pub fn break_menu(mut self, inputs: &[u8]) -> Result<Option<Self>> {
        self.battle.step_player_with_boost_inputs(Command::Boost {
            id:"attack_dagger".into(), target:Some(ActorId(1)), bp:1,
        }, inputs)?;
        if self.battle.state().outcome != Outcome::Ongoing || !self.battle.state().actors[1].broken() {
            return Ok(None);
        }
        ensure(self.battle.current_actor() == Some(ActorId(0)), "Broken guard must yield the player menu")?;
        Ok(Some(self))
    }
    pub fn attempt_broken(mut self, inputs:&[u8]) -> Result<Attempt> {
        ensure(self.battle.state().actors[1].broken(),"Guaranteed Escape needs a broken guard")?;
        let menu=self.battle.state().rng;
        if !inputs.is_empty() {self.battle.boost_sequence(inputs)?;}
        let before_check = self.battle.state().rng;
        let success = self.battle.escape_check(None);
        Ok(Attempt { success, guaranteed:true, chance_percent:100, menu,
            before_check, after:self.battle.state().rng, state:self.battle.state().clone() })
    }
    pub fn break_then_escape(self, inputs:&[u8]) -> Result<Option<Attempt>> {
        self.break_menu(inputs)?.map(|menu|menu.attempt_broken(&[])).transpose()
    }
    /// Seed boundary: enemy composition finalized, player timing ALREADY drawn.
    /// Level two, original loadout. Night talent is unlocked by event 1600.
    pub fn mansion(
        seed: u32,
        situation: Situation,
        resources: Resources,
        trace: bool,
    ) -> Result<Self> {
        let mut config = BattleConfig::from_json(include_str!("../data/mansion_escape.json"))?;
        config.seed = seed;
        config.players[0].resources = Some(Resources { bp: 1, ..resources });
        let mut battle = Battle::new_mansion_escape(config, situation, trace)?;
        let mut enemy_actions_before_menu = Vec::new();
        while battle
            .current_actor()
            .is_some_and(|a| battle.side(a) == Some(Side::Enemy))
        {
            ensure(
                enemy_actions_before_menu.len() < 2,
                "First Escape menu was not reached",
            )?;
            let a = &battle.state().actors[1];
            ensure(
                a.resources.bp < a.enemy_charge_max && a.resources.hp == 217,
                "Unsupported guard action before Escape",
            )?;
            enemy_actions_before_menu.push(battle.step_enemy()?);
        }
        ensure(
            battle.current_actor().is_some(),
            "Throne cannot reach the first Escape menu",
        )?;
        Ok(Self {
            battle,
            situation,
            enemy_actions_before_menu,
        })
    }
    pub fn state(&self) -> &BattleState {
        self.battle.state()
    }
    /// Consumes this snapshot. A failed result stops at the check and must not
    /// be mistaken for a field return or a fully simulated failed turn.
    pub fn attempt(mut self, inputs: &[u8], danger: u32) -> Result<Attempt> {
        let menu = self.battle.state().rng;
        if !inputs.is_empty() {
            self.battle.boost_sequence(inputs)?;
        }
        let before_check = self.battle.state().rng;
        let guaranteed = self.situation == Situation::Preemptive && self.battle.state().round == 1;
        let chance_percent = if guaranteed {
            100
        } else {
            success_percent(&[2], danger, 0, 0)?
        };
        let success = self
            .battle
            .escape_check((!guaranteed).then_some(chance_percent as f32 * 0.01_f32));
        Ok(Attempt {
            success,
            guaranteed,
            chance_percent,
            menu,
            before_check,
            after: self.battle.state().rng,
            state: self.battle.state().clone(),
        })
    }
}
