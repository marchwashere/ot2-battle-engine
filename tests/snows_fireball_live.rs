use ot2_battle_engine::{field_escape::opening_spell_break_menu,escape::Situation,rng::CrtRng};
#[test]
fn retail_two_mole_fireball_escape_callbacks_match() {
 let f:serde_json::Value=serde_json::from_str(include_str!("../data/source/live-snows-fireball-20261007.json")).unwrap();
 let players=serde_json::from_value::<Vec<ot2_battle_engine::PlayerCharacter>>(f["players"].clone()).unwrap();
 let history=serde_json::from_value(f["history"].clone()).unwrap();
 let entry:CrtRng=serde_json::from_value(f["entry"].clone()).unwrap();
 let b=opening_spell_break_menu(entry,Situation::Normal,&[5,1],&["ENE_NML_FLD_SNW_010","ENE_NML_FLD_SNW_010"],"ABI_SET_SCH_010",&[],false,&history,&players).unwrap().unwrap().attempt(&[1,0,1,0,1,0,1,0]).unwrap();
 let offset=f["title_battle_entry"].as_u64().unwrap();
 assert_eq!(offset+b.at_attack_menu.draws,12035637);
 assert_eq!(offset+b.at_escape_menu.draws,12035651);
 let spans:Vec<_>=b.attempt.trace.iter().filter(|s|s.before.draws>=b.at_attack_menu.draws).collect();
 assert_eq!(spans.iter().filter(|s|s.reason=="break_slow_timer").count(),1);
 assert_eq!(spans.iter().find(|s|s.reason=="order_voice").map(|s|s.after.draws-s.before.draws),Some(7));
 let expected=["ability_voice","critical_hit_and_damage","damage_notification_timer","critical_hit_and_damage","damage_notification_timer","break_voice","break_slow_timer","order_voice"];
 assert_eq!(spans.iter().take(8).map(|s|s.reason).collect::<Vec<_>>(),expected);
 for span in &spans {if span.after.draws>span.before.draws {let record=f["records"].as_array().unwrap().iter().find(|r|r["position"].as_u64()==Some(offset+span.before.draws)).unwrap();assert_eq!(record["state"].as_u64(),Some(u64::from(span.before.state)));}}
 let mut returned=b.attempt.after;returned.next_u15();returned.next_u15();
 assert_eq!(offset+returned.draws,12035657);assert_eq!(u64::from(returned.state),f["field_state"].as_u64().unwrap());
 // Second capture: user confirmed no pre-Fireball boost, then only three cycles.
 let three=opening_spell_break_menu(entry,Situation::Normal,&[5,1],&["ENE_NML_FLD_SNW_010","ENE_NML_FLD_SNW_010"],"ABI_SET_SCH_010",&[],false,&history,&players).unwrap().unwrap().attempt(&[1,0,1,0,1,0]).unwrap();
 let mut returned=three.attempt.after;returned.next_u15();returned.next_u15();
 assert_eq!(offset+three.at_escape_menu.draws,12035651);
 assert_eq!(offset+returned.draws,12035656);
 assert_eq!(returned.state,0x18A17F08);
}

#[test]
fn boosted_fireball_saves_cancel_and_preserves_safe_escape() {
 let f:serde_json::Value=serde_json::from_str(include_str!("../data/source/live-snows-fireball-20261007.json")).unwrap();
 let players=serde_json::from_value::<Vec<ot2_battle_engine::PlayerCharacter>>(f["players"].clone()).unwrap();
 let history=serde_json::from_value(f["history"].clone()).unwrap();
 let entry=serde_json::from_value(f["entry"].clone()).unwrap();
 let ids=["ENE_NML_FLD_SNW_010","ENE_NML_FLD_SNW_010"];
 let run=|inputs:&[u8]| opening_spell_break_menu(entry,Situation::Normal,&[5,1],&ids,"ABI_SET_SCH_010",inputs,false,&history,&players).unwrap().unwrap().attempt(&[1,0,1,0,1,0]).unwrap();
 let normal=run(&[1,0]); let boosted=run(&[1]);
 assert_eq!(boosted.boost,1);
 assert_eq!(boosted.attack_inputs,vec![1]);
 assert_eq!(boosted.at_attack_menu,normal.at_attack_menu);
 assert_eq!(boosted.at_escape_menu,normal.at_escape_menu);
 assert_eq!(boosted.attempt.after,normal.attempt.after);
 assert_eq!(boosted.attempt.party_sp_cost,normal.attempt.party_sp_cost);
 assert!(boosted.attempt.success);
 let damage=|b:&ot2_battle_engine::field_escape::FieldBreakEscape| b.report.events.iter().filter_map(|e|if let ot2_battle_engine::Event::Damage {amount,..}=e {Some(*amount)}else{None}).collect::<Vec<_>>();
 let a=damage(&normal);let b=damage(&boosted);assert_eq!(b.len(),2);
 for (low,high) in a.into_iter().zip(b) {assert!(high>low && high<208);}
 assert_eq!(boosted.report.events.iter().filter(|e|matches!(e,ot2_battle_engine::Event::Broken {..})).count(),2);
}
