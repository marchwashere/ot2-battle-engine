use ot2_battle_engine::{ActorId,Battle,BattleConfig,ItemCatalog,pirro::PirroRoutine};

#[test]
fn selected_max_boost_is_not_charged_again() {
    for seed in [0,1,5678,19284723,u32::MAX] {
        let mut ordinary=Battle::new_pirro(BattleConfig::throne_pirro(seed).unwrap(),ItemCatalog::builtin().unwrap(),false).unwrap();
        let mut policy=PirroRoutine::default();
        while ordinary.state().round!=3 || ordinary.current_actor()!=Some(ActorId(0)) {
            policy.step(&mut ordinary).unwrap();
        }
        let mut explicit=ordinary.clone();
        for (from,to) in [(0,1),(1,2),(2,3)] {explicit.advance_boost_input(from,to).unwrap();}
        policy.step(&mut ordinary).unwrap();
        explicit.pirro_identified_darkest_night().unwrap();
        assert_eq!(ordinary.state(),explicit.state());
    }
}

#[test]
fn cannot_rebase_or_resume_at_an_unrelated_menu() {
    let mut b=Battle::new_pirro(BattleConfig::throne_pirro(0).unwrap(),ItemCatalog::builtin().unwrap(),false).unwrap();
    let before=b.state().clone();
    assert!(b.rebase_pirro_identification().is_err());
    assert!(b.pirro_identified_darkest_night().is_err());
    assert_eq!(b.state(),&before);
}
