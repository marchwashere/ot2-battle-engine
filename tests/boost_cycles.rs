use ot2_battle_engine::{routine::ThroneOpeningRoutine, *};

#[test]
fn full_boost_path_replaces_automatic_inputs_and_rejects_invalid_endpoints() {
    let mut config = BattleConfig::throne_opening(0).unwrap();
    for player in &mut config.players {
        player.resources.as_mut().unwrap().bp = 3;
    }
    let mut battle = Battle::new_opening(config, ItemCatalog::builtin().unwrap(), false).unwrap();
    let actor = battle.current_actor().unwrap();
    let command = Command::Boost {
        id: battle.default_attack(actor).unwrap().into(),
        target: battle.actor_id("pursuer_a"),
        bp: 2,
    };
    let before = battle.state().clone();
    for invalid in [&[1, 2, 3][..], &[2][..], &[1, 2, 3, 4, 3, 2][..]] {
        assert!(
            battle
                .step_player_with_boost_inputs(command.clone(), invalid)
                .is_err()
        );
        assert_eq!(battle.state(), &before);
    }
    let mut reference = battle.clone();
    reference.boost_sequence(&[1, 2, 1, 0]).unwrap(); // +3, then the ordinary +3.
    reference.step_player(command.clone()).unwrap();
    let report = battle
        .step_player_with_boost_inputs(command, &[1, 2, 3, 2])
        .unwrap(); // +6 total.
    assert_eq!(report.rng_before, before.rng);
    assert_eq!(battle.state(), reference.state());
}

#[test]
fn level_two_draws_twice_and_reductions_draw_nothing() {
    let mut battle = Battle::new_opening(
        BattleConfig::throne_opening(0).unwrap(),
        ItemCatalog::builtin().unwrap(),
        true,
    )
    .unwrap();
    let before = battle.state().clone();
    assert!(battle.boost_sequence(&[1, 2, 1, 0]).is_err()); // only 1 BP
    assert_eq!(battle.state(), &before);
    let mut routine = ThroneOpeningRoutine::default();
    while battle.state().round < 2
        || battle.state().actors[battle.current_actor().unwrap().0]
            .resources
            .bp
            < 2
    {
        routine.step(&mut battle).unwrap();
    }
    let before = battle.state().clone();
    for invalid in [
        &[2, 0][..],
        &[1, 2][..],
        &[1, 2, 0][..],
        &[1, 2, 3, 2, 1, 0][..],
    ] {
        assert!(battle.boost_sequence(invalid).is_err());
        assert_eq!(battle.state(), &before);
    }
    battle.boost_sequence(&[1, 2, 1, 2, 1, 0]).unwrap();
    let mut expected = before.rng;
    expected.advance(5);
    assert_eq!(battle.state().rng, expected);
    assert_eq!(battle.state().actors, before.actors);
    let trace = battle
        .state()
        .opening_crt
        .as_ref()
        .unwrap()
        .trace
        .as_ref()
        .unwrap();
    let tail = &trace[before.opening_crt.unwrap().trace.unwrap().len()..];
    assert_eq!(
        tail.iter()
            .map(|s| s.after.draws - s.before.draws)
            .collect::<Vec<_>>(),
        [1, 2, 0, 2, 0, 0]
    );
}

#[test]
fn boost_cancel_adds_one_draw_without_spending_bp_or_advancing_the_turn() {
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..16 {
        let mut battle = Battle::new_opening(
            BattleConfig::throne_opening(seed).unwrap(),
            ItemCatalog::builtin().unwrap(),
            true,
        )
        .unwrap();
        let mut routine = ThroneOpeningRoutine::default();
        while battle.state().outcome == Outcome::Ongoing {
            let actor = battle.current_actor().unwrap();
            let before = battle.state().clone();
            if battle.side(actor) == Some(Side::Player) && before.actors[actor.0].resources.bp > 0 {
                seen.insert(actor.0);
                let mut expected_rng = before.rng;
                for _ in 0..8 {
                    battle.boost_cancel_cycle().unwrap();
                    expected_rng.next_u15();
                    assert_eq!(battle.state().rng, expected_rng);
                    assert_eq!(battle.state().actors, before.actors);
                    assert_eq!(battle.current_actor(), Some(actor));
                    assert_eq!(battle.state().round, before.round);
                    assert_eq!(battle.state().next_queue, before.next_queue);
                    assert_eq!(
                        battle.state().opening_crt.as_ref().unwrap().used_voices,
                        before.opening_crt.as_ref().unwrap().used_voices
                    );
                }
            } else {
                assert!(battle.boost_cancel_cycle().is_err());
                assert_eq!(battle.state(), &before);
            }
            routine.step(&mut battle).unwrap();
        }
        let before = battle.state().clone();
        assert!(battle.boost_cancel_cycle().is_err());
        assert_eq!(battle.state(), &before);
    }
    assert_eq!(seen.len(), 3);
}
