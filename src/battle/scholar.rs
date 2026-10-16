//! Advanced Magic's use counter and Fire Storm's authored multi-target timeline.
use super::*;
impl Battle {
    pub(super) fn scholar_cast_voice(&mut self,actor:ActorId,child:&crate::bg_effects::Child) {
        let ability=if child.id.starts_with("ABI_SCH_011_") {"ABI_SCH_011_01"}else{"ABI_SCH_070_01"};
        let before=self.state.rng;
        crate::scholar_magic::cast_voice(&mut self.state.rng,&self.scenario.actors[actor.0].id,ability).expect("validated Scholar caster");
        self.trace_rng("ability_voice",Some(actor),before);
    }
    pub(super) fn scholar_step(&mut self,command:Command,levels:Option<&[u8]>)->Result<StepReport> {
        ensure(self.state.opening_crt.is_some(),"Scholar magic requires a CRT battle profile")?;
        let actor=self.current_actor().ok_or_else(||SimError("Battle ended".into()))?;
        let (id,bp,target)=match command {Command::Skill{id,target}=>(id,0,target),Command::Boost{id,bp,target}=>(id,bp,target),_=>unreachable!()};
        let advanced=id=="ABI_SET_SCH_070";
        let learned=self.catalog().skill_id(if advanced {"ABI_SET_SCH_070"}else{"ABI_SET_SCH_010"})?;
        ensure(self.scenario.actors[actor.0].skills.contains(&learned),"Skill not learned")?;
        ensure(advanced||self.state.actors[actor.0].has_status(StatusKind::AdvancedMagic),"Fire Storm requires Advanced Magic")?;
        ensure(!self.state.actors[actor.0].has_status(StatusKind::Silence),"Silence prevents this skill")?;
        let cost=if advanced {15}else{14};
        ensure(bp<=3&&self.state.actors[actor.0].resources.bp>=bp,"Not enough BP")?;
        ensure(self.state.actors[actor.0].resources.sp>=cost,"Not enough SP")?;
        let targets=if advanced {
            let target=target.ok_or_else(||SimError("Advanced Magic requires an ally target".into()))?;
            ensure(self.side(target)==Some(Side::Player)&&self.state.actors[target.0].alive(),"Advanced Magic requires a living ally")?;vec![target]
        }else{
            ensure(target.is_none(),"Fire Storm targets all enemies")?;
            (0..self.state.actors.len()).map(ActorId).filter(|t|self.side(*t)==Some(Side::Enemy)&&self.state.actors[t.0].alive()).collect()
        };
        let source=format!("ABI_SCH_{}_0{}",if advanced {"070"}else{"011"},bp+1);
        let child=crate::bg_effects::child(&source)?;
        // Validate voice availability before any mutation; all public steps are transactional.
        crate::scholar_magic::cast_voice(&mut self.state.rng.clone(),&self.scenario.actors[actor.0].id,if advanced {"ABI_SCH_070_01"}else{"ABI_SCH_011_01"})?;
        let before=self.state.rng;
        let normal:Vec<_>=(1..=bp).collect();let inputs=levels.unwrap_or(&normal);
        self.validate_boost_inputs(inputs,bp)?;self.apply_boost_inputs(inputs);
        let mut events=vec![Event::Action {actor,name:if advanced {"ABI_SET_SCH_070"}else{"ABI_SET_SCH_011"}.into()}];
        let state=&mut self.state.actors[actor.0];state.guard=false;state.boosted_this_round|=bp>0;
        if bp>0 {let old=state.resources.bp;state.resources.bp-=bp;events.push(Event::ResourceChanged {actor,resource:Resource::Bp,before:old.into(),after:state.resources.bp.into()});}
        let old=state.resources.sp;state.resources.sp-=cost;events.push(Event::ResourceChanged {actor,resource:Resource::Sp,before:old,after:state.resources.sp});
        if !advanced {state.main_weapon=self.scenario.actors[actor.0].combat.as_ref().ok_or_else(||SimError("Magic requires combat profile".into()))?.weapons.iter().max_by_key(|w|w.magic).map(|w|w.weapon);}
        self.state.opening_crt.as_mut().unwrap().settled_damage.fill(false);
        self.grace_common_child(actor,child,&targets,&mut events)?;
        if !advanced {
            // BP_ActionCommandBase.ExecPayPostCost: once per upgraded action,
            // after its entire sequence, irrespective of hit/target count.
            let statuses=&mut self.state.actors[actor.0].statuses;
            if let Some(s)=statuses.iter_mut().find(|s|s.kind==StatusKind::AdvancedMagic) {if s.turns!=255 {s.turns=s.turns.saturating_sub(1);}}
            statuses.retain(|s|s.turns>0);
        }
        Ok(self.complete_player_action(actor,events,before))
    }
}
#[cfg(test)]
mod tests {
 use super::*;
 fn battle()->Battle {
  let mut config:BattleConfig=serde_json::from_str(include_str!("../../data/cape_cold_mug.json")).unwrap();
  config.players[0].skills.push("ABI_SET_SCH_070".into());
  config.players[0].stats.max_sp=999;config.players[0].resources.as_mut().unwrap().sp=999;
  config.players[0].resources.as_mut().unwrap().bp=5;
  config.enemies[0].stats.max_hp=100000;config.enemies[0].resources.as_mut().unwrap().hp=100000;
  config.enemies[0].combat.as_mut().unwrap().element_weaknesses=vec![combat::Element::Fire];
  config.enemies[0].combat.as_mut().unwrap().shields=8;
  let mut second=config.enemies[0].clone();second.id="scholar_test_second".into();config.enemies.push(second);
  Battle::new_carried_step_ahead(config,crate::escape::Situation::Normal,vec![1]).unwrap()
 }
 fn command(id:&str,bp:u8)->Command {Command::Boost{id:id.into(),target:(id=="ABI_SET_SCH_070").then_some(ActorId(0)),bp}}
 fn charges(b:&Battle)->u8 {b.state.actors[0].statuses.iter().find(|s|s.kind==StatusKind::AdvancedMagic).map_or(0,|s|s.turns)}
 fn reset_menu(b:&mut Battle){b.state.cursor=0;b.state.queue=vec![ActorId(0),ActorId(1),ActorId(2)];b.state.actors[0].resources.bp=5;}
 #[test] fn advanced_magic_all_boosts_have_exact_status_and_cast_voice_draws(){
  for bp in 0..=3 {let mut b=battle();let start=b.state.rng;let sp=b.state.actors[0].resources.sp;
   let r=b.step_player(command("ABI_SET_SCH_070",bp)).unwrap();assert_eq!(charges(&b),2+bp);assert_eq!(b.state.actors[0].resources.sp,sp-15);
   let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
   let draws=|name|trace.iter().filter(|s|s.before.draws>=start.draws&&s.reason==name).map(|s|s.after.draws-s.before.draws).sum::<u64>();
   assert_eq!(draws("ability_voice"),1);assert_eq!(draws("bg_status_application"),1);assert_eq!(draws("bg_status_duration"),1);assert_eq!(draws("bg_status_remaining_slots"),19);
   assert!(!r.events.iter().any(|e|matches!(e,Event::Damage{..})));
   let n=charges(&b);b.start_ot2_round(&mut vec![]);assert_eq!(charges(&b),n,"round transitions must not consume uses");
  }
 }
 #[test] fn fire_storm_replaces_fireball_hits_all_targets_twice_and_consumes_one_use(){
  for bp in 0..=3 {let mut b=battle();b.step_player(command("ABI_SET_SCH_070",0)).unwrap();reset_menu(&mut b);let before=b.clone();let sp=b.state.actors[0].resources.sp;
   let r=b.step_player(command("ABI_SET_SCH_010",bp)).unwrap();
   let hits:Vec<_>=r.events.iter().filter_map(|e|if let Event::Damage{target,..}=e{Some(target.0)}else{None}).collect();assert_eq!(hits,vec![1,2,1,2]);assert_eq!(charges(&b),1);assert_eq!(b.state.actors[0].resources.sp,sp-14);
   assert_eq!(b.state.actors[1].shields,6);assert_eq!(b.state.actors[2].shields,6);
   let mut replay=before;let rr=replay.step_player(command("ABI_SET_SCH_011",bp)).unwrap();assert_eq!(r.rng_after,rr.rng_after);assert_eq!(serde_json::to_value(&r).unwrap(),serde_json::to_value(rr).unwrap());
   reset_menu(&mut b);b.step_player(command("ABI_SET_SCH_010",0)).unwrap();assert_eq!(charges(&b),0);
   reset_menu(&mut b);let r=b.step_player(command("ABI_SET_SCH_010",0)).unwrap();assert_eq!(r.events.iter().filter(|e|matches!(e,Event::Damage{..})).count(),2,"returns to ordinary Fireball");
  }
 }
 #[test] fn scholar_invalid_commands_are_atomic_and_reapplication_caps_at_nine(){
  let mut b=battle();let before=serde_json::to_value(b.state()).unwrap();assert!(b.step_player(command("ABI_SET_SCH_011",0)).is_err());assert_eq!(serde_json::to_value(b.state()).unwrap(),before);
  b.step_player(command("ABI_SET_SCH_070",3)).unwrap();reset_menu(&mut b);b.step_player(command("ABI_SET_SCH_070",3)).unwrap();assert_eq!(charges(&b),9);
  reset_menu(&mut b);b.state.actors[0].statuses.push(Status{kind:StatusKind::Silence,turns:2});let before=serde_json::to_value(b.state()).unwrap();assert!(b.step_player(command("ABI_SET_SCH_010",0)).is_err());assert_eq!(serde_json::to_value(b.state()).unwrap(),before);
 }
 #[test] fn all_imported_cast_voices_use_one_draw(){for id in ["throne","osvald","agnea","partitio","hikari","temenos"] {for ability in ["ABI_SCH_011_01","ABI_SCH_070_01"] {let mut r=CrtRng::new(0);crate::scholar_magic::cast_voice(&mut r,id,ability).unwrap();assert_eq!(r.draws,1);}}}
 #[test] fn fire_storm_break_timer_is_shared_and_final_hit_keeps_prepared_targets(){
  let mut b=battle();b.step_player(command("ABI_SET_SCH_070",0)).unwrap();reset_menu(&mut b);
  for a in &mut b.state.actors[1..] {a.shields=1;}
  let start=b.state.rng.draws;b.step_player(command("ABI_SET_SCH_010",0)).unwrap();
  let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
  assert_eq!(trace.iter().filter(|s|s.before.draws>=start&&s.reason=="break_slow_timer").count(),1);
  assert_eq!(trace.iter().filter(|s|s.before.draws>=start&&s.reason=="broken_hit_slow_timer").count(),0,"ability camera suppresses finish-blow slow timers");
  let mut b=battle();b.step_player(command("ABI_SET_SCH_070",0)).unwrap();reset_menu(&mut b);
  for a in &mut b.state.actors[1..] {a.resources.hp=1;}
  let r=b.step_player(command("ABI_SET_SCH_010",0)).unwrap();
  assert_eq!(r.events.iter().filter(|e|matches!(e,Event::Damage{..})).count(),4);
  assert_eq!(r.outcome,Outcome::Victory);assert_eq!(charges(&b),1);
 }

}
