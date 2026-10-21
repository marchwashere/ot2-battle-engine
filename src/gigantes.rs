//! Nighttime Gigantes routine, with carried equipment and four A Step Ahead users.
use crate::{Battle,BattleConfig,CrtRng,PlayerCharacter,Result,ensure,ActorId,Command,Outcome,Event,Resource,opening_crt::RngSpan,progression::Reward};
use crate::lord_sands::{Action,Prediction};
use std::collections::BTreeMap;
#[derive(Debug,serde::Serialize)]pub struct DamageFloor {pub sword_three:u32,pub barrage_six:u32,pub beastlings_four:u32,pub total:u32}
/// Conservative floor: no criticals, minimum variance, six non-fire Barrage
/// hits, every sword/Barrage hit unbroken, and no credit for turn-one attacks.
pub fn damage_floor(players:&[PlayerCharacter])->Result<DamageFloor> {
 use crate::combat::{self,Attack,DamageKind,DamageInput,Weapon};
 ensure(players.len()==4,"Expected four players")?;
 let t=players[0].combat.as_ref().ok_or_else(||crate::SimError("Missing Throné stats".into()))?;
 let o=players[1].combat.as_ref().ok_or_else(||crate::SimError("Missing Osvald stats".into()))?;
 ensure(combat::hit_probability(t.accuracy,110,20)==1.,"Throné sword hits are not guaranteed")?;
 let sword=t.weapons.iter().find(|w|w.weapon==Weapon::Sword).ok_or_else(||crate::SimError("Throné sword missing".into()))?;
 let calc=|attack,growth,defense,invocation,ability_ratio,magical:bool,broken:bool,up:bool|{
  combat::damage(DamageInput{attack,defense,growth,guard:false,weapon_weak:!magical&&!broken,element_weak:magical&&!broken,broken,critical:false,attack_up:up,defense_down:true,defense_up:false},
   &Attack{kind:if magical{DamageKind::Magical}else{DamageKind::Physical},weapon:None,element:None,invocation,ability_ratio,hit_bonus:0,critical_bonus:0,critical_power:125,normal_attack:false},1.01)
 };
 let sword_three=3*calc((t.attack+sword.attack).min(999),t.growth_attack,295,100,80,false,false,true);
 let barrage_six=6*calc((o.magic+o.weapons.iter().map(|w|w.magic).max().unwrap_or(0)).min(999),o.growth_magic,280,80,170,true,false,true);
 let beastlings_four=4*calc(400,400,295,180,260,false,true,false);
 Ok(DamageFloor{sword_three,barrage_six,beastlings_four,total:sword_three+barrage_six+beastlings_four})
}
pub fn predict(before:CrtRng,players:&[PlayerCharacter],history:&crate::knowledge::EnemyHistory)->Result<Prediction> {
 predict_boss(before,players,history,Routine::Gigantes,0)
}
/// Extra 0->1->0 cycles at Partitio's T2 finisher, then normal 0->3 boost.
pub fn predict_with_finisher_cycles(before:CrtRng,players:&[PlayerCharacter],history:&crate::knowledge::EnemyHistory,cycles:u32)->Result<Prediction> {
 predict_boss(before,players,history,Routine::Gigantes,cycles)
}
pub fn predict_sands(before:CrtRng,players:&[PlayerCharacter],history:&crate::knowledge::EnemyHistory,cycles:u32)->Result<Prediction> {
 predict_boss(before,players,history,Routine::Sands,cycles)
}
enum Routine {Gigantes,Sands,#[cfg(test)] CapturedGigantes}
fn predict_boss(before:CrtRng,players:&[PlayerCharacter],history:&crate::knowledge::EnemyHistory,routine:Routine,cycles:u32)->Result<Prediction> {
 let sands=matches!(routine,Routine::Sands);
 ensure(players.iter().map(|p|p.id.as_str()).eq(["throne","osvald","temenos","partitio"]),"Gigantes requires Galdera party")?;
 ensure(players[1].boost_start&&players[3].boost_start,"Gigantes requires Osvald and Partitio Boost-Start")?;
 let mut config=BattleConfig::from_json(include_str!("../data/gigantes.json"))?;
 if sands {config.enemies=BattleConfig::from_json(include_str!("../data/lord_sands.json"))?.enemies;}
 config.seed=before.state;config.players=players.to_vec();config.enemy_knowledge=history.clone();
 for p in &mut config.players {p.resources.as_mut().ok_or_else(||crate::SimError("Missing resources".into()))?.bp=1+u8::from(p.boost_start);}
 config.players[0].skills=vec!["attack_sword".into()];config.players[1].skills=vec!["elemental_barrage".into()];config.players[2].skills=vec!["attack_staff".into()];config.players[3].skills=vec!["attack_lance".into(),"hired_help_beastlings".into()];
 if sands {config.players[0].skills.push("critical_scope".into());config.players[1].skills.push("aggressive_slash".into());config.players[2].skills.push("holy_light".into());}
 let mut b=Battle::new_route_boss(config,None)?;
 let abs=|r:CrtRng|CrtRng{state:r.state,draws:before.draws+r.draws};
 let at_command_menu=abs(b.state().rng);let mut actions=vec![];let mut defended=[false;2];
 for _ in 0..8 {
  if b.state().outcome==Outcome::Victory {break;}
  let actor=b.current_actor().ok_or_else(||crate::SimError("No actor".into()))?.0;let turn=b.state().round;
  let (name,bp,command)=if sands {match(turn,actor){
   (1,0)=>("Critical Scope",0,Command::Skill{id:"critical_scope".into(),target:Some(ActorId(4))}),
   (1,1)=>{defended[1]=true;("Defend",0,Command::Defend)},
   (1,2)=>("Holy Light",0,Command::Skill{id:"holy_light".into(),target:Some(ActorId(4))}),
   (1,3) if !defended[1]=>("Defend",0,Command::Defend),
   (1,3)=>("Lance attack",0,Command::Skill{id:"attack_lance".into(),target:Some(ActorId(4))}),
   (2,1)=>("Aggressive Slash",3,Command::Boost{id:"aggressive_slash".into(),bp:3,target:Some(ActorId(4))}),
   (2,3)=>{ensure(b.state().actors[4].broken(),"Lord Sands must be broken before Beastlings")?;("Hired Help: Beastlings",3,Command::Boost{id:"hired_help_beastlings".into(),bp:3,target:None})},
   (2,0)|(2,2)=>("Defend",0,Command::Defend),
   _=>return Err(crate::SimError("Unexpected Lord Sands enemy action".into()))
  }}else{match (turn,actor) {
   // Both breakers gain Defend priority over the two T1 attackers on T2.
   (1,0)|(1,1)=>{defended[actor]=true;("Defend",0,Command::Defend)},
   // Preserve the old retail capture as an independent mechanics regression.
   #[cfg(test)]
   (1,3) if matches!(routine,Routine::CapturedGigantes)&&!defended[0]&&!defended[1]=>("Defend",0,Command::Defend),
   (1,3)=>("Lance attack",0,Command::Skill{id:"attack_lance".into(),target:Some(ActorId(4))}),
   (1,2)=>("Staff attack",0,Command::Skill{id:"attack_staff".into(),target:Some(ActorId(4))}),
   (2,0)=>("Sword x3",2,Command::Boost{id:"attack_sword".into(),bp:2,target:Some(ActorId(4))}),
   (2,1)=>("Elemental Barrage",3,Command::Boost{id:"elemental_barrage".into(),bp:3,target:Some(ActorId(4))}),
   (2,2)=>("Defend",0,Command::Defend),
   (2,3)=>{ensure(b.state().actors[4].broken(),"Gigantes must be broken before Beastlings")?;("Hired Help: Beastlings",3,Command::Boost{id:"hired_help_beastlings".into(),bp:3,target:None})},
   _=>return Err(crate::SimError(format!("Unexpected Gigantes turn {turn}, actor {actor}; refusing unmodeled enemy action")))
  }};
  let mut report=if actor==3&&turn==2&&cycles>0{let mut inputs:Vec<_>=(0..cycles).flat_map(|_|[1,0]).collect();inputs.extend([1,2,3]);b.step_player_with_boost_inputs(command,&inputs)?}else{b.step_player(command)?};report.rng_before=abs(report.rng_before);report.rng_after=abs(report.rng_after);actions.push(Action{turn,actor,name:name.into(),bp,report});
 }
 ensure(b.state().outcome==Outcome::Victory&&b.state().actors[..4].iter().all(|p|p.alive()),"Gigantes routine did not win with everyone alive")?;
 let mut broken=false;let mut overkill=false;let mut last_damage=0;
 for a in &actions {for e in &a.report.events {match *e {Event::Damage{target,amount,..} if target.0==4=>last_damage=amount,Event::Broken{actor,..} if actor.0==4=>broken=true,Event::ResourceChanged{actor,resource:Resource::Hp,before,after:0} if actor.0==4&&before>0=>overkill|=last_damage>=2*before,_=>{}}}}
 let d:serde_json::Value=serde_json::from_str(if sands {include_str!("../data/lord_sands_data.json")}else{include_str!("../data/gigantes_data.json")})?;
 let base:Reward=serde_json::from_value(d["reward"].clone())?;
 let rewards=Reward{exp:base.exp+(u32::from(broken)+u32::from(overkill))*(base.exp/10).max(1),jp:base.jp+(base.jp/10).max(1),money:base.money+(base.money/10).max(1)};
 let mut trace:Vec<_>=b.state().opening_crt.as_ref().unwrap().trace.as_ref().unwrap().iter().map(|s|RngSpan{reason:s.reason,actor:s.actor,before:abs(s.before),after:abs(s.after)}).collect();
 let mut after=abs(b.state().rng);let start=after;let mut drops=BTreeMap::new();if after.weighted_bool(d["drop_rate"].as_u64().unwrap() as f32*0.01) {drops.insert(d["drop_item"].as_str().unwrap().into(),1);}trace.push(RngSpan{reason:"enemy_drop",actor:Some(ActorId(4)),before:start,after});
 Ok(Prediction{before,at_command_menu,after,actions,trace,rewards,drops,money_spent:crate::lord_sands::data().beastlings_cost,broken,overkill,enemy_knowledge:b.enemy_knowledge(),party_resources_after:b.state().actors[..4].iter().map(|p|p.resources.clone()).collect()})
}
#[cfg(test)]mod tests {
 use super::*;
 #[test]fn english_barrage_120fps_reveals_ice_after_fourth_hit() {
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../data/source/gigantes-vod-483f1bed.json")).unwrap();
  let before:CrtRng=serde_json::from_value(fixture["battle_entry"].clone()).unwrap();
  let players:Vec<PlayerCharacter>=serde_json::from_value(fixture["players"].clone()).unwrap();
  let history=serde_json::from_value(fixture["enemy_knowledge"].clone()).unwrap();
  let r=predict(before,&players,&history).unwrap();
  let barrage=r.actions.iter().find(|a|a.name=="Elemental Barrage").unwrap();
  let sword=r.actions.iter().find(|a|a.name=="Sword x3").unwrap();
  let damage=|a:&Action|a.report.events.iter().filter_map(|e|if let Event::Damage{amount,..}=e{Some(*amount)}else{None}).collect::<Vec<_>>();
  assert_eq!(damage(barrage),[4513,2884,4538,2834,2853,2853,2827]);
  assert_eq!(damage(sword),[364,453,554]);
  // Only the fourth-hit discovery speaks: the earlier lightning reveal is
  // still inside the cast clip. The native voice selector consumes the draw.
  let voices:Vec<_>=r.trace.iter().filter(|s|s.reason=="weakness_voice"
   &&s.before.draws>=barrage.report.rng_before.draws&&s.after.draws<=barrage.report.rng_after.draws).collect();
  assert_eq!(voices.len(),1);
  assert_eq!((voices[0].before.draws,voices[0].after.draws),(7375,7376));
  assert_eq!(barrage.report.rng_after.draws,7405);
  assert_eq!(sword.report.rng_after.draws,7432);
  assert_eq!(r.after.draws,7464);
  for pair in r.trace.windows(2){assert_eq!(pair[0].after,pair[1].before);}
 }
 #[test]fn retail_gigantes_full_battle() {
  let mut players:Vec<PlayerCharacter>=serde_json::from_str(include_str!("../data/source/lord-sands-party.json")).unwrap();
  for i in [1,3] {players[i].boost_start=true;}
  players[0].stats.speed+=112;
  for stat in ["CON","ATK"] {crate::jobs::add_nut(&mut players[0],7,70,Some(11),stat,28).unwrap();}
  let o=players[1].combat.as_mut().unwrap();o.magic=499;o.fire_damage_bonus=60;
  o.weapons.iter_mut().find(|w|w.weapon==crate::combat::Weapon::Sword).unwrap().magic=298;
  let r=predict_boss(CrtRng{state:0x4dde2f3b,draws:9597},&players,&Default::default(),Routine::CapturedGigantes,0).unwrap();
  let live:serde_json::Value=serde_json::from_str(include_str!("../data/source/gigantes-live-20261016.json")).unwrap();
  assert_eq!(r.at_command_menu.draws,9666);
  assert_eq!(r.actions.iter().map(|a|a.report.rng_after.draws).collect::<Vec<_>>(),[9687,9726,9767,9825,9891,9926,9957]);
  assert_eq!(r.after.draws,9958);
  // User confirmed eight Barrage hits, followed by a break on the first sword hit.
  let barrage=r.actions.iter().find(|a|a.name=="Elemental Barrage").unwrap();
  assert_eq!(barrage.report.events.iter().filter(|e|matches!(e,Event::Damage{..})).count(),8);
  assert!(!barrage.report.events.iter().any(|e|matches!(e,Event::Broken{..})));
  let sword=r.actions.iter().find(|a|a.name=="Sword x3").unwrap();
  let hit_breaks=sword.report.events.iter().filter_map(|e|match e {Event::Damage{broken_before,..}=>Some(*broken_before),_=>None}).collect::<Vec<_>>();
  assert_eq!(hit_breaks,[false,true,true]);
  let mut checked=0;
  for span in &r.trace {
   for pos in span.before.draws..span.after.draws {
    let row=&live["records"][(pos-9597) as usize];
    let mut rng=span.before;rng.advance(pos-span.before.draws);
    assert_eq!(row["before"].as_u64(),Some(u64::from(rng.state)),"state at {pos}");
    let node=row["node"].as_str().unwrap_or("");
    if span.reason.ends_with("voice") {assert!(node.ends_with(".PlayBattleVoice"),"{pos}: {} vs {node}",span.reason);}
    if span.reason.ends_with("timer") {assert_eq!(row["caller"].as_str(),Some("0x2C52E57"),"timer at {pos}");}
    if span.reason=="normal_accuracy" {assert!(node.ends_with(".CheckHit"),"accuracy at {pos}: {node}");}
    if span.reason=="critical_hit_and_damage" {assert!(node.ends_with(".CheckCritical")||node.ends_with(".CheckHit")||node.ends_with(".GetCalcValueRandomRatio"),"damage at {pos}: {node}");}
    checked+=1;
   }
  }
  assert_eq!(checked,live["records"].as_array().unwrap().len());
 }
 #[test]fn geared_routine_always_attacks_and_keeps_both_breakers_before_beastlings() {
  let mut players:Vec<PlayerCharacter>=serde_json::from_str(include_str!("../data/source/lord-sands-party.json")).unwrap();
  for i in [1,3] {players[i].boost_start=true;}
  let o=players[1].combat.as_mut().unwrap();o.magic=499;o.fire_damage_bonus=60;
  o.weapons.iter_mut().find(|w|w.weapon==crate::combat::Weapon::Sword).unwrap().magic=298;
  players[0].stats.speed+=112;
  let floor=damage_floor(&players).unwrap();assert!(floor.total>50000,"{floor:?}");
  let mut counts=std::collections::BTreeSet::new();let mut first=std::collections::BTreeSet::new();let mut partitio_before_both=false;
  for seed in 0u32..256 {
   let r=predict(CrtRng::new(seed.wrapping_mul(1775700753)),&players,&Default::default()).unwrap();
   let barrage=r.actions.iter().find(|a|a.name=="Elemental Barrage").unwrap();
   let hits=barrage.report.events.iter().filter(|e|matches!(e,Event::Damage{..})).count();counts.insert(hits);
   let opening:Vec<_>=r.actions.iter().filter(|a|a.turn==1).collect();
   let position=|actor|opening.iter().position(|a|a.actor==actor).unwrap();
   partitio_before_both|=position(3)<position(0)&&position(3)<position(1);
   assert_eq!(opening[position(3)].name,"Lance attack");assert_eq!(opening[position(3)].bp,0);
   assert_eq!(opening[position(2)].name,"Staff attack");assert_eq!(opening[position(2)].bp,0);
   let second:Vec<_>=r.actions.iter().filter(|a|a.turn==2).map(|a|a.actor).collect();
   assert!(matches!(second[..2],[0,1]|[1,0]),"Breakers must lead T2: {second:?}");
   assert_eq!(r.actions.last().unwrap().name,"Hired Help: Beastlings");
   first.insert(r.actions.iter().find(|a|a.turn==2).unwrap().actor);
   assert!(r.broken);assert!(r.actions.iter().all(|a|a.turn<=2));
   for pair in r.trace.windows(2){assert_eq!(pair[0].after,pair[1].before);}
   assert_eq!(r.trace.last().unwrap().after,r.after);
  }
  assert_eq!(counts.into_iter().collect::<Vec<_>>(),[6,7,8]);
  assert!(partitio_before_both,"Exercise the former conditional-Defend branch");
  assert!(first.contains(&0)&&first.contains(&1));
 }
 #[test]fn faster_sands_has_six_hits_and_preserves_finisher_prefix() {
  let mut players:Vec<PlayerCharacter>=serde_json::from_str(include_str!("../data/source/lord-sands-party.json")).unwrap();
  for i in [1,3] {players[i].boost_start=true;}
  for seed in 0u32..128 {
   let rng=CrtRng::new(seed.wrapping_mul(1775700753));
   let a=predict_sands(rng,&players,&Default::default(),0).unwrap();
   let b=predict_sands(rng,&players,&Default::default(),2).unwrap();
   assert_eq!(a.actions.iter().find(|a|a.name=="Aggressive Slash").unwrap().report.events.iter().filter(|e|matches!(e,Event::Damage{..})).count(),6);
   assert!(a.actions.iter().all(|a|a.turn<=2));
   for (x,y) in a.actions.iter().zip(&b.actions).take(a.actions.len()-1){assert_eq!(x.report.rng_after,y.report.rng_after);}
   assert_ne!(a.after,b.after);
  }
 }
}

