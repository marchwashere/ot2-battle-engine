use ot2_battle_engine::{CrtRng, escape::Situation, field_escape::FieldEscapeMenu};

fn main() {
    let mut rare_entry=CrtRng::new(0);rare_entry.advance(6514);
    let rare=ot2_battle_engine::field_escape::opening_shadow_soulstone_m(rare_entry,Situation::Normal,&[5,1],
        &["ENE_NML_FLD_SNW_110","ENE_NML_OTR_140","ENE_NML_FLD_SNW_150"],false).unwrap().unwrap();
    println!("Rare history {:?}",rare.enemy_knowledge);
    let m=FieldEscapeMenu::with_history(CrtRng {state:923866087,draws:6581},Situation::Normal,&[14,14],
        &["ENE_NML_FLD_SNW_110","ENE_NML_FLD_SNW_110","ENE_NML_FLD_SNW_150"],&rare.enemy_knowledge).unwrap();
    println!("History menu {}",m.rng.draws);
    for span in m.trace.iter().filter(|s|s.reason=="study_foe_weakness") {println!("Study {} -> {}",span.before.draws,span.after.draws);}
    println!("History escape {:?}",m.clone().attempt(&[],14).unwrap().success);
    for n in 1..20 {
        let inputs:Vec<_>=(0..n).flat_map(|_|[1,0]).collect();
        let a=m.clone().attempt(&inputs,14).unwrap();
        if a.success {println!("Corrected escape: {n} cycles, check at {}",a.before_check.draws);break;}
    }

    let enemies = ["ENE_NML_FLD_SNW_110", "ENE_NML_FLD_SNW_110", "ENE_NML_FLD_SNW_150"];
    for level in [14, 15] {
        let menu = FieldEscapeMenu::new_party(
            CrtRng { state: 923866087, draws: 6581 },
            Situation::Normal, [level, level], &enemies,
        ).unwrap();
        let attempt = menu.clone().attempt(&[], 14).unwrap();
        println!("Lv{level}: menu {}, actor {}, order {:?}, Escape {}% / {}",
            menu.rng.draws, menu.first_actor, menu.current_order,
            attempt.chance_percent, attempt.success);
    }
}
