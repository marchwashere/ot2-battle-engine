use ot2_battle_engine::{*,escape::Situation,field_escape::{carried_step_ahead,route_player},knowledge::EnemyHistory};

fn opening(seed:u32)->Battle {
    let party:Vec<_>=[7,1,8,4].map(|id|route_player(id,5000).unwrap()).into();
    carried_step_ahead(CrtRng::new(seed),Situation::Normal,&party,
        &["ENE_NML_FLD_WLD_250","ENE_NML_FLD_WLD_160","ENE_NML_OTR_150"],true,&EnemyHistory::new()).unwrap()
}
#[test]
fn bonus_turn_has_four_players_and_preserves_normal_preview() {
    for seed in [0,31,3285,123456] {
        let mut b=opening(seed);
        assert_eq!(b.state().turn_order().len(),4);
        assert!(b.state().turn_order().iter().all(|id|b.side(*id)==Some(Side::Player)));
        assert_eq!(b.state().next_queue.len(),7);
        for _ in 0..4 {b.step_player(Command::Defend).unwrap();}
        assert_eq!(b.state().round,2);
        assert!(b.state().actors[..4].iter().all(|a|a.resources.bp==2));
        let trace=b.state().opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
        assert_eq!(trace.iter().filter(|s|s.reason=="step_ahead_current_order_shuffle").count(),1);
        assert_eq!(trace.iter().filter(|s|s.reason=="round_bp_timer").count(),1);
    }
}
#[test]
fn setup_actions_update_resources_and_order() {
    // Exercise every possible first actor across deterministic initializations.
    let mut seen=[false;4];
    for seed in 0..100 {
        let mut b=opening(seed);let actor=b.current_actor().unwrap();seen[actor.0]=true;
        match actor.0 {
            0=>{
                b.step_player(Command::Skill {id:"springy_boots".into(),target:Some(ActorId(2))}).unwrap();
                assert_eq!(b.state().next_queue[0],ActorId(2));
                assert!(b.state().actors[2].has_status(combat::StatusKind::ActFirst));
            }
            2=>{
                let before=b.state().turn_order().to_vec();let target=before[3];let sp=b.state().actors[2].resources.sp;
                b.step_player(Command::Skill {id:"stimulate".into(),target:Some(target)}).unwrap();
                assert_eq!(b.state().turn_order()[2],target);
                assert_eq!(b.state().actors[2].resources.sp,sp-5);
            }
            _=>{
                b.step_player(Command::Item {id:"energizing_pomegranate_m".into(),target:Some(ActorId(2))}).unwrap();
                assert_eq!(b.state().actors[2].resources.bp,3);
            }
        }
    }
    assert!(seen.into_iter().all(|v|v));
}
#[test]
fn guards_source_damage_uses_cat_base_before_multipliers() {
    use combat::*;
    let skill=Attack {kind:DamageKind::Physical,weapon:Some(Weapon::Staff),element:None,invocation:150,ability_ratio:280,hit_bonus:0,critical_bonus:0,critical_power:125,normal_attack:false};
    let input=DamageInput {attack:320,defense:400,growth:320,guard:false,weapon_weak:true,element_weak:false,broken:false,critical:false,attack_up:false,defense_down:false,defense_up:false};
    let mut range=(u32::MAX,0);
    for raw in 0..=32767 {
        let variance=1.01_f32+(1.05_f32-1.01_f32)*(raw as f32*3.051851e-5_f32);
        let damage=damage_with_cat_type(input,&skill,variance,true);
        range.0=range.0.min(damage);range.1=range.1.max(damage);
    }
    assert_eq!(range,(31,32));
    assert_eq!(elemental_fixed_damage(2700,false,false,true),27);
    assert!(2*range.0+27>=80);
    assert!(range.1+27<80);
}
#[test]
fn search_resumes_and_replays_each_unique_state_exactly() {
    use fight_search::BoostFrontier;
    let battle=opening(77);let before=battle.state().clone();
    let command=Command::Item {id:"energizing_pomegranate_m".into(),target:Some(ActorId(2))};
    let mut search=BoostFrontier::new(&battle).unwrap();
    let first=search.next(&command,0).unwrap().unwrap();assert!(first.inputs.is_empty());
    assert!(search.next(&command,0).unwrap().is_none());
    let second=search.next(&command,2).unwrap().unwrap();assert_eq!(second.inputs,vec![1,0]);
    let mut reference=battle.clone();let report=reference.step_player_with_boost_inputs(command,&second.inputs).unwrap();
    assert_eq!(reference.state(),second.after_action.state());assert_eq!(report.rng_after,second.report.rng_after);
    assert_eq!(battle.state(),&before);
}

#[test]
fn galdera_merchant_osvald_search_replays_exp100_with_everyone_alive() {
    let mut party:Vec<_>=[7,6,8,4].map(|id|route_player(id,5000).unwrap()).into();
    jobs::change_secondary_job(&mut party[1],6,field_escape::level_for_experience(5000),None,Some(0)).unwrap();
    let battle=carried_step_ahead(CrtRng::new(77),Situation::Normal,&party,
        &["ENE_NML_FLD_WLD_250","ENE_NML_FLD_WLD_160","ENE_NML_OTR_150"],true,&EnemyHistory::new()).unwrap();
    let fight=gravell_routine::search(battle.clone(),4096).unwrap().expect("Galdera EXP100 victory");
    assert_eq!(fight.evaluation,gravell_routine::Evaluation::Accepted);
    assert_eq!(fight.state.outcome,Outcome::Victory);
    assert!(fight.state.actors[..4].iter().all(|p|p.alive()));
    assert_eq!(fight.state.grace.as_ref().unwrap().exp_percent(),10_000);
    let bg=&fight.actions.iter().find(|a|a.action==gravell_routine::Action::BewilderingGrace).unwrap().inputs;
    let guards=fight.actions.iter().find(|a|a.action==gravell_routine::Action::Guards).unwrap().inputs.as_slice();
    let unchanged=gravell_routine::replay_finisher(battle.clone(),&fight,&[]).unwrap();
    assert_eq!(unchanged.state,fight.state);
    let menu=gravell_routine::finisher_menu(battle.clone(),&fight).unwrap();
    assert_eq!(menu.state().actors[menu.current_actor().unwrap().0].resources.bp,0);
    assert!(gravell_routine::replay_finisher(battle.clone(),&fight,&[1,0]).is_err());
    let replay=gravell_routine::replay(battle,bg,guards).unwrap();
    assert_eq!(replay.state,fight.state);
    assert_eq!(replay.after,fight.after);
    assert!(gravell_routine::rewards(&fight,true).unwrap().awarded.exp>=50_000);
}
#[test]
fn late_finisher_adjustment_replays_callbacks_and_preserves_the_frozen_prefix() {
    for seed in 0..128 {
        let battle=opening(seed);
        let Some(fight)=gravell_routine::search(battle.clone(),4096).unwrap() else {continue;};
        let menu=gravell_routine::finisher_menu(battle.clone(),&fight).unwrap();
        if menu.state().actors[menu.current_actor().unwrap().0].resources.bp==0 {continue;}
        let mut direct=menu;
        direct.step_player_with_boost_inputs(gravell_routine::Action::ThunderSoulstone.command(),&[1,0]).unwrap();
        let adjusted=gravell_routine::replay_finisher(battle,&fight,&[1,0]).unwrap();
        assert_eq!(adjusted.state,*direct.state());
        assert_eq!(adjusted.evaluation,gravell_routine::Evaluation::Accepted);
        assert_eq!(serde_json::to_value(&adjusted.actions[..adjusted.actions.len()-1]).unwrap(),
            serde_json::to_value(&fight.actions[..fight.actions.len()-1]).unwrap());
        assert_eq!(adjusted.actions.last().unwrap().inputs,vec![1,0]);
        assert!(adjusted.after.draws>fight.after.draws);
        return;
    }
    panic!("No finisher with available BP tested");
}
#[test]
fn complete_fight_search_replays_victory_exp100_and_all_four_rewards() {
    let battle=opening(77);
    let fight=gravell_routine::search(battle.clone(),4096).unwrap().expect("winning routine within test frontier");
    assert_eq!(fight.evaluation,gravell_routine::Evaluation::Accepted);
    assert_eq!(fight.state.outcome,Outcome::Victory);
    assert!(fight.state.actors[..4].iter().all(|p|p.alive()));
    assert_eq!(fight.state.grace.as_ref().unwrap().exp_percent(),10_000);
    let bg=&fight.actions.iter().find(|a|a.action==gravell_routine::Action::BewilderingGrace).unwrap().inputs;
    let guards=fight.actions.iter().find(|a|a.action==gravell_routine::Action::Guards).map_or(&[][..],|a|a.inputs.as_slice());
    let work=std::cell::Cell::new(0);
    let target=fight.trace.iter().find(|s|s.reason=="bewildering_grace_lottery").unwrap().before.state;
    assert!(gravell_routine::search_target_windowed(battle.clone(),4096,target,12,Some(&work)).unwrap().is_none());
    let reference=gravell_routine::replay(battle,bg,guards).unwrap();
    assert_eq!(reference.state,fight.state);
    let reward=gravell_routine::rewards(&fight,true).unwrap();
    assert!(reward.awarded.exp>=reward.base.exp*100);
    assert_eq!(reward.hired_help_cost,7_000);
    assert!(reward.after.draws>=fight.after.draws);
    println!("Gravell {:?}: {} inputs, {} calls",fight.evaluation,fight.actions.iter().map(|a|a.inputs.len()).sum::<usize>(),fight.after.draws);
}

#[test]
fn donation_funds_four_guards_and_defend_promotes_agnea() {
    let mut donors=std::collections::BTreeSet::new();
    for seed in 0..100 {
        let mut battle=opening(seed);
        let plan=gravell_routine::OpeningPlan::from_battle(&battle).unwrap();
        for row in plan.bonus_turn {
            if battle.state().outcome!=Outcome::Ongoing {break;}
            assert_eq!(battle.current_actor(),Some(row.actor));
            let before=battle.state().actors[row.actor.0].resources.clone();
            let report=battle.step_player(row.action.command()).unwrap();
            match row.action {
                gravell_routine::Action::DonateBp {target}=>{
                    donors.insert(row.actor);
                    assert_eq!(battle.state().actors[target.0].resources.bp,3);
                    assert_eq!(battle.state().actors[row.actor.0].resources.sp,before.sp-6);
                    assert_eq!(battle.state().actors[row.actor.0].resources.bp,0);
                },
                gravell_routine::Action::Guards=>{
                    assert_eq!(before.bp,3);
                    assert!(report.events.iter().any(|e|matches!(e,Event::ResourceChanged {resource:Resource::Bp,before:3,after:0,..})));
                    assert_eq!(battle.state().opening_crt.as_ref().unwrap().trace.as_ref().unwrap().iter().filter(|s|s.reason=="summon_ability_voice").count(),4);
                },_=>()
            }
        }
        if battle.state().outcome==Outcome::Ongoing {
            assert_eq!(battle.state().round,2);
            assert_eq!(battle.current_actor(),battle.actor_id("agnea"));
            assert!(battle.state().actors[2].resources.bp>=3);
        }
    }
    assert_eq!(donors.len(),2);
}

#[test]
fn step_ahead_escape_projection_and_item_clear_share_real_bonus_menu() {
 let party:Vec<_>=[7,1,8,4].map(|id|route_player(id,5000).unwrap()).into();
 let ids=["ENE_NML_FLD_SEA_130"];
 for seed in 0..8 {
  let entry=CrtRng::new(seed);
  let menu=field_escape::FieldEscapeMenu::with_step_ahead(entry,Situation::Normal,&party,&[14;4],&ids,&EnemyHistory::new()).unwrap();
  assert_eq!(menu.current_order.len(),4);
  let win=field_escape::carried_soulstone_with_step_ahead(entry,Situation::Normal,&party,&[14;4],&ids,false,&EnemyHistory::new(),&[1,0],"wind_soulstone_l",true).unwrap().unwrap();
  assert_eq!(win.actor,menu.first_actor);
  assert_eq!(win.at_command_menu,menu.rng);
  let attempt=menu.attempt(&[1,0],20).unwrap();
  let boost=win.trace.iter().find(|s|s.reason=="boost_voice");
  if let Some(boost)=boost {assert!(boost.after.draws<=attempt.before_check.draws);}
 }
}


#[test]
fn seven_defends_promote_the_whole_party_and_replay_both_rounds() {
 let party:Vec<_>=[7,6,8,4].map(|id|route_player(id,5000).unwrap()).into();
 let ids=["ENE_NML_FLD_SEA_130"];
 for situation in [Situation::Normal,Situation::Preemptive] {
 for seed in 0..16 {
  let mut rng=CrtRng::new(seed);rng.advance(200);
  let mut battle=carried_step_ahead(rng,situation,&party,&ids,false,&EnemyHistory::new()).unwrap();
  let mut menu=field_escape::FieldEscapeMenu::with_step_ahead(rng,situation,&party,&[14;4],&ids,&EnemyHistory::new()).unwrap();
  for i in 0..7 {
   assert_eq!(menu.round(),if i<4 {1}else{2});
   let (next,report)=menu.safe_step_ahead_defend_through_next_round().unwrap().unwrap();
   let actual=battle.step_player(Command::Defend).unwrap();
   assert_eq!(report.rng_after.state,actual.rng_after.state);
   assert_eq!(next.rng.draws,rng.draws+battle.state().rng.draws);
   assert_eq!(next.first_actor,battle.current_actor().unwrap().0);
   assert!(next.first_actor<4);menu=next;
  }
  assert_eq!(menu.round(),2);
  assert!(menu.safe_step_ahead_defend_through_next_round().unwrap().is_none());
  let win=field_escape::carried_soulstone_after_defends(rng,situation,&party,&[14;4],&ids,false,&EnemyHistory::new(),&[1,0],"wind_soulstone_l",true,7).unwrap().unwrap();
  assert_eq!(win.at_command_menu,menu.rng);assert_eq!(win.actor,menu.first_actor);
  assert!(menu.attempt(&[1,0],1).unwrap().success);
 }
 }
}
#[test]
fn safe_step_ahead_defends_stop_before_enemies_and_match_battle_rng() {
 let party:Vec<_>=[7,1,8,4].map(|id|route_player(id,5000).unwrap()).into();
 let ids=["ENE_NML_FLD_SEA_130"];
 let mut rng=CrtRng::new(39);rng.advance(200);
 let mut battle=carried_step_ahead(rng,Situation::Normal,&party,&ids,false,&EnemyHistory::new()).unwrap();
 let mut menu=field_escape::FieldEscapeMenu::with_step_ahead(rng,Situation::Normal,&party,&[14;4],&ids,&EnemyHistory::new()).unwrap();
 for _ in 0..3 {
  let (next,report)=menu.safe_step_ahead_defend().unwrap().unwrap();
  let actual=battle.step_player(Command::Defend).unwrap();
  assert_eq!(report.rng_after.state,actual.rng_after.state);
  assert_eq!(next.rng.draws,rng.draws+battle.state().rng.draws);
  assert_eq!(next.first_actor,battle.current_actor().unwrap().0);
  assert_eq!(battle.state().round,1);menu=next;
 }
 assert!(menu.safe_step_ahead_defend().unwrap().is_none());
}

#[test]
fn exp100_target_can_be_a_later_executed_bg_child() {
 for seed in 0..16 {
  let battle=opening(seed);
  let Some(fight)=gravell_routine::search(battle.clone(),512).unwrap() else {continue};
  let lotteries:Vec<_>=fight.trace.iter().filter(|s|s.reason=="bewildering_grace_lottery").collect();
  let target=lotteries.iter().enumerate().find_map(|(i,s)|{
   let mut rng=s.before;
   let ability=bewildering_grace::Dance::new(3).unwrap().select(&mut rng).unwrap().unwrap().ability;
   (i>0&&ability=="ABI_DAN_MAKA_002").then_some(s.before.state)
  });
  if let Some(target)=target {
   let found=gravell_routine::search_target(battle,512,Some(target)).unwrap().unwrap();
   assert_eq!(found.evaluation,gravell_routine::Evaluation::Accepted);
   assert!(found.trace.iter().any(|s|s.reason=="bewildering_grace_lottery"&&s.before.state==target));
   return;
  }
 }
 panic!("No later-child EXP100 case exercised");
}

#[test]
fn bg_early_victory_does_not_credit_unexecuted_exp100() {
 for seed in 0..512 {
  let fight=gravell_routine::replay(opening(seed),&[1,2,3],&[1,2,3]).unwrap();
  let lotteries:Vec<_>=fight.trace.iter().filter(|s|s.reason=="bewildering_grace_lottery").collect();
  if fight.state.outcome==Outcome::Victory&&lotteries.len()>0&&lotteries.len()<4
    &&fight.state.grace.as_ref().unwrap().exp_percent()!=10_000 {
   assert!(matches!(fight.evaluation,gravell_routine::Evaluation::Loss {reasons}
    if reasons.contains(&gravell_routine::LossReason::MissingExp100)));
   assert!(!fight.actions.iter().any(|a|a.action==gravell_routine::Action::ThunderSoulstone));
   return;
  }
 }
 panic!("No early BG victory exercised");
}

#[test]
fn step_ahead_supports_four_players_with_five_or_six_enemies() {
 let party:Vec<_>=[7,1,8,4].map(|id|route_player(id,5000).unwrap()).into();
 for enemies in 5..=6 {
  let ids=vec!["ENE_NML_FLD_SEA_130";enemies];
  for seed in 0..8 {
   let mut menu=field_escape::FieldEscapeMenu::with_step_ahead(CrtRng::new(seed),Situation::Normal,&party,&[14;4],&ids,&EnemyHistory::new()).unwrap();
   assert_eq!(menu.current_order.len(),4);assert_eq!(menu.next_order.len(),4+enemies);
   for _ in 0..3 {menu=menu.safe_step_ahead_defend().unwrap().unwrap().0;}
   assert!(menu.safe_step_ahead_defend().unwrap().is_none());
   menu.attempt(&[1,0],20).unwrap();
  }
 }
}

#[test]
fn all_source_rare_replacements_support_escape_and_step_ahead() {
 let source:serde_json::Value=serde_json::from_str(include_str!("../data/source/highroad-return.json")).unwrap();
 let party:Vec<_>=[7,1,8,4].map(|id|route_player(id,5000).unwrap()).into();
 let rewards:serde_json::Value=serde_json::from_str(include_str!("../data/field_rare_rewards.json")).unwrap();
 let rare=source["rare_enemy_ids"].as_array().unwrap();
 assert!(rare.iter().any(|r|r=="ENE_NML_WTR_580"));assert_eq!(rare.len(),13);
 for enemy in rare {
  let id=enemy.as_str().unwrap();assert!(rewards.get(id).is_some());
  let ids=["ENE_NML_FLD_WLD_010",id];
  for situation in [Situation::Normal,Situation::Preemptive] {
   let entry=CrtRng::new(42);
   let ordinary=field_escape::FieldEscapeMenu::with_players(entry,situation,&party,&[14;4],&ids,&EnemyHistory::new()).unwrap();
   ordinary.attempt(&[1,0],20).unwrap();
   let step=field_escape::FieldEscapeMenu::with_step_ahead(entry,situation,&party,&[14;4],&ids,&EnemyHistory::new()).unwrap();
   let after=step.safe_step_ahead_defend().unwrap().unwrap().0;
   after.attempt(&[1,0],20).unwrap();
  }
 }
}

#[test]
fn target_deadline_and_cross_frontier_dedup_preserve_reachable_states() {
    use fight_search::BoostFrontier;
    let battle=opening(77);
    let command=Command::Item {id:"energizing_pomegranate_m".into(),target:Some(ActorId(2))};
    let deadline=battle.state().rng.draws+2;
    let mut reference=BoostFrontier::new(&battle).unwrap();
    let mut expected=vec![];
    while let Some(c)=reference.next(&command,8).unwrap() {
        if c.rng.draws<=deadline {expected.push((c.inputs,c.after_action.state().clone()));}
    }
    let mut pruned=BoostFrontier::new(&battle).unwrap();
    let mut actual=vec![];
    let mut seen=std::collections::BTreeMap::new();
    while let Some(c)=pruned.next_unique_before(&command,8,Some(deadline),&mut seen).unwrap() {
        actual.push((c.inputs,c.after_action.state().clone()));
    }
    assert_eq!(actual,expected);
    assert!(!actual.is_empty());
    let mut duplicate=BoostFrontier::new(&battle).unwrap();
    assert!(duplicate.next_unique_before(&command,8,Some(deadline),&mut seen).unwrap().is_none());
}
