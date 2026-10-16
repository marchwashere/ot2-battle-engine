//! Nighttime Winged Landwalker x2: Shipwreck opening, JP100 then +2 Fireball.
use crate::{Battle,Result,gravell_routine::{MenuAction,Fight}};

pub fn prefix(battle:Battle)->Result<Option<(Battle,Vec<MenuAction>)>> {
 crate::nameless_routine::prefix(battle)
}
pub fn search(battle:Battle,max_inputs:usize)->Result<Option<Fight>> {
 crate::nameless_routine::search_roque(battle,max_inputs)
}

#[cfg(test)]mod tests {
 use super::*;
 use crate::{ActorId,Outcome,gravell_routine::Action};
 fn players()->Vec<crate::PlayerCharacter> {
  let mut p:Vec<crate::PlayerCharacter>=serde_json::from_str(include_str!("../data/source/lord-sands-party.json")).unwrap();
  p[1].boost_start=true;p[3].boost_start=true;
  // Lv70, Gravell Optimize + Elemental Augmentor + Elemental Augmentation;
  // no Magic nuts. Firestarter supplies the best weapon MATK.
  let o=p[1].combat.as_mut().unwrap();o.magic=476;o.fire_damage_bonus=60;
  o.weapons.iter_mut().find(|w|w.weapon==crate::combat::Weapon::Sword).unwrap().magic=298;p
 }
 #[test]fn same_five_openings_as_shipwreck_supply_bg_then_osvald() {
  let mut orders=std::collections::BTreeSet::new();
  let mut donate_positions=std::collections::BTreeSet::new();
  for seed in 0..1024 {
   let b=crate::field_escape::roque_jp_battle(crate::CrtRng::new(seed),crate::escape::Situation::Normal,&players(),&Default::default()).unwrap();
   let order=b.state().turn_order().iter().map(|a|a.0).collect::<Vec<_>>();
   let relative=order.iter().copied().filter(|&i|i!=3).collect::<Vec<_>>();orders.insert(relative.clone());
   let result=prefix(b).unwrap();assert_eq!(result.is_none(),relative==[2,0,1]);
   if let Some((b,actions))=result {
    donate_positions.insert(order.iter().position(|&i|i==3).unwrap());
    assert_eq!(&b.state().turn_order()[..2],&[ActorId(2),ActorId(1)]);
    assert_eq!(b.state().actors[2].resources.bp,3);assert_eq!(b.state().actors[1].resources.bp,3);
    assert!(actions.iter().any(|a|a.actor==ActorId(3)&&a.action==Action::DonateBpUnboosted{target:ActorId(2)}));
    assert!(b.state().inventory.iter().all(|&n|n==0));
   }
  }
  assert_eq!(orders.len(),6);assert_eq!(donate_positions.len(),4);
 }
 #[test]fn no_nut_fireball_floor_clears_both_unbroken_landwalkers() {
  use crate::combat::{self,Attack,DamageInput,DamageKind};
  let p=players();let o=p[1].combat.as_ref().unwrap();
  let skill=Attack{kind:DamageKind::Magical,weapon:None,element:None,invocation:105,ability_ratio:280,hit_bonus:0,critical_bonus:0,critical_power:125,normal_attack:false};
  let input=DamageInput{attack:774,defense:403,growth:o.growth_magic,guard:false,weapon_weak:false,element_weak:false,broken:false,critical:false,attack_up:true,defense_down:true,defense_up:false};
  assert_eq!(combat::damage_with_equipment(input,&skill,1.01,false,false,60),7243);
  assert!(combat::damage_with_equipment(input,&skill,1.01,false,false,60)>6897);
 }
 #[test]fn full_jp_search_replays_fireball_victory_without_pom_or_guards() {
  for seed in 0u32..128 {
   let b=crate::field_escape::roque_jp_battle(crate::CrtRng::new(seed.wrapping_mul(2654435761)),crate::escape::Situation::Normal,&players(),&Default::default()).unwrap();
   let mut replay=b.clone();
   if let Some(f)=search(b,64).unwrap() {
    assert_eq!(f.actions.len(),6);assert_eq!(f.actions.last().unwrap().action,Action::FireballTwo);
    assert_eq!(f.state.outcome,Outcome::Victory);assert!(f.state.actors[..4].iter().all(|a|a.alive()));
    assert_eq!(f.state.grace.as_ref().unwrap().jp_percent(),10_000);
    for a in &f.actions {
     assert_eq!(replay.current_actor(),Some(a.actor));
     let r=if a.inputs.is_empty(){replay.step_player(a.action.command())}else{replay.step_player_with_boost_inputs(a.action.command(),&a.inputs)}.unwrap();
     assert_eq!(r.rng_after,a.report.rng_after);
    }
    assert_eq!(replay.state().rng,f.after);
    let r=crate::gravell_routine::rewards(&f,false).unwrap();
    assert_eq!(r.awarded.jp,12760);assert_eq!(r.hired_help_cost,0);return;
   }
  }panic!("Expected a reachable Roque JP100 candidate");
 }
}
