//! Daytime Ebony Eel x3: accept five opening orders, BG JP100 then +1 Fireball.
use crate::{Battle,Result,ActorId,Outcome,fight_search::BoostFrontier,gravell_routine::{Action,MenuAction,Fight,Evaluation}};
fn record(b:&mut Battle,rows:&mut Vec<MenuAction>,action:Action)->Result<()> {
 let round=b.state().round;let report=b.step_player(action.command())?;
 rows.push(MenuAction{round,actor:report.actor,action,inputs:vec![],before:report.rng_before,report});Ok(())
}
pub fn prefix(mut b:Battle)->Result<Option<(Battle,Vec<MenuAction>)>> {
 let order=b.state().turn_order();let pos=|i|order.iter().position(|a|a.0==i).unwrap();
 let (h,o,t)=(pos(0),pos(1),pos(2));
 // No extra opening adjustment: reject only Temenos -> Throne -> Osvald.
 if t<h&&h<o{return Ok(None);}
 let branch=if o<t{0}else if h<t{1}else{2};
 let mut rows=vec![];
 for _ in 0..4 {
  if b.state().round!=1||b.state().outcome!=Outcome::Ongoing{return Ok(None);}
  let a=match b.current_actor().map(|a|a.0) {
   Some(0)=>match branch{0=>Action::Dagger{target:ActorId(4)},1=>Action::SpringyBoots{target:ActorId(1)},_=>Action::SpringyBoots{target:ActorId(2)}},
   Some(1)=>if branch==1{Action::Staff{target:ActorId(4)}}else{Action::Defend},
   Some(2)=>Action::Defend,
   Some(3)=>Action::SmallPomegranate{target:ActorId(2)},
   _=>return Ok(None),
  };record(&mut b,&mut rows,a)?;
 }
 if b.state().round!=2||b.state().turn_order().get(..2)!=Some(&[ActorId(2),ActorId(1)][..])||b.state().actors[2].resources.bp<3{return Ok(None);}
 Ok(Some((b,rows)))
}
pub fn search(battle:Battle,max_inputs:usize)->Result<Option<Fight>> {
 search_accepting(battle,max_inputs,|_|Ok(true))
}
/// The route may reject a successful finish whose field-reset counter cannot
/// cover the remaining measured path. Keep searching finisher boost inputs.
pub fn search_accepting(battle:Battle,max_inputs:usize,mut accept:impl FnMut(&Fight)->Result<bool>)->Result<Option<Fight>> {
 search_with_finisher(battle,max_inputs,Action::Fireball,1,&mut accept)
}
pub fn search_shipwreck(battle:Battle,max_inputs:usize)->Result<Option<Fight>> {
 search_with_finisher(battle,max_inputs,Action::FireballMax,3,&mut |_|Ok(true))
}
fn search_with_finisher(battle:Battle,max_inputs:usize,finisher:Action,bp:usize,accept:&mut impl FnMut(&Fight)->Result<bool>)->Result<Option<Fight>> {
 let Some((menu,rows))=prefix(battle)?else{return Ok(None);};
 let mut dance=BoostFrontier::new(&menu)?;let mut finishes=vec![];
 for cost in (3+bp..=max_inputs).step_by(2) {
  while let Some(c)=dance.next(&Action::BewilderingGrace.command(),cost-bp)? {
   let b=&c.after_action;
   if b.state().grace.as_ref().is_none_or(|g|g.jp_percent()!=10_000)||b.state().outcome!=Outcome::Ongoing||b.current_actor()!=Some(ActorId(1))||b.state().actors[..4].iter().any(|a|!a.alive()) {continue;}
   let o=&b.state().actors[1];
   if usize::from(o.resources.bp)<bp||o.resources.sp<14||o.has_status(crate::combat::StatusKind::Silence){continue;}
   finishes.push((BoostFrontier::new(b)?,c));
  }
  for (fire,c) in &mut finishes {
   while let Some(f)=fire.next(&finisher.command(),cost-c.inputs.len())? {
    let b=f.after_action;
    if b.state().outcome!=Outcome::Victory||b.state().actors[..4].iter().any(|a|!a.alive())||b.state().grace.as_ref().is_none_or(|g|g.jp_percent()!=10_000){continue;}
    let mut actions=rows.clone();
    actions.push(MenuAction{round:2,actor:ActorId(2),action:Action::BewilderingGrace,inputs:c.inputs.clone(),before:c.report.rng_before,report:c.report.clone()});
    actions.push(MenuAction{round:2,actor:ActorId(1),action:finisher,inputs:f.inputs,before:f.report.rng_before,report:f.report});
    let fight=Fight{actions,evaluation:Evaluation::Accepted,after:b.state().rng,state:b.state().clone(),enemy_ids:(4..b.state().actors.len()).map(|i|b.actor_name_id(ActorId(i)).unwrap().to_owned()).collect(),enemy_knowledge:b.enemy_knowledge(),trace:b.state().opening_crt.as_ref().and_then(|c|c.trace.clone()).unwrap_or_default()};
    if accept(&fight)?{return Ok(Some(fight));}
   }
  }
 }
 Ok(None)
}

#[cfg(test)]mod tests {
 use super::*;
 fn players()->Vec<crate::PlayerCharacter> {
  let mut p:Vec<crate::PlayerCharacter>=serde_json::from_str(include_str!("../data/source/lord-sands-party.json")).unwrap();
  p[1].boost_start=true;p[3].boost_start=true;
  let o=p[1].combat.as_mut().unwrap();o.magic=499;o.fire_damage_bonus=60;
  o.weapons.iter_mut().find(|w|w.weapon==crate::combat::Weapon::Sword).unwrap().magic=298;p
 }
 #[test]fn all_relative_orders_and_pom_positions() {
  let mut orders=std::collections::BTreeSet::new();let mut pom_positions=std::collections::BTreeSet::new();
  for seed in 0..1024 {
   let b=crate::field_escape::nameless_jp_battle(crate::CrtRng::new(seed),crate::escape::Situation::Normal,&players(),&Default::default()).unwrap();
   let order=b.state().turn_order().iter().map(|a|a.0).collect::<Vec<_>>();let relative=order.iter().copied().filter(|&i|i!=3).collect::<Vec<_>>();orders.insert(relative.clone());
   let result=prefix(b).unwrap();assert_eq!(result.is_none(),relative==[2,0,1]);
   if let Some((b,rows))=result {pom_positions.insert(order.iter().position(|&i|i==3).unwrap());assert_eq!(&b.state().turn_order()[..2],&[ActorId(2),ActorId(1)]);assert_eq!(b.state().actors[2].resources.bp,3);assert!(b.state().actors[1].resources.bp>=1);assert_eq!(rows.len(),4);}
  }
  assert_eq!(orders.len(),6);assert_eq!(pom_positions.len(),4);
 }
 #[test]fn full_jp_search_finishes_with_fireball() {
  for seed in 0u32..128 {
   let b=crate::field_escape::nameless_jp_battle(crate::CrtRng::new(seed.wrapping_mul(2654435761)),crate::escape::Situation::Normal,&players(),&Default::default()).unwrap();
   let mut replay=b.clone();let mut accepted=0;
   if let Some(f)=search_accepting(b,96,|_|{accepted+=1;Ok(accepted>1)}).unwrap(){
    assert!(accepted>1);assert_eq!(f.actions.last().unwrap().action,Action::Fireball);assert_eq!(f.state.outcome,Outcome::Victory);
    assert!(f.state.actors[..4].iter().all(|a|a.alive()));
    for a in &f.actions {
     assert_eq!(replay.current_actor(),Some(a.actor));
     let r=if a.inputs.is_empty(){replay.step_player(a.action.command())}else{replay.step_player_with_boost_inputs(a.action.command(),&a.inputs)}.unwrap();
     assert_eq!(r.rng_after,a.report.rng_after);
    }
    assert_eq!(replay.state().rng,f.after);
    let r=crate::gravell_routine::rewards(&f,false).unwrap();assert_eq!(r.awarded.jp,12210);assert_eq!(r.hired_help_cost,0);return;
   }
  }panic!("Expected a reachable JP100 candidate");
 }
 #[test]fn shipwreck_all_six_land_formations_win_with_max_fireball() {
  let formations:Vec<Vec<&str>>=vec![vec!["ENE_NML_DNG_CAV_210";4],vec!["ENE_NML_OCN_090","ENE_NML_OCN_070","ENE_NML_OCN_070"],vec!["ENE_NML_DNG_CAV_210","ENE_NML_DNG_CAV_120","ENE_NML_DNG_CAV_120"],vec!["ENE_NML_FLD_SEA_240";3],vec!["ENE_NML_DNG_CAV_210","ENE_NML_DNG_CAV_120","ENE_NML_OCN_080"],vec!["ENE_NML_OCN_090","ENE_NML_OCN_080"]];
  for enemies in formations {
   let mut success=false;
   for seed in 0u32..128 {
    let b=crate::field_escape::shipwreck_jp_battle(crate::CrtRng::new(seed.wrapping_mul(2654435761)),crate::escape::Situation::Normal,&players(),&enemies,&Default::default()).unwrap();
    if let Some(f)=search_shipwreck(b,64).unwrap(){assert_eq!(f.actions.last().unwrap().action,Action::FireballMax);assert_eq!(f.state.outcome,Outcome::Victory);assert!(f.state.actors[..4].iter().all(|a|a.alive()));let r=crate::gravell_routine::rewards(&f,false).unwrap();assert!(r.awarded.jp>=12210);assert_eq!(r.hired_help_cost,0);success=true;break;}
   }assert!(success,"Failed formation {enemies:?}");
  }
 }
}
