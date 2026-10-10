use ot2_battle_engine::{routine::ThroneOpeningRoutine, *};

fn opening(seed: u32, trace: bool) -> Battle {
    let mut config = BattleConfig::from_json(include_str!("../data/throne_opening.json")).unwrap();
    config.seed = seed;
    Battle::new_opening(config, ItemCatalog::builtin().unwrap(), trace).unwrap()
}

#[test]
fn supplied_routine_wins_across_shuffled_orders_and_defends_the_booster() {
    let config = BattleConfig::from_json(include_str!("../data/throne_opening.json")).unwrap();
    let mut defended = 0;
    let mut enemy_actions = 0;
    let mut orders = std::collections::BTreeSet::new();
    for seed in 0..1024 {
        let mut c = config.clone();
        c.seed = seed;
        let mut battle = Battle::new_opening(c, ItemCatalog::builtin().unwrap(), false).unwrap();
        let mut routine = ThroneOpeningRoutine::default();
        let mut replay = battle.clone();
        let mut replay_routine = routine.clone();
        for _ in 0..16 {
            if battle.state().outcome != Outcome::Ongoing {
                break;
            }
            let round = battle.state().round;
            let party_index = routine.first_round.len();
            let second_index = routine.second_round_attackers.len();
            let result = routine
                .step(&mut battle)
                .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            assert_eq!(result, replay_routine.step(&mut replay).unwrap());
            enemy_actions += usize::from(battle.side(result.actor) == Some(Side::Enemy));
            if battle.side(result.actor) == Some(Side::Player) {
                let defending = round == 2 && result.actor == routine.first_round[0];
                let expected_bp = if round == 1 {
                    u8::from(party_index == 0)
                } else if defending {
                    0
                } else {
                    2
                };
                let expected_target = battle
                    .actor_id(
                        if (round == 1 && party_index == 0) || (round == 2 && second_index == 0) {
                            "pursuer_a"
                        } else {
                            "pursuer_b"
                        },
                    )
                    .unwrap();
                let hits: Vec<_> = result
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        Event::Damage { target, .. } | Event::Miss { target, .. } => Some(*target),
                        _ => None,
                    })
                    .collect();
                if defending {
                    assert!(hits.is_empty());
                    assert!(result.events.iter().any(
                        |event| matches!(event, Event::Action { name, .. } if name == "defend")
                    ));
                } else {
                    assert_eq!(
                        hits,
                        vec![expected_target; usize::from(expected_bp) + 1],
                        "seed {seed}, round {round}"
                    );
                    assert!(result.events.iter().any(|event| matches!(event, Event::Action { name, .. } if name == battle.default_attack(result.actor).unwrap())));
                }
                let spent: u32 = result
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        Event::ResourceChanged {
                            actor,
                            resource,
                            before,
                            after,
                        } if *actor == result.actor
                            && *resource == Resource::Bp
                            && before > after =>
                        {
                            Some(before - after)
                        }
                        _ => None,
                    })
                    .sum();
                assert_eq!(spent, u32::from(expected_bp));
            }
        }
        assert_eq!(battle.state().outcome, Outcome::Victory, "seed {seed}");
        assert_eq!(battle.state().round, 2);
        assert_eq!(routine.first_round.len(), 3);
        assert_eq!(routine.second_round_attackers.len(), 2);
        assert!(
            !routine
                .second_round_attackers
                .contains(&routine.first_round[0])
        );
        assert_eq!(battle.state(), replay.state());
        orders.insert(routine.first_round.clone());
        defended += u32::from(routine.second_round_defended);
    }
    assert_eq!(orders.len(), 6);
    assert!(defended > 0 && defended < 1024);
    assert!(enemy_actions > 0);
}

#[test]
fn trace_covers_every_draw_and_tracing_does_not_change_results() {
    let mut traced = opening(123, true);
    let mut fast = opening(123, false);
    let mut a = ThroneOpeningRoutine::default();
    let mut b = a.clone();
    let mut attacks = Vec::new();
    while traced.state().outcome == Outcome::Ongoing {
        let report = a.step(&mut traced).unwrap();
        assert_eq!(report, b.step(&mut fast).unwrap());
        attacks.push(report);
    }
    let crt = traced.state().opening_crt.as_ref().unwrap();
    let mut expected = CrtRng::new(123);
    for span in crt.trace.as_ref().unwrap() {
        assert_eq!(span.before, expected, "gap before {}", span.reason);
        while expected.draws < span.after.draws {
            expected.next_u15();
        }
        assert_eq!(span.after, expected, "{}", span.reason);
    }
    assert_eq!(expected, traced.state().rng);
    // Fixture describes this simulator/source projection, not a live-game trace.
    assert_eq!(
        expected,
        CrtRng {
            state: 3_217_448_444,
            draws: 163
        }
    );
    let after = crt.after_battle.as_ref().unwrap();
    assert_eq!(
        after.rng_after_rewards.draws - after.rng_before_rewards.draws,
        2
    );
    assert_eq!(
        after.rng_after_field_reset.draws - after.rng_after_rewards.draws,
        2
    );
    assert_eq!(
        after.rng_after_post_battle_event.draws - after.rng_after_field_reset.draws,
        2
    );
    assert_eq!(after.encounter_steps, 639);
    assert_eq!(
        after,
        fast.state()
            .opening_crt
            .as_ref()
            .unwrap()
            .after_battle
            .as_ref()
            .unwrap()
    );
    // Throné boosts first and Defends in round two; after that, Pirro's
    // first second-round attack still resolves all three hits.
    let killing_action = &attacks[5];
    assert_eq!(
        killing_action
            .events
            .iter()
            .filter(|e| matches!(e, Event::Damage { .. }))
            .count(),
        3
    );
    assert_eq!(
        killing_action
            .events
            .iter()
            .filter(|e| matches!(e, Event::KnockedOut { .. }))
            .count(),
        1
    );
    assert_eq!(traced.restarted(123).state(), opening(123, true).state());
}

#[test]
fn night_talent_moves_first_order_but_preserves_cached_preview_speed() {
    let mut config = BattleConfig::from_json(include_str!("../data/throne_opening.json")).unwrap();
    config.night_buff_turns = 0;
    let unbuffed = Battle::new(config.clone(), ItemCatalog::builtin().unwrap()).unwrap();
    let original = serde_json::to_value(unbuffed.state()).unwrap()["queue"].clone();
    // Nine initialization calls, eight shuffle calls: source rows sort before
    // the talent applies. This order is deliberately not a sort by buffed AGI.
    assert_eq!(original, serde_json::json!([0, 3, 4, 1, 2]));
    config.night_buff_turns = 3;
    let buffed = Battle::new(config, ItemCatalog::builtin().unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(buffed.state()).unwrap()["queue"],
        serde_json::json!([0, 3, 1, 2, 4])
    );
    assert_eq!(unbuffed.state().next_queue, buffed.state().next_queue);
    assert_eq!(unbuffed.state().rng, buffed.state().rng); // direct talent costs no CRT
    for actor in &buffed.state().actors[..3] {
        assert!(actor.statuses.iter().all(|s| s.turns == 3));
        assert_eq!(actor.statuses.len(), 3);
    }
}

#[test]
fn normal_opening_promotes_first_shuffled_player_without_drawing_rng() {
    let mut config = BattleConfig::from_json(include_str!("../data/throne_opening.json")).unwrap();
    config.seed = 0x9CE1_BC81;
    config.night_buff_turns = 0;
    let battle = Battle::new_opening(config, ItemCatalog::builtin().unwrap(), true).unwrap();
    // Source sort/shuffle produces [4, 3, 0, 2, 1]. SetupFirstTurnPlayer
    // removes the first player and inserts it at zero, preserving other rows.
    assert_eq!(
        serde_json::to_value(battle.state()).unwrap()["queue"],
        serde_json::json!([0, 4, 3, 2, 1])
    );
    assert_eq!(battle.current_actor(), Some(ActorId(0)));
    let trace = battle
        .state()
        .opening_crt
        .as_ref()
        .unwrap()
        .trace
        .as_ref()
        .unwrap();
    let shuffled = trace
        .iter()
        .find(|s| s.reason == "current_order_shuffle")
        .unwrap();
    let speaker = trace
        .iter()
        .find(|s| s.reason == "battle_start_speaker")
        .unwrap();
    assert_eq!(shuffled.after.draws, 17);
    assert_eq!(speaker.before, shuffled.after);
}

#[test]
fn routine_and_crt_state_roll_back_rejected_actions() {
    let mut battle = opening(123, true);
    let before = battle.state().clone();
    assert!(battle.step_player(Command::Wait).is_err());
    assert!(
        battle
            .step_player(Command::Skill {
                id: "attack_sword".into(),
                target: Some(ActorId(3))
            })
            .is_err()
    );
    assert!(
        battle
            .step_player(Command::Skill {
                id: "ABI_SET_THI_020".into(),
                target: Some(ActorId(3))
            })
            .is_err()
    );
    assert_eq!(battle.state(), &before);
    let mut routine = ThroneOpeningRoutine {
        first_round: vec![ActorId(0)],
        ..Default::default()
    };
    let policy_before = routine.clone();
    assert!(routine.step(&mut battle).is_err());
    assert_eq!(routine, policy_before);
    assert_eq!(battle.state(), &before);
}

#[test]
fn defend_preserves_captured_guard_cost_and_next_actor_voice_cost() {
    for seed in 0..64 {
        let mut battle = opening(seed, true);
        let mut routine = ThroneOpeningRoutine::default();
        while battle.state().outcome == Outcome::Ongoing {
            let result = routine.step(&mut battle).unwrap();
            if result
                .events
                .iter()
                .any(|e| matches!(e, Event::Action { name, .. } if name == "defend"))
            {
                let trace = battle
                    .state()
                    .opening_crt
                    .as_ref()
                    .unwrap()
                    .trace
                    .as_ref()
                    .unwrap();
                let guard = trace
                    .iter()
                    .rev()
                    .find(|s| s.reason == "guard_disease")
                    .unwrap();
                assert_eq!(guard.after.draws - guard.before.draws, 21);
                let next_cost = if battle.current_actor() == Some(ActorId(0)) {
                    17
                } else {
                    3
                };
                assert_eq!(
                    result.rng_after.draws - result.rng_before.draws,
                    22 + next_cost
                );
                return;
            }
        }
    }
    panic!("no Defend branch exercised");
}
