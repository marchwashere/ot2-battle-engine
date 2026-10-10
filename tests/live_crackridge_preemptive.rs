use ot2_battle_engine::{*, escape::Situation, field_escape::FieldEscapeMenu, knowledge::EnemyHistory};

#[derive(serde::Deserialize)]
struct Fixture {
    entry: CrtRng,
    players: Vec<PlayerCharacter>,
    levels: Vec<u32>,
    enemies: Vec<String>,
    current: Vec<usize>,
    next: Vec<usize>,
    menu_state: u32,
    draws: u64,
}

#[test]
fn retail_preemptive_does_not_add_a_step_ahead_turn() {
    let f: Fixture = serde_json::from_str(include_str!("../data/source/live-crackridge-preemptive-20261010.json")).unwrap();
    let enemies: Vec<_> = f.enemies.iter().map(String::as_str).collect();
    let menu = FieldEscapeMenu::with_step_ahead(f.entry, Situation::Preemptive,
        &f.players, &f.levels, &enemies, &EnemyHistory::new()).unwrap();
    assert_eq!(menu.current_order, f.current);
    // The two identical Moles are indistinguishable in the reported timeline.
    let normalize = |rows: &[usize]| rows.iter().map(|&id| if id == 5 {4} else {id}).collect::<Vec<_>>();
    assert_eq!(normalize(&menu.next_order), normalize(&f.next));
    assert_eq!(menu.rng, CrtRng {state:f.menu_state, draws:f.entry.draws+f.draws});
    assert!(!menu.trace.iter().any(|s| s.reason == "step_ahead_current_order_shuffle"));
    let preview = menu.trace.iter().find(|s| s.reason == "next_order_shuffle").unwrap();
    assert_eq!(preview.after.draws-preview.before.draws,13);
    // Same-turn Defend remains safe, but never permit crossing into enemy turns.
    let mut at = menu;
    for _ in 0..3 {at = at.safe_step_ahead_defend().unwrap().unwrap().0;}
    assert!(at.safe_step_ahead_defend().unwrap().is_none());
}

#[test]
fn all_octopuff_variants_have_source_cat_fixed_damage_modifier() {
    let data: serde_json::Value = serde_json::from_str(include_str!("../data/field_break.json")).unwrap();
    for number in (510..=600).step_by(10) {
        let id = format!("ENE_NML_WTR_{number}");
        let enemy = &data["enemies"][&id];
        assert_eq!(enemy["combat"]["use_cat_damage_type"],true,"{id}");
        assert_eq!(combat::elemental_fixed_damage(1100,false,false,true),11);
    }
}
