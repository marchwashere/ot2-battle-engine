//! The supplied two-round strategy, independent of shuffled party order.
use crate::{ActorId, Battle, Command, Outcome, Result, Side, StepReport, ensure};
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ThroneOpeningRoutine {
    pub first_round: Vec<ActorId>,
    pub second_round_attackers: Vec<ActorId>,
    pub second_round_defended: bool,
}
impl ThroneOpeningRoutine {
    pub fn step(&mut self, battle: &mut Battle) -> Result<StepReport> {
        ensure(
            battle.state().outcome == Outcome::Ongoing,
            "battle is already over",
        )?;
        let actor = battle.current_actor().expect("ongoing order");
        if battle.side(actor) == Some(Side::Enemy) {
            return battle.step_enemy();
        }
        let round = battle.state().round;
        let enemy_one = battle
            .actor_id("pursuer_a")
            .ok_or_else(|| crate::SimError("routine requires the opening formation".into()))?;
        let enemy_two = battle
            .actor_id("pursuer_b")
            .ok_or_else(|| crate::SimError("routine requires the opening formation".into()))?;
        let is_defend = round == 2 && self.first_round.get(1) == Some(&actor);
        let command = if round == 1 {
            ensure(
                self.first_round.len() < 3 && !self.first_round.contains(&actor),
                "unexpected repeated first-round actor",
            )?;
            let second = self.first_round.len() == 1;
            Command::Boost {
                id: battle.default_attack(actor)?.into(),
                target: Some(if second { enemy_two } else { enemy_one }),
                bp: u8::from(second),
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
        let report = battle.step_player(command)?;
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
}
