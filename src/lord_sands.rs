//! Source-backed, no-enemy-turn Lord of the Sands route routine.
use crate::{Battle,BattleConfig,CrtRng,PlayerCharacter,Result,ensure,ActorId,Command,Outcome,Resources,Event,Resource,opening_crt::RngSpan,progression::Reward};
use serde::{Serialize,Deserialize};use std::{collections::BTreeMap,sync::OnceLock};
#[derive(Deserialize)]pub struct Variant {pub id:String,pub weight:u32}
#[derive(Deserialize)]pub struct Data {pub beastlings_cost:u32,pub beastlings_variants:Vec<Variant>,pub reward:Reward,pub drop_item:String,pub drop_rate:u32}
pub fn data()->&'static Data {static D:OnceLock<Data>=OnceLock::new();D.get_or_init(||serde_json::from_str(include_str!("../data/lord_sands_data.json")).expect("Lord Sands import"))}
#[derive(Debug,Serialize)]pub struct Action {pub turn:u32,pub actor:usize,pub name:String,pub bp:u8,pub report:crate::StepReport}
#[derive(Debug,Serialize)]pub struct Prediction {pub before:CrtRng,pub at_command_menu:CrtRng,pub after:CrtRng,pub actions:Vec<Action>,pub trace:Vec<RngSpan>,pub rewards:Reward,pub drops:BTreeMap<String,u32>,pub money_spent:u32,pub broken:bool,pub overkill:bool,pub enemy_knowledge:crate::knowledge::EnemyHistory,pub party_resources_after:Vec<Resources>}
pub fn predict(before:CrtRng,players:&[PlayerCharacter],history:&crate::knowledge::EnemyHistory)->Result<Prediction> {
 predict_with_finisher_boost_cycles(before,players,history,0)
}
pub fn predict_with_finisher_boost_cycles(before:CrtRng,players:&[PlayerCharacter],history:&crate::knowledge::EnemyHistory,cycles:u32)->Result<Prediction> {
 ensure(players.iter().map(|p|p.id.as_str()).eq(["throne","osvald","temenos","partitio"]),"Lord Sands requires current Galdera party")?;
 let mut config=BattleConfig::from_json(include_str!("../data/lord_sands.json"))?;config.seed=before.state;config.players=players.to_vec();config.enemy_knowledge=history.clone();
 for p in &mut config.players {p.resources.as_mut().ok_or_else(||crate::SimError("Missing resources".into()))?.bp=1;}
 config.players[0].skills=vec!["attack_dagger".into(),"critical_scope".into()];config.players[1].skills=vec!["aggressive_slash".into()];config.players[2].skills=vec!["attack_staff".into()];config.players[3].skills=vec!["hired_help_beastlings".into()];
 let mut b=Battle::new_lord_sands(config)?;let abs=|r:CrtRng|CrtRng{state:r.state,draws:before.draws+r.draws};let at_command_menu=abs(b.state().rng);let mut actions=vec![];
 for _ in 0..8 {
  ensure(b.state().outcome==Outcome::Ongoing,"Unexpected early end to Lord Sands routine")?;
  let actor=b.current_actor().ok_or_else(||crate::SimError("No actor".into()))?.0;let turn=b.state().round;
  let (name,bp,command)=match (turn,actor) {
   (1,0)=>("Dagger x2",1,Command::Boost{id:"attack_dagger".into(),bp:1,target:Some(ActorId(4))}),
   (1,1)|(1,3)|(2,3)=>("Defend",0,Command::Defend),
   (2,0)=>("Critical Scope",0,Command::Skill{id:"critical_scope".into(),target:Some(ActorId(4))}),
   (2,1)=>("Aggressive Slash",2,Command::Boost{id:"aggressive_slash".into(),bp:2,target:Some(ActorId(4))}),
   (2,2)=>("Staff attack",0,Command::Skill{id:"attack_staff".into(),target:Some(ActorId(4))}),
   (3,3)=>("Hired Help: Beastlings",3,Command::Boost{id:"hired_help_beastlings".into(),bp:3,target:None}),
   _=>return Err(crate::SimError(format!("Unexpected Lord Sands turn {turn}, actor {actor}; refusing unmodeled enemy action")))
  };
  let mut report=if turn==3&&actor==3&&cycles>0 {let mut inputs=Vec::new();for _ in 0..cycles {inputs.extend([1,0]);}inputs.extend([1,2,3]);b.step_player_with_boost_inputs(command,&inputs)?}else{b.step_player(command)?};report.rng_before=abs(report.rng_before);report.rng_after=abs(report.rng_after);actions.push(Action{turn,actor,name:name.into(),bp,report});
 }
 ensure(b.state().outcome==Outcome::Victory&&b.state().actors[..4].iter().all(|p|p.alive()),"Lord Sands routine did not win with everyone alive")?;
 let mut broken=false;let mut overkill=false;let mut last_damage=0;
 for a in &actions {for e in &a.report.events {match *e {Event::Damage{target,amount,..} if target.0==4=>last_damage=amount,Event::Broken{actor,..} if actor.0==4=>broken=true,Event::ResourceChanged{actor,resource:Resource::Hp,before,after:0} if actor.0==4&&before>0=>overkill|=last_damage>=2*before,_=>{}}}}
 let d=data();let rewards=Reward{exp:d.reward.exp+(u32::from(broken)+u32::from(overkill))*(d.reward.exp/10).max(1),jp:d.reward.jp,money:d.reward.money+(d.reward.money/10).max(1)};
 let mut trace:Vec<_>=b.state().opening_crt.as_ref().unwrap().trace.as_ref().unwrap().iter().map(|s|RngSpan{reason:s.reason,actor:s.actor,before:abs(s.before),after:abs(s.after)}).collect();
 let mut after=abs(b.state().rng);let start=after;let mut drops=BTreeMap::new();if after.weighted_bool(d.drop_rate as f32*0.01) {drops.insert(d.drop_item.clone(),1);}trace.push(RngSpan{reason:"enemy_drop",actor:Some(ActorId(4)),before:start,after});
 Ok(Prediction{before,at_command_menu,after,actions,trace,rewards,drops,money_spent:d.beastlings_cost,broken,overkill,enemy_knowledge:b.enemy_knowledge(),party_resources_after:b.state().actors[..4].iter().map(|p|p.resources.clone()).collect()})
}
#[cfg(test)]mod tests {
 use super::*;
 #[test]fn agreed_lord_sands_routine_wins_across_seeds() {
  let players:Vec<PlayerCharacter>=serde_json::from_str(include_str!("../data/source/lord-sands-party.json")).unwrap();
  for seed in 0u32..128 {
   let r=predict(CrtRng::new(seed.wrapping_mul(1775700753)),&players,&Default::default()).unwrap();
   assert_eq!(r.actions.len(),8);assert!(r.actions[..3].iter().all(|a|a.actor!=2&&a.turn==1));
   assert!(r.broken);assert_eq!(r.actions[7].actor,3);assert_eq!(r.actions[7].turn,3);
   let damage:Vec<_>=r.actions[7].report.events.iter().filter_map(|e|if let Event::Damage {amount,..}=e{Some(*amount)}else{None}).collect();assert_eq!(damage,vec![9999;4]);
   for pair in r.trace.windows(2){assert_eq!(pair[0].after,pair[1].before);}
   assert_eq!(r.trace.last().unwrap().after,r.after);
  }
 }
 #[test]fn finisher_boost_cycles_preserve_earlier_actions() {
  let players:Vec<PlayerCharacter>=serde_json::from_str(include_str!("../data/source/lord-sands-party.json")).unwrap();
  for seed in 0u32..16 {
   let rng=CrtRng::new(seed.wrapping_mul(1775700753));
   let base=predict(rng,&players,&Default::default()).unwrap();
   let adjusted=predict_with_finisher_boost_cycles(rng,&players,&Default::default(),2).unwrap();
   assert_eq!(base.at_command_menu,adjusted.at_command_menu);
   for i in 0..7 {assert_eq!(base.actions[i].report.rng_after,adjusted.actions[i].report.rng_after);}
   assert_eq!(base.actions[7].report.rng_before,adjusted.actions[7].report.rng_before);
   assert_ne!(base.after,adjusted.after);assert_eq!(adjusted.actions[7].report.outcome,Outcome::Victory);
  }
 }
}
