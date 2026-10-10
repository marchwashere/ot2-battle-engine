use ot2_battle_engine::{field_escape::opening_spell_break_menu, escape::Situation, rng::CrtRng};
fn main(){
 let v:serde_json::Value=serde_json::from_slice(&std::fs::read(std::env::args().nth(1).unwrap()).unwrap()).unwrap();
 let p=&v["prediction"];let leg=&p["post_battle_field"]["flats_escape"]["snows_escape"]["before_osvald"];
 let old=&leg["western"]["escape"];let w=&leg["eastern_entry"]["world_after"];
 let players:Vec<ot2_battle_engine::PlayerCharacter>=w["party"].as_array().unwrap().iter().map(|id|serde_json::from_value(w["character_progression"][id.to_string()]["character"].clone()).unwrap()).collect();
 let entry: CrtRng=serde_json::from_value(old["attempt"]["trace"][0]["before"].clone()).unwrap();
 let history=serde_json::from_value(w["enemy_knowledge"].clone()).unwrap();
 let b=opening_spell_break_menu(entry,Situation::Normal,&[5,1],&["ENE_NML_FLD_SNW_010","ENE_NML_FLD_SNW_010"],"ABI_SET_SCH_010",&[],false,&history,&players).unwrap().unwrap().attempt(&[1,0,1,0,1,0,1,0]).unwrap();
 println!("{}",serde_json::to_string(&b.attempt.trace).unwrap());println!("menus {:?} {:?}; after {:?}; field return +2",b.at_attack_menu,b.at_escape_menu,b.attempt.after);
}

