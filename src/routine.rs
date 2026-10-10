//! The supplied two-round strategy, independent of shuffled party order.
use crate::{ActorId, Battle, Command, Outcome, Result, Side, StepReport, ensure};
use serde::Serialize;

/// Fixed targets by screen position, executed on each actor's actual turn.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ThroneSecondRoutine;
impl ThroneSecondRoutine {
    pub const ID: &'static str = "throne_second_darkest_night";
    pub fn step(&mut self, battle: &mut Battle) -> Result<StepReport> {
        let actor = battle
            .current_actor()
            .ok_or_else(|| crate::SimError("battle is already over".into()))?;
        if battle.side(actor) == Some(Side::Enemy) {
            battle.step_enemy()
        } else {
            battle.step_player(self.command(battle)?)
        }
    }
    pub fn command(&self, battle: &Battle) -> Result<Command> {
        let actor = battle
            .current_actor()
            .ok_or_else(|| crate::SimError("battle is already over".into()))?;
        ensure(
            battle.side(actor) == Some(Side::Player),
            "it is an enemy turn",
        )?;
        let round = battle.state().round;
        ensure(
            round <= 2,
            "second-fight routine did not win within two rounds",
        )?;
        let (skill, target) = if battle.actor_id("throne") == Some(actor) {
            (
                "ABI_SET_THI_020",
                if round == 1 { "grunt_right" } else { "leader" },
            )
        } else if battle.actor_id("pirro") == Some(actor) {
            if round == 1 {
                ("ABI_SET_GST_0190", "leader")
            } else {
                ("ABI_SET_GST_0170", "grunt_left")
            }
        } else if battle.actor_id("scaracci") == Some(actor) {
            (
                "attack_axe",
                if round == 1 {
                    "grunt_left"
                } else {
                    "grunt_right"
                },
            )
        } else {
            return Err(crate::SimError(
                "second-fight routine requires Throné, Pirro and Scaracci".into(),
            ));
        };
        let target = battle
            .actor_id(target)
            .ok_or_else(|| crate::SimError("routine requires the second sewer formation".into()))?;
        Ok(Command::Boost {
            id: skill.into(),
            target: Some(target),
            bp: if round == 1 { 0 } else { 2 },
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ThroneOpeningRoutine {
    pub first_round: Vec<ActorId>,
    pub second_round_attackers: Vec<ActorId>,
    pub second_round_defended: bool,
}
impl ThroneOpeningRoutine {
    pub const ID: &'static str = "first_actor_boosts_enemy_one";

    pub fn step(&mut self, battle: &mut Battle) -> Result<StepReport> {
        self.step_inputs(battle, None)
    }
    pub fn step_with_boost_inputs(
        &mut self,
        battle: &mut Battle,
        levels: &[u8],
    ) -> Result<StepReport> {
        self.step_inputs(battle, Some(levels))
    }
    fn step_inputs(&mut self, battle: &mut Battle, levels: Option<&[u8]>) -> Result<StepReport> {
        let actor = battle
            .current_actor()
            .ok_or_else(|| crate::SimError("battle is already over".into()))?;
        if battle.side(actor) == Some(Side::Enemy) {
            ensure(levels.is_none(), "boost inputs require a player menu")?;
            return battle.step_enemy();
        }
        let command = self.command(battle)?;
        let round = battle.state().round;
        let is_defend = matches!(command, Command::Defend);
        let report = match levels {
            Some(levels) => battle.step_player_with_boost_inputs(command, levels)?,
            None => battle.step_player(command)?,
        };
        // Failed commands must not advance either battle state or policy state.
        if round == 1 {
            self.first_round.push(actor);
        } else if is_defend {
            self.second_round_defended = true;
        } else {
            self.second_round_attackers.push(actor);
        }
        Ok(report)
    }
    /// Inspect the next player action without changing the policy or battle.
    pub fn command(&self, battle: &Battle) -> Result<Command> {
        ensure(
            battle.state().outcome == Outcome::Ongoing,
            "battle is already over",
        )?;
        let actor = battle.current_actor().expect("ongoing order");
        ensure(
            battle.side(actor) == Some(Side::Player),
            "it is an enemy turn",
        )?;
        let round = battle.state().round;
        // PMT_TYPE_2_001: slot A is at X=-100 (right), B at X=-700 (left).
        // Preserve actor/initialization order; number targets right to left.
        let enemy_one = battle
            .actor_id("pursuer_a")
            .ok_or_else(|| crate::SimError("routine requires the opening formation".into()))?;
        let enemy_two = battle
            .actor_id("pursuer_b")
            .ok_or_else(|| crate::SimError("routine requires the opening formation".into()))?;
        let is_defend = round == 2 && self.first_round.first() == Some(&actor);
        let command = if round == 1 {
            ensure(
                self.first_round.len() < 3 && !self.first_round.contains(&actor),
                "unexpected repeated first-round actor",
            )?;
            let first = self.first_round.is_empty();
            Command::Boost {
                id: battle.default_attack(actor)?.into(),
                target: Some(if first { enemy_one } else { enemy_two }),
                bp: u8::from(first),
            }
        } else {
            ensure(
                round == 2 && self.first_round.len() == 3,
                "routine exceeded its two-round plan",
            )?;
            if is_defend {
                ensure(
                    !self.second_round_defended,
                    "unexpected repeated Defend turn",
                )?;
                Command::Defend
            } else {
                ensure(
                    self.second_round_attackers.len() < 2
                        && !self.second_round_attackers.contains(&actor),
                    "routine exhausted without victory",
                )?;
                Command::Boost {
                    id: battle.default_attack(actor)?.into(),
                    target: Some(if self.second_round_attackers.is_empty() {
                        enemy_one
                    } else {
                        enemy_two
                    }),
                    bp: 2,
                }
            }
        };
        Ok(command)
    }
}
