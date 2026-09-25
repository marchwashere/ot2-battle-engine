//! Source-derived combat arithmetic for the opening encounter. No presentation code.
use crate::{CrtRng, Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageKind {
    Physical,
    Magical,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weapon {
    Sword,
    Lance,
    Dagger,
    Axe,
    Bow,
    Staff,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    Fire,
    Ice,
    Lightning,
    Wind,
    Light,
    Dark,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusKind {
    AttackUp,
    MagicUp,
    SpeedUp,
    DefenseDown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub kind: StatusKind,
    pub turns: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponStats {
    pub weapon: Weapon,
    pub attack: u32,
    pub magic: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatProfile {
    pub attack: u32,
    pub defense: u32,
    pub magic: u32,
    pub magic_defense: u32,
    pub accuracy: u32,
    pub evasion: u32,
    pub critical: u32,
    pub growth_attack: u32,
    pub growth_magic: u32,
    #[serde(default)]
    pub weapons: Vec<WeaponStats>,
    pub main_weapon: Option<Weapon>,
    #[serde(default)]
    pub shields: u8,
    #[serde(default)]
    pub weapon_weaknesses: Vec<Weapon>,
    #[serde(default)]
    pub element_weaknesses: Vec<Element>,
    #[serde(default)]
    pub statuses: Vec<Status>,
    pub steal_item: Option<String>,
    #[serde(default)]
    pub enemy_bp_base: u8,
}
impl CombatProfile {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure(
            [
                self.attack,
                self.defense,
                self.magic,
                self.magic_defense,
                self.accuracy,
                self.evasion,
                self.critical,
                self.growth_attack,
                self.growth_magic,
            ]
            .iter()
            .all(|v| *v <= 999),
            "combat stat exceeds supported 999 cap",
        )?;
        ensure(self.shields <= 99, "shield cap exceeds 99")?;
        ensure(
            self.enemy_bp_base <= 3,
            "enemy BP base exceeds supported cap",
        )?;
        ensure(
            self.statuses.iter().enumerate().all(|(i, s)| {
                s.turns > 0
                    && s.turns <= 9
                    && !self.statuses[..i].iter().any(|old| old.kind == s.kind)
            }),
            "status duration must be 1..9 and each kind unique",
        )?;
        ensure(
            self.weapons.iter().enumerate().all(|(i, w)| {
                w.attack <= 999
                    && w.magic <= 999
                    && !self.weapons[..i].iter().any(|v| v.weapon == w.weapon)
            }),
            "invalid/duplicate weapon stats",
        )?;
        ensure(
            self.main_weapon
                .is_none_or(|w| self.weapons.iter().any(|v| v.weapon == w)),
            "main weapon is not equipped",
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attack {
    pub kind: DamageKind,
    pub weapon: Option<Weapon>,
    pub element: Option<Element>,
    pub invocation: u32,
    pub ability_ratio: u32,
    pub hit_bonus: i32,
    pub critical_bonus: i32,
    pub critical_power: u32,
    /// NormalAttackCommand checks accuracy before the damage pipeline's critical check.
    #[serde(default)]
    pub normal_attack: bool,
}
impl Attack {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure(
            !self.normal_attack
                || (self.kind == DamageKind::Physical
                    && matches!(
                        self.weapon,
                        Some(Weapon::Sword | Weapon::Dagger | Weapon::Axe)
                    )),
            "normal attack has no supported weapon callback plan",
        )?;
        ensure(
            self.invocation <= 10000
                && self.ability_ratio <= 10000
                && self.critical_power <= 1000
                && (-100..=100).contains(&self.hit_bonus)
                && (-100..=100).contains(&self.critical_bonus),
            "attack coefficients exceed supported range",
        )
    }
}

/// Inputs are current battle values, with status ratios kept separate from stats.
#[derive(Debug, Clone, Copy)]
pub struct DamageInput {
    pub attack: u32,
    pub defense: u32,
    pub growth: u32,
    pub guard: bool,
    pub weapon_weak: bool,
    pub element_weak: bool,
    pub broken: bool,
    pub critical: bool,
    pub attack_up: bool,
    pub defense_down: bool,
}
/// CalcAilmentDamageHP: preserve each binary32 operation and its order.
pub fn damage(input: DamageInput, skill: &Attack, variance: f32) -> u32 {
    let attack = input.attack as f32 * (skill.invocation as f32 * 0.01_f32);
    let defense = input.defense as f32
        * if input.guard {
            130.0_f32 * 0.01_f32
        } else {
            1.0
        };
    let mut value = attack / 2.0_f32 - defense / 4.0_f32;
    let resistance = if input.broken {
        1.0
    } else if input.weapon_weak && input.element_weak {
        1.3_f32 * 1.3_f32
    } else if input.weapon_weak || input.element_weak {
        1.3
    } else {
        1.0
    };
    value *= resistance;
    value *= if input.critical {
        skill.critical_power as f32 * 0.01_f32
    } else {
        1.0
    };
    value *= variance;
    value *= 1.0; // no outgoing equipment/support modifier in the opening loadout
    value *= if input.broken { 2.0 } else { 1.0 };
    value *= skill.ability_ratio as f32 * 0.01_f32;
    value *= if input.attack_up { 1.5 } else { 1.0 };
    value *= if input.defense_down { 1.5 } else { 1.0 };
    value *= input.growth as f32 * 0.0078125_f32 + 0.5_f32;
    (value as i32).clamp(1, 9999) as u32
}
pub fn hit_probability(accuracy: u32, evasion: u32, bonus: i32) -> f32 {
    (50.0_f32 * (3.0_f32 - evasion as f32 / (accuracy as f32).max(1.0)) + bonus as f32)
        .clamp(20.0, 100.0)
        * 0.01_f32
}
pub fn critical_probability(critical: u32, speed: u32, speed_up: bool, bonus: i32) -> f32 {
    ((critical as f32 - speed as f32 * if speed_up { 1.5 } else { 1.0 }) / 2.0_f32 + bonus as f32)
        .clamp(0.0, 100.0)
        * 0.01_f32
}
pub fn variance(rng: &mut CrtRng) -> f32 {
    1.01_f32 + (1.05_f32 - 1.01_f32) * rng.unit()
}

/// Native sort's <=8 entry selection-sort path, including its unstable ties.
pub(crate) fn sort_order<T: Copy>(rows: &mut [T], speed: impl Fn(T) -> u32) {
    for end in (1..rows.len()).rev() {
        let mut lowest = 0;
        for i in 1..=end {
            if speed(rows[lowest]) > speed(rows[i]) {
                lowest = i;
            }
        }
        rows.swap(lowest, end);
    }
}
/// Native 140e47360: back-to-front sliding triples, not a uniform shuffle.
pub(crate) fn shuffle_order<T: Copy>(
    rows: &mut [T],
    eligible: impl Fn(T) -> bool,
    rng: &mut CrtRng,
) {
    if rows.len() == 2 {
        if eligible(rows[0]) && eligible(rows[1]) && rng.weighted_bool(0.5) {
            rows.swap(0, 1);
        }
    } else if rows.len() > 2 {
        for last in (2..rows.len()).rev() {
            let (first, middle) = (last - 2, last - 1);
            if !eligible(rows[first]) || !eligible(rows[middle]) || !eligible(rows[last]) {
                continue;
            }
            let branch = rng.weighted_bool(0.5);
            if rng.weighted_bool(0.66) {
                if !branch {
                    rows.swap(first, middle);
                }
                let selected = if rng.weighted_bool(0.5) {
                    first
                } else {
                    middle
                };
                rows.swap(selected, last);
            } else if branch {
                rows.swap(first, middle);
            }
        }
    }
}
