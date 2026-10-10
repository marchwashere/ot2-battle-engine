use ot2_battle_engine::{
    combat::{Element, Weapon},
    routine::ThroneSecondRoutine,
    *,
};

fn make(seed: u32, trace: bool) -> Battle {
    Battle::new_throne_second(
        BattleConfig::throne_second(seed).unwrap(),
        ItemCatalog::builtin().unwrap(),
        trace,
    )
    .unwrap()
}

#[test]
fn imported_formation_and_loadout_match_second_sewer_records() {
    let c = BattleConfig::throne_second(0).unwrap();
    assert_eq!(
        c.enemies
            .iter()
            .map(|e| (
                e.id.as_str(),
                e.stats.max_hp,
                e.combat.as_ref().unwrap().shields
            ))
            .collect::<Vec<_>>(),
        [
            ("grunt_right", 185, 1),
            ("leader", 375, 3),
            ("grunt_left", 185, 1)
        ]
    );
    let leader = c.enemies[1].combat.as_ref().unwrap();
    assert_eq!(leader.weapon_weaknesses, [Weapon::Axe]);
    assert_eq!(leader.element_weaknesses, [Element::Dark]);
    assert_eq!(c.players[1].combat.as_ref().unwrap().accuracy, 89);
    assert!(c.players[1].skills.contains(&"ABI_SET_GST_0190".into()));
    assert_eq!(c.night_buff_turns, 0);
}

#[test]
fn user_routine_wins_with_fixed_targets_and_costs_across_orders() {
    let base = make(0, false);
    for seed in 0..4096 {
        let mut battle = base.restarted(seed);
        let mut routine = ThroneSecondRoutine;
        let mut party_actions = 0;
        while battle.state().outcome == Outcome::Ongoing {
            let actor = battle.current_actor().unwrap();
            let round = battle.state().round;
            let player = battle.side(actor) == Some(Side::Player);
            if player {
                let (skill, target, bp) = match routine.command(&battle).unwrap() {
                    Command::Boost { id, target, bp } => (id, target.unwrap(), bp),
                    _ => panic!("expected skill"),
                };
                let expected = match (round, actor.0) {
                    (1, 0) => ("ABI_SET_THI_020", "grunt_right", 0),
                    (1, 1) => ("ABI_SET_GST_0190", "leader", 0),
                    (1, 2) => ("attack_axe", "grunt_left", 0),
                    (2, 0) => ("ABI_SET_THI_020", "leader", 2),
                    (2, 1) => ("ABI_SET_GST_0170", "grunt_left", 2),
                    (2, 2) => ("attack_axe", "grunt_right", 2),
                    _ => panic!("unexpected actor/round at seed {seed}"),
                };
                assert_eq!(
                    (skill.as_str(), target, bp),
                    (expected.0, battle.actor_id(expected.1).unwrap(), expected.2)
                );
                party_actions += 1;
            }
            let report = routine.step(&mut battle).unwrap();
            if player && actor.0 == 1 && round == 1 {
                assert!(report.events.iter().any(|e| matches!(
                    e,
                    Event::Damage {
                        amount: 262,
                        critical: false,
                        weak: false,
                        ..
                    }
                )));
                assert_eq!(battle.state().actors[4].resources.hp, 113);
                assert_eq!(battle.state().actors[4].shields, 3);
            }
        }
        assert_eq!(battle.state().outcome, Outcome::Victory, "seed {seed}");
        assert_eq!(battle.state().round, 2);
        assert_eq!(party_actions, 6);
        for (i, sp) in [26, 22, 35].into_iter().enumerate() {
            assert_eq!(battle.state().actors[i].resources.sp, sp);
            assert_eq!(battle.state().actors[i].resources.bp, 0);
        }
        assert_eq!(battle.item_count("inspiriting_plum").unwrap(), 1);
    }
}

#[test]
fn second_trace_preserves_all_draws_and_does_not_apply_field_resets() {
    let mut traced = make(123, true);
    let mut plain = make(123, false);
    let mut routine = ThroneSecondRoutine;
    while traced.state().outcome == Outcome::Ongoing {
        assert_eq!(
            routine.step(&mut traced).unwrap(),
            routine.step(&mut plain).unwrap()
        );
    }
    let crt = traced.state().opening_crt.as_ref().unwrap();
    let mut rng = CrtRng::new(123);
    for span in crt.trace.as_ref().unwrap() {
        assert_eq!(span.before, rng, "{}", span.reason);
        rng.advance(span.after.draws - span.before.draws);
        assert_eq!(span.after, rng);
    }
    assert_eq!(rng, traced.state().rng);
    assert!(crt.after_battle.is_none());
    assert_eq!(crt.rewards.as_ref().unwrap().after, rng);
    assert_eq!(
        crt.trace
            .as_ref()
            .unwrap()
            .iter()
            .filter(|s| s.reason == "ability_voice")
            .count(),
        4
    );
    let ratio = crt
        .trace
        .as_ref()
        .unwrap()
        .iter()
        .find(|s| s.reason == "hp_ratio_accuracy")
        .unwrap();
    assert_eq!(ratio.after.draws - ratio.before.draws, 1);
}

#[test]
fn incomplete_routine_or_invalid_command_is_not_reported_as_a_win() {
    let c = BattleConfig::throne_second_at_level(123, 1).unwrap();
    assert_eq!(c.players[1].combat.as_ref().unwrap().accuracy, 88);
    assert!(BattleConfig::throne_second_at_level(123, 3).is_err());
    let mut battle = make(123, true);
    let before = battle.state().clone();
    assert!(
        battle
            .step_player(Command::Boost {
                id: "ABI_SET_THI_020".into(),
                target: battle.actor_id("leader"),
                bp: 3
            })
            .is_err()
    );
    assert_eq!(*battle.state(), before);
}
