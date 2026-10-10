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
    DefenseUp,
    MagicDefenseUp,
    ActFirst,
    Silence,
    Poison,
    MagicDefenseDown,
    MagicReflection,
    LastAct,
    AttackDown,
    MagicDown,
    SpeedDown,
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
    #[serde(default)]
    pub use_cat_damage_type: bool,
    /// Imported initial idle resource and synchronization gates for enemy CRT.
    #[serde(default)]
    pub enemy_initialization: Option<EnemyInitialization>,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyInitialization {
    pub initial_idle_frames: u32,
    pub sync_to_parent: bool,
    pub sync_children: bool,
}
/// Fixed elemental damage applies resistance, then the enemy CatDamageType divisor.
pub fn elemental_fixed_damage(amount:u32,weak:bool,broken:bool,cat:bool)->u32 {
    let value=((amount as f32 * if !broken && weak {1.3_f32}else{1.0}) * if broken {2.0_f32}else{1.0}) as u32;
    (if cat {value/100}else{value}).max(1)
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
            self.enemy_bp_base <= 97,
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
                        Some(Weapon::Sword | Weapon::Lance | Weapon::Dagger | Weapon::Axe | Weapon::Staff)
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
    pub defense_up: bool,
}
/// CalcAilmentDamageHP: preserve each binary32 operation and its order.
pub fn damage(input: DamageInput, skill: &Attack, variance: f32) -> u32 {
    damage_with_cat_type(input, skill, variance, false)
}
/// CalcAilmentDamageHP selects the Cait base BEFORE the shared multipliers.
/// This is distinct from the /100 rule for fixed-damage soulstones.
pub fn damage_with_cat_type(input: DamageInput, skill: &Attack, variance: f32, cat: bool) -> u32 {
    damage_with_statuses(input, skill, variance, cat, false)
}
/// Disease ratios are applied before truncation, including simultaneous up/down.
pub fn damage_with_statuses(input: DamageInput, skill: &Attack, variance: f32, cat: bool, attack_down: bool) -> u32 {
    let attack = input.attack as f32 * (skill.invocation as f32 * 0.01_f32);
    let defense = input.defense as f32
        * if input.guard {
            130.0_f32 * 0.01_f32
        } else {
            1.0
        };
    let mut value = if cat {
        (attack - defense) / 96.0_f32 + 2.0_f32
    } else {
        attack / 2.0_f32 - defense / 4.0_f32
    };
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
    value *= parameter_disease_ratio(input.attack_up, attack_down);
    // GetDEFDiseaseRatio/GetMDEFDiseaseRatio: up then down, binary32.
    let defense_ratio =
        if input.defense_up { 0.667_f32 } else { 1.0 } * if input.defense_down { 1.5 } else { 1.0 };
    value *= defense_ratio;
    value *= input.growth as f32 * 0.0078125_f32 + 0.5_f32;
    (value as i32).clamp(1, 9999) as u32
}
pub fn hit_probability(accuracy: u32, evasion: u32, bonus: i32) -> f32 {
    (50.0_f32 * (3.0_f32 - evasion as f32 / (accuracy as f32).max(1.0)) + bonus as f32)
        .clamp(20.0, 100.0)
        * 0.01_f32
}
pub fn critical_probability(critical: u32, speed: u32, speed_up: bool, bonus: i32) -> f32 {
    critical_probability_with_statuses(critical, speed, speed_up, false, bonus)
}
pub fn parameter_disease_ratio(up: bool, down: bool) -> f32 {
    // GetATKDiseaseRatio/GetMATKDiseaseRatio/GetAGIDiseaseRatio: up, then down.
    (if up { 1.5_f32 } else { 1.0 }) * if down { 0.667_f32 } else { 1.0 }
}
pub fn critical_probability_with_statuses(critical: u32, speed: u32, speed_up: bool, speed_down: bool, bonus: i32) -> f32 {
    ((critical as f32 - speed as f32 * parameter_disease_ratio(speed_up, speed_down)) / 2.0_f32 + bonus as f32)
        .clamp(0.0, 100.0)
        * 0.01_f32
}
pub fn variance(rng: &mut CrtRng) -> f32 {
    1.01_f32 + (1.05_f32 - 1.01_f32) * rng.unit()
}

/// Native 140e2b490/140e2b820: selection leaves, midpoint partition and heap
/// fallback. Port of the instruction-oracle-tested C++ native_order_sort.hpp.
pub(crate) fn sort_order<T: Copy>(rows: &mut [T], speed: impl Fn(T) -> u32) {
    fn heap<T:Copy>(r:&mut [T],speed:&impl Fn(T)->u32) {
        fn sift<T:Copy>(r:&mut [T],mut node:usize,end:usize,speed:&impl Fn(T)->u32) {
            while node*2+1<end {
                let mut child=node*2+1;
                if child+1<end && speed(r[child+1])<=speed(r[child]) {child+=1;}
                if speed(r[node])<=speed(r[child]) {break;}
                r.swap(node,child);node=child;
            }
        }
        if r.len()<2 {return;}
        for root in (0..=(r.len()-2)/2).rev() {sift(r,root,r.len(),speed);}
        for end in (1..r.len()).rev() {r.swap(0,end);sift(r,0,end,speed);}
    }
    if rows.len()<2 {return;}
    let logarithm=(rows.len() as f32).ln();
    let mut pending=vec![(0usize,rows.len()-1,(logarithm+logarithm) as u32)];
    while let Some((lo,hi,depth))=pending.pop() {
        let size=hi-lo+1;
        if depth==0 {heap(&mut rows[lo..=hi],&speed);}
        else if size<=8 {
            for end in (lo+1..=hi).rev() {
                let mut lowest=lo;
                for i in lo+1..=end {if speed(rows[lowest])>speed(rows[i]) {lowest=i;}}
                rows.swap(lowest,end);
            }
        } else {
            rows.swap(lo,lo+size/2);
            let mut left=lo;let mut right=hi+1;
            loop {
                left+=1;
                while left<=hi && speed(rows[lo])<=speed(rows[left]) {left+=1;}
                right-=1;
                while right>lo && speed(rows[right])<=speed(rows[lo]) {right-=1;}
                if left>right {break;}
                rows.swap(left,right);
            }
            rows.swap(lo,right);
            let lower=(right>lo+1).then(||(lo,right-1,depth-1));
            let upper=(left<hi).then(||(left,hi,depth-1));
            // Process the smaller partition first, as in the native stack.
            if (right as isize-1-lo as isize) < (hi as isize-left as isize) {
                if let Some(r)=upper {pending.push(r);} if let Some(r)=lower {pending.push(r);}
            }else{
                if let Some(r)=lower {pending.push(r);} if let Some(r)=upper {pending.push(r);}
            }
        }
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


#[cfg(test)]
mod large_order_tests {
 #[test]
 fn native_instruction_oracle_orders_match_through_sixty_four_actors() {
  #[derive(serde::Deserialize)] struct Case {speeds:Vec<u32>,order:Vec<usize>}
  let cases:Vec<Case>=serde_json::from_str(include_str!("../data/source/native-order-large-fixtures.json")).unwrap();
  assert_eq!(cases.len(),325);
  for c in cases {let mut rows:Vec<_>=(0..c.speeds.len()).collect();super::sort_order(&mut rows,|i|c.speeds[i]);assert_eq!(rows,c.order,"speeds {:?}",c.speeds);}
 }
}
