//! Saved enemy weakness icons and Study Foe preparation history.
use crate::{CrtRng, combat::{CombatProfile, Weapon, Element}};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemyKnowledge {
    pub weapons: u8,
    pub elements: u8,
    pub studied: bool,
}
pub type EnemyHistory = BTreeMap<String, EnemyKnowledge>;

/// ExecSpecialAction_Preparation: one new icon, or two on a previously
/// prepared type. Choose only closed icons; a fully known type draws nothing.
pub fn study(profile: &CombatProfile, known: &mut EnemyKnowledge, rng: &mut CrtRng)
    -> Vec<(CrtRng, CrtRng)>
{
    let mut icons: Vec<_> = profile.weapon_weaknesses.iter().map(|&w| (false, w as u8))
        .chain(profile.element_weaknesses.iter().map(|&e| (true, e as u8))).collect();
    icons.sort_unstable(); icons.dedup();
    icons.retain(|&(element, bit)| (if element {known.elements} else {known.weapons}) & (1 << bit) == 0);
    let count = if known.studied {2} else {1};
    let mut trace = Vec::new();
    for _ in 0..count {
        if icons.is_empty() {break;}
        let before = *rng;
        let choice = rng.index(icons.len() as u32) as usize;
        let (element, bit) = icons.remove(choice);
        if element {known.elements |= 1 << bit;} else {known.weapons |= 1 << bit;}
        known.studied = true;
        trace.push((before, *rng));
    }
    trace
}

pub fn reveal_weapon(known: &mut EnemyKnowledge, weapon: Weapon) {known.weapons |= 1 << weapon as u8;}
pub fn reveal_element(known: &mut EnemyKnowledge, element: Element) {known.elements |= 1 << element as u8;}
