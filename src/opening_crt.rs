//! Scalar CRT callbacks for the opening routine. No audio or presentation runtime.
use crate::{ActorId, CrtRng, combat::Weapon};
use serde::{Deserialize, Serialize};

pub(crate) fn initialize_enemy_rng(
    rng: &mut CrtRng,
    bp_base: u8,
    init: crate::combat::EnemyInitialization,
) -> (u8, u16) {
    // FinishCallbackLoaded skips synchronized actors. Native RandomInteger(0)
    // returns 0 without rand(); a positive frame count, even 1, consumes it.
    if !init.sync_to_parent && !init.sync_children {
        rng.index(init.initial_idle_frames);
    }
    let max_bp = bp_base + rng.index(3) as u8;
    (max_bp, rng.next_u15()) // initial timing, including its fixed range
}
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RngSpan {
    pub reason: &'static str,
    pub actor: Option<ActorId>,
    pub before: CrtRng,
    pub after: CrtRng,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpeningCrtState {
    /// Shares scalar callback machinery with the second sewer encounter.
    pub second_battle: bool,
    pub pirro_battle: bool,
    pub cape_cold_mug: bool,
    /// Narrow field profile, entering after the party timing draw.
    pub mansion_situation: Option<crate::escape::Situation>,
    pub field_break: bool,
    pub field_item: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub step_ahead: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub hikari_recruitment: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carried_voices: Option<Vec<usize>>,
    /// Candidate indices removed by the original PlayOnce branch, per player.
    pub used_voices: Vec<u64>,
    pub settled_damage: Vec<bool>,
    pub trace: Option<Vec<RngSpan>>,
    pub after_battle: Option<AfterBattle>,
    pub rewards: Option<BattleRewards>,
}
impl OpeningCrtState {
    pub(crate) fn new(trace: bool) -> Self {
        Self {
            second_battle: false,
            pirro_battle: false,
            cape_cold_mug: false,
            mansion_situation: None,
            field_break: false,
            field_item: false,
            step_ahead: false,
            hikari_recruitment: false,
            carried_voices: None,
            used_voices: vec![0; 3],
            settled_damage: vec![false; 3],
            trace: trace.then(Vec::new),
            after_battle: None,
            rewards: None,
        }
    }
}
fn is_false(value: &bool) -> bool { !*value }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BattleRewards {
    pub before: CrtRng,
    pub after: CrtRng,
    pub items: BTreeMap<String, u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AfterBattle {
    pub rng_before_rewards: CrtRng,
    pub rng_after_rewards: CrtRng,
    pub rng_after_field_reset: CrtRng,
    pub rng_after_post_battle_event: CrtRng,
    pub healing_grapes: u32,
    pub encounter_steps_after_field_reset: u32,
    pub encounter_steps: u32,
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AttackStep {
    Check {
        tick: u32,
    },
    Voice {
        tick: u32,
        r#type: u8,
    },
    Damage {
        tick: u32,
        last: bool,
        #[serde(default)]
        effect: Option<usize>,
    },
}
impl AttackStep {
    pub(crate) fn tick(self) -> u32 {
        match self {
            Self::Check { tick } | Self::Voice { tick, .. } | Self::Damage { tick, .. } => tick,
        }
    }
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
    #[serde(default)]
    meat_only: bool,
    #[serde(default)]
    ability: Option<String>,
}
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct VoiceCondition {
    pub border: u32,
    pub pinch: bool,
    pub decondition: bool,
    pub ability: Option<&'static str>,
    pub target_self: bool,
    pub ally_breaker: Option<&'static str>,
    pub recovery_enforcer: Option<&'static str>,
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
pub(crate) struct LatentData {
    pub initial: u16,
    pub maximum: u16,
}
#[derive(Debug, Deserialize)]
pub(crate) struct OpeningData {
    voices: Vec<Vec<VoiceCandidate>>,
    attacks: WeaponPlans,
    pub break_timer_delay_ticks: u32,
    pub break_busy_ticks: u32,
    pub post_event_encounter_reset: bool,
    pub encounter: EncounterCounter,
    pub drop_rate: u32,
    pub victory_voice: bool,
    /// EnemyData.DeadEffectSlow -> PlayEnemyDeadCamera's CRT timer.
    #[serde(default)]
    pub enemy_death_slow: bool,
    pub battle_start_voice: bool,
    /// Imported TalkPlayerID resolved to this profile's actor ID. A living
    /// designated speaker bypasses the random-party-speaker draw.
    #[serde(default)]
    pub battle_start_speaker: Option<String>,
    #[serde(default)]
    pub latent: Option<LatentData>,
    #[serde(default)]
    pub skills: BTreeMap<String, Vec<AttackStep>>,
    /// Scalar PlayFinishBlow gate only; no camera/presentation is executed.
    /// Items use GetActionCommandForItem's override, not the USE_ITEM row.
    #[serde(default)]
    pub skill_uses_ability_camera: BTreeMap<String, bool>,
    #[serde(default)]
    pub skill_voice_duration_ticks: BTreeMap<String, u32>,
    #[serde(default)]
    pub drops: Vec<Drop>,
    #[serde(default)]
    pub ability_voice_ids: BTreeMap<String, String>,
    #[serde(default)]
    pub staff_attack: Vec<AttackStep>,
    #[serde(default)]
    pub normal_attacks: BTreeMap<String, Vec<AttackStep>>,
    #[serde(default)]
    pub player_latent_maxima: Vec<u16>,
}
pub(crate) fn hikari_recruitment_data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| {
        let d: OpeningData = serde_json::from_str(include_str!("../data/hikari_recruitment_crt.json"))
            .expect("imported Hikari recruitment callbacks");
        assert_eq!(d.voices.len(), 4);
        assert!(d.voices.iter().all(|rows| rows.len() <= 64));
        d
    })
}
pub(crate) fn field_carried_data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(include_str!("../data/field_carried_crt.json"))
        .expect("imported carried field voices"))
}
pub(crate) fn carried_field_item_data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(include_str!("../data/carried_field_item_crt.json")).expect("carried item callbacks"))
}
pub(crate) fn gravell_data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(include_str!("../data/gravell_crt.json")).expect("Gravell callbacks"))
}
pub(crate) fn mug_data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/cape_cold_mug_crt.json"))
            .expect("imported Mug callbacks")
    })
}
pub(crate) fn field_party_data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(include_str!("../data/field_party_crt.json"))
        .expect("imported Throné/Osvald field voices"))
}

pub(crate) fn field_rare_data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(include_str!("../data/field_rare_crt.json")).expect("field rare callbacks"))
}
pub(crate) fn pirro_data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/throne_pirro_crt.json"))
            .expect("imported Pirro callback data")
    })
}
#[derive(Debug, Deserialize)]
pub(crate) struct Drop {
    pub rate: u32,
    pub item: String,
}
pub(crate) fn second_data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| {
        let d: OpeningData = serde_json::from_str(include_str!("../data/throne_second_crt.json"))
            .expect("imported second-fight callback data");
        assert!(d.voices.iter().all(|rows| rows.len() <= 64));
        d
    })
}
pub(crate) fn mansion_data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/mansion_escape_crt.json"))
            .expect("imported mansion callback data")
    })
}
pub(crate) fn field_break_data() -> &'static OpeningData {
    static DATA: OnceLock<OpeningData> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/field_break_crt.json"))
            .expect("imported field break callbacks")
    })
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
                    .all(|p| matches!(p.last(), Some(AttackStep::Damage { last: true, .. })))
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
#[cfg(test)]
pub(crate) fn select_voice(
    actor: usize,
    voice_type: u8,
    condition: VoiceCondition,
    used: &mut u64,
    rng: &mut CrtRng,
) -> Option<&'static str> {
    select_voice_from(data(), actor, voice_type, condition, used, rng)
}
pub(crate) fn select_voice_from(
    data: &'static OpeningData,
    actor: usize,
    voice_type: u8,
    condition: VoiceCondition,
    used: &mut u64,
    rng: &mut CrtRng,
) -> Option<&'static str> {
    for (i, row) in data.voices[actor].iter().enumerate() {
        if row.r#type != voice_type || *used & (1 << i) != 0 {
            continue;
        }
        // Ability voices are keyed by ability ID before traversing candidates;
        // boosted variants fall back to their _01 row.
        if voice_type == 18 && row.ability.as_deref() != condition.ability {
            continue;
        }
        if !rng.weighted_bool(row.rate as f32 * 0.01_f32) {
            continue;
        }
        let eligible = match voice_type {
            3 => {
                // Opening guests have no matching Throné ally-reaction rows.
                (if row.just_break {row.target.as_deref().is_some_and(|target|Some(target)==condition.ally_breaker)} else {row.target.is_none()})
                    && (!row.self_only || condition.pinch || condition.decondition)
                    && (!row.pinch || condition.pinch)
                    && (!row.decondition || condition.decondition)
            }
            4 => {
                !row.counter
                    && !row.decondition
                    && (row.border == 0 || row.border >= condition.border)
            }
            8 | 18 => row.border == 0 || row.border >= condition.border,
            15 => !row.self_only || condition.target_self,
            13 | 14 => !row.meat_only && (row.target.is_none() || row.target.as_deref()==condition.recovery_enforcer),
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
    fn supported_field_spells_have_eligible_voice_rows() {
        for (data,actor,id) in [(field_break_data(),0,"ABI_THI_020_01"),
            (field_party_data(),0,"ABI_THI_020_01"),(field_party_data(),1,"ABI_SCH_010_01"),
            (field_party_data(),1,"ABI_SCH_020_01")] {
            let mut rng=CrtRng::new(0);let mut used=0;
            let selected=select_voice_from(data,actor,18,VoiceCondition {ability:Some(id),..Default::default()},&mut used,&mut rng);
            assert!(selected.is_some(),"missing ability voice {id}");
            assert_eq!(rng.draws,1);
            assert!(data.voices[actor].len()<=64);
        }
    }
    #[test]
    fn enemy_resource_and_sync_gates_preserve_bp_and_timing_draws() {
        for frames in [0, 1, 24] {
            for sync_to_parent in [false, true] {
                for sync_children in [false, true] {
                    let init = crate::combat::EnemyInitialization {
                        initial_idle_frames: frames, sync_to_parent, sync_children,
                    };
                    let mut rng = CrtRng::new(0);
                    let mut expected = rng;
                    if frames > 0 && !sync_to_parent && !sync_children {
                        expected.next_u15();
                    }
                    let bp = 3 + expected.index(3) as u8;
                    let timing = expected.next_u15();
                    assert_eq!(initialize_enemy_rng(&mut rng, 3, init), (bp, timing));
                    assert_eq!(rng, expected);
                }
            }
        }
    }
    #[test]
    fn all_opening_boost_levels_have_fixed_draw_cost_without_play_once() {
        for actor in 0..3 {
            let mut rng = CrtRng::new(0);
            let mut used = 0;
            for _ in 0..4 {
                for level in 1..=3 {
                    let before = rng.draws;
                    select_voice(
                        actor,
                        8,
                        VoiceCondition {
                            border: level,
                            ..Default::default()
                        },
                        &mut used,
                        &mut rng,
                    );
                    assert_eq!(rng.draws - before, u64::from(level));
                    assert_eq!(used, 0);
                }
            }
        }
    }

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
