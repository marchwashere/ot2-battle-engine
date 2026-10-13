//! Bounded Temenos recruitment: all four have A Step Ahead; first actor +1 BP attack.
use crate::{Battle,BattleConfig,CrtRng,PlayerCharacter,Result,ensure,ActorId,Command,Outcome,Resources,Event,Resource,opening_crt::RngSpan,progression::Reward};
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Debug,Serialize)]
pub struct Prediction {
 pub before:CrtRng,pub at_command_menu:CrtRng,pub after:CrtRng,pub actor:usize,
 pub report:crate::StepReport,pub trace:Vec<RngSpan>,pub rewards:Reward,pub drops:BTreeMap<String,u32>,
 pub boost_cycles:u32,pub overkill:bool,pub enemy_knowledge:crate::knowledge::EnemyHistory,pub party_resources_after:Vec<Resources>,
}
pub fn predict(before:CrtRng,players:&[PlayerCharacter],history:&crate::knowledge::EnemyHistory)->Result<Prediction> {
 ensure(players.iter().map(|p|p.id.as_str()).eq(["throne","osvald","agnea","partitio"]),"Temenos requires carried Galdera party")?;
 let mut config=BattleConfig::from_json(include_str!("../data/temenos_recruitment.json"))?;
 config.seed=before.state;config.players=players.to_vec();config.enemy_knowledge=history.clone();
 for (p,weapon) in config.players.iter_mut().zip(["dagger","staff","dagger","lance"]) {
  p.resources.as_mut().ok_or_else(||crate::SimError("Missing carried resources".into()))?.bp=1;
  p.skills=vec![format!("attack_{weapon}")];
 }
 let battle=Battle::new_temenos_recruitment(config,crate::escape::Situation::Normal,vec![0,1,2,3])?;
 let absolute=|r:CrtRng|CrtRng {state:r.state,draws:before.draws+r.draws};
 let actor=battle.current_actor().ok_or_else(||crate::SimError("Missing Temenos first actor".into()))?;
 ensure(actor.0<4,"Recruitment must start with a player")?;
 let at_command_menu=absolute(battle.state().rng);
 let command=Command::Boost {id:battle.default_attack(actor)?.into(),target:Some(ActorId(4)),bp:1};
 let mut boost_cycles=0;
 let (battle,mut report)=loop {
  let mut trial=battle.clone();
  let inputs:Vec<_>=(0..boost_cycles).flat_map(|_|[1,0]).chain([1]).collect();
  let report=trial.step_player_with_boost_inputs(command.clone(),&inputs)?;
  if trial.state().outcome==Outcome::Victory {break (trial,report);}
  boost_cycles+=1;
  ensure(boost_cycles<65536,"No Temenos first-action kill found within the boost search; refusing a surviving enemy")?;
 };
 report.rng_before=absolute(report.rng_before);report.rng_after=absolute(report.rng_after);
 let mut last_damage=0;let mut overkill=false;let mut broken=false;
 for e in &report.events {match *e {
  Event::Damage {target,amount,..} if target.0==4=>last_damage=amount,
  Event::Broken {actor,..} if actor.0==4=>broken=true,
  Event::ResourceChanged {actor,resource:Resource::Hp,before,after} if actor.0==4&&after==0&&before>0=>overkill|=last_damage>=2*before,
  _=>{}
 }}
 let mut trace:Vec<_>=battle.state().opening_crt.as_ref().unwrap().trace.as_ref().unwrap().iter().map(|s|RngSpan {reason:s.reason,actor:s.actor,before:absolute(s.before),after:absolute(s.after)}).collect();
 let mut after=absolute(battle.state().rng);let start=after;
 let mut drops=BTreeMap::new();if after.weighted_bool(1.0) {drops.insert("ITM_CSM_0500".into(),1);}
 trace.push(RngSpan {reason:"enemy_drop",actor:Some(ActorId(4)),before:start,after});
 let base=crate::progression::formation_reward("ENG_EVE_CLE_C00_010")?;
 let rewards=Reward {exp:base.exp+(u32::from(overkill)+u32::from(broken))*(base.exp/10).max(1),jp:base.jp+(base.jp/10).max(1),money:base.money+(base.money/10).max(1)};
 Ok(Prediction {boost_cycles,before,at_command_menu,after,actor:actor.0,report,trace,rewards,drops,overkill,enemy_knowledge:battle.enemy_knowledge(),party_resources_after:battle.state().actors[..4].iter().map(|a|a.resources.clone()).collect()})
}
#[cfg(test)]mod tests {
 use super::*;
 #[test]fn level70_all_first_actors_win_with_two_hits() {
  let exp=crate::field_escape::experience_for_level(70).unwrap();
  let players:Vec<_>=[7,6,8,4].iter().map(|&id|crate::field_escape::route_player(id,exp).unwrap()).collect();
  let mut seen=std::collections::BTreeSet::new();
  for seed in 0..128 {let r=predict(CrtRng::new(seed),&players,&Default::default()).unwrap();seen.insert(r.actor);
   assert_eq!(r.report.events.iter().filter(|e|matches!(e,Event::Damage {target,..} if target.0==4)).count(),2);
   assert_eq!(r.drops.get("ITM_CSM_0500"),Some(&1));
   for pair in r.trace.windows(2){assert_eq!(pair[0].after,pair[1].before);}
   assert_eq!(r.trace.last().unwrap().after,r.after);
   if seen.len()==4{return;}
  }panic!("All four first actors must be covered");
 }
}

