//! Scalar CRT callbacks for the opening routine. No audio or presentation runtime.
use crate::{ActorId, CrtRng, combat::Weapon};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RngSpan {
    pub reason: &'static str,
    pub actor: Option<ActorId>,
    pub before: CrtRng,
    pub after: CrtRng,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpeningCrtState {
    /// Candidate indices removed by the original PlayOnce branch, per player.
    pub used_voices: [u64; 3],
    pub settled_damage: [bool; 3],
    pub trace: Option<Vec<RngSpan>>,
    pub after_battle: Option<AfterBattle>,
}
impl OpeningCrtState {
    pub(crate) fn new(trace: bool) -> Self {
        Self {
            used_voices: [0; 3],
            settled_damage: [false; 3],
            trace: trace.then(Vec::new),
            after_battle: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AfterBattle {
    pub rng_before_rewards: CrtRng,
    pub rng_after_rewards: CrtRng,
    pub rng_after_field_reset: CrtRng,
    pub healing_grapes: u32,
    pub encounter_steps: u32,
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AttackStep {
    Check,
    Voice { r#type: u8 },
    Damage { last: bool },
}
#[derive(Debug, Deserialize)]
struct VoiceCandidate {
    id: String,
    r#type: u8,
    rate: u32,
    once: bool,
    border: u32,
    self_only: bool,
    target: Option<String>,
    counter: bool,
    decondition: bool,
    pinch: bool,
    just_break: bool,
}
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct VoiceCondition {
    pub border: u32,
    pub pinch: bool,
    pub decondition: bool,
}
#[derive(Debug, Deserialize)]
struct WeaponPlans {
    sword: [Vec<AttackStep>; 4],
    dagger: [Vec<AttackStep>; 4],
    axe: [Vec<AttackStep>; 4],
}
#[derive(Debug, Deserialize)]
pub struct EncounterCounter {
    pub min: u32,
    pub range_a: u32,
    pub range_b: u32,
    pub first_ratio: f32,
}
impl EncounterCounter {
    /// EncountManagerBP.CalcEncountStepCount: B is evaluated before A.
    pub fn reset(&self, rng: &mut CrtRng, first_encounter: bool) -> u32 {
        let b = rng.index(self.range_b);
        let a = rng.index(self.range_a);
        let value = (self.min + a + b) as f32;
        (value
            * if first_encounter {
                self.first_ratio
            } else {
                1.0
            }) as u32
    }
}
#[derive(Debug, Deserialize)]
pub(crate) struct OpeningData {
    voices: [Vec<VoiceCandidate>; 3],
    attacks: WeaponPlans,
    pub encounter: EncounterCounter,
    pub drop_rate: u32,
    pub victory_voice: bool,
}
pub(crate) fn data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| {
        let d: OpeningData = serde_json::from_str(include_str!("../data/opening_crt.json"))
            .expect("checked-in source projection");
        assert!(d.voices.iter().all(|rows| rows.len() <= 64));
        for plans in [&d.attacks.sword, &d.attacks.dagger, &d.attacks.axe] {
            assert!(
                plans
                    .iter()
                    .all(|p| matches!(p.last(), Some(AttackStep::Damage { last: true })))
            );
        }
        d
    })
}

pub(crate) fn attack_plan(weapon: Weapon, boost: u8) -> &'static [AttackStep] {
    let plans = &data().attacks;
    match weapon {
        Weapon::Sword => &plans.sword[boost as usize],
        Weapon::Dagger => &plans.dagger[boost as usize],
        Weapon::Axe => &plans.axe[boost as usize],
        _ => unreachable!("validated opening weapon"),
    }
}
/// PlayBattleVoice rolls the rate BEFORE checking the candidate's conditions.
/// Only this encounter's three party members and ordinary routine types enter.
pub(crate) fn select_voice(
    actor: usize,
    voice_type: u8,
    condition: VoiceCondition,
    used: &mut u64,
    rng: &mut CrtRng,
) -> Option<&'static str> {
    for (i, row) in data().voices[actor].iter().enumerate() {
        if row.r#type != voice_type || *used & (1 << i) != 0 {
            continue;
        }
        if !rng.weighted_bool(row.rate as f32 * 0.01_f32) {
            continue;
        }
        let eligible = match voice_type {
            3 => {
                // Opening guests have no matching Throné ally-reaction rows.
                row.target.is_none()
                    && !row.just_break
                    && (!row.self_only || condition.pinch || condition.decondition)
                    && (!row.pinch || condition.pinch)
                    && (!row.decondition || condition.decondition)
            }
            4 => {
                !row.counter
                    && !row.decondition
                    && (row.border == 0 || row.border >= condition.border)
            }
            8 => row.border == 0 || row.border >= condition.border,
            _ => true,
        };
        if eligible {
            if row.once {
                *used |= 1 << i;
            }
            return Some(&row.id);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_candidate_order_rolls_ineligible_rows_and_removes_play_once() {
        let mut rng = CrtRng::new(123);
        let mut used = 0;
        assert_eq!(
            select_voice(0, 3, VoiceCondition::default(), &mut used, &mut rng),
            Some("THIEF_eORDER_START_COMMON")
        );
        assert_eq!(rng.draws, 17); // Same traversal pattern as captured traveler order-start voices.
        let mut used = 0;
        for (turn, expected) in [
            (1, "GUEST_006_eORDER_START_TURN_1"),
            (2, "GUEST_006_eORDER_START_TURN_2"),
        ] {
            assert_eq!(
                select_voice(1, 3, VoiceCondition::default(), &mut used, &mut rng),
                Some(expected)
            );
            assert_eq!(rng.draws, 17 + 3 * turn);
        }
        let before = rng.draws;
        for level in 1..=2 {
            select_voice(
                1,
                8,
                VoiceCondition {
                    border: level,
                    ..Default::default()
                },
                &mut used,
                &mut rng,
            );
        }
        assert_eq!(rng.draws - before, 3); // Two inputs: Lv1 rolls once, Lv2 twice.
        let before = rng.draws;
        select_voice(
            1,
            4,
            VoiceCondition {
                border: 10,
                ..Default::default()
            },
            &mut used,
            &mut rng,
        );
        assert_eq!(rng.draws - before, 3); // counter, debuff, then small damage
    }

    #[test]
    fn encounter_reset_uses_b_then_a_and_keeps_first_encounter_ratio() {
        let counter = EncounterCounter {
            min: 1000,
            range_a: 400,
            range_b: 17,
            first_ratio: 0.5,
        };
        let mut rng = CrtRng::new(123);
        // Independent CRT outputs 440 and 19053: floor(440/32767*17)=0,
        // floor(19053/32767*400)=232. Reversing the calls gives a different sum.
        assert_eq!(counter.reset(&mut rng, true), 616);
        assert_eq!(
            rng,
            CrtRng {
                state: 3_396_176_589,
                draws: 2
            }
        );
        let zero = EncounterCounter {
            min: 1000,
            range_a: 0,
            range_b: 0,
            first_ratio: 0.5,
        };
        let before = rng;
        assert_eq!(zero.reset(&mut rng, false), 1000);
        assert_eq!(rng, before);
    }
}
