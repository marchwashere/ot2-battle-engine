use ot2_battle_engine::{
    CrtRng, PlayerCharacter, escape::{Situation, success_percent},
    field_escape::FieldEscapeMenu, knowledge::EnemyHistory,
};

#[derive(serde::Deserialize)]
struct Fixture {
    entry: CrtRng,
    players: Vec<PlayerCharacter>,
    levels: Vec<u32>,
    enemies: Vec<String>,
    enemy_knowledge: EnemyHistory,
    danger: u32,
    defend_actors: Vec<usize>,
    menu: CrtRng,
    escape_actor: usize,
}
fn fixture() -> Fixture {
    serde_json::from_str(include_str!("../data/source/southern-escape-1965a5f4.json")).unwrap()
}
fn start(f: &Fixture, situation: Situation, levels: &[u32]) -> FieldEscapeMenu {
    let enemies: Vec<_> = f.enemies.iter().map(String::as_str).collect();
    FieldEscapeMenu::with_step_ahead(
        f.entry, situation, &f.players, levels, &enemies, &f.enemy_knowledge,
    ).unwrap()
}

#[test]
fn reported_southern_escape_fails_after_seven_defends() {
    let f = fixture();
    let mut menu = start(&f, Situation::Preemptive, &f.levels);
    for &actor in &f.defend_actors {
        assert_eq!(menu.first_actor, actor);
        menu = menu.safe_step_ahead_defend_through_next_round().unwrap().unwrap().0;
    }
    assert_eq!(menu.round(), 2);
    assert_eq!(menu.first_actor, f.escape_actor);
    assert_eq!(menu.rng, f.menu);
    let attempt = menu.clone().attempt(&[], f.danger).unwrap();
    assert_eq!(attempt.chance_percent, 40);
    assert!(!attempt.success);
    assert_eq!(attempt.after.draws, f.menu.draws + 1);
    assert_eq!(attempt.after.state, 3490800007);

    // Two complete +1/cancel cycles reach a successful roll; each candidate
    // must be checked rather than accepting an unconditional preemptive escape.
    let adjusted = menu.attempt(&[1, 0, 1, 0], f.danger).unwrap();
    assert_eq!(adjusted.chance_percent, 40);
    assert!(adjusted.success);
    assert_eq!(adjusted.after.draws, f.menu.draws + 3);
}

#[test]
fn preemptive_bypass_expires_after_first_turn_even_for_overlevelled_parties() {
    let f = fixture();
    for situation in [Situation::Normal, Situation::Preemptive] {
        for levels in [&f.levels[..], &[70, 70, 70, 70][..]] {
            let mut menu = start(&f, situation, levels);
            for defended in 0..=7 {
                assert_eq!(menu.round(), if defended < 4 {1} else {2});
                let bypass = situation == Situation::Preemptive && defended < 4;
                let mut expected = menu.rng;
                let chance = if bypass {100} else {success_percent(levels, f.danger, 0, 0).unwrap()};
                let success = bypass || expected.weighted_bool(chance as f32 * 0.01);
                let attempt = menu.clone().attempt(&[], f.danger).unwrap();
                assert_eq!(attempt.chance_percent, chance);
                assert_eq!(attempt.success, success);
                assert_eq!(attempt.after, expected);
                assert_eq!(attempt.after.draws - menu.rng.draws, u64::from(!bypass));
                if defended < 7 {
                    menu = menu.safe_step_ahead_defend_through_next_round().unwrap().unwrap().0;
                }
            }
        }
    }
}
