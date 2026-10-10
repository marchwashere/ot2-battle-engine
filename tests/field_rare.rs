use ot2_battle_engine::{CrtRng,escape::Situation,field_escape::{opening_shadow_soulstone_m,FieldEscapeMenu}};
#[test]
fn shadow_m_clears_cultured_cait_and_dark_weak_yak_with_one_item_voice() {
    let enemies=["ENE_NML_OTR_140","ENE_NML_FLD_SNW_150","ENE_NML_FLD_SNW_110"];
    for seed in 0..8 {
        let entry=CrtRng {state:seed,draws:6000};
        let p=opening_shadow_soulstone_m(entry,Situation::Normal,&[5,1],&enemies,false).unwrap().unwrap();
        let menu=FieldEscapeMenu::new_party(entry,Situation::Normal,[5,1],&enemies).unwrap();
        assert_eq!(p.at_command_menu,menu.rng);
        assert_eq!(p.trace.iter().filter(|s|s.reason=="item_voice").count(),1,"AoE command plays its item voice once, not per target");
        assert_eq!(p.trace.iter().filter(|s|s.reason=="enemy_drop").count(),3);
        assert!(p.base_jp>=130);
        let mut previous=entry;
        for s in &p.trace {assert_eq!(s.before,previous);previous=s.after;}
        assert_eq!(previous,p.after);
    }
}
#[test]
fn rejects_surprise_and_a_pack_surviving_one_soulstone() {
    assert!(opening_shadow_soulstone_m(CrtRng::new(0),Situation::Surprise,&[5,1],&["ENE_NML_OTR_140"],false).unwrap().is_none());
    assert!(opening_shadow_soulstone_m(CrtRng::new(0),Situation::Normal,&[5,1],&["ENE_NML_OTR_140","ENE_NML_OTR_110"],true).unwrap().is_none());
}
