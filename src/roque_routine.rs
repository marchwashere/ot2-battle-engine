//! Four ASA actions, Temenos JP100, then Partitio Guards. No enemy turns accepted.
use crate::{Battle,Result,ActorId,Outcome,Side,ensure,fight_search::BoostFrontier,gravell_routine::{Action,MenuAction,Fight,Evaluation}};
fn record(b:&mut Battle,actions:&mut Vec<MenuAction>,a:Action,inputs:&[u8])->Result<()> {
 let round=b.state().round;
 let report=if inputs.is_empty(){b.step_player(a.command())?}else{b.step_player_with_boost_inputs(a.command(),inputs)?};
 actions.push(MenuAction {round,actor:report.actor,action:a,inputs:inputs.to_vec(),before:report.rng_before,report});Ok(())
}
fn prefix(mut b:Battle)->Result<Option<(Battle,Vec<MenuAction>)>> {
 let order=b.state().turn_order();
 if order.iter().position(|a|a.0==3)>=order.iter().position(|a|a.0==2){return Ok(None);}
 let mut actions=vec![];
 for _ in 0..4 {
  if b.state().round!=1||b.state().outcome!=Outcome::Ongoing {return Ok(None);}
  let a=match b.current_actor().map(|a|a.0) {Some(0)=>Action::Dagger {target:ActorId(4)},Some(1)=>Action::SmallPomegranate {target:ActorId(2)},Some(2|3)=>Action::Defend,_=>return Ok(None)};
  record(&mut b,&mut actions,a,&[])?;
 }
 if b.state().round!=2||b.current_actor()!=Some(ActorId(2))||b.state().actors[2].resources.bp<3 {return Ok(None);}
 Ok(Some((b,actions)))
}
fn finish(b:Battle,actions:Vec<MenuAction>)->Fight {
 let enemy_ids=(4..b.state().actors.len()).map(|i|b.actor_name_id(ActorId(i)).unwrap().to_owned()).collect();
 Fight {actions,evaluation:Evaluation::Accepted,after:b.state().rng,state:b.state().clone(),enemy_ids,enemy_knowledge:b.enemy_knowledge(),trace:b.state().opening_crt.as_ref().and_then(|c|c.trace.clone()).unwrap_or_default()}
}
pub fn search(battle:Battle,max_inputs:usize)->Result<Option<Fight>> {
 let Some((menu,actions))=prefix(battle)? else {return Ok(None);};
 let mut dance=BoostFrontier::new(&menu)?;
 let mut continuations=vec![];
 for cost in (6..=max_inputs).step_by(2) {
 while let Some(candidate)=dance.next(&Action::BewilderingGrace.command(),cost-3)? {
  let b=&candidate.after_action;
  if b.state().grace.as_ref().is_none_or(|g|g.jp_percent()!=10_000)||b.state().outcome!=Outcome::Ongoing||b.current_actor()!=Some(ActorId(3))||b.state().actors[..4].iter().any(|a|!a.alive())||b.state().actors[3].resources.bp<3 {continue;}
  if b.state().actors[3].statuses.iter().any(|s|matches!(s.kind,crate::combat::StatusKind::Silence)) {continue;}
  continuations.push((BoostFrontier::new(b)?,candidate));
 }
 for (guard,candidate) in &mut continuations {
  while let Some(g)=guard.next(&Action::Guards.command(),cost-candidate.inputs.len())? {
   let b=g.after_action;
   if b.state().outcome!=Outcome::Victory||b.state().actors[..4].iter().any(|a|!a.alive()) {continue;}
   let mut rows=actions.clone();rows.push(MenuAction {round:2,actor:ActorId(2),action:Action::BewilderingGrace,inputs:candidate.inputs.clone(),before:candidate.report.rng_before,report:candidate.report.clone()});
   rows.push(MenuAction {round:2,actor:ActorId(3),action:Action::Guards,inputs:g.inputs,before:g.report.rng_before,report:g.report});
   ensure(b.side(ActorId(3))==Some(Side::Player),"Invalid Roque finish")?;
   return Ok(Some(finish(b,rows)));
  }
 }
 }
 Ok(None)
}

#[cfg(test)]mod tests {
 use super::*;
 #[test]fn defend_order_and_small_pom_supply_both_max_boosts() {
  let mut players:Vec<crate::PlayerCharacter>=serde_json::from_str(include_str!("../data/source/lord-sands-party.json")).unwrap();players[3].boost_start=true;
  let mut accepted=0;
  for seed in 0..128 {
   let b=crate::field_escape::roque_jp_battle(crate::CrtRng::new(seed),crate::escape::Situation::Normal,&players,&Default::default()).unwrap();
   let expected=b.state().turn_order().iter().position(|a|a.0==3)<b.state().turn_order().iter().position(|a|a.0==2);
   let result=prefix(b).unwrap();assert_eq!(result.is_some(),expected);
   if let Some((b,actions))=result {
    accepted+=1;assert_eq!(&b.state().turn_order()[..2],&[ActorId(2),ActorId(3)]);
    assert_eq!(b.state().actors[2].resources.bp,3);assert_eq!(b.state().actors[3].resources.bp,3);
    assert_eq!(actions.len(),4);assert!(b.state().actors[4].has_status(crate::combat::StatusKind::DefenseDown));
    assert_eq!(actions.iter().find(|a|a.report.actor==ActorId(0)).unwrap().action,Action::Dagger{target:ActorId(4)});
    assert_eq!(actions.iter().find(|a|a.report.actor==ActorId(1)).unwrap().action,Action::SmallPomegranate{target:ActorId(2)});
   }
  }
  assert!(accepted>0&&accepted<128);
 }
}
