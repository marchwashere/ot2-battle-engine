//! Bounded first Mug of Cape Cold's valuables-carrying man, original-gear Lv1 Osvald.
//! Entry is after field/menu/loading callbacks; exit is after victory/drop callbacks.
use crate::{
    ActorId, Battle, BattleConfig, Command, CrtRng, Outcome, Result, Side, StepReport, ensure,
    opening_crt::RngSpan,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Menu {
    pub round: u32,
    pub rng: CrtRng,
}
#[derive(Debug, Serialize)]
pub struct Prediction {
    pub enemy_knowledge: crate::knowledge::EnemyHistory,
    pub before: CrtRng,
    pub after: CrtRng,
    pub menus: Vec<Menu>,
    pub steps: Vec<StepReport>,
    pub trace: Vec<RngSpan>,
    pub survived_round_two: bool,
    pub outcome: Outcome,
}

pub fn predict(before: CrtRng) -> Result<Prediction> {
    predict_with_boost_inputs(before, None)
}

/// Replace round two's automatic boosts with explicit levels ending at 2 BP.
/// Every increase executes the ordinary core boost callbacks; decreases are free.
pub fn predict_with_boost_inputs(before: CrtRng, round_two: Option<&[u8]>) -> Result<Prediction> {
    predict_with_history(before,round_two,&Default::default())
}
pub fn predict_with_history(before:CrtRng,round_two:Option<&[u8]>,history:&crate::knowledge::EnemyHistory)->Result<Prediction> {
    let mut config = BattleConfig::from_json(include_str!("../data/cape_cold_mug.json"))?;
    config.seed = before.state;
    config.enemy_knowledge=history.clone();
    let mut b = Battle::new_cape_cold_mug(config, true)?;
    let mut menus = Vec::new();
    let mut steps = Vec::new();
    let mut survived_round_two = false;
    while let Some(actor) = b.current_actor() {
        ensure(
            b.state().round <= 3,
            "Mug continuation beyond the supplied routine is unsupported",
        )?;
        let mut report = if b.side(actor) == Some(Side::Enemy) {
            b.step_enemy()?
        } else {
            let mut rng = b.state().rng;
            rng.draws += before.draws;
            let round = b.state().round;
            menus.push(Menu { round, rng });
            let command = match round {
                1 => Command::Skill {
                    id: "ABI_SET_SCH_020".into(),
                    target: None,
                },
                2 => Command::Boost {
                    id: "ABI_SET_SCH_010".into(),
                    target: None,
                    bp: 2,
                },
                _ => Command::Skill {
                    id: "attack_staff".into(),
                    target: Some(ActorId(1)),
                },
            };
            let report = if round == 2 {
                match round_two {
                    Some(path) => b.step_player_with_boost_inputs(command, path)?,
                    None => b.step_player(command)?,
                }
            } else { b.step_player(command)? };
            if round == 2 {
                survived_round_two = b.state().actors[1].alive();
            }
            report
        };
        report.rng_before.draws += before.draws;
        report.rng_after.draws += before.draws;
        steps.push(report);
    }
    let mut trace = b
        .state()
        .opening_crt
        .as_ref()
        .unwrap()
        .trace
        .clone()
        .unwrap();
    for span in &mut trace {
        span.before.draws += before.draws;
        span.after.draws += before.draws;
    }
    let mut after = b.state().rng;
    after.draws += before.draws;
    Ok(Prediction {
        enemy_knowledge:b.enemy_knowledge(),
        before,
        after,
        menus,
        steps,
        trace,
        survived_round_two,
        outcome: b.state().outcome,
    })
}
