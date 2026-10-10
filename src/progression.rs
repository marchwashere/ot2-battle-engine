//! Persistent rewards and character growth. These operations consume no RNG.
use crate::{Result,SimError,PlayerCharacter};
use serde::{Serialize,Deserialize};
use std::{collections::BTreeMap,sync::OnceLock};
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
pub struct Growth {pub level:u32,pub total_exp:u32,pub parameters:BTreeMap<String,u32>,pub growth_attack:u32,pub growth_magic:u32}
#[derive(Debug,Clone,Copy,Default,Serialize,Deserialize,PartialEq,Eq)]
pub struct Reward {pub exp:u32,pub jp:u32,pub money:u32}
#[derive(Deserialize)]
struct Data {starters:BTreeMap<u32,PlayerCharacter>,characters:BTreeMap<u32,Vec<Growth>>,rewards:BTreeMap<String,Reward>,formations:BTreeMap<String,Vec<String>>}
fn data()->&'static Data {static D:OnceLock<Data>=OnceLock::new();D.get_or_init(||serde_json::from_str(include_str!("../data/progression.json")).expect("progression import"))}
pub fn growth(character:u32,experience:u32)->Result<&'static Growth> {data().characters.get(&character).and_then(|rows|rows.iter().rev().find(|r|r.total_exp<=experience)).ok_or_else(||SimError(format!("Missing growth for character {character}")))}
pub fn growth_at_level(character:u32,level:u32)->Result<&'static Growth> {data().characters.get(&character).and_then(|rows|rows.iter().find(|r|r.level==level)).ok_or_else(||SimError(format!("Missing growth for character {character} at level {level}")))}
pub fn enemy_reward(id:&str)->Result<Reward> {data().rewards.get(id).copied().ok_or_else(||SimError(format!("Missing reward for {id}")))}
pub fn formation_reward(id:&str)->Result<Reward> {let mut r=Reward::default();for id in data().formations.get(id).ok_or_else(||SimError(format!("Missing formation {id}")))? {let e=enemy_reward(id)?;r.exp+=e.exp;r.jp+=e.jp;r.money+=e.money;}Ok(r)}
/// Preserve equipment contributions while replacing level-dependent parameters.
pub fn apply_growth(player:&mut PlayerCharacter,character:u32,from_level:u32,to_level:u32)->Result<()> {
 let rows=data().characters.get(&character).ok_or_else(||SimError("Missing character growth".into()))?;
 let a=rows.iter().find(|r|r.level==from_level).ok_or_else(||SimError("Missing starting level".into()))?;
 let b=rows.iter().find(|r|r.level==to_level).ok_or_else(||SimError("Missing ending level".into()))?;
 let delta=|value:&mut u32,key:&str| {*value=(*value as i64+b.parameters[key] as i64-a.parameters[key] as i64).max(0) as u32;};
 delta(&mut player.stats.max_hp,"HP");delta(&mut player.stats.max_sp,"MP");delta(&mut player.stats.speed,"AGI");
 if let Some(c)=&mut player.combat {delta(&mut c.attack,"ATK");delta(&mut c.defense,"DEF");delta(&mut c.magic,"MATK");delta(&mut c.magic_defense,"MDEF");delta(&mut c.accuracy,"ACC");delta(&mut c.evasion,"EVA");delta(&mut c.critical,"CON");c.growth_attack=b.growth_attack;c.growth_magic=b.growth_magic;}
 Ok(())
}

/// Imported original equipment, for newly recruited route members.
pub fn starter(character:u32)->Result<PlayerCharacter> {data().starters.get(&character).cloned().ok_or_else(||SimError(format!("Missing starter {character}")))}

#[cfg(test)]
mod recruitment_tests {
    use super::*;
    #[test]
    fn agnea_original_gear_and_growth_are_carried_without_rng() {
        let initial=crate::field_escape::route_player(8,0).unwrap();
        assert_eq!(initial.id,"agnea");
        assert_eq!(initial.combat.as_ref().unwrap().main_weapon,Some(crate::combat::Weapon::Dagger));
        let level_two=data().characters[&8].iter().find(|g|g.level==2).unwrap();
        let later=crate::field_escape::route_player(8,level_two.total_exp).unwrap();
        assert_eq!(later.stats.max_hp as i64-initial.stats.max_hp as i64,
            level_two.parameters["HP"] as i64-growth(8,0).unwrap().parameters["HP"] as i64);
        assert_eq!(serde_json::to_value(&later.combat.as_ref().unwrap().weapons).unwrap(),serde_json::to_value(&initial.combat.as_ref().unwrap().weapons).unwrap());
    }
    #[test]
    fn partitio_joins_with_original_lance_bow_and_character_growth() {
        let initial=crate::field_escape::route_player(4,0).unwrap();
        assert_eq!((initial.id.as_str(),initial.stats.max_hp,initial.stats.max_sp,initial.stats.speed),("partitio",275,50,72));
        let combat=initial.combat.as_ref().unwrap();
        assert_eq!(combat.main_weapon,Some(crate::combat::Weapon::Lance));
        assert_eq!(combat.weapons.iter().map(|w|w.weapon).collect::<Vec<_>>(),vec![crate::combat::Weapon::Lance,crate::combat::Weapon::Bow]);
        let level_two=data().characters[&4].iter().find(|g|g.level==2).unwrap();
        let later=crate::field_escape::route_player(4,level_two.total_exp).unwrap();
        assert_eq!(later.stats.max_hp-initial.stats.max_hp,
            level_two.parameters["HP"]-growth(4,0).unwrap().parameters["HP"]);
        assert_eq!(serde_json::to_value(&later.combat.as_ref().unwrap().weapons).unwrap(),serde_json::to_value(&combat.weapons).unwrap());
    }
}
